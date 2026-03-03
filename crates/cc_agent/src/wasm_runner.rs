//! WASM async agent loop — runs LLM calls on the browser event loop.
//!
//! Replaces the native tokio-based llm_runner for WASM targets.
//! Uses wasm_bindgen_futures::spawn_local for async execution.

use bevy::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::agent_bridge::{AgentBridge, AgentChannels, AgentRequest, AgentResponse, AgentSource};
use crate::fallback_client::FallbackClient;
use crate::llm_client::{
    AgentStatus, ChatMessage, LlmBackend, LlmClient, LlmConfig, LlmResponse, MockLlmClient,
    OpenAiCompatibleClient,
};
use crate::mcp_tools;
use crate::webllm_client::WebLlmClient;

/// Maximum tool-call rounds per request to prevent runaway loops.
const MAX_TOOL_ROUNDS: usize = 5;

/// System prompt for game-loop AI decisions (same as native llm_runner).
const GAME_LOOP_SYSTEM_PROMPT: &str = r#"You are Minstral, an AI commander for the catGPT faction in the RTS game ClawedCommand. You control an army of cats in a post-singularity world.

Analyze the game state and issue tool calls to manage your army effectively. Priorities:
1. Gather resources (food, GPU cores) to fund your army
2. Train units and build structures for defense and offense
3. Scout for enemies and respond to threats
4. Attack when you have a decisive advantage

Be efficient — issue only necessary commands. You have limited GPU budget per decision cycle."#;

/// Bevy startup system: creates the AgentBridge with proper channels
/// and spawns the async agent loop on the browser event loop.
pub fn init_wasm_agent(mut commands: Commands, config: Option<Res<LlmConfig>>) {
    let config = config.map(|c| c.clone()).unwrap_or_default();

    let (bridge, channels) = AgentBridge::new();
    commands.insert_resource(bridge);
    commands.insert_resource(AgentStatus::Initializing(0.0));

    spawn_local(async move {
        run_agent_loop(config, channels).await;
    });
}

/// Build the appropriate LLM client from config.
fn build_client(config: &LlmConfig) -> Box<dyn LlmClient> {
    match &config.backend {
        LlmBackend::WebLlm => Box::new(WebLlmClient::new(config.model.clone())),
        LlmBackend::OpenAiCompatible | LlmBackend::Anthropic => {
            Box::new(OpenAiCompatibleClient::new(
                config.base_url.clone(),
                config.api_key.clone(),
                config.model.clone(),
                config.temperature,
            ))
        }
        LlmBackend::Fallback => {
            let mut fallback = FallbackClient::new();

            // Try WebLLM first if WebGPU is available
            if crate::webllm_client::webgpu_available() {
                fallback = fallback.add_provider(
                    "WebLLM".into(),
                    Box::new(WebLlmClient::new(config.model.clone())),
                );
            }

            // Then try local server (Ollama)
            fallback = fallback.add_provider(
                "Local".into(),
                Box::new(OpenAiCompatibleClient::new(
                    "http://localhost:11434".into(),
                    String::new(),
                    config.model.clone(),
                    config.temperature,
                )),
            );

            // Then remote API if key is configured
            if !config.api_key.is_empty() {
                fallback = fallback.add_provider(
                    "Remote".into(),
                    Box::new(OpenAiCompatibleClient::new(
                        config.base_url.clone(),
                        config.api_key.clone(),
                        config.model.clone(),
                        config.temperature,
                    )),
                );
            }

            Box::new(fallback)
        }
        LlmBackend::Mock => Box::new(MockLlmClient::new(vec![LlmResponse {
            content: "No action needed at this time.".to_string(),
            tool_calls: vec![],
        }])),
    }
}

/// The main async loop that processes agent requests.
async fn run_agent_loop(config: LlmConfig, channels: AgentChannels) {
    // Initialize WebLLM if that's the backend
    if config.backend == LlmBackend::WebLlm || config.backend == LlmBackend::Fallback {
        if crate::webllm_client::webgpu_available() {
            if let Err(e) = crate::webllm_client::init(&config.model).await {
                log::warn!("WebLLM init failed: {e}");
            }
        }
    }

    let client = build_client(&config);
    log::info!(
        "WASM agent loop started with provider: {}",
        client.model_name()
    );

    let AgentChannels {
        request_rx,
        response_tx,
        token_tx: _token_tx,
    } = channels;

    loop {
        let request = match request_rx.recv().await {
            Ok(req) => req,
            Err(_) => break, // Channel closed
        };

        let response = process_request(client.as_ref(), &request).await;

        if response_tx.send(response).await.is_err() {
            break; // Response channel closed
        }
    }
}

/// Process a single agent request: call LLM with multi-turn tool loop.
async fn process_request(client: &dyn LlmClient, request: &AgentRequest) -> AgentResponse {
    let mut messages = Vec::new();

    match request.source {
        AgentSource::ConstructMode => {
            if let Some(history) = &request.chat_history {
                messages.extend(history.iter().cloned());
            } else {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: request.prompt.clone(),
                });
            }
        }
        AgentSource::GameLoop | AgentSource::QuickCommand | AgentSource::Prompt => {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: GAME_LOOP_SYSTEM_PROMPT.to_string(),
            });
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: request.prompt.clone(),
            });
        }
    }

    let tool_defs = mcp_tools::tool_definitions(request.tier);
    let tools = if tool_defs.is_empty() {
        None
    } else {
        Some(tool_defs.as_slice())
    };

    let mut all_commands = Vec::new();
    let mut final_content = String::new();

    for _round in 0..MAX_TOOL_ROUNDS {
        let result = client.complete(&messages, tools).await;

        match result {
            Ok(response) => {
                if !response.content.is_empty() {
                    final_content = response.content.clone();
                }

                if response.tool_calls.is_empty() {
                    break;
                }

                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content.clone(),
                });

                for call in &response.tool_calls {
                    let (result_json, commands) = mcp_tools::execute_tool(
                        &call.name,
                        &call.arguments,
                        request.player_id,
                        None,
                        request.tier,
                    );
                    all_commands.extend(commands);

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
