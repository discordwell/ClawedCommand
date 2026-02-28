//! Game state snapshots for the wet test harness.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use cc_core::components::*;

use crate::ai::MultiAiState;
use crate::resources::PlayerResources;

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameStateSnapshot {
    pub tick: u64,
    pub players: Vec<PlayerSnapshot>,
    pub units: Vec<UnitSnapshot>,
    pub buildings: Vec<BuildingSnapshot>,
    pub projectile_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerSnapshot {
    pub player_id: u8,
    pub food: u32,
    pub gpu_cores: u32,
    pub nfts: u32,
    pub supply: u32,
    pub supply_cap: u32,
    pub unit_count: u32,
    pub building_count: u32,
    pub ai_phase: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnitSnapshot {
    pub entity_bits: u64,
    pub kind: String,
    pub owner: u8,
    pub grid_x: i32,
    pub grid_y: i32,
    pub health_current: f32,
    pub health_max: f32,
    pub is_dead: bool,
    pub has_move_target: bool,
    pub has_attack_target: bool,
    pub is_gathering: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BuildingSnapshot {
    pub entity_bits: u64,
    pub kind: String,
    pub owner: u8,
    pub health_current: f32,
    pub health_max: f32,
    pub is_under_construction: bool,
    pub queue_length: u32,
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Capture a full game state snapshot from the world.
pub fn capture_snapshot(world: &mut World, tick: u64) -> GameStateSnapshot {
    // Player resources
    let player_res = world.resource::<PlayerResources>();
    let multi_ai = world.get_resource::<MultiAiState>();

    let mut players = Vec::new();
    for (i, pres) in player_res.players.iter().enumerate() {
        let ai_phase = multi_ai
            .and_then(|mai| mai.players.get(i))
            .map(|ai| format!("{:?}", ai.phase))
            .unwrap_or_else(|| "Unknown".into());

        players.push(PlayerSnapshot {
            player_id: i as u8,
            food: pres.food,
            gpu_cores: pres.gpu_cores,
            nfts: pres.nfts,
            supply: pres.supply,
            supply_cap: pres.supply_cap,
            unit_count: 0, // filled below
            building_count: 0,
            ai_phase,
        });
    }

    // Units
    let mut units = Vec::new();
    for (entity, pos, owner, ut, health, dead, move_target, attack_target, gathering) in world
        .query::<(
            Entity,
            &cc_core::components::Position,
            &Owner,
            &UnitType,
            &Health,
            Option<&Dead>,
            Option<&MoveTarget>,
            Option<&AttackTarget>,
            Option<&Gathering>,
        )>()
        .iter(world)
    {
        let grid = pos.world.to_grid();
        units.push(UnitSnapshot {
            entity_bits: entity.to_bits(),
            kind: format!("{:?}", ut.kind),
            owner: owner.player_id,
            grid_x: grid.x,
            grid_y: grid.y,
            health_current: health.current.to_num::<f32>(),
            health_max: health.max.to_num::<f32>(),
            is_dead: dead.is_some(),
            has_move_target: move_target.is_some(),
            has_attack_target: attack_target.is_some(),
            is_gathering: gathering.is_some(),
        });

        // Count per-player
        if let Some(ps) = players.get_mut(owner.player_id as usize) {
            ps.unit_count += 1;
        }
    }

    // Buildings
    let mut buildings = Vec::new();
    for (entity, owner, building, health, uc, queue) in world
        .query::<(
            Entity,
            &Owner,
            &Building,
            &Health,
            Option<&UnderConstruction>,
            Option<&ProductionQueue>,
        )>()
        .iter(world)
    {
        buildings.push(BuildingSnapshot {
            entity_bits: entity.to_bits(),
            kind: format!("{:?}", building.kind),
            owner: owner.player_id,
            health_current: health.current.to_num::<f32>(),
            health_max: health.max.to_num::<f32>(),
            is_under_construction: uc.is_some(),
            queue_length: queue.map(|q| q.queue.len() as u32).unwrap_or(0),
        });

        if let Some(ps) = players.get_mut(owner.player_id as usize) {
            ps.building_count += 1;
        }
    }

    // Projectiles
    let projectile_count = world
        .query::<&Projectile>()
        .iter(world)
        .count() as u32;

    GameStateSnapshot {
        tick,
        players,
        units,
        buildings,
        projectile_count,
    }
}
