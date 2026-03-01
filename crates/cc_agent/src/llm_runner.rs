//! Background async task that processes AgentRequests by calling an LLM.
//!
//! Receives requests from the AgentBridge channel, calls the LLM via
//! OpenAiCompatibleClient, parses tool calls, executes them through
//! mcp_tools, and sends AgentResponses back.

use crate::agent_bridge::{AgentChannels, AgentRequest, AgentResponse, AgentSource};
use crate::llm_client::{ChatMessage, LlmClient, LlmConfig, LlmBackend, OpenAiCompatibleClient, MockLlmClient, LlmResponse};
use crate::mcp_tools;
use crate::snapshot::GameStateSnapshot;

/// Maximum tool-call rounds per request to prevent runaway loops.
const MAX_TOOL_ROUNDS: usize = 5;

/// System prompt for game-loop AI decisions.
const GAME_LOOP_SYSTEM_PROMPT: &str = r#"You are Minstral, an AI commander for the catGPT faction in the RTS game ClawedCommand. You control an army of cats in a post-singularity world.

Analyze the game state and issue tool calls to manage your army effectively. Priorities:
1. Gather resources (food, GPU cores) to fund your army
2. Train units and build structures for defense and offense
3. Scout for enemies and respond to threats
4. Attack when you have a decisive advantage

Be efficient — issue only necessary commands. You have limited GPU budget per decision cycle."#;

/// Summarize a snapshot into a compact text description for the LLM.
pub fn summarize_snapshot(snap: &GameStateSnapshot) -> String {
    let mut s = format!(
        "Tick: {} | Map: {}x{} | Player: {}\n",
        snap.tick, snap.map_width, snap.map_height, snap.player_id
    );
    s.push_str(&format!(
        "Resources: Food={}, GPU={}, NFTs={}, Supply={}/{}\n",
        snap.my_resources.food,
        snap.my_resources.gpu_cores,
        snap.my_resources.nfts,
        snap.my_resources.supply,
        snap.my_resources.supply_cap,
    ));

    // Unit summary by kind
    let mut unit_counts = std::collections::HashMap::new();
    for u in &snap.my_units {
        if !u.is_dead {
            *unit_counts.entry(format!("{:?}", u.kind)).or_insert(0u32) += 1;
        }
    }
    if !unit_counts.is_empty() {
        s.push_str("My units: ");
        let parts: Vec<String> = unit_counts.iter().map(|(k, v)| format!("{}x{}", v, k)).collect();
        s.push_str(&parts.join(", "));
        s.push('\n');
    } else {
        s.push_str("My units: none\n");
    }

    // Enemy summary
    let alive_enemies = snap.enemy_units.iter().filter(|u| !u.is_dead).count();
    if alive_enemies > 0 {
        s.push_str(&format!("Visible enemies: {}\n", alive_enemies));
    } else {
        s.push_str("Visible enemies: none\n");
    }

    // Buildings
    let my_buildings: Vec<String> = snap
        .my_buildings
        .iter()
        .map(|b| {
            if b.under_construction {
                format!("{:?}(building)", b.kind)
            } else {
                format!("{:?}", b.kind)
            }
        })
        .collect();
    if !my_buildings.is_empty() {
        s.push_str(&format!("My buildings: {}\n", my_buildings.join(", ")));
    }

    let enemy_buildings = snap.enemy_buildings.len();
    if enemy_buildings > 0 {
        s.push_str(&format!("Visible enemy buildings: {}\n", enemy_buildings));
    }

    // Deposits
    let deposits = snap.resource_deposits.len();
    if deposits > 0 {
        s.push_str(&format!("Resource deposits: {}\n", deposits));
    }

    s
}

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
    snapshot: Option<&GameStateSnapshot>,
) -> AgentResponse {
    // Build message list
    let mut messages = Vec::new();

    match request.source {
        AgentSource::ConstructMode => {
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
                    content: format!("{}\n\n{}", summarize_snapshot(snap), request.prompt),
                });
            } else {
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: request.prompt.clone(),
                });
            }
        }
    }

    // Get tool definitions for this tier
    let tool_defs = mcp_tools::tool_definitions(request.tier);
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

                let response = process_request(client.as_ref(), &request, None).await;

                if response_tx.send(response).is_err() {
                    break; // Response channel closed
                }
            }
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let request = AgentRequest {
            player_id: 0,
            prompt: "What should I do?".into(),
            tier: ToolTier::Basic,
            source: AgentSource::GameLoop,
            chat_history: None,
        };

        let response = process_request(&client, &request, None).await;
        assert_eq!(response.content, "Standing by.");
        assert!(response.error.is_none());
        assert!(response.commands.is_empty());
        assert_eq!(response.source, AgentSource::GameLoop);
    }
}
