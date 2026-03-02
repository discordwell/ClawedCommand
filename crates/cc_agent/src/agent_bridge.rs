use std::collections::VecDeque;

use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(target_arch = "wasm32")]
use async_channel::{Receiver, Sender, unbounded};

use cc_core::commands::GameCommand;
use cc_sim::resources::CommandQueue;

use crate::construct_mode::{ConstructModeState, LuaScript};
use crate::llm_client::ChatMessage;
use crate::snapshot::GameStateSnapshot;
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
    /// `/` key prompt overlay — uses fine-tuned Devstral, auto-registers scripts.
    Prompt,
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
    /// Game state snapshot for tool execution context.
    pub snapshot: Option<GameStateSnapshot>,
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

/// Both halves of the channels for the LLM background thread/task.
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
    mut registry: ResMut<crate::runner::ScriptRegistry>,
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
            AgentSource::ConstructMode | AgentSource::Prompt => {
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

                    // Prompt source: auto-register script in the Lua layer
                    if response.source == AgentSource::Prompt {
                        registry.register_lua_script(
                            &script.name,
                            &script.source,
                            response.player_id,
                        );
                        log::info!(
                            "Auto-registered prompt script '{}' for player {}",
                            script.name,
                            response.player_id
                        );
                    }

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

/// Extract a Lua script from an LLM response.
/// Tries fenced ```lua blocks first (base/API model), then raw Lua (fine-tuned model).
pub fn extract_lua_script(content: &str) -> Option<LuaScript> {
    // Prefer fenced code blocks (backward compatible with base model output)
    if let Some(script) = extract_fenced_lua_script(content) {
        return Some(script);
    }
    // Fall back to raw Lua detection (fine-tuned model outputs Lua directly)
    extract_raw_lua_script(content)
}

/// Extract Lua from a ```lua fenced code block.
fn extract_fenced_lua_script(content: &str) -> Option<LuaScript> {
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

/// Detect and extract raw Lua output (no fenced block).
/// The fine-tuned model outputs Lua directly, typically starting with `-- Intent:` headers.
fn extract_raw_lua_script(content: &str) -> Option<LuaScript> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Heuristic: raw Lua starts with a comment or `local`, and contains ctx API calls
    let first_line = trimmed.lines().next().unwrap_or("");
    let starts_like_lua = first_line.starts_with("--") || first_line.starts_with("local ");
    let has_lua_markers = trimmed.contains("ctx:") || trimmed.contains("ctx.behaviors:");

    if !starts_like_lua || !has_lua_markers {
        return None;
    }

    // Check there's no prose preamble (sentences before the Lua code)
    // Prose lines don't start with --, local, if, for, end, return, or be blank
    for line in trimmed.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if l.starts_with("--")
            || l.starts_with("local ")
            || l.starts_with("if ")
            || l.starts_with("for ")
            || l.starts_with("while ")
            || l.starts_with("return")
            || l.starts_with("end")
            || l.starts_with("else")
            || l.starts_with("ctx")
            || l.starts_with("table.")
            || l.starts_with("math.")
        {
            // Looks like Lua — keep going
            continue;
        }
        // Non-Lua line found → probably prose mixed in, not raw Lua
        return None;
    }

    let source = trimmed.to_string();
    let intents = extract_intents_from_source(&source);
    let name = extract_name_from_source(&source).unwrap_or_else(|| "untitled_script".to_string());

    Some(LuaScript {
        name,
        source,
        intents,
        description: String::new(),
    })
}

/// Parse `-- Intents: gather, harvest` or `-- Intent: gather` from Lua source.
pub fn extract_intents_from_source(source: &str) -> Vec<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        // Handle both plural (base model) and singular (fine-tuned model)
        let rest = trimmed
            .strip_prefix("-- Intents:")
            .or_else(|| trimmed.strip_prefix("-- Intent:"));
        if let Some(rest) = rest {
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
/// Falls back to deriving a name from `-- Description:` if no explicit name is found.
pub fn extract_name_from_source(source: &str) -> Option<String> {
    let mut description_text = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("-- ") {
            // Skip intent metadata
            if rest.starts_with("Intents:") || rest.starts_with("Intent:") {
                continue;
            }
            // Capture description for fallback name derivation
            if let Some(desc) = rest.strip_prefix("Description:") {
                description_text = Some(desc.trim().to_string());
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

    // Fallback: derive snake_case name from description (fine-tuned model format)
    if let Some(desc) = description_text {
        let words: Vec<&str> = desc.split_whitespace().take(4).collect();
        if !words.is_empty() {
            let name = words
                .iter()
                .map(|w| {
                    w.chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect::<String>()
                        .to_lowercase()
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("_");
            if !name.is_empty() {
                return Some(name);
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

    #[test]
    fn extract_lua_script_claude_code_output() {
        // Claude Code CLI output typically wraps the code block in prose
        let response = r#"I'll create a kiting script for your ranged units. Here's the script:

```lua
-- ranged_kite: Kite enemies with ranged units
-- Intents: kite, retreat, ranged

local hissers = ctx:my_units("Hisser")
local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Find centroid of own army
local cx, cy = 0, 0
for _, u in ipairs(hissers) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #hissers
cy = cy / #hissers

-- Find closest enemy to centroid
local target = nil
local best_dist = math.huge
for _, e in ipairs(enemies) do
    local d = (e.x - cx)^2 + (e.y - cy)^2
    if d < best_dist then
        best_dist = d
        target = e
    end
end

if target then
    local ids = {}
    for _, u in ipairs(hissers) do
        table.insert(ids, u.id)
    end
    ctx:attack(ids, target.id)
end
```

This script implements focus fire on the closest enemy to your army's center of mass."#;

        let script = extract_lua_script(response).unwrap();
        assert_eq!(script.name, "ranged_kite");
        assert_eq!(script.intents, vec!["kite", "retreat", "ranged"]);
        assert!(script.source.contains("ctx:my_units"));
        assert!(script.source.contains("ctx:attack"));
        assert!(script.description.contains("kiting script"));
    }

    #[test]
    fn extract_lua_script_raw_output() {
        // Fine-tuned model outputs raw Lua with -- Intent: headers (no fenced block)
        let raw_lua = r#"-- Intent: gather
-- Description: Send idle workers to nearest fish pond

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

for _, w in ipairs(workers) do
    local deposit = ctx:nearest_deposit(w.x, w.y, "Food")
    if deposit then
        ctx:gather({w.id}, deposit.id)
    end
end"#;

        let script = extract_lua_script(raw_lua).unwrap();
        assert_eq!(script.name, "send_idle_workers_to"); // Derived from Description (first 4 words)
        assert_eq!(script.intents, vec!["gather"]);
        assert!(script.source.contains("ctx:idle_units"));
        assert!(script.source.contains("ctx:gather"));
        assert!(script.description.is_empty()); // No prose preamble for raw output
    }

    #[test]
    fn extract_singular_intent() {
        let source = "-- Intent: attack, defend\nlocal x = ctx:my_units()";
        let intents = extract_intents_from_source(source);
        assert_eq!(intents, vec!["attack", "defend"]);
    }

    #[test]
    fn extract_lua_script_fenced_preferred_over_raw() {
        // When content has BOTH a fenced block AND raw Lua markers, prefer fenced
        let content = r#"Here's a script:

```lua
-- fenced_script
-- Intents: build
local b = ctx:my_buildings()
```

That should work."#;

        let script = extract_lua_script(content).unwrap();
        assert_eq!(script.name, "fenced_script");
        assert_eq!(script.intents, vec!["build"]);
    }

    #[test]
    fn agent_source_prompt_variant_exists() {
        // Verify the Prompt variant is accessible and distinct
        let source = AgentSource::Prompt;
        assert_ne!(source, AgentSource::ConstructMode);
        assert_ne!(source, AgentSource::GameLoop);
        assert_ne!(source, AgentSource::QuickCommand);
    }
}
