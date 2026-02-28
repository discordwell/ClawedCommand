use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{
    AttackMoveTarget, AttackStats, AttackTarget, AttackType, AttackTypeMarker, Building,
    BuildingKind, ChasingTarget, Dead, Gathering, Health, MoveTarget, MovementSpeed, Owner, Path,
    Position, ProductionQueue, ResourceDeposit, ResourceType, UnitKind, UnitType,
    UnderConstruction,
};
use cc_core::coords::{GridPos, WorldPos};
use cc_core::math::Fixed;
use cc_sim::resources::{PlayerResourceState, PlayerResources};

/// Read-only snapshot of a unit's state. Pure data, no ECS references.
#[derive(Debug, Clone)]
pub struct UnitSnapshot {
    pub id: EntityId,
    pub kind: UnitKind,
    pub pos: GridPos,
    pub world_pos: WorldPos,
    pub owner: u8,
    pub health_current: Fixed,
    pub health_max: Fixed,
    pub speed: Fixed,
    pub attack_damage: Fixed,
    pub attack_range: Fixed,
    pub attack_speed: u32,
    pub attack_type: AttackType,
    pub is_moving: bool,
    pub is_attacking: bool,
    pub is_idle: bool,
    pub is_dead: bool,
    pub is_gathering: bool,
}

/// Read-only snapshot of a building's state.
#[derive(Debug, Clone)]
pub struct BuildingSnapshot {
    pub id: EntityId,
    pub kind: BuildingKind,
    pub pos: GridPos,
    pub owner: u8,
    pub health_current: Fixed,
    pub health_max: Fixed,
    pub under_construction: bool,
    pub construction_progress: f32,
    pub production_queue: Vec<UnitKind>,
}

/// Read-only snapshot of a resource deposit.
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub id: EntityId,
    pub resource_type: ResourceType,
    pub pos: GridPos,
    pub remaining: u32,
}

/// Complete game state snapshot for a specific player's perspective.
#[derive(Debug, Clone)]
pub struct GameStateSnapshot {
    pub tick: u64,
    pub map_width: u32,
    pub map_height: u32,
    pub player_id: u8,
    pub my_units: Vec<UnitSnapshot>,
    pub enemy_units: Vec<UnitSnapshot>,
    pub my_buildings: Vec<BuildingSnapshot>,
    pub enemy_buildings: Vec<BuildingSnapshot>,
    pub resource_deposits: Vec<ResourceSnapshot>,
    pub my_resources: PlayerResourceState,
}

impl GameStateSnapshot {
    /// Find a unit by EntityId across both my_units and enemy_units.
    pub fn unit_by_id(&self, id: EntityId) -> Option<&UnitSnapshot> {
        self.my_units
            .iter()
            .chain(self.enemy_units.iter())
            .find(|u| u.id == id)
    }

    /// Find a building by EntityId across both my and enemy buildings.
    pub fn building_by_id(&self, id: EntityId) -> Option<&BuildingSnapshot> {
        self.my_buildings
            .iter()
            .chain(self.enemy_buildings.iter())
            .find(|b| b.id == id)
    }
}

/// Build a complete game state snapshot from the ECS World for a given player.
pub fn build_snapshot(
    tick: u64,
    map_width: u32,
    map_height: u32,
    player_id: u8,
    player_resources: &PlayerResources,
    units: &[(Entity, &Position, &Owner, &UnitType, &Health, &MovementSpeed,
              Option<&AttackStats>, Option<&AttackTypeMarker>,
              Option<&MoveTarget>, Option<&AttackTarget>, Option<&Path>,
              Option<&Gathering>, Option<&ChasingTarget>,
              Option<&AttackMoveTarget>, Option<&Dead>)],
    buildings: &[(Entity, &Position, &Owner, &Building, &Health,
                  Option<&UnderConstruction>, Option<&ProductionQueue>)],
    deposits: &[(Entity, &Position, &ResourceDeposit)],
) -> GameStateSnapshot {
    let mut my_units = Vec::new();
    let mut enemy_units = Vec::new();

    for &(entity, pos, owner, unit_type, health, speed,
          attack_stats, attack_type_marker,
          move_target, attack_target, path,
          gathering, chasing, attack_move, dead) in units
    {
        let is_moving = move_target.is_some() || path.is_some() || chasing.is_some();
        let is_attacking = attack_target.is_some() || attack_move.is_some();
        let is_dead = dead.is_some();
        let is_idle = !is_moving && !is_attacking && !is_dead && gathering.is_none();

        let (atk_damage, atk_range, atk_speed) = attack_stats
            .map(|s| (s.damage, s.range, s.attack_speed))
            .unwrap_or((Fixed::ZERO, Fixed::ZERO, 0));

        let atk_type = attack_type_marker
            .map(|m| m.attack_type)
            .unwrap_or(AttackType::Melee);

        let snap = UnitSnapshot {
            id: EntityId(entity.to_bits()),
            kind: unit_type.kind,
            pos: pos.world.to_grid(),
            world_pos: pos.world,
            owner: owner.player_id,
            health_current: health.current,
            health_max: health.max,
            speed: speed.speed,
            attack_damage: atk_damage,
            attack_range: atk_range,
            attack_speed: atk_speed,
            attack_type: atk_type,
            is_moving,
            is_attacking,
            is_idle,
            is_dead,
            is_gathering: gathering.is_some(),
        };

        if owner.player_id == player_id {
            my_units.push(snap);
        } else {
            enemy_units.push(snap);
        }
    }

    let mut my_buildings = Vec::new();
    let mut enemy_buildings = Vec::new();

    for &(entity, pos, owner, building, health, under_construction, production_queue) in buildings {
        let (is_constructing, progress) = under_construction
            .map(|uc| {
                let total = uc.total_ticks as f32;
                let remaining = uc.remaining_ticks as f32;
                (true, if total > 0.0 { 1.0 - remaining / total } else { 1.0 })
            })
            .unwrap_or((false, 1.0));

        let queue = production_queue
            .map(|pq| pq.queue.iter().map(|(kind, _)| *kind).collect())
            .unwrap_or_default();

        let snap = BuildingSnapshot {
            id: EntityId(entity.to_bits()),
            kind: building.kind,
            pos: pos.world.to_grid(),
            owner: owner.player_id,
            health_current: health.current,
            health_max: health.max,
            under_construction: is_constructing,
            construction_progress: progress,
            production_queue: queue,
        };

        if owner.player_id == player_id {
            my_buildings.push(snap);
        } else {
            enemy_buildings.push(snap);
        }
    }

    let resource_deposits: Vec<ResourceSnapshot> = deposits
        .iter()
        .map(|&(entity, pos, deposit)| ResourceSnapshot {
            id: EntityId(entity.to_bits()),
            resource_type: deposit.resource_type,
            pos: pos.world.to_grid(),
            remaining: deposit.remaining,
        })
        .collect();

    let my_resources = player_resources
        .players
        .get(player_id as usize)
        .cloned()
        .unwrap_or_default();

    GameStateSnapshot {
        tick,
        map_width,
        map_height,
        player_id,
        my_units,
        enemy_units,
        my_buildings,
        enemy_buildings,
        resource_deposits,
        my_resources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::make_unit as make_unit_snapshot;

    #[test]
    fn snapshot_unit_by_id_finds_own_units() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![make_unit_snapshot(1, UnitKind::Hisser, 5, 5, 0)],
            enemy_units: vec![make_unit_snapshot(2, UnitKind::Chonk, 10, 10, 1)],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };

        assert!(snap.unit_by_id(EntityId(1)).is_some());
        assert_eq!(snap.unit_by_id(EntityId(1)).unwrap().kind, UnitKind::Hisser);
    }

    #[test]
    fn snapshot_unit_by_id_finds_enemy_units() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![make_unit_snapshot(1, UnitKind::Hisser, 5, 5, 0)],
            enemy_units: vec![make_unit_snapshot(2, UnitKind::Chonk, 10, 10, 1)],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };

        assert!(snap.unit_by_id(EntityId(2)).is_some());
        assert_eq!(snap.unit_by_id(EntityId(2)).unwrap().kind, UnitKind::Chonk);
    }

    #[test]
    fn snapshot_unit_by_id_returns_none_for_missing() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };

        assert!(snap.unit_by_id(EntityId(999)).is_none());
    }

    #[test]
    fn unit_idle_detection() {
        let mut unit = make_unit_snapshot(1, UnitKind::Pawdler, 0, 0, 0);
        assert!(unit.is_idle);

        unit.is_moving = true;
        unit.is_idle = false;
        assert!(!unit.is_idle);
    }
}
