//! Tool tier system for progressive AI capability unlocking.
//!
//! Tiers are cumulative: Strategic includes Basic + Tactical + Strategic.
//!
//! In multiplayer, tier = completed ServerRack count (0→Basic, 1→Tactical, 2→Strategic, 3+→Advanced).
//! In campaign, tier is set per-mission via MissionDefinition.ai_tool_tier.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::llm_client::ToolDef;

/// Progressive unlock tiers for AI tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolTier {
    /// Raw commands + worker management. Prologue / Act 1 early / 0 ServerRacks.
    Basic = 0,
    /// Combat behaviors (kite, focus fire, retreat). Act 1 mid / 1 ServerRack.
    Tactical = 1,
    /// Multi-squad + economy automation. Act 2+ / 2 ServerRacks.
    Strategic = 2,
    /// Adaptive AI + research prioritization. Act 3+ / 3+ ServerRacks.
    Advanced = 3,
}

impl ToolTier {
    /// Convert a ServerRack count to the corresponding tier.
    pub fn from_rack_count(racks: u32) -> Self {
        match racks {
            0 => ToolTier::Basic,
            1 => ToolTier::Tactical,
            2 => ToolTier::Strategic,
            _ => ToolTier::Advanced,
        }
    }
}

/// Categorizes what kind of tool this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Query,
    Command,
    Behavior,
    Economy,
    Ability,
}

/// A single tool entry in the registry.
#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub name: String,
    pub tier: ToolTier,
    pub category: ToolCategory,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Registry of all tools with tier assignments.
#[derive(Resource, Clone)]
pub struct ToolRegistry {
    entries: Vec<ToolEntry>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::build_default()
    }
}

impl ToolRegistry {
    /// Build the default registry with all tools and their tier assignments.
    pub fn build_default() -> Self {
        let mut entries = Vec::new();

        // =====================================================================
        // Basic (Tier 0) — Always available
        // =====================================================================

        // Queries
        entries.push(ToolEntry {
            name: "get_units".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Query,
            description: "Get all units owned by this player, with positions, health, type, and status".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": {"type": "string", "description": "Optional unit type filter (e.g. 'Hisser', 'Chonk')"}
                }
            }),
        });
        entries.push(ToolEntry {
            name: "get_buildings".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Query,
            description: "Get all buildings owned by this player".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        });
        entries.push(ToolEntry {
            name: "get_visible_enemies".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Query,
            description: "Get all visible enemy units with positions, health, and type".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        });
        entries.push(ToolEntry {
            name: "get_resources".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Query,
            description: "Get current resource amounts (food, GPU cores, NFTs, supply)".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        });
        entries.push(ToolEntry {
            name: "get_map_info".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Query,
            description: "Get map dimensions, tick, and resource deposit locations".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        });

        // Commands
        entries.push(ToolEntry {
            name: "move_units".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Move units to a position".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"}
                },
                "required": ["unit_ids", "x", "y"]
            }),
        });
        entries.push(ToolEntry {
            name: "attack_units".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Attack a target unit".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "target_id": {"type": "integer"}
                },
                "required": ["unit_ids", "target_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "build".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Build a structure".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "builder_id": {"type": "integer"},
                    "building_type": {"type": "string"},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"}
                },
                "required": ["builder_id", "building_type", "x", "y"]
            }),
        });
        entries.push(ToolEntry {
            name: "train_unit".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Train a unit from a building".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"},
                    "unit_type": {"type": "string"}
                },
                "required": ["building_id", "unit_type"]
            }),
        });
        entries.push(ToolEntry {
            name: "gather_resource".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Send workers to gather from a deposit".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "deposit_id": {"type": "integer"}
                },
                "required": ["unit_ids", "deposit_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "set_rally_point".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Set rally point for a building".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"}
                },
                "required": ["building_id", "x", "y"]
            }),
        });
        entries.push(ToolEntry {
            name: "patrol".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Patrol / attack-move to a position".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"}
                },
                "required": ["unit_ids", "x", "y"]
            }),
        });
        entries.push(ToolEntry {
            name: "stop".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Stop units immediately".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["unit_ids"]
            }),
        });
        entries.push(ToolEntry {
            name: "hold_position".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Command,
            description: "Hold position: attack enemies in range only, no chasing".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["unit_ids"]
            }),
        });

        // Basic behaviors
        entries.push(ToolEntry {
            name: "assign_idle_workers".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Economy,
            description: "Send idle Pawdlers to nearest resource deposit".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        });
        entries.push(ToolEntry {
            name: "attack_move_group".into(),
            tier: ToolTier::Basic,
            category: ToolCategory::Behavior,
            description: "Group attack-move with ranged behind melee".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"}
                },
                "required": ["unit_ids", "x", "y"]
            }),
        });

        // =====================================================================
        // Tactical (Tier 1) — 1 ServerRack / mid Act 1
        // =====================================================================

        entries.push(ToolEntry {
            name: "focus_fire".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "All attackers focus fire on the same target".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "attacker_ids": {"type": "array", "items": {"type": "integer"}},
                    "target_id": {"type": "integer"}
                },
                "required": ["attacker_ids", "target_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "focus_weakest".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Find weakest enemy in range, then focus fire all on it".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "range": {"type": "number"}
                },
                "required": ["unit_ids", "range"]
            }),
        });
        entries.push(ToolEntry {
            name: "kite_squad".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Ranged units maintain attack range from nearest enemy".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["unit_ids"]
            }),
        });
        entries.push(ToolEntry {
            name: "retreat_wounded".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Move units below HP% threshold to safe positions".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "threshold": {"type": "number", "description": "HP percentage threshold 0.0-1.0"}
                },
                "required": ["threshold"]
            }),
        });
        entries.push(ToolEntry {
            name: "defend_area".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Attack enemies inside radius, hold position otherwise".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"},
                    "radius": {"type": "number"}
                },
                "required": ["unit_ids", "x", "y", "radius"]
            }),
        });
        entries.push(ToolEntry {
            name: "scout_pattern".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Move scout to nearest unvisited waypoint".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "scout_id": {"type": "integer"},
                    "waypoints": {"type": "array", "items": {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}}}}
                },
                "required": ["scout_id", "waypoints"]
            }),
        });
        entries.push(ToolEntry {
            name: "harass_economy".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Attack enemy workers, or attack-move toward buildings if none visible".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "raider_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["raider_ids"]
            }),
        });
        entries.push(ToolEntry {
            name: "use_ability".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Ability,
            description: "Smart ability activation (checks cooldown, GPU cost, range)".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_id": {"type": "integer"},
                    "slot": {"type": "integer", "description": "Ability slot 0-2"},
                    "target_type": {"type": "string", "description": "'self', 'position', or 'entity'"},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"},
                    "target_id": {"type": "integer"}
                },
                "required": ["unit_id", "slot", "target_type"]
            }),
        });
        entries.push(ToolEntry {
            name: "split_squads".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Categorize units into melee/ranged/support groups".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}}
                },
                "required": ["unit_ids"]
            }),
        });
        entries.push(ToolEntry {
            name: "protect_unit".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Escort units stay near a VIP, engage threats in range".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "escort_ids": {"type": "array", "items": {"type": "integer"}},
                    "vip_id": {"type": "integer"},
                    "guard_radius": {"type": "number"}
                },
                "required": ["escort_ids", "vip_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "surround_target".into(),
            tier: ToolTier::Tactical,
            category: ToolCategory::Behavior,
            description: "Position units in ring around enemy, then attack".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "target_id": {"type": "integer"},
                    "ring_radius": {"type": "number"}
                },
                "required": ["unit_ids", "target_id"]
            }),
        });

        // =====================================================================
        // Strategic (Tier 2) — 2 ServerRacks / Act 2+
        // =====================================================================

        entries.push(ToolEntry {
            name: "auto_produce".into(),
            tier: ToolTier::Strategic,
            category: ToolCategory::Economy,
            description: "Check resources vs unit costs, train if affordable".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"},
                    "unit_type": {"type": "string"}
                },
                "required": ["building_id", "unit_type"]
            }),
        });
        entries.push(ToolEntry {
            name: "balanced_production".into(),
            tier: ToolTier::Strategic,
            category: ToolCategory::Economy,
            description: "Analyze army composition, auto-queue missing unit types".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"}
                },
                "required": ["building_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "expand_economy".into(),
            tier: ToolTier::Strategic,
            category: ToolCategory::Economy,
            description: "Build FishMarkets near deposits, LitterBoxes for supply".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "builder_id": {"type": "integer"}
                },
                "required": ["builder_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "coordinate_assault".into(),
            tier: ToolTier::Strategic,
            category: ToolCategory::Behavior,
            description: "Split army into main force + flanking group attack".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "target_x": {"type": "integer"},
                    "target_y": {"type": "integer"}
                },
                "required": ["unit_ids", "target_x", "target_y"]
            }),
        });

        // =====================================================================
        // Advanced (Tier 3) — 3+ ServerRacks / Act 3+
        // =====================================================================

        entries.push(ToolEntry {
            name: "research_priority".into(),
            tier: ToolTier::Advanced,
            category: ToolCategory::Ability,
            description: "Evaluate and auto-queue best upgrade at a building".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"}
                },
                "required": ["building_id"]
            }),
        });
        entries.push(ToolEntry {
            name: "adaptive_defense".into(),
            tier: ToolTier::Advanced,
            category: ToolCategory::Behavior,
            description: "Position defenses based on enemy composition + terrain".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": {"type": "array", "items": {"type": "integer"}},
                    "center_x": {"type": "integer"},
                    "center_y": {"type": "integer"},
                    "radius": {"type": "number"}
                },
                "required": ["unit_ids", "center_x", "center_y", "radius"]
            }),
        });

        Self { entries }
    }

    /// Return all tools at or below the given tier.
    pub fn tools_for_tier(&self, tier: ToolTier) -> Vec<&ToolEntry> {
        self.entries.iter().filter(|e| e.tier <= tier).collect()
    }

    /// Check if a tool is available at a given tier.
    pub fn is_available(&self, name: &str, tier: ToolTier) -> bool {
        self.entries
            .iter()
            .any(|e| e.name == name && e.tier <= tier)
    }

    /// Return MCP-format ToolDef list filtered by tier.
    pub fn tool_definitions_for_tier(&self, tier: ToolTier) -> Vec<ToolDef> {
        self.tools_for_tier(tier)
            .iter()
            .map(|e| ToolDef {
                name: e.name.clone(),
                description: e.description.clone(),
                parameters: e.parameters.clone(),
            })
            .collect()
    }

    /// Get all tool names at a given tier (inclusive of lower tiers).
    pub fn tool_names_for_tier(&self, tier: ToolTier) -> Vec<&str> {
        self.tools_for_tier(tier)
            .iter()
            .map(|e| e.name.as_str())
            .collect()
    }

    /// Get the required tier for a named tool, or None if unknown.
    pub fn required_tier(&self, name: &str) -> Option<ToolTier> {
        self.entries.iter().find(|e| e.name == name).map(|e| e.tier)
    }
}

/// Per-faction tool state, tracking the current tier.
#[derive(Debug, Clone)]
pub struct FactionToolState {
    pub player_id: u8,
    pub current_tier: ToolTier,
    pub server_rack_count: u32,
}

/// Resource: tracks tool tier state for each player.
#[derive(Resource, Default)]
pub struct FactionToolStates {
    pub states: HashMap<u8, FactionToolState>,
}

impl FactionToolStates {
    /// Get the current tier for a player, defaulting to Basic.
    pub fn tier_for(&self, player_id: u8) -> ToolTier {
        self.states
            .get(&player_id)
            .map(|s| s.current_tier)
            .unwrap_or(ToolTier::Basic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_all_tiers() {
        let reg = ToolRegistry::build_default();
        let basic = reg.tools_for_tier(ToolTier::Basic);
        let tactical = reg.tools_for_tier(ToolTier::Tactical);
        let strategic = reg.tools_for_tier(ToolTier::Strategic);
        let advanced = reg.tools_for_tier(ToolTier::Advanced);

        // Basic should have queries + commands + 2 new basic behaviors
        assert!(basic.len() >= 15);
        // Each tier is cumulative
        assert!(tactical.len() > basic.len());
        assert!(strategic.len() > tactical.len());
        assert!(advanced.len() > strategic.len());
        // Advanced should have everything
        assert_eq!(advanced.len(), reg.entries.len());
    }

    #[test]
    fn basic_tools_available_at_basic() {
        let reg = ToolRegistry::build_default();
        assert!(reg.is_available("get_units", ToolTier::Basic));
        assert!(reg.is_available("move_units", ToolTier::Basic));
        assert!(reg.is_available("assign_idle_workers", ToolTier::Basic));
        assert!(reg.is_available("attack_move_group", ToolTier::Basic));
    }

    #[test]
    fn tactical_tools_not_at_basic() {
        let reg = ToolRegistry::build_default();
        assert!(!reg.is_available("focus_fire", ToolTier::Basic));
        assert!(!reg.is_available("kite_squad", ToolTier::Basic));
        assert!(!reg.is_available("retreat_wounded", ToolTier::Basic));
    }

    #[test]
    fn tactical_tools_available_at_tactical() {
        let reg = ToolRegistry::build_default();
        assert!(reg.is_available("focus_fire", ToolTier::Tactical));
        assert!(reg.is_available("kite_squad", ToolTier::Tactical));
        assert!(reg.is_available("use_ability", ToolTier::Tactical));
        assert!(reg.is_available("split_squads", ToolTier::Tactical));
        // Basic still available
        assert!(reg.is_available("get_units", ToolTier::Tactical));
    }

    #[test]
    fn strategic_tools_available_at_strategic() {
        let reg = ToolRegistry::build_default();
        assert!(reg.is_available("auto_produce", ToolTier::Strategic));
        assert!(reg.is_available("balanced_production", ToolTier::Strategic));
        assert!(reg.is_available("coordinate_assault", ToolTier::Strategic));
        // Lower tiers still available
        assert!(reg.is_available("focus_fire", ToolTier::Strategic));
        assert!(reg.is_available("get_units", ToolTier::Strategic));
    }

    #[test]
    fn advanced_tools_only_at_advanced() {
        let reg = ToolRegistry::build_default();
        assert!(!reg.is_available("research_priority", ToolTier::Strategic));
        assert!(!reg.is_available("adaptive_defense", ToolTier::Strategic));
        assert!(reg.is_available("research_priority", ToolTier::Advanced));
        assert!(reg.is_available("adaptive_defense", ToolTier::Advanced));
    }

    #[test]
    fn tool_definitions_filtered_by_tier() {
        let reg = ToolRegistry::build_default();
        let basic_defs = reg.tool_definitions_for_tier(ToolTier::Basic);
        let advanced_defs = reg.tool_definitions_for_tier(ToolTier::Advanced);
        assert!(basic_defs.len() < advanced_defs.len());
        // All basic defs should have names
        assert!(basic_defs.iter().all(|d| !d.name.is_empty()));
    }

    #[test]
    fn rack_count_to_tier() {
        assert_eq!(ToolTier::from_rack_count(0), ToolTier::Basic);
        assert_eq!(ToolTier::from_rack_count(1), ToolTier::Tactical);
        assert_eq!(ToolTier::from_rack_count(2), ToolTier::Strategic);
        assert_eq!(ToolTier::from_rack_count(3), ToolTier::Advanced);
        assert_eq!(ToolTier::from_rack_count(10), ToolTier::Advanced);
    }

    #[test]
    fn faction_tool_states_defaults_to_basic() {
        let states = FactionToolStates::default();
        assert_eq!(states.tier_for(0), ToolTier::Basic);
        assert_eq!(states.tier_for(5), ToolTier::Basic);
    }

    #[test]
    fn required_tier_lookup() {
        let reg = ToolRegistry::build_default();
        assert_eq!(reg.required_tier("get_units"), Some(ToolTier::Basic));
        assert_eq!(reg.required_tier("focus_fire"), Some(ToolTier::Tactical));
        assert_eq!(reg.required_tier("auto_produce"), Some(ToolTier::Strategic));
        assert_eq!(reg.required_tier("research_priority"), Some(ToolTier::Advanced));
        assert_eq!(reg.required_tier("nonexistent"), None);
    }
}
