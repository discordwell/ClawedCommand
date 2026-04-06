//! Lua bindings for the Strait dream sequence (DEFCON-style drone warfare).
//!
//! Provides the `ctx.strait:` table with drone patrol, satellite scan,
//! compute allocation, zero-day, and tanker status queries.
//!
//! These bindings are registered conditionally when the current mission
//! is a Strait dream sequence.

use cc_core::strait::{ComputeAllocation, DroneMode, PatriotMode, ZeroDayState, ZeroDayType};

/// Strait-specific state snapshot passed into Lua closures.
/// Captures the current state at the time of script execution.
#[derive(Debug, Clone)]
pub struct StraitSnapshot {
    // -- Compute flow --
    pub allocation: ComputeAllocation,
    pub satellite_focal: Option<(i32, i32)>,

    // -- Logistics charges --
    pub airstrike_charges: u32,
    pub airstrike_max_charges: u32,
    pub drone_rebuild_charges: u32,
    pub drone_rebuild_max_charges: u32,

    // -- Patriots + base --
    pub patriot_count: u32,
    pub patriot_mode: PatriotMode,
    pub base_hp: u32,

    // -- Convoy --
    pub convoy_hold: bool,
    pub tankers_arrived: u32,
    pub tankers_destroyed: u32,
    pub tankers_spawned: u32,
    pub total_tankers: u32,

    // -- Drones --
    pub drones_alive: u32,
    pub drone_positions: Vec<DroneInfo>,

    // -- Zero-day --
    pub zero_day_slot: ZeroDayState,

    // -- Mission --
    pub mission_tick: u64,
    pub mission_complete: bool,

    // -- Enemies --
    pub tanker_positions: Vec<TankerInfo>,
    pub visible_enemies: Vec<EnemyInfo>,
    pub incoming_shaheeds: Vec<ShaheedInfo>,
    pub incoming_missiles: Vec<MissileInfo>,
}

#[derive(Debug, Clone)]
pub struct DroneInfo {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub alive: bool,
    pub mode: String,
    pub flare_cooldown: u32,
    pub bomb_ready: bool,
}

#[derive(Debug, Clone)]
pub struct TankerInfo {
    pub index: u32,
    pub x: f32,
    pub y: i32,
    pub hp: u32,
    pub arrived: bool,
    pub destroyed: bool,
}

#[derive(Debug, Clone)]
pub struct EnemyInfo {
    pub kind: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct ShaheedInfo {
    pub entity_id: u32,
    pub x: f32,
    pub y: f32,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct MissileInfo {
    pub x: f32,
    pub y: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub progress: f32,
}

/// Commands produced by strait Lua bindings, to be applied back to ECS.
#[derive(Debug, Clone)]
pub enum StraitCommand {
    // -- Existing --
    SetPatrol { drone_id: u32, waypoints: Vec<(i32, i32)> },
    SetSatelliteFocal { x: i32, y: i32 },
    AllocateCompute(ComputeAllocation),
    BuildZeroDay(ZeroDayType),
    DeployZeroDay { exploit_type: ZeroDayType, target_x: i32, target_y: i32 },

    // -- V2: drone commands --
    DroneBomb { drone_id: u32, target_x: i32, target_y: i32 },
    DroneGuardBase { drone_id: u32 },
    DroneMoveTo { drone_id: u32, x: i32, y: i32 },

    // -- V2: logistics --
    CallAirstrike { x: i32, y: i32 },
    RebuildDrone,

    // -- V2: convoy --
    LaunchAllBoats,

    // -- V2: patriots --
    SetPatriotMode { missiles_only: bool },
}

/// Parse a zero-day type from a string.
pub fn parse_zero_day_type(s: &str) -> Option<ZeroDayType> {
    match s.to_lowercase().as_str() {
        "spoof" => Some(ZeroDayType::Spoof),
        "blind" => Some(ZeroDayType::Blind),
        "hijack" => Some(ZeroDayType::Hijack),
        "brick" => Some(ZeroDayType::Brick),
        _ => None,
    }
}

/// Format a zero-day state for Lua.
pub fn zero_day_state_string(state: &ZeroDayState) -> &'static str {
    match state {
        ZeroDayState::Idle => "idle",
        ZeroDayState::Building { .. } => "building",
        ZeroDayState::Ready(_) => "ready",
    }
}

/// Format a drone mode for Lua.
pub fn drone_mode_string(mode: &DroneMode) -> &'static str {
    match mode {
        DroneMode::Patrol => "patrol",
        DroneMode::MoveTo { .. } => "move_to",
        DroneMode::BombTarget { .. } => "bomb_target",
        DroneMode::GuardBase => "guard_base",
    }
}

/// Format a patriot mode for Lua.
pub fn patriot_mode_string(mode: &PatriotMode) -> &'static str {
    match mode {
        PatriotMode::Auto => "auto",
        PatriotMode::MissilesOnly => "missiles_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zero_day_types() {
        assert_eq!(parse_zero_day_type("spoof"), Some(ZeroDayType::Spoof));
        assert_eq!(parse_zero_day_type("BLIND"), Some(ZeroDayType::Blind));
        assert_eq!(parse_zero_day_type("Hijack"), Some(ZeroDayType::Hijack));
        assert_eq!(parse_zero_day_type("brick"), Some(ZeroDayType::Brick));
        assert_eq!(parse_zero_day_type("unknown"), None);
    }

    #[test]
    fn zero_day_state_strings() {
        assert_eq!(zero_day_state_string(&ZeroDayState::Idle), "idle");
        assert_eq!(
            zero_day_state_string(&ZeroDayState::Building {
                exploit_type: ZeroDayType::Spoof,
                progress: 50.0,
                required: 80.0,
            }),
            "building"
        );
        assert_eq!(
            zero_day_state_string(&ZeroDayState::Ready(ZeroDayType::Brick)),
            "ready"
        );
    }

    #[test]
    fn drone_mode_strings() {
        assert_eq!(drone_mode_string(&DroneMode::Patrol), "patrol");
        assert_eq!(drone_mode_string(&DroneMode::GuardBase), "guard_base");
        assert_eq!(drone_mode_string(&DroneMode::BombTarget { x: 1.0, y: 2.0 }), "bomb_target");
    }

    #[test]
    fn strait_snapshot_captures_v2_state() {
        let snapshot = StraitSnapshot {
            allocation: ComputeAllocation::default(),
            satellite_focal: None,
            airstrike_charges: 2,
            airstrike_max_charges: 3,
            drone_rebuild_charges: 1,
            drone_rebuild_max_charges: 2,
            patriot_count: 18,
            patriot_mode: PatriotMode::Auto,
            base_hp: 10,
            convoy_hold: true,
            tankers_arrived: 0,
            tankers_destroyed: 0,
            tankers_spawned: 0,
            total_tankers: 12,
            drones_alive: 16,
            drone_positions: vec![DroneInfo {
                id: 0, x: 10.0, y: 8.0, alive: true,
                mode: "patrol".into(), flare_cooldown: 0, bomb_ready: true,
            }],
            zero_day_slot: ZeroDayState::Idle,
            mission_tick: 500,
            mission_complete: false,
            tanker_positions: vec![],
            visible_enemies: vec![],
            incoming_shaheeds: vec![],
            incoming_missiles: vec![],
        };

        assert_eq!(snapshot.drone_positions.len(), 1);
        assert_eq!(snapshot.patriot_count, 18);
        assert!(snapshot.convoy_hold);
    }
}
