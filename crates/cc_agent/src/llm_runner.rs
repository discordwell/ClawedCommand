//! Background async task that processes AgentRequests by calling an LLM.
//!
//! Receives requests from the AgentBridge channel, calls the LLM via
//! OpenAiCompatibleClient, parses tool calls, executes them through
//! mcp_tools, and sends AgentResponses back.

use bevy::prelude::*;

use crate::agent_bridge::{AgentChannels, AgentRequest, AgentResponse, AgentSource, TokenChunk};
use crate::llm_client::{
    AgentStatus, ChatMessage, LlmBackend, LlmClient, LlmConfig, LlmResponse, MockLlmClient,
    OpenAiCompatibleClient,
};
use crate::mcp_tools;
use crate::snapshot;
#[cfg(test)]
use crate::snapshot::GameStateSnapshot;

#[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::Sender;

#[cfg(target_arch = "wasm32")]
use async_channel::Sender;

/// Resource holding the channel endpoints for the LLM background thread.
/// Consumed by `startup_llm_runner` on first frame.
#[derive(Resource)]
pub struct LlmRunnerChannels(pub Option<AgentChannels>);

/// Maximum tool-call rounds per request to prevent runaway loops.
const MAX_TOOL_ROUNDS: usize = 5;

/// System prompt from training data — used when `finetuned_lua` is enabled
/// and the request is from ConstructMode. The fine-tuned model generates raw
/// Lua directly (no tool calls, no fenced blocks).
const FINETUNE_CONSTRUCT_SYSTEM_PROMPT: &str =
    include_str!("../../../training/data/system_prompt.txt");

/// System prompt for game-loop AI decisions.
const GAME_LOOP_SYSTEM_PROMPT: &str = r#"You are Le Chat, an AI commander for the catGPT faction in the RTS game ClawedCommand. You control an army of cats in a post-singularity world.

Analyze the game state and issue tool calls to manage your army effectively. Priorities:
1. Gather resources (food, GPU cores) to fund your army
2. Train units and build structures for defense and offense
3. Scout for enemies and respond to threats
4. Attack when you have a decisive advantage

Be efficient — issue only necessary commands. You have limited GPU budget per decision cycle."#;

/// Build the LLM client from config.
fn build_client(config: &LlmConfig) -> Box<dyn LlmClient> {
    match config.backend {
        LlmBackend::OpenAiCompatible | LlmBackend::Anthropic => {
            Box::new(OpenAiCompatibleClient::new(
                config.base_url.clone(),
                config.api_key.clone(),
                config.model.clone(),
                config.temperature,
            ))
        }
        LlmBackend::Fallback => {
            // On native, fallback just uses the configured OpenAI-compatible client.
            Box::new(OpenAiCompatibleClient::new(
                config.base_url.clone(),
                config.api_key.clone(),
                config.model.clone(),
                config.temperature,
            ))
        }
        LlmBackend::Mock => Box::new(MockLlmClient::new(vec![LlmResponse {
            content: "No action needed at this time.".to_string(),
            tool_calls: vec![],
        }])),
    }
}

/// Process a single request: call LLM, execute tool calls, return response.
/// When `streaming_client` is Some and the request is Prompt/ConstructMode (no tools),
/// uses SSE streaming to send tokens progressively to the UI via `token_tx`.
async fn process_request(
    client: &dyn LlmClient,
    request: &AgentRequest,
    config: &LlmConfig,
    streaming_client: Option<&OpenAiCompatibleClient>,
    token_tx: &Sender<TokenChunk>,
) -> AgentResponse {
    let snapshot = request.snapshot.as_ref();

    // Build message list
    let mut messages = Vec::new();

    match request.source {
        AgentSource::ConstructMode | AgentSource::Prompt => {
            // Both ConstructMode and Prompt use the fine-tuned Lua generation path
            if config.finetuned_lua {
                let has_finetune_prompt = request.chat_history.as_ref().is_some_and(|h| {
                    h.iter().any(|m| {
                        m.role == "system" && m.content == FINETUNE_CONSTRUCT_SYSTEM_PROMPT
                    })
                });
                if !has_finetune_prompt {
                    messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: FINETUNE_CONSTRUCT_SYSTEM_PROMPT.to_string(),
                    });
                }
            } else if request.source == AgentSource::Prompt {
                // Fallback: base model needs a system prompt for Prompt source
                log::warn!("finetuned_lua not enabled — Prompt using base model system prompt");
                messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: FINETUNE_CONSTRUCT_SYSTEM_PROMPT.to_string(),
                });
            }
            // Use the full chat history from the request
            if let Some(history) = &request.chat_history {
                messages.extend(history.iter().cloned());
            } else {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: request.prompt.clone(),
                });
            }
        }
        AgentSource::GameLoop | AgentSource::QuickCommand => {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: GAME_LOOP_SYSTEM_PROMPT.to_string(),
            });
            if let Some(snap) = snapshot {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!(
                        "{}\n\n{}",
                        snapshot::summarize_snapshot(snap),
                        request.prompt
                    ),
                });
            } else {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: request.prompt.clone(),
                });
            }
        }
    }

    // Fine-tuned models generate Lua directly — no tool calls needed
    let use_tools = !(config.finetuned_lua
        && matches!(
            request.source,
            AgentSource::ConstructMode | AgentSource::Prompt
        ));

    // Get tool definitions for this tier
    let tool_defs = if use_tools {
        mcp_tools::tool_definitions(request.tier)
    } else {
        vec![]
    };
    let tools = if tool_defs.is_empty() {
        None
    } else {
        Some(tool_defs.as_slice())
    };

    let mut all_commands = Vec::new();
    let mut final_content = String::new();

    // Use streaming for Prompt/ConstructMode when no tool calls and we have a streaming client
    let use_streaming = !use_tools
        && streaming_client.is_some()
        && matches!(
            request.source,
            AgentSource::Prompt | AgentSource::ConstructMode
        );

    if use_streaming {
        let sc = streaming_client.unwrap();
        match sc
            .stream_complete(&messages, token_tx, request.source)
            .await
        {
            Ok(response) => {
                return AgentResponse {
                    content: response.content,
                    commands: vec![],
                    error: None,
                    source: request.source,
                    player_id: request.player_id,
                };
            }
            Err(e) => {
                return AgentResponse {
                    content: String::new(),
                    commands: vec![],
                    error: Some(e.to_string()),
                    source: request.source,
                    player_id: request.player_id,
                };
            }
        }
    }

    // Multi-turn tool calling loop (non-streaming path)
    for _round in 0..MAX_TOOL_ROUNDS {
        let result = client.complete(&messages, tools).await;

        match result {
            Ok(response) => {
                if !response.content.is_empty() {
                    final_content = response.content.clone();
                }

                if response.tool_calls.is_empty() {
                    // No more tool calls — done
                    break;
                }

                // Add assistant message with tool calls
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content.clone(),
                });

                // Execute each tool call
                for call in &response.tool_calls {
                    let (result_json, commands) = mcp_tools::execute_tool(
                        &call.name,
                        &call.arguments,
                        request.player_id,
                        snapshot,
                        request.tier,
                    );
                    all_commands.extend(commands);

                    // Add tool result as message for next round
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: serde_json::to_string(&serde_json::json!({
                            "tool_call_id": call.id,
                            "name": call.name,
                            "result": result_json,
                        }))
                        .unwrap_or_default(),
                    });
                }
            }
            Err(e) => {
                return AgentResponse {
                    content: String::new(),
                    commands: all_commands,
                    error: Some(e.to_string()),
                    source: request.source,
                    player_id: request.player_id,
                };
            }
        }
    }

    AgentResponse {
        content: final_content,
        commands: all_commands,
        error: None,
        source: request.source,
        player_id: request.player_id,
    }
}

/// Spawn the LLM runner as a background tokio task.
/// Returns the tokio JoinHandle (can be ignored — runs until channel closes).
pub fn spawn_llm_runner(config: LlmConfig, channels: AgentChannels) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for LLM runner");

        rt.block_on(async move {
            let AgentChannels {
                request_rx,
                response_tx,
                token_tx,
            } = channels;

            // Build a single concrete client and derive both the trait object and
            // the streaming reference from it, avoiding duplicate reqwest::Client instances.
            match config.backend {
                LlmBackend::OpenAiCompatible | LlmBackend::Anthropic | LlmBackend::Fallback => {
                    let client = OpenAiCompatibleClient::new(
                        config.base_url.clone(),
                        config.api_key.clone(),
                        config.model.clone(),
                        config.temperature,
                    );
                    run_loop(
                        &client,
                        Some(&client),
                        &config,
                        request_rx,
                        response_tx,
                        token_tx,
                    )
                    .await;
                }
                _ => {
                    let client = build_client(&config);
                    run_loop(
                        client.as_ref(),
                        None,
                        &config,
                        request_rx,
                        response_tx,
                        token_tx,
                    )
                    .await;
                }
            }
        });
    })
}

/// Inner loop: receive requests, process, send responses.
async fn run_loop(
    client: &dyn LlmClient,
    streaming_client: Option<&OpenAiCompatibleClient>,
    config: &LlmConfig,
    request_rx: crossbeam_channel::Receiver<AgentRequest>,
    response_tx: crossbeam_channel::Sender<AgentResponse>,
    token_tx: Sender<TokenChunk>,
) {
    while let Ok(request) = request_rx.recv() {
        let response = process_request(client, &request, config, streaming_client, &token_tx).await;

        if response_tx.send(response).is_err() {
            break;
        }
    }
}

/// Bevy Startup system: takes the channels from `LlmRunnerChannels`,
/// reads `LlmConfig`, and spawns the background LLM thread.
pub fn startup_llm_runner(
    config: Res<LlmConfig>,
    mut channels_res: ResMut<LlmRunnerChannels>,
    mut agent_status: ResMut<AgentStatus>,
) {
    if let Some(channels) = channels_res.0.take() {
        let config = config.clone();
        spawn_llm_runner(config, channels);
        *agent_status = AgentStatus::Ready;
        log::info!("LLM runner thread spawned");
    } else {
        log::warn!("LlmRunnerChannels already consumed or missing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::summarize_snapshot;
    use crate::tool_tier::ToolTier;

    #[test]
    fn summarize_snapshot_basic() {
        use cc_sim::resources::PlayerResourceState;

        let snap = GameStateSnapshot {
            tick: 100,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 500,
                gpu_cores: 100,
                nfts: 0,
                supply: 5,
                supply_cap: 20,
                ..Default::default()
            },
        };

        let summary = summarize_snapshot(&snap);
        assert!(summary.contains("Tick: 100"));
        assert!(summary.contains("Food=500"));
        assert!(summary.contains("Supply=5/20"));
        assert!(summary.contains("My units: none"));
    }

    #[test]
    fn build_client_mock() {
        let config = LlmConfig::default();
        let client = build_client(&config);
        assert_eq!(client.model_name(), "mock");
    }

    #[tokio::test]
    async fn process_request_mock() {
        let client = MockLlmClient::new(vec![LlmResponse {
            content: "Standing by.".to_string(),
            tool_calls: vec![],
        }]);

        let config = LlmConfig::default();
        let request = AgentRequest {
            player_id: 0,
            prompt: "What should I do?".into(),
            tier: ToolTier::Basic,
            source: AgentSource::GameLoop,
            chat_history: None,
            snapshot: None,
        };

        let (_tx, _rx) = crossbeam_channel::unbounded();
        let response = process_request(&client, &request, &config, None, &_tx).await;
        assert_eq!(response.content, "Standing by.");
        assert!(response.error.is_none());
        assert!(response.commands.is_empty());
        assert_eq!(response.source, AgentSource::GameLoop);
    }

    #[tokio::test]
    async fn process_request_finetuned_construct_injects_system_prompt() {
        use std::sync::Mutex;

        // Capture the messages sent to the mock client
        struct CaptureMockClient {
            captured: Mutex<Vec<Vec<ChatMessage>>>,
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[async_trait::async_trait]
        impl LlmClient for CaptureMockClient {
            async fn complete(
                &self,
                messages: &[ChatMessage],
                _tools: Option<&[crate::llm_client::ToolDef]>,
            ) -> Result<LlmResponse, crate::llm_client::LlmError> {
                self.captured.lock().unwrap().push(messages.to_vec());
                Ok(LlmResponse {
                    content: "-- Intent: gather\nlocal w = ctx:idle_units()\n".to_string(),
                    tool_calls: vec![],
                })
            }
            fn model_name(&self) -> &str {
                "capture-mock"
            }
        }

        let client = CaptureMockClient {
            captured: Mutex::new(vec![]),
        };

        let mut config = LlmConfig::default();
        config.finetuned_lua = true;

        let request = AgentRequest {
            player_id: 0,
            prompt: "send workers to gather".into(),
            tier: ToolTier::Basic,
            source: AgentSource::ConstructMode,
            chat_history: Some(vec![ChatMessage {
                role: "user".to_string(),
                content: "send workers to gather".to_string(),
            }]),
            snapshot: None,
        };

        let (_tx, _rx) = crossbeam_channel::unbounded();
        let response = process_request(&client, &request, &config, None, &_tx).await;
        assert!(response.error.is_none());

        let captured = client.captured.lock().unwrap();
        assert!(!captured.is_empty());
        let messages = &captured[0];
        // First message should be the system prompt
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("ctx API"));
    }

    #[tokio::test]
    async fn process_request_prompt_uses_finetuned_path() {
        use std::sync::Mutex;

        struct CaptureMockClient {
            captured: Mutex<Vec<Vec<ChatMessage>>>,
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[async_trait::async_trait]
        impl LlmClient for CaptureMockClient {
            async fn complete(
                &self,
                messages: &[ChatMessage],
                _tools: Option<&[crate::llm_client::ToolDef]>,
            ) -> Result<LlmResponse, crate::llm_client::LlmError> {
                self.captured.lock().unwrap().push(messages.to_vec());
                Ok(LlmResponse {
                    content: "-- Intent: kite\nlocal h = ctx:my_units('Hisser')\n".to_string(),
                    tool_calls: vec![],
                })
            }
            fn model_name(&self) -> &str {
                "capture-mock"
            }
        }

        let client = CaptureMockClient {
            captured: Mutex::new(vec![]),
        };

        let mut config = LlmConfig::default();
        config.finetuned_lua = true;

        let request = AgentRequest {
            player_id: 0,
            prompt: "kite with hissers".into(),
            tier: ToolTier::Basic,
            source: AgentSource::Prompt,
            chat_history: Some(vec![ChatMessage {
                role: "user".to_string(),
                content: "kite with hissers".to_string(),
            }]),
            snapshot: None,
        };

        let (_tx, _rx) = crossbeam_channel::unbounded();
        let response = process_request(&client, &request, &config, None, &_tx).await;
        assert!(response.error.is_none());
        assert_eq!(response.source, AgentSource::Prompt);

        let captured = client.captured.lock().unwrap();
        assert!(!captured.is_empty());
        let messages = &captured[0];
        // First message should be the fine-tuned system prompt (same as ConstructMode)
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("ctx API"));
    }

    #[test]
    fn spawn_llm_runner_processes_request_and_responds() {
        use crate::agent_bridge::AgentBridge;

        let config = LlmConfig::default(); // Mock backend
        let (bridge, channels) = AgentBridge::new();

        let _handle = spawn_llm_runner(config, channels);

        // Send a request through the bridge
        bridge
            .request_tx
            .try_send(AgentRequest {
                player_id: 0,
                prompt: "Test request".into(),
                tier: ToolTier::Basic,
                source: AgentSource::GameLoop,
                chat_history: None,
                snapshot: None,
            })
            .expect("send should succeed");

        // Should receive a response within a reasonable time
        let response = bridge
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("should receive response from LLM runner");

        assert_eq!(response.player_id, 0);
        assert_eq!(response.source, AgentSource::GameLoop);
        assert!(response.error.is_none());
        assert_eq!(response.content, "No action needed at this time.");
    }

    #[test]
    fn spawn_llm_runner_passes_snapshot_to_process_request() {
        use crate::agent_bridge::AgentBridge;
        use cc_sim::resources::PlayerResourceState;

        let config = LlmConfig::default();
        let (bridge, channels) = AgentBridge::new();

        let _handle = spawn_llm_runner(config, channels);

        let snap = GameStateSnapshot {
            tick: 42,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 200,
                ..Default::default()
            },
        };

        bridge
            .request_tx
            .try_send(AgentRequest {
                player_id: 0,
                prompt: "Assess".into(),
                tier: ToolTier::Basic,
                source: AgentSource::GameLoop,
                chat_history: None,
                snapshot: Some(snap),
            })
            .expect("send should succeed");

        let response = bridge
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("should receive response");

        assert!(response.error.is_none());
    }

    #[test]
    fn prompt_source_routes_to_llm() {
        use crate::agent_bridge::AgentBridge;

        let config = LlmConfig::default(); // Mock backend
        let (bridge, channels) = AgentBridge::new();

        let _handle = spawn_llm_runner(config, channels);

        // Send a Prompt request — routes through same LLM path as ConstructMode
        bridge
            .request_tx
            .try_send(AgentRequest {
                player_id: 0,
                prompt: "make my hissers kite".into(),
                tier: ToolTier::Basic,
                source: AgentSource::Prompt,
                chat_history: None,
                snapshot: None,
            })
            .expect("send should succeed");

        let response = bridge
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("should receive response from Prompt/LLM path");

        assert_eq!(response.source, AgentSource::Prompt);
        assert_eq!(response.player_id, 0);
        assert!(response.error.is_none());
        // Mock returns "No action needed at this time." — verifies it went through LLM not CLI
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn process_request_no_streaming_fallback_for_mock() {
        // When streaming_client is None (mock), Prompt requests fall through to
        // non-streaming complete() — no tokens sent, full response returned.
        let client = MockLlmClient::new(vec![LlmResponse {
            content: "-- Intent: gather\nlocal w = ctx:idle_units()\n".to_string(),
            tool_calls: vec![],
        }]);

        let mut config = LlmConfig::default();
        config.finetuned_lua = true;

        let request = AgentRequest {
            player_id: 0,
            prompt: "gather resources".into(),
            tier: ToolTier::Basic,
            source: AgentSource::Prompt,
            chat_history: Some(vec![ChatMessage {
                role: "user".to_string(),
                content: "gather resources".to_string(),
            }]),
            snapshot: None,
        };

        let (tx, rx) = crossbeam_channel::unbounded();
        // streaming_client = None → falls through to non-streaming path
        let response = process_request(&client, &request, &config, None, &tx).await;
        assert!(response.error.is_none());
        assert!(response.content.contains("ctx:idle_units"));

        // No streaming tokens should have been sent
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn process_request_gameloop_never_streams() {
        // Even with a streaming client available, GameLoop uses non-streaming path
        let client = MockLlmClient::new(vec![LlmResponse {
            content: "Standing by.".to_string(),
            tool_calls: vec![],
        }]);

        let config = LlmConfig::default();
        let request = AgentRequest {
            player_id: 0,
            prompt: "What should I do?".into(),
            tier: ToolTier::Basic,
            source: AgentSource::GameLoop,
            chat_history: None,
            snapshot: None,
        };

        let (tx, rx) = crossbeam_channel::unbounded();
        // Even if we provided a streaming client, GameLoop wouldn't use it
        // (streaming_client=None here, but the condition also checks source)
        let response = process_request(&client, &request, &config, None, &tx).await;
        assert_eq!(response.content, "Standing by.");
        assert!(rx.try_recv().is_err());
    }
}
