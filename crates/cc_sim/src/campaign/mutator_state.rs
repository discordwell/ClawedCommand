use bevy::prelude::*;

use cc_core::components::UnitKind;
use cc_core::math::{FIXED_ONE, Fixed};

/// Runtime state for active mutators during a mission.
#[derive(Resource, Default)]
pub struct MutatorState {
    /// Per-mutator active flags (index matches MissionDefinition.mutators).
    pub active: Vec<bool>,
    /// How many rows of lava have advanced.
    pub lava_advance_count: u32,
    /// How many rings of toxic tide have advanced.
    pub toxic_advance_count: u32,
    /// Current water level for flooding.
    pub current_water_level: u8,
    /// Whether wind is currently active (within a gust window).
    pub wind_active: bool,
    /// Whether fog is currently cleared (during a periodic clearing window).
    pub fog_cleared: bool,
    /// Whether the time limit warning has already fired (prevents duplicates).
    pub time_warning_fired: bool,
}

impl MutatorState {
    /// Check if a mutator at the given index is active.
    pub fn is_active(&self, index: usize) -> bool {
        self.active.get(index).copied().unwrap_or(false)
    }
}

/// Check if a periodic hazard should fire this tick, accounting for initial delay and interval.
/// Returns false if `interval_ticks` is zero (prevents division-by-zero).
pub fn should_fire(tick: u64, initial_delay_ticks: u64, interval_ticks: u64) -> bool {
    if interval_ticks == 0 || tick < initial_delay_ticks {
        return false;
    }
    (tick - initial_delay_ticks).is_multiple_of(interval_ticks)
}

/// Control restrictions derived from mutators — checked by command filtering
/// and input systems to gate what the player can do.
#[derive(Resource, Clone, Debug)]
pub struct ControlRestrictions {
    /// Whether mouse/keyboard unit commands are allowed.
    pub mouse_keyboard_enabled: bool,
    /// Whether voice commands are allowed.
    pub voice_enabled: bool,
    /// Whether AI agent commands are allowed.
    pub ai_enabled: bool,
    /// If set, only these unit kinds can be commanded/trained.
    pub allowed_unit_kinds: Option<Vec<UnitKind>>,
    /// Maximum number of units the player can have.
    pub max_unit_count: Option<u32>,
    /// Whether building placement is allowed.
    pub building_enabled: bool,
    /// Multiplier applied to enemy stats (higher = harder).
    pub enemy_difficulty_multiplier: Fixed,
}

impl Default for ControlRestrictions {
    fn default() -> Self {
        Self {
            mouse_keyboard_enabled: true,
            voice_enabled: true,
            ai_enabled: true,
            allowed_unit_kinds: None,
            max_unit_count: None,
            building_enabled: true,
            enemy_difficulty_multiplier: FIXED_ONE,
        }
    }
}

/// Vision modifier set by DenseFog mutator — consumed by client rendering.
#[derive(Resource, Default)]
pub struct FogState {
    /// Vision range reduction in tiles (0 = no fog).
    pub vision_reduction: u32,
    /// Whether fog is currently cleared by periodic clearing.
    pub currently_clear: bool,
}
