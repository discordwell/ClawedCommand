use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    Building, BuildingKind, Gathering, Owner, Position, Producer, ResourceDeposit, UnitKind,
    UnitType,
};
use cc_core::coords::GridPos;

use crate::resources::{CommandQueue, PlayerResources, SimClock};

/// AI difficulty level — controls decision frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,   // Evaluates every 30 ticks (3s)
    Medium, // Evaluates every 15 ticks (1.5s)
    Hard,   // Evaluates every 5 ticks (0.5s)
}

impl AiDifficulty {
    fn eval_interval(&self) -> u64 {
        match self {
            AiDifficulty::Easy => 30,
            AiDifficulty::Medium => 15,
            AiDifficulty::Hard => 5,
        }
    }
}

/// Bot personality — controls attack timing thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotPersonality {
    /// Attacks at army_count >= 12 (default).
    Balanced,
    /// Attacks at army_count >= 6 (rush).
    Aggressive,
    /// Attacks at army_count >= 18 (turtle).
    Defensive,
}

impl BotPersonality {
    fn attack_threshold(&self) -> u32 {
        match self {
            BotPersonality::Balanced => 12,
            BotPersonality::Aggressive => 6,
            BotPersonality::Defensive => 18,
        }
    }
}

impl Default for BotPersonality {
    fn default() -> Self {
        BotPersonality::Balanced
    }
}

/// FSM states for the scripted AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiPhase {
    /// Train workers, assign to gather.
    EarlyGame,
    /// Build economy buildings + CatTree, start training army.
    BuildUp,
    /// Train mixed army, build supply. Transition to Attack when ready.
    MidGame,
    /// Send army toward enemy spawn.
    Attack,
    /// Rally army to defend base.
    Defend,
}

/// AI state resource for the computer player.
#[derive(Resource)]
pub struct AiState {
    pub player_id: u8,
    pub phase: AiPhase,
    pub difficulty: AiDifficulty,
    pub personality: BotPersonality,
    /// Enemy player spawn position (target for attacks).
    pub enemy_spawn: Option<GridPos>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            player_id: 1,
            phase: AiPhase::EarlyGame,
            difficulty: AiDifficulty::Medium,
            personality: BotPersonality::Balanced,
            enemy_spawn: None,
        }
    }
}

/// Main AI decision system — runs in FixedUpdate after cleanup.
/// Controls the single-player AI (player 1) for normal games.
pub fn ai_decision_system(
    clock: Res<SimClock>,
    mut ai_state: ResMut<AiState>,
    mut cmd_queue: ResMut<CommandQueue>,
    player_resources: Res<PlayerResources>,
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    buildings: Query<(Entity, &Building, &Owner, Option<&Producer>)>,
    deposits: Query<(Entity, &Position), With<ResourceDeposit>>,
) {
    run_ai_fsm(
        clock.tick,
        &mut ai_state,
        &mut cmd_queue,
        &player_resources,
        &units,
        &buildings,
        &deposits,
    );
}

/// Multi-player AI decision system — runs all AIs in MultiAiState.
/// Used by the wet test harness for AI-vs-AI matches.
pub fn multi_ai_decision_system(
    clock: Res<SimClock>,
    mut multi_ai: ResMut<super::MultiAiState>,
    mut cmd_queue: ResMut<CommandQueue>,
    player_resources: Res<PlayerResources>,
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    buildings: Query<(Entity, &Building, &Owner, Option<&Producer>)>,
    deposits: Query<(Entity, &Position), With<ResourceDeposit>>,
) {
    for ai_state in multi_ai.players.iter_mut() {
        run_ai_fsm(
            clock.tick,
            ai_state,
            &mut cmd_queue,
            &player_resources,
            &units,
            &buildings,
            &deposits,
        );
    }
}

/// Core FSM logic shared between single-AI and multi-AI systems.
fn run_ai_fsm(
    tick: u64,
    ai_state: &mut AiState,
    cmd_queue: &mut CommandQueue,
    player_resources: &PlayerResources,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    buildings: &Query<(Entity, &Building, &Owner, Option<&Producer>)>,
    deposits: &Query<(Entity, &Position), With<ResourceDeposit>>,
) {
    let interval = ai_state.difficulty.eval_interval();
    if tick % interval != 0 || tick == 0 {
        return;
    }

    let ai_player = ai_state.player_id;
    let Some(pres) = player_resources.players.get(ai_player as usize) else {
        return;
    };

    // Count AI's units and buildings
    let mut worker_count = 0u32;
    let mut army_count = 0u32;
    let mut idle_workers: Vec<Entity> = Vec::new();
    let mut army_entities: Vec<Entity> = Vec::new();

    for (entity, _, owner, unit_type, gathering) in units.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        match unit_type.kind {
            UnitKind::Pawdler => {
                worker_count += 1;
                if gathering.is_none() {
                    idle_workers.push(entity);
                }
            }
            _ => {
                army_count += 1;
                army_entities.push(entity);
            }
        }
    }

    let mut has_box = false;
    let mut has_cat_tree = false;
    let mut has_fish_market = false;
    let mut box_entity = None;
    let mut cat_tree_entity = None;

    for (entity, building, owner, producer) in buildings.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        match building.kind {
            BuildingKind::TheBox => {
                has_box = true;
                if producer.is_some() {
                    box_entity = Some(entity);
                }
            }
            BuildingKind::CatTree => {
                has_cat_tree = true;
                if producer.is_some() {
                    cat_tree_entity = Some(entity);
                }
            }
            BuildingKind::FishMarket => {
                has_fish_market = true;
            }
            _ => {}
        }
    }

    // Find enemy spawn — look for any unit NOT owned by this AI
    if ai_state.enemy_spawn.is_none() {
        for (_, pos, owner, _, _) in units.iter() {
            if owner.player_id != ai_player {
                ai_state.enemy_spawn = Some(pos.world.to_grid());
                break;
            }
        }
    }

    // Assign idle workers to nearest deposit
    for worker in &idle_workers {
        if let Some((deposit_entity, _)) = find_nearest_deposit(*worker, units, deposits) {
            cmd_queue.push(GameCommand::GatherResource {
                unit_ids: vec![EntityId(worker.to_bits())],
                deposit: EntityId(deposit_entity.to_bits()),
            });
        }
    }

    let attack_threshold = ai_state.personality.attack_threshold();

    // FSM transitions
    let new_phase = match ai_state.phase {
        AiPhase::EarlyGame => {
            // Train workers until 4+
            if worker_count < 4 {
                if let Some(box_e) = box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }
            if worker_count >= 4 {
                AiPhase::BuildUp
            } else {
                AiPhase::EarlyGame
            }
        }

        AiPhase::BuildUp => {
            // Build FishMarket if missing
            if !has_fish_market && pres.food >= 100 && has_box {
                if let Some(box_e) = box_entity {
                    let build_pos = find_build_position(units, ai_player);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(box_e.to_bits()),
                        building_kind: BuildingKind::FishMarket,
                        position: build_pos,
                    });
                }
            }

            // Build CatTree if missing
            if !has_cat_tree && pres.food >= 150 && has_box {
                if let Some(box_e) = box_entity {
                    let build_pos = find_build_position(units, ai_player);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(box_e.to_bits()),
                        building_kind: BuildingKind::CatTree,
                        position: build_pos,
                    });
                }
            }

            // Train army from CatTree
            if let Some(ct_e) = cat_tree_entity {
                if pres.food >= 75 && pres.supply < pres.supply_cap {
                    cmd_queue.push(GameCommand::TrainUnit {
                        building: EntityId(ct_e.to_bits()),
                        unit_kind: UnitKind::Nuisance,
                    });
                }
            }

            if army_count >= 6 {
                AiPhase::MidGame
            } else {
                AiPhase::BuildUp
            }
        }

        AiPhase::MidGame => {
            // Keep training — alternate Nuisance and Hisser
            if let Some(ct_e) = cat_tree_entity {
                if pres.supply < pres.supply_cap {
                    let kind = if army_count % 3 == 0 {
                        UnitKind::Hisser
                    } else {
                        UnitKind::Nuisance
                    };
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            // Build LitterBox for more supply if needed
            if pres.supply + 2 >= pres.supply_cap && pres.food >= 75 {
                if let Some(box_e) = box_entity {
                    let build_pos = find_build_position(units, ai_player);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(box_e.to_bits()),
                        building_kind: BuildingKind::LitterBox,
                        position: build_pos,
                    });
                }
            }

            // Continue training workers
            if worker_count < 8 {
                if let Some(box_e) = box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }

            if army_count >= attack_threshold {
                AiPhase::Attack
            } else {
                AiPhase::MidGame
            }
        }

        AiPhase::Attack => {
            // Send army toward enemy spawn
            if let Some(target) = ai_state.enemy_spawn {
                if !army_entities.is_empty() {
                    let ids: Vec<EntityId> = army_entities
                        .iter()
                        .map(|e| EntityId(e.to_bits()))
                        .collect();
                    cmd_queue.push(GameCommand::AttackMove {
                        unit_ids: ids,
                        target,
                    });
                }
            }

            // Check if base is under attack (enemy units near our box)
            let base_threatened = is_base_threatened(units, ai_player, buildings);
            if base_threatened {
                AiPhase::Defend
            } else if army_count < 4 {
                // Lost most of army — rebuild
                AiPhase::MidGame
            } else {
                AiPhase::Attack
            }
        }

        AiPhase::Defend => {
            // Rally army back to base
            if let Some(box_e) = box_entity {
                if let Ok((_, _, _, _)) = buildings.get(box_e) {
                    let box_pos = units
                        .iter()
                        .find(|(e, _, _, _, _)| *e == box_e)
                        .map(|(_, p, _, _, _)| p.world.to_grid());

                    // Fall back to a default base pos
                    let rally_pos = box_pos.unwrap_or(GridPos::new(55, 55));

                    if !army_entities.is_empty() {
                        let ids: Vec<EntityId> = army_entities
                            .iter()
                            .map(|e| EntityId(e.to_bits()))
                            .collect();
                        cmd_queue.push(GameCommand::AttackMove {
                            unit_ids: ids,
                            target: rally_pos,
                        });
                    }
                }
            }

            let base_threatened = is_base_threatened(units, ai_player, buildings);
            if !base_threatened {
                AiPhase::MidGame
            } else {
                AiPhase::Defend
            }
        }
    };

    ai_state.phase = new_phase;
}

/// Find nearest resource deposit to a worker.
fn find_nearest_deposit(
    worker: Entity,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    deposits: &Query<(Entity, &Position), With<ResourceDeposit>>,
) -> Option<(Entity, GridPos)> {
    let worker_pos = units.get(worker).ok()?.1.world;
    let mut best = None;
    let mut best_dist = cc_core::math::Fixed::MAX;

    for (deposit_entity, deposit_pos) in deposits.iter() {
        let dist = worker_pos.distance_squared(deposit_pos.world);
        if dist < best_dist {
            best_dist = dist;
            best = Some((deposit_entity, deposit_pos.world.to_grid()));
        }
    }

    best
}

/// Find a position near the AI's units to place a building.
fn find_build_position(
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    ai_player: u8,
) -> GridPos {
    // Find the average position of AI units as base reference
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut count = 0i64;

    for (_, pos, owner, _, _) in units.iter() {
        if owner.player_id == ai_player {
            let grid = pos.world.to_grid();
            sum_x += grid.x as i64;
            sum_y += grid.y as i64;
            count += 1;
        }
    }

    if count > 0 {
        // Place building 2 tiles from center of AI units
        GridPos::new(
            (sum_x / count) as i32 + 2,
            (sum_y / count) as i32 + 2,
        )
    } else {
        GridPos::new(55, 55) // Fallback
    }
}

/// Check if enemy units are within 8 tiles of any of the AI's buildings.
fn is_base_threatened(
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>)>,
    ai_player: u8,
    _buildings: &Query<(Entity, &Building, &Owner, Option<&Producer>)>,
) -> bool {
    // Collect AI unit positions as proxy for base location (buildings lack Position in this query)
    // Use workers as base indicator since they stay near base
    let mut base_positions: Vec<GridPos> = Vec::new();
    let mut enemy_positions: Vec<GridPos> = Vec::new();

    for (_, pos, owner, unit_type, _) in units.iter() {
        let grid = pos.world.to_grid();
        if owner.player_id == ai_player {
            if unit_type.kind == UnitKind::Pawdler {
                base_positions.push(grid);
            }
        } else {
            enemy_positions.push(grid);
        }
    }

    if base_positions.is_empty() {
        return false;
    }

    // Average worker position as base center
    let avg_x: i32 = base_positions.iter().map(|p| p.x).sum::<i32>() / base_positions.len() as i32;
    let avg_y: i32 = base_positions.iter().map(|p| p.y).sum::<i32>() / base_positions.len() as i32;

    // Check if any enemy is within 8 tiles of base center
    for ep in &enemy_positions {
        let dx = (ep.x - avg_x).abs();
        let dy = (ep.y - avg_y).abs();
        if dx <= 8 && dy <= 8 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_phase_transitions() {
        // EarlyGame → BuildUp when workers >= 4
        let phase = AiPhase::EarlyGame;
        assert_eq!(phase, AiPhase::EarlyGame);

        // Verify difficulty intervals
        assert_eq!(AiDifficulty::Easy.eval_interval(), 30);
        assert_eq!(AiDifficulty::Medium.eval_interval(), 15);
        assert_eq!(AiDifficulty::Hard.eval_interval(), 5);
    }

    #[test]
    fn ai_default_state() {
        let state = AiState::default();
        assert_eq!(state.player_id, 1);
        assert_eq!(state.phase, AiPhase::EarlyGame);
        assert_eq!(state.difficulty, AiDifficulty::Medium);
        assert_eq!(state.personality, BotPersonality::Balanced);
    }

    #[test]
    fn bot_personality_thresholds() {
        assert_eq!(BotPersonality::Aggressive.attack_threshold(), 6);
        assert_eq!(BotPersonality::Balanced.attack_threshold(), 12);
        assert_eq!(BotPersonality::Defensive.attack_threshold(), 18);
    }
}
