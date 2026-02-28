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

/// State for the Construct Mode UI.
#[derive(Resource)]
pub struct ConstructModeState {
    pub active: bool,
    pub chat_history: Vec<ChatMessage>,
    pub current_script: Option<LuaScript>,
    pub chat_input: String,
}

impl Default for ConstructModeState {
    fn default() -> Self {
        Self {
            active: false,
            chat_history: Vec::new(),
            current_script: None,
            chat_input: String::new(),
        }
    }
}
