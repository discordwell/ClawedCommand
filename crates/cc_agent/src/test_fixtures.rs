use cc_core::commands::EntityId;
use cc_core::components::{AttackType, UnitKind};
use cc_core::coords::{GridPos, WorldPos};
use cc_core::math::fixed_from_i32;
use cc_core::unit_stats::base_stats;
use cc_sim::resources::PlayerResourceState;

use crate::snapshot::{GameStateSnapshot, UnitSnapshot};

/// Full-params unit constructor for tests.
/// Uses the unit's canonical attack_type from base_stats.
pub fn make_unit(id: u64, kind: UnitKind, x: i32, y: i32, owner: u8) -> UnitSnapshot {
    let stats = base_stats(kind);
    UnitSnapshot {
        id: EntityId(id),
        kind,
        pos: GridPos::new(x, y),
        world_pos: WorldPos::from_grid(GridPos::new(x, y)),
        owner,
        health_current: fixed_from_i32(100),
        health_max: fixed_from_i32(100),
        speed: fixed_from_i32(1),
        attack_damage: fixed_from_i32(10),
        attack_range: fixed_from_i32(5),
        attack_speed: 10,
        attack_type: stats.attack_type,
        is_moving: false,
        is_attacking: false,
        is_idle: true,
        is_dead: false,
        is_gathering: false,
        status_effects: vec![],
        abilities: vec![],
    }
}

/// Simple unit constructor — defaults to Hisser, owner 0.
pub fn make_unit_simple(id: u64, x: i32, y: i32) -> UnitSnapshot {
    make_unit(id, UnitKind::Hisser, x, y, 0)
}

/// Unit constructor with owner — defaults to Hisser.
pub fn make_unit_owned(id: u64, x: i32, y: i32, owner: u8) -> UnitSnapshot {
    make_unit(id, UnitKind::Hisser, x, y, owner)
}

/// Minimal snapshot with my_units and enemy_units, everything else empty/default.
pub fn make_snapshot(my_units: Vec<UnitSnapshot>, enemy_units: Vec<UnitSnapshot>) -> GameStateSnapshot {
    GameStateSnapshot {
        tick: 0,
        map_width: 64,
        map_height: 64,
        player_id: 0,
        my_units,
        enemy_units,
        my_buildings: vec![],
        enemy_buildings: vec![],
        resource_deposits: vec![],
        my_resources: PlayerResourceState::default(),
    }
}
