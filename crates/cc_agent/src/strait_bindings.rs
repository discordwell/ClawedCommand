//! Lua bindings for the Strait dream sequence (DEFCON-style drone warfare).
//!
//! Provides the `ctx.strait:` table with drone patrol, satellite scan,
//! compute allocation, zero-day, and tanker status queries.
//!
//! These bindings are registered conditionally when the current mission
//! is a Strait dream sequence.

use cc_core::strait::{ComputeAllocation, ZeroDayState, ZeroDayType};

/// Strait-specific state snapshot passed into Lua closures.
/// Captures the current state at the time of script execution.
#[derive(Debug, Clone)]
pub struct StraitSnapshot {
    pub compute: f32,
    pub max_compute: f32,
    pub allocation: ComputeAllocation,
    pub interceptor_count: u32,
    pub tankers_arrived: u32,
    pub tankers_destroyed: u32,
    pub tankers_spawned: u32,
    pub drones_alive: u32,
    pub zero_day_slot: ZeroDayState,
    pub mission_tick: u64,
    pub drone_positions: Vec<DroneInfo>,
    pub tanker_positions: Vec<TankerInfo>,
    pub visible_enemies: Vec<EnemyInfo>,
}

#[derive(Debug, Clone)]
pub struct DroneInfo {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub alive: bool,
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

/// Commands produced by strait Lua bindings, to be applied back to ECS.
#[derive(Debug, Clone)]
pub enum StraitCommand {
    /// Set drone patrol waypoints.
    SetPatrol {
        drone_id: u32,
        waypoints: Vec<(i32, i32)>,
    },
    /// Request a satellite scan at a position.
    SatelliteScan {
        x: i32,
        y: i32,
    },
    /// Change compute allocation.
    AllocateCompute(ComputeAllocation),
    /// Start building a zero-day exploit.
    BuildZeroDay(ZeroDayType),
    /// Deploy a ready zero-day at a target.
    DeployZeroDay {
        exploit_type: ZeroDayType,
        target_x: i32,
        target_y: i32,
    },
}

/// Register strait-specific Lua functions.
///
/// In the full integration with `lua_runtime.rs`, this module's
/// `register_strait_bindings` function is called to add a `ctx.strait`
/// sub-table with the following methods:
///
/// - `ctx.strait:my_drones()` — returns table of `{id, x, y, alive}`
/// - `ctx.strait:tanker_status()` — returns table of `{index, x, y, hp, arrived, destroyed}`
/// - `ctx.strait:visible_enemies()` — returns table of `{kind, x, y}`
/// - `ctx.strait:compute_status()` — returns `{compute, max, allocation: {drone_vision, satellite, zero_day}}`
/// - `ctx.strait:interceptor_count()` — returns integer
/// - `ctx.strait:zero_day_status()` — returns `{state, type, progress}` or `{state: "idle"}`
/// - `ctx.strait:set_patrol(drone_id, waypoints)` — set patrol path (produces StraitCommand)
/// - `ctx.strait:satellite_scan(x, y)` — request satellite scan (produces StraitCommand)
/// - `ctx.strait:allocate_compute(table)` — change allocation (produces StraitCommand)
/// - `ctx.strait:build_zero_day(type_str)` — start building exploit (produces StraitCommand)
/// - `ctx.strait:deploy_exploit(type_str, x, y)` — deploy ready exploit (produces StraitCommand)
///
/// The commands are collected into a `Vec<StraitCommand>` that the caller
/// applies back into the Bevy ECS after script execution.

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
    fn strait_snapshot_captures_state() {
        let snapshot = StraitSnapshot {
            compute: 75.0,
            max_compute: 100.0,
            allocation: ComputeAllocation::default(),
            interceptor_count: 10,
            tankers_arrived: 3,
            tankers_destroyed: 1,
            tankers_spawned: 5,
            drones_alive: 6,
            zero_day_slot: ZeroDayState::Idle,
            mission_tick: 500,
            drone_positions: vec![DroneInfo {
                id: 0,
                x: 10.0,
                y: 8.0,
                alive: true,
            }],
            tanker_positions: vec![TankerInfo {
                index: 0,
                x: 25.0,
                y: 10,
                hp: 3,
                arrived: false,
                destroyed: false,
            }],
            visible_enemies: vec![],
        };

        assert_eq!(snapshot.drone_positions.len(), 1);
        assert_eq!(snapshot.tanker_positions[0].hp, 3);
    }
}
