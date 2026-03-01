//! Background async task that processes AgentRequests by calling an LLM.
//!
//! Receives requests from the AgentBridge channel, calls the LLM via
//! OpenAiCompatibleClient, parses tool calls, executes them through
//! mcp_tools, and sends AgentResponses back.

use bevy::prelude::*;

use crate::agent_bridge::{AgentChannels, AgentRequest, AgentResponse, AgentSource};
use crate::claude_cli;
use crate::llm_client::{AgentStatus, ChatMessage, LlmClient, LlmConfig, LlmBackend, OpenAiCompatibleClient, MockLlmClient, LlmResponse};
use crate::mcp_tools;
use crate::snapshot;
#[cfg(test)]
use crate::snapshot::GameStateSnapshot;

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

/// System prompt for `/` key prompt overlay — sent to Claude Code CLI.
pub const PROMPT_OVERLAY_SYSTEM_PROMPT: &str = r#"You are Minstral, an AI assistant in the RTS game ClawedCommand. The player is asking you to create a Lua script for army automation.

Generate a Lua script using the ctx API. Available methods:

Queries:
- ctx:my_units(kind?) -> [{id, kind, x, y, idle, moving, attacking, gathering, hp, hp_max, speed, damage, range, owner}]
- ctx:enemy_units() -> same format
- ctx:my_buildings(kind?) -> [{id, kind, x, y, under_construction}]
- ctx:enemy_buildings() -> same format
- ctx:get_resources() -> {food, gpu_cores, nfts, supply, supply_cap}
- ctx:resource_deposits() -> [{id, x, y, remaining, kind}]
- ctx:movement_cost(x, y) -> f64 or nil

Commands:
- ctx:move_units(ids_table, x, y)
- ctx:attack(ids_table, target_id)
- ctx:attack_move(ids_table, x, y)
- ctx:stop(ids_table)
- ctx:hold(ids_table)
- ctx:gather(ids_table, deposit_id)
- ctx:build(builder_id, building_type_string, x, y)
- ctx:train(building_id, unit_type_string)

Unit kinds: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, Catnapper, FerretSapper, MechCommander
Building kinds: TheBox, CatTree, FishMarket, ServerRack, ScratchingPost, LitterBox, CatFlap, LaserPointer

Proven patterns:
- GROUP focus fire: calculate army centroid, find closest enemy to centroid, all units attack same target
- Conditional kite: ranged units retreat when outnumbered, perpendicular movement when path blocked
- Retreat wounded: pull back units below 30% HP when outnumbered
- Push to HQ: move toward enemy base when army advantage >= 3

Format: put the script in a single ```lua code block.
Start with: -- script_name: Short description
Add: -- Intents: comma, separated, voice, triggers

Keep scripts concise and focused on one behavior."#;

/// System prompt for game-loop AI decisions.
const GAME_LOOP_SYSTEM_PROMPT: &str = r#"You are Minstral, an AI commander for the catGPT faction in the RTS game ClawedCommand. You control an army of cats in a post-singularity world.

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
async fn process_request(
    client: &dyn LlmClient,
    request: &AgentRequest,
    config: &LlmConfig,
) -> AgentResponse {
    // Prompt source: use Claude Code CLI instead of the LLM API
    if request.source == AgentSource::Prompt {
        let mut system_prompt = PROMPT_OVERLAY_SYSTEM_PROMPT.to_string();
        if let Some(snap) = &request.snapshot {
            system_prompt.push_str("\n\nCurrent game state:\n");
            system_prompt.push_str(&snapshot::summarize_snapshot(snap));
        }

        match claude_cli::invoke_claude_cli(&request.prompt, &system_prompt) {
            Ok(content) => {
                return AgentResponse {
                    content,
                    commands: Vec::new(),
                    error: None,
                    source: request.source,
                    player_id: request.player_id,
                };
            }
            Err(e) => {
                return AgentResponse {
                    content: String::new(),
                    commands: Vec::new(),
                    error: Some(e.to_string()),
                    source: request.source,
                    player_id: request.player_id,
                };
            }
        }
    }

    let snapshot = request.snapshot.as_ref();

    // Build message list
    let mut messages = Vec::new();

    match request.source {
        AgentSource::ConstructMode => {
            // For fine-tuned models, prepend the training system prompt
            if config.finetuned_lua {
                let has_finetune_prompt = request
                    .chat_history
                    .as_ref()
                    .is_some_and(|h| {
                        h.iter().any(|m| {
                            m.role == "system"
                                && m.content == FINETUNE_CONSTRUCT_SYSTEM_PROMPT
                        })
                    });
                if !has_finetune_prompt {
                    messages.push(ChatMessage {
                        role: "system".to_string(),
                        content: FINETUNE_CONSTRUCT_SYSTEM_PROMPT.to_string(),
                    });
                }
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
        AgentSource::Prompt => unreachable!("Handled above"),
        AgentSource::GameLoop | AgentSource::QuickCommand => {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: GAME_LOOP_SYSTEM_PROMPT.to_string(),
            });
            if let Some(snap) = snapshot {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: format!("{}\n\n{}", snapshot::summarize_snapshot(snap), request.prompt),
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
    let use_tools = !(config.finetuned_lua && request.source == AgentSource::ConstructMode);

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

    // Multi-turn tool calling loop
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
pub fn spawn_llm_runner(
    config: LlmConfig,
    channels: AgentChannels,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for LLM runner");

        rt.block_on(async move {
            let client = build_client(&config);
            let AgentChannels {
                request_rx,
                response_tx,
            } = channels;

            loop {
                let request = match request_rx.recv() {
                    Ok(req) => req,
                    Err(_) => break, // Channel closed
                };

                let response = process_request(client.as_ref(), &request, &config).await;

                if response_tx.send(response).is_err() {
                    break; // Response channel closed
                }
            }
        });
    })
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

        let response = process_request(&client, &request, &config).await;
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

        let response = process_request(&client, &request, &config).await;
        assert!(response.error.is_none());

        let captured = client.captured.lock().unwrap();
        assert!(!captured.is_empty());
        let messages = &captured[0];
        // First message should be the system prompt
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
    fn prompt_source_routes_to_claude_cli() {
        use crate::agent_bridge::AgentBridge;

        let config = LlmConfig::default();
        let (bridge, channels) = AgentBridge::new();

        let _handle = spawn_llm_runner(config, channels);

        // Send a Prompt request — will invoke claude CLI
        bridge
            .request_tx
            .try_send(AgentRequest {
                player_id: 0,
                prompt: "test prompt".into(),
                tier: ToolTier::Basic,
                source: AgentSource::Prompt,
                chat_history: None,
                snapshot: None,
            })
            .expect("send should succeed");

        let response = bridge
            .response_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("should receive response from Prompt path");

        assert_eq!(response.source, AgentSource::Prompt);
        assert_eq!(response.player_id, 0);
        // Response will have either content (claude installed) or error (not installed)
        assert!(
            !response.content.is_empty() || response.error.is_some(),
            "Should get content or error from Claude CLI"
        );
    }
}
