use bevy::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::agent_bridge::{AgentBridge, AgentChatEntry, AgentChatLog, AgentResponse, AgentSource};
use crate::llm_client::{AgentStatus, LlmConfig, LlmBackend, OpenAiCompatibleClient};
use crate::mcp_tools;
use crate::webllm_client::WebLlmClient;
use crate::fallback_client::FallbackClient;

/// Startup system: initialize the WASM agent loop.
/// Reads LlmConfig and spawns an async task that listens for requests.
pub fn init_wasm_agent(
    bridge: Res<AgentBridge>,
    config: Option<Res<LlmConfig>>,
    mut status: ResMut<AgentStatus>,
) {
    let config = config.map(|c| c.clone()).unwrap_or_default();

    // Clone the channels for the async task
    let request_rx = bridge.request_rx.clone();
    let response_tx = bridge.response_tx.clone();

    *status = AgentStatus::Initializing(0.0);

    spawn_local(async move {
        // Build the client based on config
        let client: Box<dyn crate::llm_client::LlmClient> = match config.backend {
            LlmBackend::WebLlm => {
                match crate::webllm_client::webllm_init(&config.model).await {
                    Ok(()) => Box::new(WebLlmClient::new(config.model.clone())),
                    Err(e) => {
                        log::error!("WebLLM init failed: {e}, falling back to mock");
                        Box::new(crate::llm_client::MockLlmClient::new(vec![]))
                    }
                }
            }
            LlmBackend::OpenAiCompatible => {
                Box::new(OpenAiCompatibleClient::new(
                    config.base_url.clone(),
                    config.api_key.clone(),
                    config.model.clone(),
                    config.temperature,
                ))
            }
            LlmBackend::Fallback => {
                let mut providers: Vec<Box<dyn crate::llm_client::LlmClient>> = Vec::new();

                // Try WebLLM first if available
                if crate::webllm_client::webgpu_available() {
                    if let Ok(()) = crate::webllm_client::webllm_init(&config.model).await {
                        providers.push(Box::new(WebLlmClient::new(config.model.clone())));
                    }
                }

                // Then local server
                providers.push(Box::new(OpenAiCompatibleClient::new(
                    "http://localhost:11434".into(),
                    String::new(),
                    "devstral-small-2-2512".into(),
                    0.2,
                )));

                // Then remote API
                if !config.api_key.is_empty() {
                    providers.push(Box::new(OpenAiCompatibleClient::new(
                        "https://api.mistral.ai".into(),
                        config.api_key.clone(),
                        "devstral-2-2512".into(),
                        0.2,
                    )));
                }

                Box::new(FallbackClient::new(providers))
            }
            _ => Box::new(crate::llm_client::MockLlmClient::new(vec![])),
        };

        log::info!("WASM agent loop started with provider: {}", client.model_name());

        // Main agent loop — process requests as they come in
        loop {
            match request_rx.recv().await {
                Ok(request) => {
                    let tool_defs = mcp_tools::tool_definitions(request.tier);
                    let mut messages = request.chat_history.unwrap_or_default();
                    messages.push(crate::llm_client::ChatMessage {
                        role: "user".into(),
                        content: request.prompt,
                    });

                    match client.complete(&messages, Some(&tool_defs)).await {
                        Ok(response) => {
                            let mut commands = Vec::new();
                            for tc in &response.tool_calls {
                                let (_, cmds) = mcp_tools::execute_tool(
                                    &tc.name,
                                    &tc.arguments,
                                    request.player_id,
                                    None,
                                    request.tier,
                                );
                                commands.extend(cmds);
                            }
                            let _ = response_tx.try_send(AgentResponse {
                                player_id: request.player_id,
                                content: response.content,
                                commands,
                                error: None,
                                source: request.source,
                            });
                        }
                        Err(e) => {
                            let _ = response_tx.try_send(AgentResponse {
                                player_id: request.player_id,
                                content: String::new(),
                                commands: vec![],
                                error: Some(e.to_string()),
                                source: request.source,
                            });
                        }
                    }
                }
                Err(_) => break, // Channel closed
            }
        }
    });
}
