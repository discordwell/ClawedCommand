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

    /// Register a script using annotation parsing. Replaces existing with same name.
    pub fn register_from_source(&mut self, source: &str, fallback_name: &str, player_id: u8) {
        let reg = ScriptRegistration::from_source(source, fallback_name, player_id);
        self.unregister(&reg.name);
        self.register(reg);
    }

    /// Enable or disable a script by name.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        for script in &mut self.scripts {
            if script.name == name {
                script.enabled = enabled;
                return;
            }
        }
    }

    /// Get all scripts for a specific player.
    pub fn scripts_for_player(&self, player_id: u8) -> Vec<&ScriptRegistration> {
        self.scripts
            .iter()
            .filter(|s| s.player_id == player_id)
            .collect()
    }

    /// Find a script by name (immutable).
    pub fn find(&self, name: &str) -> Option<&ScriptRegistration> {
        self.scripts.iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ActivationMode;

    #[test]
    fn register_from_source_parses_annotations() {
        let mut registry = ScriptRegistry::default();
        let source = "-- @name: my_script\n-- @interval: 2\nlocal x = 1";
        registry.register_from_source(source, "fallback", 0);

        assert_eq!(registry.scripts.len(), 1);
        assert_eq!(registry.scripts[0].name, "my_script");
        assert_eq!(registry.scripts[0].tick_interval, 2);
    }

    #[test]
    fn register_from_source_replaces_existing() {
        let mut registry = ScriptRegistry::default();
        registry.register_from_source("-- @name: test\nlocal a = 1", "x", 0);
        registry.register_from_source("-- @name: test\nlocal b = 2", "x", 0);
        assert_eq!(registry.scripts.len(), 1);
        assert!(registry.scripts[0].source.contains("local b = 2"));
    }

    #[test]
    fn set_enabled_toggles() {
        let mut registry = ScriptRegistry::default();
        registry.register_lua_script("test", "local x = 1", 0);
        assert!(registry.scripts[0].enabled);

        registry.set_enabled("test", false);
        assert!(!registry.scripts[0].enabled);

        registry.set_enabled("test", true);
        assert!(registry.scripts[0].enabled);
    }

    #[test]
    fn set_enabled_nonexistent_is_noop() {
        let mut registry = ScriptRegistry::default();
        registry.set_enabled("nonexistent", false); // should not panic
    }

    #[test]
    fn scripts_for_player_filters() {
        let mut registry = ScriptRegistry::default();
        registry.register_lua_script("p0_script", "local x = 1", 0);
        registry.register_lua_script("p1_script", "local y = 2", 1);
        registry.register_lua_script("p0_other", "local z = 3", 0);

        let p0 = registry.scripts_for_player(0);
        assert_eq!(p0.len(), 2);
        assert!(p0.iter().all(|s| s.player_id == 0));

        let p1 = registry.scripts_for_player(1);
        assert_eq!(p1.len(), 1);
        assert_eq!(p1[0].name, "p1_script");
    }

    #[test]
    fn find_by_name() {
        let mut registry = ScriptRegistry::default();
        registry.register_lua_script("abc", "local x = 1", 0);

        assert!(registry.find("abc").is_some());
        assert!(registry.find("xyz").is_none());
    }

    #[test]
    fn register_from_source_manual_mode() {
        let mut registry = ScriptRegistry::default();
        let source = "-- @name: retreat\n-- @manual\nctx:stop({})";
        registry.register_from_source(source, "fallback", 0);

        assert_eq!(registry.scripts[0].activation_mode, ActivationMode::Manual);
    }
}
