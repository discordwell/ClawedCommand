//! Cross-platform ScriptRegistry — available on both native and WASM.

use bevy::prelude::*;

use crate::events::ScriptRegistration;

/// Resource: registered scripts that respond to game events.
#[derive(Resource, Default)]
pub struct ScriptRegistry {
    pub scripts: Vec<ScriptRegistration>,
}

impl ScriptRegistry {
    pub fn register(&mut self, script: ScriptRegistration) {
        self.scripts.push(script);
    }

    pub fn unregister(&mut self, name: &str) {
        self.scripts.retain(|s| s.name != name);
    }

    /// Register a Lua script for on_tick execution (tick_interval=3).
    /// Replaces any existing script with the same name.
    pub fn register_lua_script(&mut self, name: &str, source: &str, player_id: u8) {
        let mut reg = ScriptRegistration::new(
            name.to_string(),
            source.to_string(),
            vec!["on_tick".to_string()],
            player_id,
        );
        reg.tick_interval = 3;
        self.unregister(name);
        self.register(reg);
    }
}
