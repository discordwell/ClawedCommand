use bevy::prelude::*;

use crate::llm_client::ChatMessage;

/// A Lua script stored in the player's script library.
#[derive(Debug, Clone)]
pub struct LuaScript {
    pub name: String,
    pub source: String,
    pub intents: Vec<String>,
    pub description: String,
}

/// Player's library of saved Lua scripts.
#[derive(Resource, Default)]
pub struct ScriptLibrary {
    pub scripts: Vec<LuaScript>,
}

impl ScriptLibrary {
    /// Create a library seeded with starter scripts + any saved player scripts.
    pub fn with_starters() -> Self {
        let mut scripts = vec![
            LuaScript {
                name: "basic_attack".into(),
                source: include_str!("../../../assets/scripts/basic_attack.lua").into(),
                intents: vec!["attack".into(), "fight".into(), "charge".into()],
                description: "Attack-move all combat units toward the nearest enemy".into(),
            },
            LuaScript {
                name: "basic_retreat".into(),
                source: include_str!("../../../assets/scripts/basic_retreat.lua").into(),
                intents: vec!["retreat".into(), "run".into(), "fall back".into()],
                description: "Move all units back toward the base".into(),
            },
            LuaScript {
                name: "basic_gather".into(),
                source: include_str!("../../../assets/scripts/basic_gather.lua").into(),
                intents: vec!["gather".into(), "harvest".into(), "mine".into()],
                description: "Send idle Pawdlers to the nearest resource deposit".into(),
            },
            LuaScript {
                name: "basic_train".into(),
                source: include_str!("../../../assets/scripts/basic_train.lua").into(),
                intents: vec!["train".into(), "make units".into(), "produce".into()],
                description: "Train units from available production buildings".into(),
            },
            LuaScript {
                name: "basic_build".into(),
                source: include_str!("../../../assets/scripts/basic_build.lua").into(),
                intents: vec!["build".into(), "construct".into()],
                description: "Order a Pawdler to build a Cat Tree near the base".into(),
            },
            LuaScript {
                name: "strait_coverage".into(),
                source: include_str!("../../../assets/scripts/strait_coverage.lua").into(),
                intents: vec!["patrol".into(), "coverage".into(), "coastline".into(), "strait".into()],
                description: "Sector-based drone patrol for strait coastline coverage".into(),
            },
        ];

        // Load player-saved scripts from disk (native) or localStorage (WASM)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let player_scripts = crate::script_persistence::load_player_scripts();
            for ps in player_scripts {
                // Avoid duplicating starter scripts
                if !scripts.iter().any(|s| s.name == ps.name) {
                    scripts.push(ps);
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let player_scripts = crate::wasm_persistence::load_player_scripts();
            for ps in player_scripts {
                if !scripts.iter().any(|s| s.name == ps.name) {
                    scripts.push(ps);
                }
            }
        }

        Self { scripts }
    }
}

/// Result of running a script test.
#[derive(Debug, Clone)]
pub struct ScriptTestResult {
    pub success: bool,
    pub message: String,
    pub command_count: usize,
}

/// State for the Construct Mode UI.
#[derive(Resource, Default)]
pub struct ConstructModeState {
    pub active: bool,
    pub chat_history: Vec<ChatMessage>,
    pub current_script: Option<LuaScript>,
    pub chat_input: String,
    /// True while waiting for an LLM response.
    pub waiting_for_response: bool,
    /// Editable copy of current_script.source for the code editor.
    pub editable_source: String,
    /// Result of last test run.
    pub test_result: Option<ScriptTestResult>,
}
