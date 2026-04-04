use std::collections::HashMap;

use cc_core::building_stats::building_stats;
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{BuildingKind, ResourceType, UnitKind, UpgradeType};
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::math::{Fixed, fixed_from_i32};
use cc_core::terrain::{CoverLevel, FactionId, TerrainType};
use cc_core::unit_stats::base_stats;
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

/// Value types storable in the per-player blackboard.
#[derive(Debug, Clone, PartialEq)]
pub enum BlackboardValue {
    String(String),
    Number(f64),
    Bool(bool),
}

/// Estimated resource income per tick.
#[derive(Debug, Clone)]
pub struct IncomeEstimate {
    pub food_per_tick: f64,
    pub gpu_per_tick: f64,
}

/// Worker allocation summary.
#[derive(Debug, Clone)]
pub struct WorkerSaturation {
    pub total: u32,
    pub gathering: u32,
    pub idle: u32,
}

/// A remembered enemy unit position (fog-of-war memory).
#[derive(Debug, Clone)]
pub struct EnemyMemoryEntry {
    pub unit_id: u64,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub hp_pct: f64,
    pub tick_last_seen: u32,
    pub confirmed_dead: bool,
}

/// Aggregate army strength summary.
#[derive(Debug, Clone)]
pub struct ArmyStrength {
    pub total_hp: f64,
    pub total_dps: f64,
    pub unit_count: u32,
}

/// An event emitted by one script for consumption by others.
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    pub name: String,
    pub data: String,
    pub tick: u32,
}

/// An unclaimed expansion site (resource deposit without a nearby allied building).
#[derive(Debug, Clone)]
pub struct ExpansionSite {
    pub deposit_id: u64,
    pub resource_type: String,
    pub x: i32,
    pub y: i32,
    pub remaining: u32,
    pub distance_to_base: f64,
}

/// Predicted outcome of an engagement between two groups of units.
#[derive(Debug, Clone)]
pub struct EngagementPrediction {
    pub winner: &'static str, // "self", "enemy", or "draw"
    pub confidence: f64,      // 0.0 to 1.0
    pub my_survivors: u32,    // estimated surviving units
    pub enemy_survivors: u32,
}

/// Default sight range for units (in Chebyshev tiles).
const SIGHT_RANGE_UNIT: i32 = 8;
/// Default sight range for buildings (in Chebyshev tiles).
const SIGHT_RANGE_BUILDING: i32 = 6;

/// Gather rate per worker per tick (simplified estimate).
const FOOD_GATHER_RATE: f64 = 0.5;
/// GPU gather rate per worker per tick (simplified estimate).
const GPU_GATHER_RATE: f64 = 0.3;

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
    pub blackboard: HashMap<String, BlackboardValue>,
    /// Fog-of-war enemy memory (optional, externally owned).
    pub enemy_memory: Option<&'a mut HashMap<u64, EnemyMemoryEntry>>,
    /// Inter-script event bus (optional, externally owned).
    pub events: Option<&'a mut Vec<ScriptEvent>>,
    /// Named squads of units (optional, externally owned).
    pub squads: Option<&'a mut HashMap<String, Vec<u64>>>,
    /// Strait dream sequence state snapshot (optional).
    pub strait_snapshot: Option<crate::strait_bindings::StraitSnapshot>,
    /// Commands produced by strait Lua bindings.
    pub strait_commands: Vec<crate::strait_bindings::StraitCommand>,
}

impl<'a> ScriptContext<'a> {
    pub fn new(
        state: &'a GameStateSnapshot,
        map: &'a GameMap,
        player_id: u8,
        faction: FactionId,
    ) -> Self {
        Self::new_with_blackboard(state, map, player_id, faction, HashMap::new())
    }

    /// Create a ScriptContext with an externally-owned blackboard.
    /// Use this when you need blackboard state to persist across script invocations.
    pub fn new_with_blackboard(
        state: &'a GameStateSnapshot,
        map: &'a GameMap,
        player_id: u8,
        faction: FactionId,
        blackboard: HashMap<String, BlackboardValue>,
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
            blackboard,
            enemy_memory: None,
            events: None,
            squads: None,
            strait_snapshot: None,
            strait_commands: Vec::new(),
        }
    }

    /// Attach an externally-owned enemy memory map. Builder pattern.
    pub fn with_enemy_memory(mut self, memory: &'a mut HashMap<u64, EnemyMemoryEntry>) -> Self {
        self.enemy_memory = Some(memory);
        self
    }

    /// Attach an externally-owned event bus. Builder pattern.
    pub fn with_events(mut self, events: &'a mut Vec<ScriptEvent>) -> Self {
        self.events = Some(events);
        self
    }

    /// Attach an externally-owned squad map. Builder pattern.
    pub fn with_squads(mut self, squads: &'a mut HashMap<String, Vec<u64>>) -> Self {
        self.squads = Some(squads);
        self
    }

    /// Attach a strait snapshot for DEFCON dream bindings. Builder pattern.
    pub fn with_strait_snapshot(mut self, snapshot: crate::strait_bindings::StraitSnapshot) -> Self {
        self.strait_snapshot = Some(snapshot);
        self
    }

    /// Take ownership of the blackboard for persistence across ticks.
    pub fn take_blackboard(&mut self) -> HashMap<String, BlackboardValue> {
        std::mem::take(&mut self.blackboard)
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
            None => self.state.my_units.iter().filter(|u| !u.is_dead).collect(),
        }
    }

    /// Get all visible enemy units.
    pub fn enemy_units(&mut self) -> Vec<&UnitSnapshot> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        self.state
            .enemy_units
            .iter()
            .filter(|u| !u.is_dead)
            .collect()
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
    pub fn nearest_ally(&mut self, from: GridPos, kind: Option<UnitKind>) -> Option<&UnitSnapshot> {
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
            if let Some(k) = kind
                && unit.kind != k
            {
                continue;
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
            .filter(|u| u.is_idle && !u.is_dead && kind.is_none_or(|k| u.kind == k))
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
            .filter(|u| !u.is_dead && kind.is_none_or(|k| u.kind == k))
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
    pub fn weakest_enemy_in_range(&mut self, pos: GridPos, range: Fixed) -> Option<&UnitSnapshot> {
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
    pub fn safe_positions(&mut self, unit: &UnitSnapshot, search_radius: i32) -> Vec<GridPos> {
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
        pathfinding::find_path(self.map, from, to, self.faction).map(|path| path.len() as u32)
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
            if let Some(k) = kind
                && deposit.resource_type != k
            {
                continue;
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
    // Blackboard (shared per-player state between scripts)
    // -----------------------------------------------------------------------

    /// Get a value from the per-player blackboard. Free (no budget cost).
    pub fn blackboard_get(&self, key: &str) -> Option<&BlackboardValue> {
        self.blackboard.get(key)
    }

    /// Set a value in the per-player blackboard. Free (no budget cost).
    pub fn blackboard_set(&mut self, key: String, value: BlackboardValue) {
        self.blackboard.insert(key, value);
    }

    /// Remove a key from the blackboard. Free (no budget cost).
    pub fn blackboard_remove(&mut self, key: &str) {
        self.blackboard.remove(key);
    }

    /// Get all keys in the blackboard. Free (no budget cost).
    pub fn blackboard_keys(&self) -> Vec<&String> {
        self.blackboard.keys().collect()
    }

    // -----------------------------------------------------------------------
    // Budget introspection
    // -----------------------------------------------------------------------

    /// How much compute budget remains. Free (no budget cost).
    pub fn remaining_budget(&self) -> u32 {
        self.budget.remaining()
    }

    // -----------------------------------------------------------------------
    // Economy analysis
    // -----------------------------------------------------------------------

    /// Estimate income rate based on currently gathering workers.
    /// Counts workers with is_gathering == true and estimates food/gpu per tick.
    /// Costs 1 budget.
    pub fn income_rate(&mut self) -> IncomeEstimate {
        if !self.budget.spend(COST_SIMPLE) {
            return IncomeEstimate {
                food_per_tick: 0.0,
                gpu_per_tick: 0.0,
            };
        }

        let mut food_workers = 0u32;
        let mut gpu_workers = 0u32;

        // Count gathering workers among my units
        for unit in &self.state.my_units {
            if unit.is_dead || !unit.is_gathering || !unit.kind.is_worker() {
                continue;
            }
            // Determine what the worker is gathering based on nearest deposit
            // Simple heuristic: find nearest deposit to this worker
            let world_pos = cc_core::coords::WorldPos::from_grid(unit.pos);
            let mut nearest_type = ResourceType::Food; // default
            let mut nearest_dist = Fixed::MAX;
            for dep in &self.state.resource_deposits {
                if dep.remaining == 0 {
                    continue;
                }
                let dep_world = cc_core::coords::WorldPos::from_grid(dep.pos);
                let dist = world_pos.distance_squared(dep_world);
                if dist < nearest_dist {
                    nearest_dist = dist;
                    nearest_type = dep.resource_type;
                }
            }
            match nearest_type {
                ResourceType::Food => food_workers += 1,
                ResourceType::GpuCores => gpu_workers += 1,
                ResourceType::Nft => {} // NFTs not tracked for income
            }
        }

        IncomeEstimate {
            food_per_tick: food_workers as f64 * FOOD_GATHER_RATE,
            gpu_per_tick: gpu_workers as f64 * GPU_GATHER_RATE,
        }
    }

    /// Check if the player can afford a unit of the given kind. Free (no budget cost).
    pub fn can_afford_unit(&self, kind: UnitKind) -> bool {
        let stats = base_stats(kind);
        let res = &self.state.my_resources;
        res.food >= stats.food_cost && res.gpu_cores >= stats.gpu_cost
    }

    /// Check if the player can afford a building of the given kind. Free (no budget cost).
    pub fn can_afford_building(&self, kind: BuildingKind) -> bool {
        let stats = building_stats(kind);
        let res = &self.state.my_resources;
        res.food >= stats.food_cost && res.gpu_cores >= stats.gpu_cost
    }

    /// Estimate ticks until a unit becomes affordable, based on current income.
    /// Returns None if no income (would never afford). Costs 1 budget.
    pub fn time_until_afford_unit(&mut self, kind: UnitKind) -> Option<u64> {
        let stats = base_stats(kind);
        self.time_until_afford_costs(stats.food_cost, stats.gpu_cost)
    }

    /// Estimate ticks until a building becomes affordable, based on current income.
    /// Returns None if no income (would never afford). Costs 1 budget.
    pub fn time_until_afford_building(&mut self, kind: BuildingKind) -> Option<u64> {
        let stats = building_stats(kind);
        self.time_until_afford_costs(stats.food_cost, stats.gpu_cost)
    }

    /// Internal: estimate ticks until we can afford (food_cost, gpu_cost).
    fn time_until_afford_costs(&mut self, food_cost: u32, gpu_cost: u32) -> Option<u64> {
        // Budget already spent by the calling method (income_rate spends 1)
        let income = self.income_rate();
        let res = &self.state.my_resources;

        let food_deficit = food_cost.saturating_sub(res.food) as f64;
        let gpu_deficit = gpu_cost.saturating_sub(res.gpu_cores) as f64;

        // Already affordable
        if food_deficit <= 0.0 && gpu_deficit <= 0.0 {
            return Some(0);
        }

        let food_ticks = if food_deficit > 0.0 {
            if income.food_per_tick <= 0.0 {
                return None;
            }
            (food_deficit / income.food_per_tick).ceil() as u64
        } else {
            0
        };

        let gpu_ticks = if gpu_deficit > 0.0 {
            if income.gpu_per_tick <= 0.0 {
                return None;
            }
            (gpu_deficit / income.gpu_per_tick).ceil() as u64
        } else {
            0
        };

        Some(food_ticks.max(gpu_ticks))
    }

    /// Count own units by kind. Costs 1 budget.
    pub fn army_composition(&mut self) -> HashMap<String, u32> {
        if !self.budget.spend(COST_SIMPLE) {
            return HashMap::new();
        }
        let mut counts = HashMap::new();
        for unit in &self.state.my_units {
            if !unit.is_dead {
                *counts.entry(format!("{:?}", unit.kind)).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Count enemy units by kind. Costs 1 budget.
    pub fn enemy_composition(&mut self) -> HashMap<String, u32> {
        if !self.budget.spend(COST_SIMPLE) {
            return HashMap::new();
        }
        let mut counts = HashMap::new();
        for unit in &self.state.enemy_units {
            if !unit.is_dead {
                *counts.entry(format!("{:?}", unit.kind)).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Count workers by state. Costs 1 budget.
    pub fn worker_saturation(&mut self) -> WorkerSaturation {
        if !self.budget.spend(COST_SIMPLE) {
            return WorkerSaturation {
                total: 0,
                gathering: 0,
                idle: 0,
            };
        }
        let mut total = 0u32;
        let mut gathering = 0u32;
        let mut idle = 0u32;
        for unit in &self.state.my_units {
            if unit.is_dead || !unit.kind.is_worker() {
                continue;
            }
            total += 1;
            if unit.is_gathering {
                gathering += 1;
            } else if unit.is_idle {
                idle += 1;
            }
        }
        WorkerSaturation {
            total,
            gathering,
            idle,
        }
    }

    // -----------------------------------------------------------------------
    // Vision queries (free — no budget cost)
    // -----------------------------------------------------------------------

    /// Check if a tile is within sight range of any own unit or building.
    pub fn is_visible(&self, pos: GridPos) -> bool {
        for unit in &self.state.my_units {
            if unit.is_dead {
                continue;
            }
            let dx = (unit.pos.x - pos.x).abs();
            let dy = (unit.pos.y - pos.y).abs();
            if dx.max(dy) <= SIGHT_RANGE_UNIT {
                return true;
            }
        }
        for building in &self.state.my_buildings {
            let dx = (building.pos.x - pos.x).abs();
            let dy = (building.pos.y - pos.y).abs();
            if dx.max(dy) <= SIGHT_RANGE_BUILDING {
                return true;
            }
        }
        false
    }

    /// Return "visible" or "fog" for a tile.
    pub fn fog_state(&self, pos: GridPos) -> &'static str {
        if self.is_visible(pos) {
            "visible"
        } else {
            "fog"
        }
    }

    // -----------------------------------------------------------------------
    // Enemy memory (budget-costed)
    // -----------------------------------------------------------------------

    /// Update enemy memory from current snapshot. Should be called once per tick.
    pub fn update_enemy_memory(&mut self) {
        let Some(memory) = self.enemy_memory.as_mut() else {
            return;
        };
        let tick = self.state.tick;
        for unit in &self.state.enemy_units {
            let hp_pct = if unit.health_max > Fixed::ZERO {
                unit.health_current.to_num::<f64>() / unit.health_max.to_num::<f64>()
            } else {
                0.0
            };
            memory.insert(
                unit.id.0,
                EnemyMemoryEntry {
                    unit_id: unit.id.0,
                    kind: format!("{:?}", unit.kind),
                    x: unit.pos.x,
                    y: unit.pos.y,
                    hp_pct,
                    tick_last_seen: tick as u32,
                    confirmed_dead: unit.is_dead,
                },
            );
        }
    }

    /// Get all remembered enemy entries. Costs 1 budget.
    pub fn last_seen_enemies(&mut self) -> Vec<EnemyMemoryEntry> {
        if !self.budget.spend(COST_SIMPLE) {
            return vec![];
        }
        match self.enemy_memory.as_ref() {
            Some(memory) => memory.values().cloned().collect(),
            None => vec![],
        }
    }

    /// Get the last known position of a specific enemy. Costs 1 budget.
    pub fn last_seen_at(&mut self, unit_id: u64) -> Option<EnemyMemoryEntry> {
        if !self.budget.spend(COST_SIMPLE) {
            return None;
        }
        self.enemy_memory
            .as_ref()
            .and_then(|m| m.get(&unit_id).cloned())
    }

    // -----------------------------------------------------------------------
    // Threat assessment (budget-costed)
    // -----------------------------------------------------------------------

    /// Sum enemy attack damage within a Chebyshev radius. Costs 1 budget.
    pub fn threat_level(&mut self, center: GridPos, radius: i32) -> f64 {
        if !self.budget.spend(COST_SIMPLE) {
            return 0.0;
        }
        let mut total = 0.0;
        for unit in &self.state.enemy_units {
            if unit.is_dead {
                continue;
            }
            let dx = (unit.pos.x - center.x).abs();
            let dy = (unit.pos.y - center.y).abs();
            if dx.max(dy) <= radius {
                total += unit.attack_damage.to_num::<f64>();
            }
        }
        total
    }

    /// Aggregate own army strength. Costs 1 budget.
    pub fn army_strength(&mut self) -> ArmyStrength {
        if !self.budget.spend(COST_SIMPLE) {
            return ArmyStrength {
                total_hp: 0.0,
                total_dps: 0.0,
                unit_count: 0,
            };
        }
        let mut total_hp = 0.0;
        let mut total_dps = 0.0;
        let mut unit_count = 0u32;
        for unit in &self.state.my_units {
            if unit.is_dead {
                continue;
            }
            total_hp += unit.health_current.to_num::<f64>();
            total_dps += unit.attack_damage.to_num::<f64>();
            unit_count += 1;
        }
        ArmyStrength {
            total_hp,
            total_dps,
            unit_count,
        }
    }

    // -----------------------------------------------------------------------
    // Inter-script events (free — no budget cost)
    // -----------------------------------------------------------------------

    /// Emit an event for other scripts to consume.
    pub fn emit_event(&mut self, name: String, data: String) {
        if let Some(ref mut events) = self.events {
            events.push(ScriptEvent {
                name,
                data,
                tick: self.state.tick as u32,
            });
        }
    }

    /// Read events matching a name, without removing them.
    pub fn poll_events(&mut self, name: &str) -> Vec<ScriptEvent> {
        match self.events.as_ref() {
            Some(events) => events.iter().filter(|e| e.name == name).cloned().collect(),
            None => vec![],
        }
    }

    /// Read and remove events matching a name.
    pub fn drain_events(&mut self, name: &str) -> Vec<ScriptEvent> {
        let Some(events) = self.events.as_mut() else {
            return vec![];
        };
        let mut matched = Vec::new();
        let mut i = 0;
        while i < events.len() {
            if events[i].name == name {
                matched.push(events.remove(i));
            } else {
                i += 1;
            }
        }
        matched
    }

    // -----------------------------------------------------------------------
    // Strategic assessment (Phase 3)
    // -----------------------------------------------------------------------

    /// Determine current game phase based on tick and army size.
    pub fn game_phase(&self) -> &'static str {
        let tick = self.state.tick;
        let army_supply: u32 = self
            .state
            .my_units
            .iter()
            .filter(|u| !u.is_dead)
            .map(|u| base_stats(u.kind).supply_cost)
            .sum();

        let barracks_kinds = [
            BuildingKind::CatTree,
            BuildingKind::NestingBox,
            BuildingKind::SpawningPools,
            BuildingKind::Rookery,
            BuildingKind::WarHollow,
            BuildingKind::ChopShop,
        ];
        let tech_kinds = [
            BuildingKind::ServerRack,
            BuildingKind::JunkTransmitter,
            BuildingKind::SunkenServer,
            BuildingKind::AntennaArray,
            BuildingKind::CoreTap,
            BuildingKind::JunkServer,
        ];

        let has_barracks = self
            .state
            .my_buildings
            .iter()
            .any(|b| barracks_kinds.contains(&b.kind));
        let has_tech = self
            .state
            .my_buildings
            .iter()
            .any(|b| tech_kinds.contains(&b.kind));

        // Early: tick < 500 OR (army_supply < 10 AND no barracks)
        if tick < 500 || (army_supply < 10 && !has_barracks) {
            return "early";
        }
        // Late: tick > 3000 AND (army_supply > 30 OR has tech building)
        if tick > 3000 && (army_supply > 30 || has_tech) {
            return "late";
        }
        // Mid: everything else
        "mid"
    }

    /// Find resource deposits not near any allied building. Costs 2 budget.
    pub fn expansion_sites(&mut self) -> Vec<ExpansionSite> {
        if !self.budget.spend(COST_SPATIAL) {
            return vec![];
        }

        let my_building_positions: Vec<GridPos> =
            self.state.my_buildings.iter().map(|b| b.pos).collect();

        let mut sites: Vec<ExpansionSite> = Vec::new();

        for deposit in &self.state.resource_deposits {
            if deposit.remaining == 0 {
                continue;
            }

            // Check if any own building is within Chebyshev distance 6
            let claimed = my_building_positions.iter().any(|bp| {
                let dx = (deposit.pos.x - bp.x).abs();
                let dy = (deposit.pos.y - bp.y).abs();
                dx.max(dy) <= 6
            });
            if claimed {
                continue;
            }

            // Compute distance to nearest own building
            let distance_to_base = if my_building_positions.is_empty() {
                f64::MAX
            } else {
                my_building_positions
                    .iter()
                    .map(|bp| {
                        let dx = (deposit.pos.x - bp.x) as f64;
                        let dy = (deposit.pos.y - bp.y) as f64;
                        (dx * dx + dy * dy).sqrt()
                    })
                    .fold(f64::MAX, f64::min)
            };

            sites.push(ExpansionSite {
                deposit_id: deposit.id.0,
                resource_type: format!("{:?}", deposit.resource_type),
                x: deposit.pos.x,
                y: deposit.pos.y,
                remaining: deposit.remaining,
                distance_to_base,
            });
        }

        sites.sort_by(|a, b| {
            a.distance_to_base
                .partial_cmp(&b.distance_to_base)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sites
    }

    /// Predict the outcome of an engagement between two groups. Costs 2 budget.
    pub fn predict_engagement(
        &mut self,
        my_ids: &[EntityId],
        enemy_ids: &[EntityId],
    ) -> EngagementPrediction {
        if !self.budget.spend(COST_SPATIAL) {
            return EngagementPrediction {
                winner: "draw",
                confidence: 0.0,
                my_survivors: 0,
                enemy_survivors: 0,
            };
        }

        let mut my_total_hp = 0.0;
        let mut my_total_dps = 0.0;
        let mut my_count = 0u32;
        for id in my_ids {
            if let Some(u) = self.state.unit_by_id(*id)
                && !u.is_dead
            {
                my_total_hp += u.health_current.to_num::<f64>();
                let dmg: f64 = u.attack_damage.to_num::<f64>();
                let aspd = u.attack_speed;
                if aspd > 0 {
                    my_total_dps += dmg / aspd as f64;
                }
                my_count += 1;
            }
        }

        let mut enemy_total_hp = 0.0;
        let mut enemy_total_dps = 0.0;
        let mut enemy_count = 0u32;
        for id in enemy_ids {
            if let Some(u) = self.state.unit_by_id(*id)
                && !u.is_dead
            {
                enemy_total_hp += u.health_current.to_num::<f64>();
                let dmg: f64 = u.attack_damage.to_num::<f64>();
                let aspd = u.attack_speed;
                if aspd > 0 {
                    enemy_total_dps += dmg / aspd as f64;
                }
                enemy_count += 1;
            }
        }

        // Edge cases: if one side has zero DPS, they can never kill the other
        if my_total_dps <= 0.0 && enemy_total_dps <= 0.0 {
            return EngagementPrediction {
                winner: "draw",
                confidence: 0.0,
                my_survivors: my_count,
                enemy_survivors: enemy_count,
            };
        }
        if my_total_dps <= 0.0 {
            return EngagementPrediction {
                winner: "enemy",
                confidence: 1.0,
                my_survivors: 0,
                enemy_survivors: enemy_count,
            };
        }
        if enemy_total_dps <= 0.0 {
            return EngagementPrediction {
                winner: "self",
                confidence: 1.0,
                my_survivors: my_count,
                enemy_survivors: 0,
            };
        }

        // Time for my army to kill enemy army
        let time_to_kill_enemy = enemy_total_hp / my_total_dps;
        // Time for enemy army to kill my army
        let time_to_kill_me = my_total_hp / enemy_total_dps;

        let (winner, loser_ttk, winner_ttk) = if time_to_kill_enemy < time_to_kill_me {
            ("self", time_to_kill_enemy, time_to_kill_me)
        } else if time_to_kill_me < time_to_kill_enemy {
            ("enemy", time_to_kill_me, time_to_kill_enemy)
        } else {
            return EngagementPrediction {
                winner: "draw",
                confidence: 0.5,
                my_survivors: 0,
                enemy_survivors: 0,
            };
        };

        // Confidence based on how one-sided the fight is
        let ratio = if winner_ttk > 0.0 {
            loser_ttk / winner_ttk
        } else {
            0.0
        };
        let confidence = (1.0 - ratio).clamp(0.0, 1.0);

        // Estimate survivors: winner's remaining HP after loser_ttk seconds
        let (my_survivors, enemy_survivors) = if winner == "self" {
            let remaining_hp = my_total_hp - enemy_total_dps * loser_ttk;
            let avg_hp = if my_count > 0 {
                my_total_hp / my_count as f64
            } else {
                1.0
            };
            let survivors = (remaining_hp / avg_hp).ceil().max(0.0) as u32;
            (survivors, 0u32)
        } else {
            let remaining_hp = enemy_total_hp - my_total_dps * loser_ttk;
            let avg_hp = if enemy_count > 0 {
                enemy_total_hp / enemy_count as f64
            } else {
                1.0
            };
            let survivors = (remaining_hp / avg_hp).ceil().max(0.0) as u32;
            (0u32, survivors)
        };

        EngagementPrediction {
            winner,
            confidence,
            my_survivors,
            enemy_survivors,
        }
    }

    // -----------------------------------------------------------------------
    // Squad management (Phase 3)
    // -----------------------------------------------------------------------

    /// Create a named squad with given unit IDs.
    pub fn squad_create(&mut self, name: String, unit_ids: Vec<u64>) {
        if let Some(ref mut squads) = self.squads {
            squads.insert(name, unit_ids);
        }
    }

    /// Add units to an existing squad. Creates the squad if it doesn't exist. Free.
    pub fn squad_add(&mut self, name: &str, unit_ids: Vec<u64>) {
        if let Some(ref mut squads) = self.squads {
            let entry = squads.entry(name.to_string()).or_default();
            for id in unit_ids {
                if !entry.contains(&id) {
                    entry.push(id);
                }
            }
        }
    }

    /// Remove units from a squad.
    pub fn squad_remove(&mut self, name: &str, unit_ids: &[u64]) {
        if let Some(ref mut squads) = self.squads
            && let Some(members) = squads.get_mut(name)
        {
            members.retain(|id| !unit_ids.contains(id));
        }
    }

    /// Get unit IDs in a squad, pruning dead units.
    pub fn squad_units(&mut self, name: &str) -> Vec<u64> {
        let Some(ref mut squads) = self.squads else {
            return vec![];
        };
        let Some(members) = squads.get_mut(name) else {
            return vec![];
        };
        // Prune dead units
        members.retain(|id| {
            self.state
                .unit_by_id(EntityId(*id))
                .map(|u| !u.is_dead)
                .unwrap_or(false)
        });
        members.clone()
    }

    /// Get the centroid position of a squad.
    pub fn squad_centroid(&mut self, name: &str) -> Option<(i32, i32)> {
        let ids = self.squad_units(name);
        if ids.is_empty() {
            return None;
        }
        let mut sum_x = 0i64;
        let mut sum_y = 0i64;
        let mut count = 0i64;
        for id in &ids {
            if let Some(unit) = self.state.unit_by_id(EntityId(*id)) {
                sum_x += unit.pos.x as i64;
                sum_y += unit.pos.y as i64;
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        Some(((sum_x / count) as i32, (sum_y / count) as i32))
    }

    /// Remove a squad entirely.
    pub fn squad_disband(&mut self, name: &str) {
        if let Some(ref mut squads) = self.squads {
            squads.remove(name);
        }
    }

    /// List all squad names.
    pub fn squad_list(&self) -> Vec<String> {
        match self.squads.as_ref() {
            Some(squads) => squads.keys().cloned().collect(),
            None => vec![],
        }
    }

    /// Compute a composite game score (positive = winning). Costs 1 budget.
    pub fn game_score(&mut self) -> f64 {
        if !self.budget.spend(COST_SIMPLE) {
            return 0.0;
        }
        let mut my_hp = 0.0;
        let mut my_count = 0.0;
        for unit in &self.state.my_units {
            if !unit.is_dead {
                my_hp += unit.health_current.to_num::<f64>();
                my_count += 1.0;
            }
        }
        let mut enemy_hp = 0.0;
        let mut enemy_count = 0.0;
        for unit in &self.state.enemy_units {
            if !unit.is_dead {
                enemy_hp += unit.health_current.to_num::<f64>();
                enemy_count += 1.0;
            }
        }
        let building_score =
            self.state.my_buildings.len() as f64 - self.state.enemy_buildings.len() as f64;
        (my_hp - enemy_hp) + (my_count - enemy_count) * 10.0 + building_score * 50.0
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
        self.commands.push(GameCommand::Stop { unit_ids: ids });
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
        self.commands
            .push(GameCommand::Research { building, upgrade });
    }

    pub fn cmd_cancel_queue(&mut self, building: EntityId) {
        self.commands.push(GameCommand::CancelQueue { building });
    }

    pub fn cmd_cancel_research(&mut self, building: EntityId) {
        self.commands.push(GameCommand::CancelResearch { building });
    }

    pub fn cmd_set_control_group(&mut self, group: u8, ids: Vec<EntityId>) {
        self.commands.push(GameCommand::SetControlGroup {
            group,
            unit_ids: ids,
        });
    }

    pub fn cmd_rally(&mut self, building: EntityId, target: GridPos) {
        self.commands
            .push(GameCommand::SetRallyPoint { building, target });
    }

    /// Drain accumulated commands.
    pub fn take_commands(&mut self) -> Vec<GameCommand> {
        std::mem::take(&mut self.commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{make_snapshot, make_unit};
    use cc_core::coords::WorldPos;
    use cc_core::math::fixed_from_i32;

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
                make_unit(10, UnitKind::Chonk, 7, 5, 1),   // 2 tiles away
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
            vec![enemy, {
                let mut far_enemy = make_unit(11, UnitKind::Hisser, 50, 50, 1);
                far_enemy.attack_range = fixed_from_i32(5);
                far_enemy
            }],
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
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
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
                research_queue: vec![],
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
                unit_id,
                slot,
                target,
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

    #[test]
    fn resource_deposits_budget_gated() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(5, 5),
                remaining: 200,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let deposits = ctx.resource_deposits();
        assert_eq!(deposits.len(), 1);
        assert_eq!(deposits[0].id, EntityId(100));

        ctx.budget = ComputeBudget::new(0);
        let empty = ctx.resource_deposits();
        assert!(empty.is_empty());
    }

    // -----------------------------------------------------------------------
    // Phase 2: Vision, Memory, Threats, Events
    // -----------------------------------------------------------------------

    #[test]
    fn is_visible_near_own_unit() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 10, 10, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Adjacent tile — should be visible
        assert!(ctx.is_visible(GridPos::new(11, 10)));
        // Within 8 tiles (Chebyshev) — should be visible
        assert!(ctx.is_visible(GridPos::new(18, 10)));
        assert!(ctx.is_visible(GridPos::new(10, 18)));
        assert!(ctx.is_visible(GridPos::new(18, 18)));
        // Exactly at the unit position
        assert!(ctx.is_visible(GridPos::new(10, 10)));
    }

    #[test]
    fn is_visible_far_from_units() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 10, 10, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // 9 tiles away (Chebyshev) — beyond default sight range of 8
        assert!(!ctx.is_visible(GridPos::new(19, 10)));
        // Far away
        assert!(!ctx.is_visible(GridPos::new(50, 50)));
    }

    #[test]
    fn is_visible_near_own_building() {
        use crate::snapshot::BuildingSnapshot;
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![BuildingSnapshot {
                id: EntityId(50),
                kind: BuildingKind::TheBox,
                pos: GridPos::new(20, 20),
                owner: 0,
                health_current: fixed_from_i32(500),
                health_max: fixed_from_i32(500),
                under_construction: false,
                construction_progress: 1.0,
                production_queue: vec![],
                research_queue: vec![],
            }],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Building sight range is 6
        assert!(ctx.is_visible(GridPos::new(26, 20)));
        assert!(!ctx.is_visible(GridPos::new(27, 20)));
    }

    #[test]
    fn fog_state_returns_correct_strings() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 10, 10, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.fog_state(GridPos::new(10, 10)), "visible");
        assert_eq!(ctx.fog_state(GridPos::new(50, 50)), "fog");
    }

    #[test]
    fn enemy_memory_update_and_query() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![make_unit(10, UnitKind::Chonk, 20, 20, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut memory = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_enemy_memory(&mut memory);

        // Update memory from snapshot
        ctx.update_enemy_memory();

        // Query all remembered enemies
        let enemies = ctx.last_seen_enemies();
        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0].unit_id, 10);
        assert_eq!(enemies[0].x, 20);
        assert_eq!(enemies[0].y, 20);
        assert!(!enemies[0].confirmed_dead);

        // Query by id
        let entry = ctx.last_seen_at(10);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().kind, "Chonk");

        // Missing id returns None
        let missing = ctx.last_seen_at(999);
        assert!(missing.is_none());
    }

    #[test]
    fn enemy_memory_persists_after_fog() {
        let mut memory = HashMap::new();

        // Tick 1: enemy visible
        {
            let snap = GameStateSnapshot {
                tick: 1,
                map_width: 64,
                map_height: 64,
                player_id: 0,
                my_units: vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
                enemy_units: vec![make_unit(10, UnitKind::Chonk, 20, 20, 1)],
                my_buildings: vec![],
                enemy_buildings: vec![],
                resource_deposits: vec![],
                my_resources: PlayerResourceState::default(),
            };
            let map = GameMap::new(64, 64);
            let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT)
                .with_enemy_memory(&mut memory);
            ctx.update_enemy_memory();
        }

        // Tick 2: enemy no longer visible (not in snapshot)
        {
            let snap = GameStateSnapshot {
                tick: 2,
                map_width: 64,
                map_height: 64,
                player_id: 0,
                my_units: vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
                enemy_units: vec![], // enemy vanished into fog
                my_buildings: vec![],
                enemy_buildings: vec![],
                resource_deposits: vec![],
                my_resources: PlayerResourceState::default(),
            };
            let map = GameMap::new(64, 64);
            let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT)
                .with_enemy_memory(&mut memory);
            ctx.update_enemy_memory();

            // Memory should still have the enemy from tick 1
            let enemies = ctx.last_seen_enemies();
            assert_eq!(enemies.len(), 1);
            assert_eq!(enemies[0].unit_id, 10);
            assert_eq!(enemies[0].tick_last_seen, 1); // still from tick 1
        }
    }

    #[test]
    fn enemy_memory_marks_dead() {
        let mut memory = HashMap::new();
        let mut dead_enemy = make_unit(10, UnitKind::Chonk, 20, 20, 1);
        dead_enemy.is_dead = true;
        dead_enemy.health_current = Fixed::ZERO;

        let snap = GameStateSnapshot {
            tick: 5,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            enemy_units: vec![dead_enemy],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_enemy_memory(&mut memory);
        ctx.update_enemy_memory();

        let entry = ctx.last_seen_at(10);
        assert!(entry.is_some());
        assert!(entry.unwrap().confirmed_dead);
    }

    #[test]
    fn threat_level_near_enemies() {
        let snap = make_snapshot(
            vec![],
            vec![
                make_unit(10, UnitKind::Hisser, 5, 5, 1),     // damage = 10
                make_unit(11, UnitKind::Chonk, 6, 5, 1),      // damage = 10
                make_unit(12, UnitKind::Nuisance, 50, 50, 1), // far away
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Radius 3 from (5,5) should include enemies at (5,5) and (6,5) but not (50,50)
        let threat = ctx.threat_level(GridPos::new(5, 5), 3);
        assert!(
            (threat - 20.0).abs() < 0.01,
            "Expected 20.0, got {}",
            threat
        );
    }

    #[test]
    fn threat_level_no_enemies_nearby() {
        let snap = make_snapshot(vec![], vec![make_unit(10, UnitKind::Hisser, 50, 50, 1)]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let threat = ctx.threat_level(GridPos::new(5, 5), 5);
        assert!((threat - 0.0).abs() < 0.01);
    }

    #[test]
    fn army_strength_sums_correctly() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),  // hp=100, damage=10
                make_unit(2, UnitKind::Chonk, 10, 10, 0), // hp=100, damage=10
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let strength = ctx.army_strength();
        assert_eq!(strength.unit_count, 2);
        assert!((strength.total_hp - 200.0).abs() < 0.01);
        assert!((strength.total_dps - 20.0).abs() < 0.01);
    }

    #[test]
    fn emit_and_poll_events() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut event_bus = Vec::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_events(&mut event_bus);

        // Emit events
        ctx.emit_event("attack".to_string(), "go".to_string());
        ctx.emit_event("retreat".to_string(), "now".to_string());
        ctx.emit_event("attack".to_string(), "charge".to_string());

        // Poll only "attack" events
        let attacks = ctx.poll_events("attack");
        assert_eq!(attacks.len(), 2);
        assert_eq!(attacks[0].data, "go");
        assert_eq!(attacks[1].data, "charge");

        // Poll should NOT remove events
        let still_there = ctx.poll_events("attack");
        assert_eq!(still_there.len(), 2);
    }

    #[test]
    fn drain_events_removes_matched() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut event_bus = Vec::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_events(&mut event_bus);

        ctx.emit_event("attack".to_string(), "go".to_string());
        ctx.emit_event("retreat".to_string(), "now".to_string());
        ctx.emit_event("attack".to_string(), "charge".to_string());

        // Drain "attack" events
        let attacks = ctx.drain_events("attack");
        assert_eq!(attacks.len(), 2);
        assert_eq!(attacks[0].data, "go");
        assert_eq!(attacks[1].data, "charge");

        // "attack" events should be gone
        let empty = ctx.poll_events("attack");
        assert!(empty.is_empty());

        // "retreat" event should remain
        let retreats = ctx.poll_events("retreat");
        assert_eq!(retreats.len(), 1);
        assert_eq!(retreats[0].data, "now");
    }

    #[test]
    fn events_without_bus_are_noop() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // No event bus attached — should not panic
        ctx.emit_event("test".to_string(), "data".to_string());
        let events = ctx.poll_events("test");
        assert!(events.is_empty());
        let drained = ctx.drain_events("test");
        assert!(drained.is_empty());
    }

    #[test]
    fn memory_without_hashmap_returns_empty() {
        let snap = make_snapshot(vec![], vec![make_unit(10, UnitKind::Hisser, 5, 5, 1)]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // No memory attached
        ctx.update_enemy_memory(); // should not panic
        let enemies = ctx.last_seen_enemies();
        assert!(enemies.is_empty());
        let entry = ctx.last_seen_at(10);
        assert!(entry.is_none());
    }

    // -----------------------------------------------------------------------
    // Phase 3 tests
    // -----------------------------------------------------------------------

    #[test]
    fn game_phase_returns_early_for_fresh_snapshot() {
        // Default make_snapshot has tick=0, no buildings, small army
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.game_phase(), "early");
    }

    #[test]
    fn game_phase_returns_early_for_low_tick() {
        let mut snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
        snap.tick = 200;
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.game_phase(), "early");
    }

    #[test]
    fn game_phase_returns_mid_with_barracks_and_army() {
        use crate::snapshot::BuildingSnapshot;
        let snap = GameStateSnapshot {
            tick: 1000,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: (1..=15)
                .map(|i| make_unit(i, UnitKind::Hisser, 5, 5, 0))
                .collect(),
            enemy_units: vec![],
            my_buildings: vec![BuildingSnapshot {
                id: EntityId(100),
                kind: BuildingKind::CatTree, // barracks
                pos: GridPos::new(10, 10),
                owner: 0,
                health_current: fixed_from_i32(500),
                health_max: fixed_from_i32(500),
                under_construction: false,
                construction_progress: 1.0,
                production_queue: vec![],
                research_queue: vec![],
            }],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.game_phase(), "mid");
    }

    #[test]
    fn game_phase_returns_late_with_tech_and_high_tick() {
        use crate::snapshot::BuildingSnapshot;
        // Need enough army (>= 10 supply) or a barracks to avoid "early",
        // plus tick > 3000 and a tech building for "late".
        let snap = GameStateSnapshot {
            tick: 4000,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            // 10 Hissers = 20 supply (each is 2 supply)
            my_units: (1..=10)
                .map(|i| make_unit(i, UnitKind::Hisser, 5, 5, 0))
                .collect(),
            enemy_units: vec![],
            my_buildings: vec![
                BuildingSnapshot {
                    id: EntityId(100),
                    kind: BuildingKind::ServerRack, // tech building
                    pos: GridPos::new(10, 10),
                    owner: 0,
                    health_current: fixed_from_i32(500),
                    health_max: fixed_from_i32(500),
                    under_construction: false,
                    construction_progress: 1.0,
                    production_queue: vec![],
                    research_queue: vec![],
                },
                BuildingSnapshot {
                    id: EntityId(101),
                    kind: BuildingKind::CatTree, // barracks (prevents "early")
                    pos: GridPos::new(12, 10),
                    owner: 0,
                    health_current: fixed_from_i32(500),
                    health_max: fixed_from_i32(500),
                    under_construction: false,
                    construction_progress: 1.0,
                    production_queue: vec![],
                    research_queue: vec![],
                },
            ],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        assert_eq!(ctx.game_phase(), "late");
    }

    #[test]
    fn expansion_sites_returns_deposits_far_from_buildings() {
        use crate::snapshot::BuildingSnapshot;
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![BuildingSnapshot {
                id: EntityId(100),
                kind: BuildingKind::TheBox,
                pos: GridPos::new(10, 10),
                owner: 0,
                health_current: fixed_from_i32(500),
                health_max: fixed_from_i32(500),
                under_construction: false,
                construction_progress: 1.0,
                production_queue: vec![],
                research_queue: vec![],
            }],
            enemy_buildings: vec![],
            resource_deposits: vec![
                // Near building (within 6 tiles) — should be filtered out
                ResourceSnapshot {
                    id: EntityId(200),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(12, 10),
                    remaining: 200,
                },
                // Far from building — should be returned
                ResourceSnapshot {
                    id: EntityId(201),
                    resource_type: ResourceType::GpuCores,
                    pos: GridPos::new(40, 40),
                    remaining: 100,
                },
                // Also far — should be returned
                ResourceSnapshot {
                    id: EntityId(202),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(30, 30),
                    remaining: 150,
                },
            ],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let sites = ctx.expansion_sites();
        assert_eq!(sites.len(), 2);
        // Sorted by distance — (30,30) is closer to (10,10) than (40,40)
        assert_eq!(sites[0].deposit_id, 202);
        assert_eq!(sites[1].deposit_id, 201);
        assert!(sites[0].distance_to_base < sites[1].distance_to_base);
    }

    #[test]
    fn expansion_sites_empty_deposits_skipped() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![ResourceSnapshot {
                id: EntityId(200),
                resource_type: ResourceType::Food,
                pos: GridPos::new(30, 30),
                remaining: 0, // exhausted
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let sites = ctx.expansion_sites();
        assert!(sites.is_empty());
    }

    #[test]
    fn predict_engagement_self_wins_when_stronger() {
        // My units: 3 Hissers (each 100hp, 10dmg/10ticks = 1 DPS)
        // Enemy: 1 Nuisance (100hp, 10dmg/10ticks = 1 DPS)
        // My total: 300hp, 3 DPS. Enemy: 100hp, 1 DPS.
        // Time for me to kill enemy: 100/3 = 33.3
        // Time for enemy to kill me: 300/1 = 300
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Hisser, 6, 5, 0),
                make_unit(3, UnitKind::Hisser, 7, 5, 0),
            ],
            vec![make_unit(10, UnitKind::Nuisance, 20, 20, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let pred =
            ctx.predict_engagement(&[EntityId(1), EntityId(2), EntityId(3)], &[EntityId(10)]);

        assert_eq!(pred.winner, "self");
        assert!(pred.confidence > 0.5);
        assert!(pred.my_survivors > 0);
        assert_eq!(pred.enemy_survivors, 0);
    }

    #[test]
    fn predict_engagement_enemy_wins_when_outnumbered() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![
                make_unit(10, UnitKind::Nuisance, 20, 20, 1),
                make_unit(11, UnitKind::Nuisance, 21, 20, 1),
                make_unit(12, UnitKind::Nuisance, 22, 20, 1),
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let pred =
            ctx.predict_engagement(&[EntityId(1)], &[EntityId(10), EntityId(11), EntityId(12)]);

        assert_eq!(pred.winner, "enemy");
        assert!(pred.confidence > 0.5);
        assert_eq!(pred.my_survivors, 0);
        assert!(pred.enemy_survivors > 0);
    }

    #[test]
    fn predict_engagement_draw_when_equal() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![make_unit(10, UnitKind::Hisser, 20, 20, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let pred = ctx.predict_engagement(&[EntityId(1)], &[EntityId(10)]);
        assert_eq!(pred.winner, "draw");
    }

    #[test]
    fn squad_create_and_units_round_trip() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1, 2]);
        let units = ctx.squad_units("alpha");
        assert_eq!(units.len(), 2);
        assert!(units.contains(&1));
        assert!(units.contains(&2));
    }

    #[test]
    fn squad_add_extends_existing_squad() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
                make_unit(3, UnitKind::Hisser, 15, 15, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1]);
        ctx.squad_add("alpha", vec![2, 3]);
        let units = ctx.squad_units("alpha");
        assert_eq!(units.len(), 3);
    }

    #[test]
    fn squad_add_no_duplicates() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1]);
        ctx.squad_add("alpha", vec![1]); // duplicate
        let units = ctx.squad_units("alpha");
        assert_eq!(units.len(), 1);
    }

    #[test]
    fn squad_remove_removes_units() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1, 2]);
        ctx.squad_remove("alpha", &[1]);
        let units = ctx.squad_units("alpha");
        assert_eq!(units.len(), 1);
        assert_eq!(units[0], 2);
    }

    #[test]
    fn squad_units_auto_prunes_dead() {
        let mut dead_unit = make_unit(2, UnitKind::Chonk, 10, 10, 0);
        dead_unit.is_dead = true;

        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0), dead_unit],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1, 2]);
        let units = ctx.squad_units("alpha");
        // Dead unit 2 should be pruned
        assert_eq!(units.len(), 1);
        assert_eq!(units[0], 1);
    }

    #[test]
    fn squad_centroid_computes_average() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 0, 0, 0),
                make_unit(2, UnitKind::Hisser, 10, 10, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1, 2]);
        let centroid = ctx.squad_centroid("alpha");
        assert_eq!(centroid, Some((5, 5)));
    }

    #[test]
    fn squad_disband_removes_squad() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1]);
        assert_eq!(ctx.squad_list().len(), 1);
        ctx.squad_disband("alpha");
        assert_eq!(ctx.squad_list().len(), 0);
        assert!(ctx.squad_units("alpha").is_empty());
    }

    #[test]
    fn squad_list_returns_all_names() {
        let snap = make_snapshot(vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)], vec![]);
        let map = GameMap::new(64, 64);
        let mut squads = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        ctx.squad_create("alpha".to_string(), vec![1]);
        ctx.squad_create("bravo".to_string(), vec![1]);
        let names = ctx.squad_list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"bravo".to_string()));
    }

    #[test]
    fn game_score_positive_when_more_army() {
        // 3 combat units vs 1 enemy combat unit, with workers and buildings
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 6, 5, 0),
                make_unit(3, UnitKind::Hisser, 7, 5, 0),
            ],
            enemy_units: vec![make_unit(10, UnitKind::Nuisance, 30, 30, 1)],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let score = ctx.game_score();
        assert!(
            score > 0.0,
            "Score should be positive with more army: {}",
            score
        );
    }

    #[test]
    fn game_score_negative_when_less_army() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![make_unit(1, UnitKind::Nuisance, 5, 5, 0)],
            enemy_units: vec![
                make_unit(10, UnitKind::Hisser, 30, 30, 1),
                make_unit(11, UnitKind::Chonk, 31, 30, 1),
                make_unit(12, UnitKind::Hisser, 32, 30, 1),
            ],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            // Set resources low to ensure negative
            my_resources: PlayerResourceState {
                food: 0,
                gpu_cores: 0,
                nfts: 0,
                supply: 1,
                supply_cap: 10,
                completed_upgrades: Default::default(),
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let score = ctx.game_score();
        assert!(
            score < 0.0,
            "Score should be negative with less army: {}",
            score
        );
    }

    #[test]
    fn game_score_accounts_for_workers() {
        // More workers = higher score
        let snap_with_workers = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![
                make_unit(1, UnitKind::Pawdler, 5, 5, 0),
                make_unit(2, UnitKind::Pawdler, 6, 5, 0),
                make_unit(3, UnitKind::Pawdler, 7, 5, 0),
            ],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap_with_workers, &map, 0, FactionId::CatGPT);
        let score_with = ctx.game_score();

        let snap_no_workers = GameStateSnapshot {
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
        let mut ctx2 = ScriptContext::new(&snap_no_workers, &map, 0, FactionId::CatGPT);
        let score_without = ctx2.game_score();

        assert!(score_with > score_without, "Workers should increase score");
    }
}
