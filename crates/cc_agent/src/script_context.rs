use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{BuildingKind, ResourceType, UnitKind, UpgradeType};
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::math::{Fixed, fixed_from_i32};
use cc_core::terrain::{CoverLevel, FactionId, TerrainType};
use cc_sim::pathfinding;
use cc_sim::resources::PlayerResourceState;

use crate::snapshot::{BuildingSnapshot, GameStateSnapshot, ResourceSnapshot, UnitSnapshot};
use crate::spatial::SpatialIndex;

/// High-level unit state classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitState {
    Moving,
    Attacking,
    Idle,
    Gathering,
}

/// Budget costs for different query types.
const COST_SIMPLE: u32 = 1;
const COST_SPATIAL: u32 = 2;
const COST_PATHFINDING: u32 = 10;
const DEFAULT_BUDGET: u32 = 500;

/// Limits per-invocation query cost to prevent runaway scripts.
pub struct ComputeBudget {
    remaining: u32,
}

impl ComputeBudget {
    pub fn new(budget: u32) -> Self {
        Self { remaining: budget }
    }

    /// Attempt to spend `cost` from the budget. Returns true if affordable.
    pub fn spend(&mut self, cost: u32) -> bool {
        if self.remaining >= cost {
            self.remaining -= cost;
            true
        } else {
            false
        }
    }

    pub fn remaining(&self) -> u32 {
        self.remaining
    }
}

impl Default for ComputeBudget {
    fn default() -> Self {
        Self::new(DEFAULT_BUDGET)
    }
}

/// The rich query API that scripts call into.
/// Wraps a game state snapshot, spatial indices, and map reference.
pub struct ScriptContext<'a> {
    pub state: &'a GameStateSnapshot,
    pub my_spatial: SpatialIndex,
    pub enemy_spatial: SpatialIndex,
    pub map: &'a GameMap,
    pub player_id: u8,
    pub faction: FactionId,
    pub budget: ComputeBudget,
    pub commands: Vec<GameCommand>,
}

impl<'a> ScriptContext<'a> {
    pub fn new(
        state: &'a GameStateSnapshot,
        map: &'a GameMap,
        player_id: u8,
        faction: FactionId,
    ) -> Self {
        let my_spatial = SpatialIndex::build(&state.my_units);
        let enemy_spatial = SpatialIndex::build(&state.enemy_units);
        Self {
            state,
            my_spatial,
            enemy_spatial,
            map,
            player_id,
            faction,
            budget: ComputeBudget::default(),
            commands: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Unit queries
    // -----------------------------------------------------------------------

    /// Get own units, optionally filtered by kind.
    pub fn my_units(&mut self, filter: Option<UnitKind>) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        match filter {
            Some(kind) => self
                .state
                .my_units
                .iter()
                .filter(|u| u.kind == kind && !u.is_dead)
                .collect(),
            None => self
                .state
                .my_units
                .iter()
                .filter(|u| !u.is_dead)
                .collect(),
        }
    }

    /// Get all visible enemy units.
    pub fn enemy_units(&mut self) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state.enemy_units.iter().filter(|u| !u.is_dead).collect()
    }

    /// Find a unit by EntityId (own or enemy).
    pub fn unit_by_id(&mut self, id: EntityId) -> Option<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        self.state.unit_by_id(id)
    }

    /// Get enemies within a Euclidean range of a position (uses Fixed distance_squared).
    pub fn enemies_in_range(&mut self, pos: GridPos, range: Fixed) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return vec![];
        }
        let range_i32: i32 = range.ceil().to_num();
        let range_sq = range * range;
        let world_center = cc_core::coords::WorldPos::from_grid(pos);

        let indices = self.enemy_spatial.units_in_radius(pos, range_i32);
        indices
            .into_iter()
            .filter_map(|idx| {
                let unit = &self.state.enemy_units[idx];
                if unit.is_dead {
                    return None;
                }
                let dist_sq = world_center.distance_squared(unit.world_pos);
                if dist_sq <= range_sq {
                    Some(unit)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get own units within a Euclidean range of a position.
    pub fn allies_in_range(&mut self, pos: GridPos, range: Fixed) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return vec![];
        }
        let range_i32: i32 = range.ceil().to_num();
        let range_sq = range * range;
        let world_center = cc_core::coords::WorldPos::from_grid(pos);

        let indices = self.my_spatial.units_in_radius(pos, range_i32);
        indices
            .into_iter()
            .filter_map(|idx| {
                let unit = &self.state.my_units[idx];
                if unit.is_dead {
                    return None;
                }
                let dist_sq = world_center.distance_squared(unit.world_pos);
                if dist_sq <= range_sq {
                    Some(unit)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find nearest living enemy to a position.
    pub fn nearest_enemy(&mut self, from: GridPos) -> Option<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }
        let search_radius = 32; // max search radius in tiles
        let world_from = cc_core::coords::WorldPos::from_grid(from);
        let max_dist = fixed_from_i32(search_radius);
        let max_dist_sq = max_dist * max_dist;

        let mut best: Option<(usize, Fixed)> = None;
        for (idx, unit) in self.state.enemy_units.iter().enumerate() {
            if unit.is_dead {
                continue;
            }
            let dist_sq = world_from.distance_squared(unit.world_pos);
            if dist_sq > max_dist_sq {
                continue;
            }
            match &best {
                Some((_, best_dist)) if dist_sq >= *best_dist => {}
                _ => best = Some((idx, dist_sq)),
            }
        }

        best.map(|(idx, _)| &self.state.enemy_units[idx])
    }

    /// Find nearest ally to a position, optionally filtered by kind.
    pub fn nearest_ally(
        &mut self,
        from: GridPos,
        kind: Option<UnitKind>,
    ) -> Option<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }
        let search_radius = 32;
        // Can't use spatial.nearest directly with filter, do manual search
        let world_from = cc_core::coords::WorldPos::from_grid(from);
        let mut best: Option<(usize, Fixed)> = None;

        for (idx, unit) in self.state.my_units.iter().enumerate() {
            if unit.is_dead {
                continue;
            }
            if let Some(k) = kind {
                if unit.kind != k {
                    continue;
                }
            }
            let dist_sq = world_from.distance_squared(unit.world_pos);
            let max_dist = fixed_from_i32(search_radius);
            if dist_sq > max_dist * max_dist {
                continue;
            }
            match &best {
                Some((_, best_dist)) if dist_sq >= *best_dist => {}
                _ => best = Some((idx, dist_sq)),
            }
        }

        best.map(|(idx, _)| &self.state.my_units[idx])
    }

    // -----------------------------------------------------------------------
    // Extended unit queries
    // -----------------------------------------------------------------------

    /// Squared distance between two units (by EntityId).
    pub fn distance_squared_between(&mut self, a_id: EntityId, b_id: EntityId) -> Option<Fixed> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        let a = self.state.unit_by_id(a_id)?;
        let b = self.state.unit_by_id(b_id)?;
        Some(a.world_pos.distance_squared(b.world_pos))
    }

    /// Squared distance from a unit to its closest visible enemy.
    pub fn distance_squared_to_nearest_enemy(&mut self, unit_id: EntityId) -> Option<Fixed> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }
        let unit = self.state.unit_by_id(unit_id)?;
        let mut best: Option<Fixed> = None;
        for enemy in &self.state.enemy_units {
            if enemy.is_dead {
                continue;
            }
            let dist_sq = unit.world_pos.distance_squared(enemy.world_pos);
            match best {
                Some(b) if dist_sq >= b => {}
                _ => best = Some(dist_sq),
            }
        }
        best
    }

    /// Own idle units, optionally filtered by kind.
    pub fn idle_units(&mut self, kind: Option<UnitKind>) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state
            .my_units
            .iter()
            .filter(|u| {
                u.is_idle
                    && !u.is_dead
                    && kind.map_or(true, |k| u.kind == k)
            })
            .collect()
    }

    /// Own units below a given HP percentage threshold (0.0–1.0).
    pub fn wounded_units(&mut self, hp_pct_threshold: f64) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state
            .my_units
            .iter()
            .filter(|u| {
                if u.is_dead || u.health_max == Fixed::ZERO {
                    return false;
                }
                let pct: f64 = (u.health_current / u.health_max).to_num();
                pct < hp_pct_threshold
            })
            .collect()
    }

    /// Filter own units by high-level state.
    pub fn units_by_state(&mut self, state: UnitState) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state
            .my_units
            .iter()
            .filter(|u| {
                if u.is_dead {
                    return false;
                }
                match state {
                    UnitState::Moving => u.is_moving,
                    UnitState::Attacking => u.is_attacking,
                    UnitState::Idle => u.is_idle,
                    UnitState::Gathering => u.is_gathering,
                }
            })
            .collect()
    }

    /// Count alive own units, optionally filtered by kind.
    pub fn count_units(&mut self, kind: Option<UnitKind>) -> usize {
        if !self.budget.spend(COST_SIMPLE) {
            return 0;
        }
        self.state
            .my_units
            .iter()
            .filter(|u| !u.is_dead && kind.map_or(true, |k| u.kind == k))
            .count()
    }

    /// Sum of supply cost for all alive own units.
    pub fn army_supply(&mut self) -> u32 {
        if !self.budget.spend(COST_SIMPLE) {
            return 0;
        }
        self.state
            .my_units
            .iter()
            .filter(|u| !u.is_dead)
            .map(|u| cc_core::unit_stats::base_stats(u.kind).supply_cost)
            .sum()
    }

    /// Visible enemy buildings.
    pub fn enemy_buildings(&mut self) -> Vec<&BuildingSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state.enemy_buildings.iter().collect()
    }

    /// Lowest HP enemy within range of a position.
    pub fn weakest_enemy_in_range(
        &mut self,
        pos: GridPos,
        range: Fixed,
    ) -> Option<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }
        let range_sq = range * range;
        let world_center = cc_core::coords::WorldPos::from_grid(pos);
        let range_i32: i32 = range.ceil().to_num();
        let indices = self.enemy_spatial.units_in_radius(pos, range_i32);

        let mut best: Option<(usize, Fixed)> = None;
        for idx in indices {
            let unit = &self.state.enemy_units[idx];
            if unit.is_dead {
                continue;
            }
            let dist_sq = world_center.distance_squared(unit.world_pos);
            if dist_sq > range_sq {
                continue;
            }
            match best {
                Some((_, best_hp)) if unit.health_current >= best_hp => {}
                _ => best = Some((idx, unit.health_current)),
            }
        }
        best.map(|(idx, _)| &self.state.enemy_units[idx])
    }

    /// Highest HP enemy within range of a position.
    pub fn strongest_enemy_in_range(
        &mut self,
        pos: GridPos,
        range: Fixed,
    ) -> Option<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }
        let range_sq = range * range;
        let world_center = cc_core::coords::WorldPos::from_grid(pos);
        let range_i32: i32 = range.ceil().to_num();
        let indices = self.enemy_spatial.units_in_radius(pos, range_i32);

        let mut best: Option<(usize, Fixed)> = None;
        for idx in indices {
            let unit = &self.state.enemy_units[idx];
            if unit.is_dead {
                continue;
            }
            let dist_sq = world_center.distance_squared(unit.world_pos);
            if dist_sq > range_sq {
                continue;
            }
            match best {
                Some((_, best_hp)) if unit.health_current <= best_hp => {}
                _ => best = Some((idx, unit.health_current)),
            }
        }
        best.map(|(idx, _)| &self.state.enemy_units[idx])
    }

    /// HP as a fraction 0.0–1.0 for a given unit.
    pub fn hp_pct(&mut self, unit_id: EntityId) -> Option<f64> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        let unit = self.state.unit_by_id(unit_id)?;
        if unit.health_max == Fixed::ZERO {
            return Some(0.0);
        }
        Some((unit.health_current / unit.health_max).to_num())
    }

    // -----------------------------------------------------------------------
    // Tactical queries
    // -----------------------------------------------------------------------

    /// Enemies whose attack range reaches the given unit.
    pub fn threats_to(&mut self, unit: &UnitSnapshot) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return vec![];
        }
        let world_pos = unit.world_pos;
        self.state
            .enemy_units
            .iter()
            .filter(|e| {
                if e.is_dead {
                    return false;
                }
                let dist_sq = world_pos.distance_squared(e.world_pos);
                let range_sq = e.attack_range * e.attack_range;
                dist_sq <= range_sq
            })
            .collect()
    }

    /// Enemies within the given unit's attack range.
    pub fn targets_for(&mut self, unit: &UnitSnapshot) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SPATIAL) {
            return vec![];
        }
        let world_pos = unit.world_pos;
        let range_sq = unit.attack_range * unit.attack_range;
        self.state
            .enemy_units
            .iter()
            .filter(|e| {
                if e.is_dead {
                    return false;
                }
                let dist_sq = world_pos.distance_squared(e.world_pos);
                dist_sq <= range_sq
            })
            .collect()
    }

    /// Find a position that is exactly `desired_range` tiles from `target`,
    /// as close to `from` as possible. Core kiting primitive.
    /// Searches passable tiles on a ring around the target.
    pub fn position_at_range(
        &mut self,
        from: GridPos,
        target: GridPos,
        desired_range: i32,
    ) -> Option<GridPos> {
        if !self.budget.spend(COST_SPATIAL) {
            return None;
        }

        let mut best: Option<(GridPos, i64)> = None;

        // Walk the ring of cells at exactly desired_range (Chebyshev) from target
        for dy in -desired_range..=desired_range {
            for dx in -desired_range..=desired_range {
                // Only ring cells (Chebyshev distance == desired_range)
                if dx.abs().max(dy.abs()) != desired_range {
                    continue;
                }
                let candidate = GridPos::new(target.x + dx, target.y + dy);
                if !self.map.is_passable_for(candidate, self.faction) {
                    continue;
                }

                let ddx = (candidate.x - from.x) as i64;
                let ddy = (candidate.y - from.y) as i64;
                let dist_sq = ddx * ddx + ddy * ddy;

                match &best {
                    Some((_, best_dist)) if dist_sq >= *best_dist => {}
                    _ => best = Some((candidate, dist_sq)),
                }
            }
        }

        best.map(|(pos, _)| pos)
    }

    /// Find passable positions within `search_radius` of the unit that are
    /// outside all visible enemy attack ranges.
    pub fn safe_positions(
        &mut self,
        unit: &UnitSnapshot,
        search_radius: i32,
    ) -> Vec<GridPos> {
        if !self.budget.spend(COST_SPATIAL * 2) {
            return vec![];
        }

        let center = unit.pos;
        let mut result = Vec::new();

        for dy in -search_radius..=search_radius {
            for dx in -search_radius..=search_radius {
                let candidate = GridPos::new(center.x + dx, center.y + dy);
                if !self.map.is_passable_for(candidate, self.faction) {
                    continue;
                }

                let world_candidate = cc_core::coords::WorldPos::from_grid(candidate);
                let safe = self.state.enemy_units.iter().all(|e| {
                    if e.is_dead {
                        return true;
                    }
                    let dist_sq = world_candidate.distance_squared(e.world_pos);
                    let range_sq = e.attack_range * e.attack_range;
                    dist_sq > range_sq
                });

                if safe {
                    result.push(candidate);
                }
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Terrain queries
    // -----------------------------------------------------------------------

    pub fn terrain_at(&mut self, pos: GridPos) -> Option<TerrainType> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        self.map.terrain_at(pos)
    }

    pub fn elevation_at(&mut self, pos: GridPos) -> u8 {
        if !self.budget.spend(COST_SIMPLE) {
            return 0;
        }
        self.map.elevation_at(pos)
    }

    pub fn cover_at(&mut self, pos: GridPos) -> CoverLevel {
        if !self.budget.spend(COST_SIMPLE) {
            return CoverLevel::None;
        }
        self.map
            .terrain_at(pos)
            .map(|t| t.cover())
            .unwrap_or(CoverLevel::None)
    }

    pub fn is_passable(&mut self, pos: GridPos) -> bool {
        if !self.budget.spend(COST_SIMPLE) {
            return false;
        }
        self.map.is_passable_for(pos, self.faction)
    }

    pub fn movement_cost(&mut self, pos: GridPos) -> Option<Fixed> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        self.map.movement_cost_for(pos, self.faction)
    }

    pub fn can_reach(&mut self, from: GridPos, to: GridPos) -> bool {
        if !self.budget.spend(COST_PATHFINDING) {
            return false;
        }
        pathfinding::find_path(self.map, from, to, self.faction).is_some()
    }

    pub fn path_length(&mut self, from: GridPos, to: GridPos) -> Option<u32> {
        if !self.budget.spend(COST_PATHFINDING) {
            return None;
        }
        pathfinding::find_path(self.map, from, to, self.faction)
            .map(|path| path.len() as u32)
    }

    // -----------------------------------------------------------------------
    // Economy queries
    // -----------------------------------------------------------------------

    pub fn resources(&self) -> &PlayerResourceState {
        // Free query — always available
        &self.state.my_resources
    }

    /// All resource deposits on the map.
    pub fn resource_deposits(&mut self) -> Vec<&ResourceSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state.resource_deposits.iter().collect()
    }

    pub fn nearest_deposit(
        &mut self,
        from: GridPos,
        kind: Option<ResourceType>,
    ) -> Option<&ResourceSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        let world_from = cc_core::coords::WorldPos::from_grid(from);
        let mut best: Option<(usize, Fixed)> = None;

        for (idx, deposit) in self.state.resource_deposits.iter().enumerate() {
            if deposit.remaining == 0 {
                continue;
            }
            if let Some(k) = kind {
                if deposit.resource_type != k {
                    continue;
                }
            }
            let world_dep = cc_core::coords::WorldPos::from_grid(deposit.pos);
            let dist_sq = world_from.distance_squared(world_dep);
            match &best {
                Some((_, best_dist)) if dist_sq >= *best_dist => {}
                _ => best = Some((idx, dist_sq)),
            }
        }

        best.map(|(idx, _)| &self.state.resource_deposits[idx])
    }

    pub fn my_buildings(&mut self, filter: Option<BuildingKind>) -> Vec<&BuildingSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        match filter {
            Some(kind) => self
                .state
                .my_buildings
                .iter()
                .filter(|b| b.kind == kind)
                .collect(),
            None => self.state.my_buildings.iter().collect(),
        }
    }

    // -----------------------------------------------------------------------
    // Game state queries
    // -----------------------------------------------------------------------

    pub fn tick(&self) -> u64 {
        self.state.tick
    }

    pub fn map_size(&self) -> (u32, u32) {
        (self.state.map_width, self.state.map_height)
    }

    pub fn my_faction(&self) -> FactionId {
        self.faction
    }

    // -----------------------------------------------------------------------
    // Command methods
    // -----------------------------------------------------------------------

    pub fn cmd_move(&mut self, ids: Vec<EntityId>, target: GridPos) {
        self.commands.push(GameCommand::Move {
            unit_ids: ids,
            target,
        });
    }

    pub fn cmd_attack(&mut self, ids: Vec<EntityId>, target: EntityId) {
        self.commands.push(GameCommand::Attack {
            unit_ids: ids,
            target,
        });
    }

    pub fn cmd_attack_move(&mut self, ids: Vec<EntityId>, target: GridPos) {
        self.commands.push(GameCommand::AttackMove {
            unit_ids: ids,
            target,
        });
    }

    pub fn cmd_stop(&mut self, ids: Vec<EntityId>) {
        self.commands
            .push(GameCommand::Stop { unit_ids: ids });
    }

    pub fn cmd_hold(&mut self, ids: Vec<EntityId>) {
        self.commands
            .push(GameCommand::HoldPosition { unit_ids: ids });
    }

    pub fn cmd_gather(&mut self, ids: Vec<EntityId>, deposit: EntityId) {
        self.commands.push(GameCommand::GatherResource {
            unit_ids: ids,
            deposit,
        });
    }

    pub fn cmd_build(&mut self, builder: EntityId, kind: BuildingKind, pos: GridPos) {
        self.commands.push(GameCommand::Build {
            builder,
            building_kind: kind,
            position: pos,
        });
    }

    pub fn cmd_train(&mut self, building: EntityId, kind: UnitKind) {
        self.commands.push(GameCommand::TrainUnit {
            building,
            unit_kind: kind,
        });
    }

    pub fn cmd_ability(&mut self, unit_id: EntityId, slot: u8, target: AbilityTarget) {
        self.commands.push(GameCommand::ActivateAbility {
            unit_id,
            slot,
            target,
        });
    }

    pub fn cmd_research(&mut self, building: EntityId, upgrade: UpgradeType) {
        self.commands.push(GameCommand::Research { building, upgrade });
    }

    pub fn cmd_cancel_queue(&mut self, building: EntityId) {
        self.commands
            .push(GameCommand::CancelQueue { building });
    }

    pub fn cmd_cancel_research(&mut self, building: EntityId) {
        self.commands
            .push(GameCommand::CancelResearch { building });
    }

    pub fn cmd_set_control_group(&mut self, group: u8, ids: Vec<EntityId>) {
        self.commands.push(GameCommand::SetControlGroup {
            group,
            unit_ids: ids,
        });
    }

    pub fn cmd_rally(&mut self, building: EntityId, target: GridPos) {
        self.commands.push(GameCommand::SetRallyPoint {
            building,
            target,
        });
    }

    /// Drain accumulated commands.
    pub fn take_commands(&mut self) -> Vec<GameCommand> {
        std::mem::take(&mut self.commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::coords::WorldPos;
    use cc_core::math::fixed_from_i32;
    use crate::test_fixtures::{make_unit, make_snapshot};

    #[test]
    fn my_units_returns_all_alive() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let units = ctx.my_units(None);
        assert_eq!(units.len(), 2);
    }

    #[test]
    fn my_units_filters_by_kind() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
                make_unit(3, UnitKind::Hisser, 15, 15, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let hissers = ctx.my_units(Some(UnitKind::Hisser));
        assert_eq!(hissers.len(), 2);
    }

    #[test]
    fn enemies_in_range_filters_by_distance() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![
                make_unit(10, UnitKind::Chonk, 7, 5, 1),  // 2 tiles away
                make_unit(11, UnitKind::Chonk, 20, 20, 1), // far away
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let enemies = ctx.enemies_in_range(GridPos::new(5, 5), fixed_from_i32(5));
        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0].id, EntityId(10));
    }

    #[test]
    fn threats_to_finds_enemies_with_range() {
        let mut enemy = make_unit(10, UnitKind::Hisser, 8, 5, 1);
        enemy.attack_range = fixed_from_i32(5); // can reach (5,5) — 3 tiles away

        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Chonk, 5, 5, 0)],
            vec![
                enemy,
                {
                    let mut far_enemy = make_unit(11, UnitKind::Hisser, 50, 50, 1);
                    far_enemy.attack_range = fixed_from_i32(5);
                    far_enemy
                },
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let my_unit = &snap.my_units[0];
        let threats = ctx.threats_to(my_unit);
        assert_eq!(threats.len(), 1);
        assert_eq!(threats[0].id, EntityId(10));
    }

    #[test]
    fn position_at_range_finds_kite_position() {
        let map = GameMap::new(64, 64);
        let snap = make_snapshot(vec![], vec![]);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Unit at (5,5), target at (5,10), desired range 3
        // Should find a position 3 tiles from (5,10), closest to (5,5)
        let pos = ctx.position_at_range(GridPos::new(5, 5), GridPos::new(5, 10), 3);
        assert!(pos.is_some());
        let p = pos.unwrap();

        // Verify Chebyshev distance from target is 3
        let dist = (p.x - 5).abs().max((p.y - 10).abs());
        assert_eq!(dist, 3);

        // Should be closer to (5,5) than other ring positions — likely (5,7)
        assert_eq!(p.y, 7);
    }

    #[test]
    fn budget_limits_queries() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);
        ctx.budget = ComputeBudget::new(2);

        // First query succeeds (cost 1)
        let units = ctx.my_units(None);
        assert_eq!(units.len(), 1);

        // Second query succeeds (cost 1, budget now 0)
        let units2 = ctx.my_units(None);
        assert_eq!(units2.len(), 1);

        // Third query fails (budget exhausted)
        let units3 = ctx.my_units(None);
        assert!(units3.is_empty());
    }

    #[test]
    fn command_accumulation() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        ctx.cmd_move(vec![EntityId(1)], GridPos::new(10, 10));
        ctx.cmd_attack(vec![EntityId(2)], EntityId(99));
        ctx.cmd_stop(vec![EntityId(3)]);

        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn nearest_enemy_finds_closest() {
        let snap = make_snapshot(
            vec![],
            vec![
                make_unit(10, UnitKind::Chonk, 20, 20, 1),
                make_unit(11, UnitKind::Hisser, 6, 5, 1),
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let nearest = ctx.nearest_enemy(GridPos::new(5, 5));
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().id, EntityId(11));
    }

    #[test]
    fn safe_positions_avoids_enemy_range() {
        let mut enemy = make_unit(10, UnitKind::Hisser, 10, 10, 1);
        enemy.attack_range = fixed_from_i32(3);

        let my_unit = make_unit(1, UnitKind::Hisser, 10, 7, 0); // 3 tiles from enemy, in range

        let snap = make_snapshot(vec![my_unit.clone()], vec![enemy]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let safe = ctx.safe_positions(&my_unit, 5);
        // All safe positions should be >3 tiles from (10,10)
        for pos in &safe {
            let world_pos = WorldPos::from_grid(*pos);
            let world_enemy = WorldPos::from_grid(GridPos::new(10, 10));
            let dist_sq = world_pos.distance_squared(world_enemy);
            let range_sq = fixed_from_i32(3) * fixed_from_i32(3);
            assert!(
                dist_sq > range_sq,
                "Position ({},{}) is within enemy range",
                pos.x,
                pos.y
            );
        }
        assert!(!safe.is_empty());
    }

    #[test]
    fn resource_deposits_spends_budget() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![
                ResourceSnapshot {
                    id: EntityId(100),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(3, 3),
                    remaining: 200,
                },
            ],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let budget_before = ctx.budget.remaining();
        let deposit_count = ctx.resource_deposits().len();
        let budget_after = ctx.budget.remaining();

        assert_eq!(deposit_count, 1);
        assert_eq!(budget_after, budget_before - 1, "resource_deposits should spend COST_SIMPLE (1)");
    }

    #[test]
    fn resource_deposits_returns_empty_when_budget_exhausted() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![
                ResourceSnapshot {
                    id: EntityId(100),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(3, 3),
                    remaining: 200,
                },
            ],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);
        ctx.budget = ComputeBudget::new(0); // exhausted

        let deposits = ctx.resource_deposits();
        assert!(deposits.is_empty(), "resource_deposits should return empty when budget is exhausted");
    }

    #[test]
    fn nearest_deposit_finds_closest_of_type() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![
                ResourceSnapshot {
                    id: EntityId(100),
                    resource_type: ResourceType::GpuCores,
                    pos: GridPos::new(5, 5),
                    remaining: 500,
                },
                ResourceSnapshot {
                    id: EntityId(101),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(20, 20),
                    remaining: 300,
                },
                ResourceSnapshot {
                    id: EntityId(102),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(3, 3),
                    remaining: 200,
                },
            ],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let nearest = ctx.nearest_deposit(GridPos::new(0, 0), Some(ResourceType::Food));
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().id, EntityId(102)); // (3,3) is closest food
    }

    // -----------------------------------------------------------------------
    // Tests for Phase 1 extended API
    // -----------------------------------------------------------------------

    #[test]
    fn idle_units_returns_only_idle() {
        let mut moving_unit = make_unit(2, UnitKind::Hisser, 10, 10, 0);
        moving_unit.is_moving = true;
        moving_unit.is_idle = false;

        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0), // idle
                moving_unit,
                make_unit(3, UnitKind::Chonk, 15, 15, 0), // idle
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let idle = ctx.idle_units(None);
        assert_eq!(idle.len(), 2);

        let idle_hissers = ctx.idle_units(Some(UnitKind::Hisser));
        assert_eq!(idle_hissers.len(), 1);
        assert_eq!(idle_hissers[0].id, EntityId(1));
    }

    #[test]
    fn wounded_units_below_threshold() {
        let mut wounded = make_unit(1, UnitKind::Hisser, 5, 5, 0);
        wounded.health_current = fixed_from_i32(30); // 30/100 = 0.3

        let snap = make_snapshot(
            vec![
                wounded,
                make_unit(2, UnitKind::Hisser, 10, 10, 0), // full HP
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let wounded = ctx.wounded_units(0.5);
        assert_eq!(wounded.len(), 1);
        assert_eq!(wounded[0].id, EntityId(1));
    }

    #[test]
    fn hp_pct_returns_fraction() {
        let mut half_hp = make_unit(1, UnitKind::Hisser, 5, 5, 0);
        half_hp.health_current = fixed_from_i32(50);

        let snap = make_snapshot(vec![half_hp], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let pct = ctx.hp_pct(EntityId(1));
        assert!(pct.is_some());
        let p = pct.unwrap();
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn enemy_buildings_returns_visible() {
        use crate::snapshot::BuildingSnapshot;
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![BuildingSnapshot {
                id: EntityId(50),
                kind: BuildingKind::TheBox,
                pos: GridPos::new(30, 30),
                owner: 1,
                health_current: fixed_from_i32(500),
                health_max: fixed_from_i32(500),
                under_construction: false,
                construction_progress: 1.0,
                production_queue: vec![],
            }],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let buildings = ctx.enemy_buildings();
        assert_eq!(buildings.len(), 1);
        assert_eq!(buildings[0].id, EntityId(50));
    }

    #[test]
    fn weakest_enemy_in_range_finds_lowest_hp() {
        let mut weak = make_unit(10, UnitKind::Hisser, 6, 5, 1);
        weak.health_current = fixed_from_i32(20);
        let strong = make_unit(11, UnitKind::Chonk, 7, 5, 1);

        let snap = make_snapshot(vec![], vec![weak, strong]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let weakest = ctx.weakest_enemy_in_range(GridPos::new(5, 5), fixed_from_i32(5));
        assert!(weakest.is_some());
        assert_eq!(weakest.unwrap().id, EntityId(10));
    }

    #[test]
    fn distance_between_two_units() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 0, 0, 0)],
            vec![make_unit(10, UnitKind::Chonk, 3, 4, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let dist = ctx.distance_squared_between(EntityId(1), EntityId(10));
        assert!(dist.is_some());
        // 3^2 + 4^2 = 25 (in world coords)
        let d: f64 = dist.unwrap().to_num();
        assert!(d > 0.0);
    }

    #[test]
    fn cmd_ability_produces_correct_command() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        ctx.cmd_ability(EntityId(1), 0, AbilityTarget::SelfCast);
        ctx.cmd_ability(
            EntityId(2),
            1,
            AbilityTarget::Position(GridPos::new(10, 10)),
        );

        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 2);
        match &cmds[0] {
            GameCommand::ActivateAbility {
                unit_id, slot, target,
            } => {
                assert_eq!(*unit_id, EntityId(1));
                assert_eq!(*slot, 0);
                assert!(matches!(target, AbilityTarget::SelfCast));
            }
            _ => panic!("Expected ActivateAbility"),
        }
    }

    #[test]
    fn cmd_research_produces_correct_command() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        ctx.cmd_research(EntityId(50), UpgradeType::SharperClaws);

        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            GameCommand::Research { building, upgrade } => {
                assert_eq!(*building, EntityId(50));
                assert_eq!(*upgrade, UpgradeType::SharperClaws);
            }
            _ => panic!("Expected Research"),
        }
    }

    #[test]
    fn count_units_by_kind() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
                make_unit(3, UnitKind::Hisser, 15, 15, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.count_units(None), 3);
        assert_eq!(ctx.count_units(Some(UnitKind::Hisser)), 2);
        assert_eq!(ctx.count_units(Some(UnitKind::Chonk)), 1);
        assert_eq!(ctx.count_units(Some(UnitKind::Pawdler)), 0);
    }
}
