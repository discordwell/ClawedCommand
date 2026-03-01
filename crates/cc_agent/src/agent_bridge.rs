use std::collections::VecDeque;

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};

use cc_core::commands::GameCommand;
use cc_sim::resources::CommandQueue;

use crate::construct_mode::{ConstructModeState, LuaScript};
use crate::llm_client::ChatMessage;
use crate::tool_tier::ToolTier;

/// Where an agent request originated — determines response routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {
    /// Autonomous game-loop AI decision.
    GameLoop,
    /// Player's construct mode chat.
    ConstructMode,
    /// Quick command from agent chat panel.
    QuickCommand,
}

/// Message from Bevy → background LLM thread.
#[derive(Debug)]
pub struct AgentRequest {
    pub player_id: u8,
    pub prompt: String,
    pub tier: ToolTier,
    pub source: AgentSource,
    /// For ConstructMode: the full chat history to send as context.
    pub chat_history: Option<Vec<ChatMessage>>,
}

/// Message from background LLM thread → Bevy.
#[derive(Debug)]
pub struct AgentResponse {
    pub content: String,
    pub commands: Vec<GameCommand>,
    pub error: Option<String>,
    pub source: AgentSource,
    pub player_id: u8,
}

/// A single entry in the agent chat log (side panel).
#[derive(Debug, Clone)]
pub struct AgentChatEntry {
    pub content: String,
    pub error: Option<String>,
}

/// Stores agent responses for the side panel chat display.
#[derive(Resource, Default)]
pub struct AgentChatLog {
    pub entries: VecDeque<AgentChatEntry>,
}

impl AgentChatLog {
    pub fn push(&mut self, content: String, error: Option<String>) {
        self.entries.push_back(AgentChatEntry { content, error });
        // Keep last 50 entries
        if self.entries.len() > 50 {
            self.entries.pop_front();
        }
    }
}

/// Both halves of the crossbeam channels for the LLM background thread.
pub struct AgentChannels {
    pub request_rx: Receiver<AgentRequest>,
    pub response_tx: Sender<AgentResponse>,
}

/// Bridge between Bevy (sync) and the background LLM runtime (async).
#[derive(Resource)]
pub struct AgentBridge {
    pub request_tx: Sender<AgentRequest>,
    pub response_rx: Receiver<AgentResponse>,
}

impl AgentBridge {
    /// Create bridge + channels. Returns (bridge_resource, channels_for_background_thread).
    pub fn new() -> (Self, AgentChannels) {
        let (request_tx, request_rx) = unbounded();
        let (response_tx, response_rx) = unbounded();
        let bridge = Self {
            request_tx,
            response_rx,
        };
        let channels = AgentChannels {
            request_rx,
            response_tx,
        };
        (bridge, channels)
    }
}

impl Default for AgentBridge {
    fn default() -> Self {
        let (request_tx, _request_rx) = unbounded();
        let (_response_tx, response_rx) = unbounded();
        Self {
            request_tx,
            response_rx,
        }
    }
}

/// Bevy system: poll for LLM responses and route by source.
/// Also clears in-flight flags per-player as responses arrive.
pub fn poll_agent_responses(
    bridge: Res<AgentBridge>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut construct_state: ResMut<ConstructModeState>,
    mut chat_log: ResMut<AgentChatLog>,
    mut decision_state: ResMut<crate::decision::AgentDecisionState>,
) {
    while let Ok(response) = bridge.response_rx.try_recv() {
        // Clear in-flight flag for this player so the decision system
        // can send new requests on the next timer tick.
        decision_state.in_flight.remove(&response.player_id);
        if let Some(err) = &response.error {
            log::warn!("Agent error: {err}");
        }

        for cmd in &response.commands {
            cmd_queue.push(cmd.clone());
        }

        match response.source {
            AgentSource::ConstructMode => {
                if !response.content.is_empty() {
                    construct_state.chat_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: response.content.clone(),
                    });
                }
                if let Some(err) = &response.error {
                    construct_state.chat_history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: format!("Error: {err}"),
                    });
                }
                if let Some(script) = extract_lua_script(&response.content) {
                    construct_state.editable_source = script.source.clone();
                    construct_state.current_script = Some(script);
                }
                construct_state.waiting_for_response = false;
            }
            AgentSource::QuickCommand | AgentSource::GameLoop => {
                chat_log.push(response.content.clone(), response.error.clone());
            }
        }

        if !response.content.is_empty() {
            log::info!("Agent: {}", response.content);
        }
    }
}

/// Extract a Lua script from an LLM response containing ```lua code blocks.
pub fn extract_lua_script(content: &str) -> Option<LuaScript> {
    let start_marker = "```lua";
    let end_marker = "```";

    let start = content.find(start_marker)?;
    let code_start = start + start_marker.len();
    let remaining = &content[code_start..];
    let end = remaining.find(end_marker)?;
    let source = remaining[..end].trim().to_string();

    if source.is_empty() {
        return None;
    }

    let intents = extract_intents_from_source(&source);
    let name = extract_name_from_source(&source).unwrap_or_else(|| "untitled_script".to_string());
    let description = content[..start].trim().to_string();

    Some(LuaScript {
        name,
        source,
        intents,
        description,
    })
}

/// Parse `-- Intents: gather, harvest` from Lua source.
pub fn extract_intents_from_source(source: &str) -> Vec<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("-- Intents:") {
            return rest
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    vec![]
}

/// Parse `-- script_name` or `-- script_name: description` from first comment.
pub fn extract_name_from_source(source: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("-- ") {
            // Skip lines that look like intents or other metadata
            if rest.starts_with("Intents:") || rest.starts_with("Description:") {
                continue;
            }
            let name = if let Some((name, _)) = rest.split_once(':') {
                name.trim()
            } else {
                rest.split_whitespace().next().unwrap_or("")
            };
            if !name.is_empty()
                && !name.contains(' ')
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_lua_script_basic() {
        let response = r#"Here's a gather script:

```lua
-- basic_gather: Send idle workers to gather
-- Intents: gather, harvest

local idle = ctx:my_units("Pawdler")
for _, u in ipairs(idle) do
    if u.is_idle then
        ctx:gather({u.id}, 1)
    end
end
```

This will send workers to resources."#;

        let script = extract_lua_script(response).unwrap();
        assert_eq!(script.name, "basic_gather");
        assert_eq!(script.intents, vec!["gather", "harvest"]);
        assert!(script.source.contains("ctx:my_units"));
        assert!(script.description.contains("gather script"));
    }

    #[test]
    fn extract_lua_script_no_code_block() {
        let response = "I can help with that but here's no code.";
        assert!(extract_lua_script(response).is_none());
    }

    #[test]
    fn extract_lua_script_empty_code_block() {
        let response = "Here:\n```lua\n```\nDone.";
        assert!(extract_lua_script(response).is_none());
    }

    #[test]
    fn extract_lua_script_no_intents() {
        let response = r#"```lua
-- unnamed_helper
local x = 1
```"#;
        let script = extract_lua_script(response).unwrap();
        assert_eq!(script.name, "unnamed_helper");
        assert!(script.intents.is_empty());
    }

    #[test]
    fn extract_lua_script_no_name_comment() {
        let response = r#"```lua
local x = ctx:my_units()
```"#;
        let script = extract_lua_script(response).unwrap();
        assert_eq!(script.name, "untitled_script");
    }

    #[test]
    fn extract_intents_parsing() {
        let source = "-- test\n-- Intents: attack, fight, charge\nlocal x = 1";
        let intents = extract_intents_from_source(source);
        assert_eq!(intents, vec!["attack", "fight", "charge"]);
    }
}
