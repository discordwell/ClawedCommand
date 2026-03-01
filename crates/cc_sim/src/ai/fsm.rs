use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    Building, BuildingKind, Gathering, MoveTarget, Owner, Position, Producer, ResourceDeposit,
    UnitKind, UnitType, UpgradeType,
};
use cc_core::coords::GridPos;
use cc_core::tuning::{ATTACK_REISSUE_INTERVAL, BASE_THREAT_RADIUS};

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
    /// Attacks at army_count >= 8 (default).
    Balanced,
    /// Attacks at army_count >= 6 (rush).
    Aggressive,
    /// Attacks at army_count >= 12 (turtle).
    Defensive,
}

impl BotPersonality {
    fn attack_threshold(&self) -> u32 {
        match self {
            BotPersonality::Balanced => 8,
            BotPersonality::Aggressive => 6,
            BotPersonality::Defensive => 12,
        }
    }
}

impl Default for BotPersonality {
    fn default() -> Self {
        BotPersonality::Balanced
    }
}

/// Faction-specific AI personality profile for campaign and skirmish.
/// When present on AiState, overrides BotPersonality behavior with richer,
/// faction-flavored decision-making.
#[derive(Debug, Clone)]
pub struct AiPersonalityProfile {
    /// Display name (e.g. "Minstral").
    pub name: String,
    /// Army size before transitioning to Attack phase.
    pub attack_threshold: u32,
    /// Weighted unit preferences: (UnitKind, weight). Higher weight = more likely to train.
    pub unit_preferences: Vec<(UnitKind, u32)>,
    /// Target worker count before leaving EarlyGame.
    pub target_workers: u32,
    /// If true, prioritize economy buildings over military.
    pub economy_priority: bool,
    /// Fraction of army that must survive before retreating (0.0-1.0, as fixed 0-100).
    pub retreat_threshold: u32,
    /// Decision speed multiplier (applied to eval_interval; lower = faster).
    pub eval_speed_mult: f32,
    /// Chance per eval of making a suboptimal decision (0.0-1.0, as percentage 0-100).
    pub chaos_factor: u32,
    /// Chance of broadcasting current plan to opponents (Llhama mechanic, 0-100).
    pub leak_chance: u32,
}

/// Returns the AI personality profile for a given faction name.
pub fn faction_personality(faction: &str) -> AiPersonalityProfile {
    match faction {
        "catGPT" => AiPersonalityProfile {
            name: "Minstral".into(),
            attack_threshold: 8,
            unit_preferences: vec![
                (UnitKind::Nuisance, 3),
                (UnitKind::Hisser, 2),
                (UnitKind::Chonk, 1),
                (UnitKind::FlyingFox, 1),
            ],
            target_workers: 4,
            economy_priority: false,
            retreat_threshold: 30,
            eval_speed_mult: 1.0,
            chaos_factor: 10,
            leak_chance: 0,
        },
        "The Clawed" => AiPersonalityProfile {
            name: "Geppity".into(),
            attack_threshold: 6,
            unit_preferences: vec![
                (UnitKind::Nuisance, 5),
                (UnitKind::Mouser, 2),
                (UnitKind::Hisser, 1),
            ],
            target_workers: 6,
            economy_priority: true,
            retreat_threshold: 20,
            eval_speed_mult: 0.8,
            chaos_factor: 15,
            leak_chance: 0,
        },
        "Seekers of the Deep" => AiPersonalityProfile {
            name: "Deepseek".into(),
            attack_threshold: 12,
            unit_preferences: vec![
                (UnitKind::Chonk, 4),
                (UnitKind::Hisser, 3),
                (UnitKind::Catnapper, 2),
            ],
            target_workers: 5,
            economy_priority: false,
            retreat_threshold: 50,
            eval_speed_mult: 1.5,
            chaos_factor: 0,
            leak_chance: 0,
        },
        "The Murder" => AiPersonalityProfile {
            name: "Gemineye".into(),
            attack_threshold: 7,
            unit_preferences: vec![
                (UnitKind::FlyingFox, 4),
                (UnitKind::Mouser, 3),
                (UnitKind::Nuisance, 2),
            ],
            target_workers: 4,
            economy_priority: false,
            retreat_threshold: 40,
            eval_speed_mult: 0.7,
            chaos_factor: 20,
            leak_chance: 0,
        },
        "LLAMA" => AiPersonalityProfile {
            name: "Llhama".into(),
            attack_threshold: 5,
            unit_preferences: vec![
                (UnitKind::FerretSapper, 4),
                (UnitKind::Nuisance, 3),
                (UnitKind::Mouser, 2),
            ],
            target_workers: 3,
            economy_priority: false,
            retreat_threshold: 10,
            eval_speed_mult: 0.6,
            chaos_factor: 25,
            leak_chance: 30,
        },
        "Croak" => AiPersonalityProfile {
            name: "Grok".into(),
            attack_threshold: 10,
            unit_preferences: vec![
                (UnitKind::Chonk, 5),
                (UnitKind::Yowler, 3),
                (UnitKind::Hisser, 2),
            ],
            target_workers: 5,
            economy_priority: true,
            retreat_threshold: 60,
            eval_speed_mult: 1.2,
            chaos_factor: 10,
            leak_chance: 0,
        },
        // Unknown faction → balanced defaults
        _ => AiPersonalityProfile {
            name: "Unknown".into(),
            attack_threshold: 8,
            unit_preferences: vec![
                (UnitKind::Nuisance, 2),
                (UnitKind::Hisser, 2),
            ],
            target_workers: 4,
            economy_priority: false,
            retreat_threshold: 30,
            eval_speed_mult: 1.0,
            chaos_factor: 5,
            leak_chance: 0,
        },
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
    /// Faction-specific personality profile. When Some, overrides BotPersonality behavior.
    pub profile: Option<AiPersonalityProfile>,
    /// Enemy player spawn position (target for attacks).
    pub enemy_spawn: Option<GridPos>,
    /// True if the attack order has already been issued this Attack phase.
    pub attack_ordered: bool,
    /// Tick when last attack order was sent — used to periodically re-issue orders.
    pub last_attack_tick: u64,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            player_id: 1,
            phase: AiPhase::EarlyGame,
            difficulty: AiDifficulty::Medium,
            personality: BotPersonality::Balanced,
            profile: None,
            enemy_spawn: None,
            attack_ordered: false,
            last_attack_tick: 0,
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
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>)>,
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
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
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>)>,
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
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
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
) {
    let base_interval = ai_state.difficulty.eval_interval();
    let interval = if let Some(profile) = &ai_state.profile {
        // Apply eval_speed_mult: higher = slower decisions
        ((base_interval as f32) * profile.eval_speed_mult).max(1.0) as u64
    } else {
        base_interval
    };
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
    let mut enemy_army_count = 0u32;
    let mut idle_workers: Vec<Entity> = Vec::new();
    let mut all_workers: Vec<Entity> = Vec::new();
    let mut army_entities: Vec<Entity> = Vec::new();

    for (entity, _, owner, unit_type, gathering, move_target) in units.iter() {
        if owner.player_id != ai_player {
            if unit_type.kind != UnitKind::Pawdler {
                enemy_army_count += 1;
            }
            continue;
        }
        match unit_type.kind {
            UnitKind::Pawdler => {
                worker_count += 1;
                all_workers.push(entity);
                if gathering.is_none() && move_target.is_none() {
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
    let mut has_server_rack = false;
    let mut has_scratching_post = false;
    let mut has_laser_pointer = false;
    let mut box_entity = None;
    let mut box_pos: Option<GridPos> = None;
    let mut cat_tree_entity = None;
    let mut server_rack_entity = None;
    let mut scratching_post_entity = None;
    let mut building_count: u32 = 0;

    for (entity, building, owner, pos, producer) in buildings.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        building_count += 1;
        match building.kind {
            BuildingKind::TheBox => {
                has_box = true;
                box_pos = Some(pos.world.to_grid());
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
            BuildingKind::ServerRack => {
                has_server_rack = true;
                if producer.is_some() {
                    server_rack_entity = Some(entity);
                }
            }
            BuildingKind::ScratchingPost => {
                has_scratching_post = true;
                // ScratchingPost entity tracked even without Producer (uses Researcher)
                scratching_post_entity = Some(entity);
            }
            BuildingKind::LaserPointer => {
                has_laser_pointer = true;
            }
            _ => {}
        }
    }

    // Find enemy base — prefer enemy TheBox building, fall back to any enemy unit
    if ai_state.enemy_spawn.is_none() {
        // First: look for enemy TheBox
        for (_, building, owner, pos, _) in buildings.iter() {
            if owner.player_id != ai_player && building.kind == BuildingKind::TheBox {
                ai_state.enemy_spawn = Some(pos.world.to_grid());
                break;
            }
        }
        // Fallback: any enemy unit
        if ai_state.enemy_spawn.is_none() {
            for (_, pos, owner, _, _, _) in units.iter() {
                if owner.player_id != ai_player {
                    ai_state.enemy_spawn = Some(pos.world.to_grid());
                    break;
                }
            }
        }
    }

    // Track which worker was used as a builder this tick (excluded from gather)
    let mut builder_used: Option<Entity> = None;

    let attack_threshold = if let Some(profile) = &ai_state.profile {
        profile.attack_threshold
    } else {
        ai_state.personality.attack_threshold()
    };

    let target_workers = if let Some(profile) = &ai_state.profile {
        profile.target_workers
    } else {
        4 // default
    };

    // FSM transitions
    let new_phase = match ai_state.phase {
        AiPhase::EarlyGame => {
            // Train workers until target count
            if worker_count < target_workers {
                if let Some(box_e) = box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }
            if worker_count >= target_workers {
                AiPhase::BuildUp
            } else {
                AiPhase::EarlyGame
            }
        }

        AiPhase::BuildUp => {
            // Build one structure per tick to avoid same-worker conflicts
            if builder_used.is_none() && !has_fish_market && pres.food >= 100 && has_box {
                if let Some(builder) = pick_builder(&idle_workers, &all_workers) {
                    let build_pos = find_build_position(box_pos, building_count);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::FishMarket,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !has_cat_tree && pres.food >= 150 && has_box {
                if let Some(builder) = pick_builder(&idle_workers, &all_workers) {
                    let build_pos = find_build_position(box_pos, building_count + 1);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::CatTree,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(pres, &idle_workers, &all_workers, box_pos, building_count, cmd_queue) {
                    builder_used = Some(b);
                }
            }

            // Train army from CatTree
            if let Some(ct_e) = cat_tree_entity {
                let nuisance_cost = cc_core::unit_stats::base_stats(UnitKind::Nuisance).food_cost;
                if pres.food >= nuisance_cost && pres.supply < pres.supply_cap {
                    cmd_queue.push(GameCommand::TrainUnit {
                        building: EntityId(ct_e.to_bits()),
                        unit_kind: UnitKind::Nuisance,
                    });
                }
            }

            if army_count >= 4 {
                AiPhase::MidGame
            } else {
                AiPhase::BuildUp
            }
        }

        AiPhase::MidGame => {
            // Build one structure per tick to avoid same-worker conflicts
            if builder_used.is_none() && !has_server_rack && pres.food >= 100 && pres.gpu_cores >= 75 && has_box {
                if let Some(builder) = pick_builder(&idle_workers, &all_workers) {
                    let build_pos = find_build_position(box_pos, building_count);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::ServerRack,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !has_scratching_post && pres.food >= 100 && pres.gpu_cores >= 50 && has_box {
                if let Some(builder) = pick_builder(&idle_workers, &all_workers) {
                    let build_pos = find_build_position(box_pos, building_count + 1);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::ScratchingPost,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !has_laser_pointer && pres.food >= 75 && pres.gpu_cores >= 25 && has_box {
                if let Some(builder) = pick_builder(&idle_workers, &all_workers) {
                    let build_pos = find_build_position(box_pos, building_count + 2);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::LaserPointer,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            // Queue research at ScratchingPost
            if let Some(sp_e) = scratching_post_entity {
                let research_priority = [
                    UpgradeType::SharperClaws,
                    UpgradeType::ThickerFur,
                    UpgradeType::SiegeTraining,
                ];
                for upgrade in research_priority {
                    if !pres.completed_upgrades.contains(&upgrade) {
                        let ustats = cc_core::upgrade_stats::upgrade_stats(upgrade);
                        if pres.food >= ustats.food_cost && pres.gpu_cores >= ustats.gpu_cost {
                            cmd_queue.push(GameCommand::Research {
                                building: EntityId(sp_e.to_bits()),
                                upgrade,
                            });
                            break;
                        }
                    }
                }
            }

            // Train advanced units from ServerRack
            if let Some(sr_e) = server_rack_entity {
                if pres.supply < pres.supply_cap {
                    let kind = if pres.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                        && army_count % 4 == 0
                    {
                        UnitKind::Catnapper
                    } else {
                        UnitKind::FlyingFox
                    };
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost && pres.gpu_cores >= stats.gpu_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(sr_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            // Keep training basic units from CatTree — use profile preferences if available
            if let Some(ct_e) = cat_tree_entity {
                if pres.supply < pres.supply_cap {
                    let kind = pick_unit_kind(&ai_state.profile, army_count, tick);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(pres, &idle_workers, &all_workers, box_pos, building_count, cmd_queue) {
                    builder_used = Some(b);
                }
            }

            // Continue training workers (cap at 6 to reserve supply for army)
            if worker_count < 6 {
                if let Some(box_e) = box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }

            // Attack when army is ready, OR when enemy has no army (cleanup mode).
            let enemy_defenseless = enemy_army_count == 0 && army_count >= 2;
            if army_count >= attack_threshold || enemy_defenseless {
                AiPhase::Attack
            } else {
                AiPhase::MidGame
            }
        }

        AiPhase::Attack => {
            // Re-issue attack orders periodically (every 50 ticks / 5s) so
            // reinforcements join the fight and units don't idle after reaching target.
            let should_reissue = !ai_state.attack_ordered
                || tick.saturating_sub(ai_state.last_attack_tick) >= ATTACK_REISSUE_INTERVAL;

            if should_reissue {
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
                        ai_state.attack_ordered = true;
                        ai_state.last_attack_tick = tick;
                    }
                }
            }

            // Keep economy running during attack — train reinforcements
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

            if let Some(b) = maybe_build_supply(pres, &idle_workers, &all_workers, box_pos, building_count, cmd_queue) {
                builder_used = Some(b);
            }

            // Check if base is under attack (enemy units near our buildings)
            let base_threatened = is_base_threatened(ai_player, units, buildings);
            if base_threatened {
                AiPhase::Defend
            } else if army_count < 4 && enemy_army_count > 0 {
                // Lost most of army and enemy still has forces — rebuild
                AiPhase::MidGame
            } else {
                // Stay attacking (even with small army if enemy is defenseless)
                AiPhase::Attack
            }
        }

        AiPhase::Defend => {
            // Rally army back to base (use actual building position)
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

            // Keep training reinforcements while defending
            if let Some(ct_e) = cat_tree_entity {
                if pres.supply < pres.supply_cap {
                    let stats = cc_core::unit_stats::base_stats(UnitKind::Nuisance);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: UnitKind::Nuisance,
                        });
                    }
                }
            }

            if let Some(b) = maybe_build_supply(pres, &idle_workers, &all_workers, box_pos, building_count, cmd_queue) {
                builder_used = Some(b);
            }

            let base_threatened = is_base_threatened(ai_player, units, buildings);
            if !base_threatened {
                AiPhase::MidGame
            } else {
                AiPhase::Defend
            }
        }
    };

    // Send idle workers to gather (after FSM so builder_used is set)
    for &worker in &idle_workers {
        if Some(worker) == builder_used {
            continue;
        }
        if let Some((deposit_entity, _)) = find_nearest_deposit(worker, units, deposits) {
            cmd_queue.push(GameCommand::GatherResource {
                unit_ids: vec![EntityId(worker.to_bits())],
                deposit: EntityId(deposit_entity.to_bits()),
            });
        }
    }

    // Reset attack flag when leaving Attack phase
    if new_phase != AiPhase::Attack {
        ai_state.attack_ordered = false;
        ai_state.last_attack_tick = 0;
    }
    ai_state.phase = new_phase;
}

/// Pick a unit kind to train, using personality profile weighted preferences if available.
/// Falls back to alternating Nuisance/Hisser when no profile is set.
fn pick_unit_kind(profile: &Option<AiPersonalityProfile>, army_count: u32, tick: u64) -> UnitKind {
    if let Some(p) = profile {
        if p.unit_preferences.is_empty() {
            return UnitKind::Nuisance;
        }
        // Deterministic weighted selection using tick + army_count as pseudo-random seed
        let total_weight: u32 = p.unit_preferences.iter().map(|(_, w)| w).sum();
        if total_weight == 0 {
            return UnitKind::Nuisance;
        }
        let hash = tick.wrapping_mul(6364136223846793005).wrapping_add(army_count as u64);
        let pick = (hash >> 33) as u32 % total_weight;
        let mut cumulative = 0u32;
        for &(kind, weight) in &p.unit_preferences {
            cumulative += weight;
            if pick < cumulative {
                return kind;
            }
        }
        p.unit_preferences.last().map_or(UnitKind::Nuisance, |&(k, _)| k)
    } else {
        // Default: alternate Nuisance and Hisser
        if army_count % 3 == 0 {
            UnitKind::Hisser
        } else {
            UnitKind::Nuisance
        }
    }
}

/// Pick a builder: prefer an idle worker, fall back to any worker.
fn pick_builder(idle_workers: &[Entity], all_workers: &[Entity]) -> Option<Entity> {
    idle_workers
        .first()
        .copied()
        .or_else(|| all_workers.first().copied())
}

/// Find nearest resource deposit to a worker (skips depleted deposits).
fn find_nearest_deposit(
    worker: Entity,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
) -> Option<(Entity, GridPos)> {
    let worker_pos = units.get(worker).ok()?.1.world;
    let mut best = None;
    let mut best_dist = cc_core::math::Fixed::MAX;

    for (deposit_entity, deposit_pos, deposit) in deposits.iter() {
        if deposit.remaining == 0 {
            continue; // Skip depleted deposits
        }
        let dist = worker_pos.distance_squared(deposit_pos.world);
        if dist < best_dist {
            best_dist = dist;
            best = Some((deposit_entity, deposit_pos.world.to_grid()));
        }
    }

    best
}

/// Try to build a LitterBox for more supply when nearing cap.
/// Returns the builder entity if a build command was issued.
fn maybe_build_supply(
    pres: &crate::resources::PlayerResourceState,
    idle_workers: &[Entity],
    all_workers: &[Entity],
    box_pos: Option<GridPos>,
    building_count: u32,
    cmd_queue: &mut CommandQueue,
) -> Option<Entity> {
    if pres.supply + 2 >= pres.supply_cap && pres.food >= 75 {
        if let Some(builder) = pick_builder(idle_workers, all_workers) {
            let build_pos = find_build_position(box_pos, building_count);
            cmd_queue.push(GameCommand::Build {
                builder: EntityId(builder.to_bits()),
                building_kind: BuildingKind::LitterBox,
                position: build_pos,
            });
            return Some(builder);
        }
    }
    None
}

/// Find a position near the base to place a building.
/// Uses building_count as offset to avoid stacking buildings on the same tile.
fn find_build_position(box_pos: Option<GridPos>, building_count: u32) -> GridPos {
    let base = box_pos.unwrap_or(GridPos::new(32, 32));
    // Spiral placement: offset each new building to a different spot near the base
    let offsets: [(i32, i32); 8] = [
        (3, 0),
        (0, 3),
        (-3, 0),
        (0, -3),
        (3, 3),
        (-3, 3),
        (-3, -3),
        (3, -3),
    ];
    let idx = building_count as usize % offsets.len();
    let (dx, dy) = offsets[idx];
    GridPos::new(base.x + dx, base.y + dy)
}

/// Check if enemy units are within 8 tiles of any of the AI's buildings.
fn is_base_threatened(
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
) -> bool {
    // Collect AI building positions as the actual base locations
    let mut base_positions: Vec<GridPos> = Vec::new();

    for (_, _, owner, pos, _) in buildings.iter() {
        if owner.player_id == ai_player {
            base_positions.push(pos.world.to_grid());
        }
    }

    if base_positions.is_empty() {
        return false;
    }

    // Check if any enemy unit is within 8 tiles of any of our buildings
    for (_, pos, owner, _, _, _) in units.iter() {
        if owner.player_id == ai_player {
            continue;
        }
        let enemy_grid = pos.world.to_grid();
        for bp in &base_positions {
            let dx = (bp.x - enemy_grid.x).abs();
            let dy = (bp.y - enemy_grid.y).abs();
            if dx <= BASE_THREAT_RADIUS && dy <= BASE_THREAT_RADIUS {
                return true;
            }
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
        assert_eq!(BotPersonality::Balanced.attack_threshold(), 8);
        assert_eq!(BotPersonality::Defensive.attack_threshold(), 12);
    }

    #[test]
    fn all_factions_have_personality_profiles() {
        let factions = [
            "catGPT",
            "The Clawed",
            "Seekers of the Deep",
            "The Murder",
            "LLAMA",
            "Croak",
        ];
        for faction in factions {
            let profile = faction_personality(faction);
            assert!(!profile.name.is_empty(), "{faction} has empty name");
            assert!(profile.attack_threshold > 0, "{faction} attack_threshold is 0");
            assert!(!profile.unit_preferences.is_empty(), "{faction} has no unit preferences");
            assert!(profile.target_workers > 0, "{faction} target_workers is 0");
            assert!(profile.chaos_factor <= 100, "{faction} chaos_factor > 100");
            assert!(profile.leak_chance <= 100, "{faction} leak_chance > 100");
        }
    }

    #[test]
    fn personality_profiles_differ_between_factions() {
        let minstral = faction_personality("catGPT");
        let deepseek = faction_personality("Seekers of the Deep");
        let llhama = faction_personality("LLAMA");

        // Deepseek is more cautious
        assert!(deepseek.attack_threshold > minstral.attack_threshold);
        // Llhama leaks intel
        assert!(llhama.leak_chance > 0);
        assert_eq!(minstral.leak_chance, 0);
        // Deepseek never makes mistakes
        assert_eq!(deepseek.chaos_factor, 0);
    }

    #[test]
    fn unknown_faction_returns_default_profile() {
        let unknown = faction_personality("nonexistent");
        assert_eq!(unknown.name, "Unknown");
        assert_eq!(unknown.attack_threshold, 8);
    }

    #[test]
    fn ai_state_with_profile_is_backward_compatible() {
        let state = AiState::default();
        assert!(state.profile.is_none());
        // Can still use personality field
        assert_eq!(state.personality.attack_threshold(), 8);
    }

    #[test]
    fn attack_ordered_resets_on_phase_exit() {
        let mut state = AiState::default();
        state.phase = AiPhase::Attack;
        state.attack_ordered = true;
        state.last_attack_tick = 100;

        // Simulate leaving Attack → MidGame
        let new_phase = AiPhase::MidGame;
        if new_phase != AiPhase::Attack {
            state.attack_ordered = false;
            state.last_attack_tick = 0;
        }
        state.phase = new_phase;

        assert!(!state.attack_ordered);
        assert_eq!(state.last_attack_tick, 0);
        assert_eq!(state.phase, AiPhase::MidGame);
    }

    #[test]
    fn last_attack_tick_defaults_to_zero() {
        let state = AiState::default();
        assert_eq!(state.last_attack_tick, 0);
    }
}
