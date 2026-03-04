//! WASM async agent loop — runs LLM calls on the browser event loop.
//!
//! Replaces the native tokio-based llm_runner for WASM targets.
//! Uses wasm_bindgen_futures::spawn_local for async execution.
//! Supports deferred initialization: the agent loop waits for a config
//! sent via `cc_connect_ai()` before building the LLM client.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use bevy::prelude::*;
use wasm_bindgen::prelude::*;
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
const GAME_LOOP_SYSTEM_PROMPT: &str = r#"You are Le Chat, an AI commander for the catGPT faction in the RTS game ClawedCommand. You control an army of cats in a post-singularity world.

Analyze the game state and issue tool calls to manage your army effectively. Priorities:
1. Gather resources (food, GPU cores) to fund your army
2. Train units and build structures for defense and offense
3. Scout for enemies and respond to threats
4. Attack when you have a decisive advantage

Be efficient — issue only necessary commands. You have limited GPU budget per decision cycle."#;

// --- Status tracking via atomics (accessible from JS) ---

/// Agent status: 0=unconfigured, 1=initializing, 2=ready, 3=error
static AGENT_STATUS: AtomicU8 = AtomicU8::new(0);
/// Error message string (only meaningful when status == 3)
static AGENT_ERROR: Mutex<String> = Mutex::new(String::new());
/// Config channel: JS sends LlmConfig via cc_connect_ai, the agent loop receives it.
static CONFIG_TX: OnceLock<async_channel::Sender<LlmConfig>> = OnceLock::new();

fn set_status(code: u8, error_msg: &str) {
    AGENT_STATUS.store(code, Ordering::Relaxed);
    if code == 3 {
        if let Ok(mut e) = AGENT_ERROR.lock() {
            *e = error_msg.to_string();
        }
    }
}

/// JS-callable: connect the AI agent with the given backend configuration.
/// Call this from play.html after WASM init to start the agent loop.
#[wasm_bindgen]
pub fn cc_connect_ai(backend: &str, model: &str, base_url: &str, api_key: &str) {
    let config = LlmConfig {
        backend: match backend {
            "webllm" => LlmBackend::WebLlm,
            "ollama" | "remote" => LlmBackend::OpenAiCompatible,
            "claude-code" => LlmBackend::Anthropic,
            "fallback" => LlmBackend::Fallback,
            _ => LlmBackend::Mock,
        },
        base_url: if base_url.is_empty() {
            "http://localhost:11434".into()
        } else {
            base_url.into()
        },
        api_key: api_key.into(),
        model: if model.is_empty() {
            "qwen3-coder:30b-a3b".into()
        } else {
            model.into()
        },
        temperature: 0.2,
        finetuned_lua: false,
    };

    match CONFIG_TX.get() {
        Some(tx) => {
            if tx.try_send(config).is_err() {
                log::warn!("cc_connect_ai: config channel full or closed — already connected?");
                set_status(3, "Already connected or connecting. Refresh to reconnect.");
            }
        }
        None => {
            log::warn!("cc_connect_ai: WASM agent not initialized yet");
            set_status(3, "Engine not ready — try again after loading completes.");
        }
    }
}

/// JS-callable: get the current AI agent status as a JSON string.
/// Returns: `{"status":"unconfigured"|"initializing"|"ready"|"error","error":"..."}`
#[wasm_bindgen]
pub fn cc_get_ai_status() -> String {
    let code = AGENT_STATUS.load(Ordering::Relaxed);
    let (status_str, error_str) = match code {
        0 => ("unconfigured", String::new()),
        1 => ("initializing", String::new()),
        2 => ("ready", String::new()),
        3 => {
            let err = AGENT_ERROR.lock().map(|e| e.clone()).unwrap_or_default();
            ("error", err)
        }
        _ => ("unconfigured", String::new()),
    };

    serde_json::json!({
        "status": status_str,
        "error": error_str,
    })
    .to_string()
}

/// Bevy startup system: creates the AgentBridge with proper channels
/// and spawns the async agent loop on the browser event loop.
/// The loop waits for config from `cc_connect_ai` before initializing.
pub fn init_wasm_agent(mut commands: Commands) {
    let (bridge, channels) = AgentBridge::new();
    commands.insert_resource(bridge);
    commands.insert_resource(AgentStatus::Unconfigured);

    // Create the config channel
    let (tx, rx) = async_channel::bounded::<LlmConfig>(1);
    if CONFIG_TX.set(tx).is_err() {
        log::warn!("init_wasm_agent called twice — config channel already set");
    }

    set_status(0, "");

    spawn_local(async move {
        // Wait for config from JS before starting the agent loop
        let config = match rx.recv().await {
            Ok(c) => c,
            Err(_) => return, // Channel closed — game shutting down
        };

        set_status(1, "");
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
                if config.backend == LlmBackend::WebLlm {
                    // Hard fail — WebLLM was the only backend
                    set_status(3, &format!("WebLLM init failed: {e}"));
                    return;
                }
                // For Fallback, continue — other providers will be tried
            }
        }
    }

    let client = build_client(&config);
    log::info!(
        "WASM agent loop started with provider: {}",
        client.model_name()
    );
    set_status(2, "");

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
