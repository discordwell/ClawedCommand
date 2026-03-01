use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    AttackType, BuildOrder, Building, BuildingKind, Dead, Faction, Gathering, Health, MoveTarget,
    Owner, Position, Producer, ResourceDeposit, UnitKind, UnitType, UpgradeType,
};
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::tuning::{AI_BUILD_SPACING, ATTACK_REISSUE_INTERVAL, BASE_THREAT_RADIUS};

use crate::resources::{CommandQueue, MapResource, PlayerResources, SimClock};

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

/// Unified AI personality profile for campaign and skirmish.
/// Replaces the previous split between `BotPersonality` (simple enum) and
/// `AiPersonalityProfile` (rich struct). All AI agents now use this single type.
#[derive(Debug, Clone)]
pub struct AiPersonalityProfile {
    /// Display name (e.g. "Geppity", "Balanced").
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

impl AiPersonalityProfile {
    /// Balanced preset (default) — attacks at army_count >= 8.
    pub fn balanced() -> Self {
        Self {
            name: "Balanced".into(),
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
        }
    }

    /// Aggressive preset — attacks at army_count >= 6 (rush).
    pub fn aggressive() -> Self {
        Self {
            name: "Aggressive".into(),
            attack_threshold: 6,
            unit_preferences: vec![
                (UnitKind::Nuisance, 3),
                (UnitKind::Hisser, 1),
            ],
            target_workers: 3,
            economy_priority: false,
            retreat_threshold: 20,
            eval_speed_mult: 0.8,
            chaos_factor: 10,
            leak_chance: 0,
        }
    }

    /// Defensive preset — attacks at army_count >= 12 (turtle).
    pub fn defensive() -> Self {
        Self {
            name: "Defensive".into(),
            attack_threshold: 12,
            unit_preferences: vec![
                (UnitKind::Chonk, 3),
                (UnitKind::Hisser, 2),
            ],
            target_workers: 5,
            economy_priority: true,
            retreat_threshold: 50,
            eval_speed_mult: 1.2,
            chaos_factor: 0,
            leak_chance: 0,
        }
    }
}

impl Default for AiPersonalityProfile {
    fn default() -> Self {
        Self::balanced()
    }
}

/// Returns the AI personality profile for a given Faction enum value.
pub fn faction_personality(faction: Faction) -> AiPersonalityProfile {
    match faction {
        Faction::CatGpt => AiPersonalityProfile {
            name: "Geppity".into(),
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
        Faction::TheClawed => AiPersonalityProfile {
            name: "Claudeus Maximus".into(),
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
        Faction::SeekersOfTheDeep => AiPersonalityProfile {
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
        Faction::TheMurder => AiPersonalityProfile {
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
        Faction::Llama => AiPersonalityProfile {
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
        Faction::Croak => AiPersonalityProfile {
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
        Faction::Neutral => AiPersonalityProfile {
            name: "Neutral".into(),
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

/// Returns the AI personality profile for a faction name string.
/// Falls back to Neutral for unknown faction names.
pub fn faction_personality_by_name(faction: &str) -> AiPersonalityProfile {
    let f = Faction::from_faction_str(faction).unwrap_or(Faction::Neutral);
    faction_personality(f)
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
    /// Unified personality profile controlling all AI behavior parameters.
    pub profile: AiPersonalityProfile,
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
            profile: AiPersonalityProfile::default(),
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
    map_res: Res<MapResource>,
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
) {
    run_ai_fsm(
        clock.tick,
        &mut ai_state,
        &mut cmd_queue,
        &player_resources,
        &map_res.map,
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
    map_res: Res<MapResource>,
    units: Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
) {
    for ai_state in multi_ai.players.iter_mut() {
        run_ai_fsm(
            clock.tick,
            ai_state,
            &mut cmd_queue,
            &player_resources,
            &map_res.map,
            &units,
            &buildings,
            &deposits,
        );
    }
}

/// Census of the AI player's units.
struct UnitCensus {
    worker_count: u32,
    army_count: u32,
    enemy_army_count: u32,
    idle_workers: Vec<Entity>,
    all_workers: Vec<Entity>,
    army_entities: Vec<Entity>,
    /// BuildOrders currently in-flight (kind + target position).
    pending_builds: Vec<(BuildingKind, GridPos)>,
}

/// Census of the AI player's buildings.
struct BuildingCensus {
    has_box: bool,
    has_cat_tree: bool,
    has_fish_market: bool,
    has_server_rack: bool,
    has_scratching_post: bool,
    has_laser_pointer: bool,
    /// Total number of FishMarkets (completed + pending).
    fish_market_count: u32,
    /// Total number of LitterBoxes (completed + pending).
    litter_box_count: u32,
    box_entity: Option<Entity>,
    box_pos: Option<GridPos>,
    cat_tree_entity: Option<Entity>,
    server_rack_entity: Option<Entity>,
    scratching_post_entity: Option<Entity>,
    building_positions: Vec<(GridPos, BuildingKind)>,
}

/// Scan all units and classify them as workers, army, or enemy army.
fn take_unit_census(
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
) -> UnitCensus {
    let mut census = UnitCensus {
        worker_count: 0,
        army_count: 0,
        enemy_army_count: 0,
        idle_workers: Vec::new(),
        all_workers: Vec::new(),
        army_entities: Vec::new(),
        pending_builds: Vec::new(),
    };
    for (entity, _, owner, unit_type, gathering, move_target, build_order) in units.iter() {
        if owner.player_id != ai_player {
            if unit_type.kind != UnitKind::Pawdler {
                census.enemy_army_count += 1;
            }
            continue;
        }
        match unit_type.kind {
            UnitKind::Pawdler => {
                census.worker_count += 1;
                // Workers with active BuildOrders are busy building — exclude from
                // both idle and available-builder lists to prevent reassignment.
                if let Some(bo) = build_order {
                    census.pending_builds.push((bo.building_kind, bo.position));
                } else {
                    census.all_workers.push(entity);
                    if gathering.is_none() && move_target.is_none() {
                        census.idle_workers.push(entity);
                    }
                }
            }
            _ => {
                census.army_count += 1;
                census.army_entities.push(entity);
            }
        }
    }
    census
}

/// Scan all buildings owned by the AI player.
fn take_building_census(
    ai_player: u8,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
) -> BuildingCensus {
    let mut census = BuildingCensus {
        has_box: false,
        has_cat_tree: false,
        has_fish_market: false,
        has_server_rack: false,
        has_scratching_post: false,
        has_laser_pointer: false,
        fish_market_count: 0,
        litter_box_count: 0,
        box_entity: None,
        box_pos: None,
        cat_tree_entity: None,
        server_rack_entity: None,
        scratching_post_entity: None,
        building_positions: Vec::new(),
    };
    for (entity, building, owner, pos, producer) in buildings.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        census.building_positions.push((pos.world.to_grid(), building.kind));
        match building.kind {
            BuildingKind::TheBox => {
                census.has_box = true;
                census.box_pos = Some(pos.world.to_grid());
                if producer.is_some() {
                    census.box_entity = Some(entity);
                }
            }
            BuildingKind::CatTree => {
                census.has_cat_tree = true;
                if producer.is_some() {
                    census.cat_tree_entity = Some(entity);
                }
            }
            BuildingKind::FishMarket => {
                census.has_fish_market = true;
                census.fish_market_count += 1;
            }
            BuildingKind::LitterBox => {
                census.litter_box_count += 1;
            }
            BuildingKind::ServerRack => {
                census.has_server_rack = true;
                if producer.is_some() {
                    census.server_rack_entity = Some(entity);
                }
            }
            BuildingKind::ScratchingPost => {
                census.has_scratching_post = true;
                census.scratching_post_entity = Some(entity);
            }
            BuildingKind::LaserPointer => {
                census.has_laser_pointer = true;
            }
            _ => {}
        }
    }
    census
}

/// Discover the enemy base position. Prefers enemy TheBox, falls back to any enemy unit.
fn discover_enemy_spawn(
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
) -> Option<GridPos> {
    // First: look for enemy TheBox
    for (_, building, owner, pos, _) in buildings.iter() {
        if owner.player_id != ai_player && building.kind == BuildingKind::TheBox {
            return Some(pos.world.to_grid());
        }
    }
    // Fallback: any enemy unit
    for (_, pos, owner, _, _, _, _) in units.iter() {
        if owner.player_id != ai_player {
            return Some(pos.world.to_grid());
        }
    }
    None
}

/// Core FSM logic shared between single-AI and multi-AI systems.
fn run_ai_fsm(
    tick: u64,
    ai_state: &mut AiState,
    cmd_queue: &mut CommandQueue,
    player_resources: &PlayerResources,
    map: &GameMap,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
) {
    let base_interval = ai_state.difficulty.eval_interval();
    // Apply eval_speed_mult from profile: higher = slower decisions
    let interval = ((base_interval as f32) * ai_state.profile.eval_speed_mult).max(1.0) as u64;
    if tick % interval != 0 || tick == 0 {
        return;
    }

    let ai_player = ai_state.player_id;
    let Some(pres) = player_resources.players.get(ai_player as usize) else {
        return;
    };

    let uc = take_unit_census(ai_player, units);
    let mut bc = take_building_census(ai_player, buildings);

    // Merge in-flight BuildOrders so the AI treats pending builds
    // as if the buildings already exist (prevents duplicate orders).
    for (kind, pos) in &uc.pending_builds {
        bc.building_positions.push((*pos, *kind));
        match kind {
            BuildingKind::FishMarket => {
                bc.has_fish_market = true;
                bc.fish_market_count += 1;
            }
            BuildingKind::CatTree => bc.has_cat_tree = true,
            BuildingKind::ServerRack => bc.has_server_rack = true,
            BuildingKind::ScratchingPost => bc.has_scratching_post = true,
            BuildingKind::LaserPointer => bc.has_laser_pointer = true,
            BuildingKind::LitterBox => {
                bc.litter_box_count += 1;
            }
            _ => {}
        }
    }

    // Discover enemy base if not yet known
    if ai_state.enemy_spawn.is_none() {
        ai_state.enemy_spawn = discover_enemy_spawn(ai_player, units, buildings);
    }

    // Track which worker was used as a builder this tick (excluded from gather)
    let mut builder_used: Option<Entity> = None;

    let attack_threshold = ai_state.profile.attack_threshold;
    let target_workers = ai_state.profile.target_workers;

    // FSM transitions
    let new_phase = match ai_state.phase {
        AiPhase::EarlyGame => {
            // Train workers until target count
            if uc.worker_count < target_workers {
                if let Some(box_e) = bc.box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }
            if uc.worker_count >= target_workers {
                AiPhase::BuildUp
            } else {
                AiPhase::EarlyGame
            }
        }

        AiPhase::BuildUp => {
            // Build one structure per tick to avoid same-worker conflicts.
            // Priority: FishMarket (up to 2) → CatTree → supply.
            // Two FishMarkets before CatTree ensures food throughput keeps up
            // with army training costs (Nuisance = 75 food each).
            if builder_used.is_none() && bc.fish_market_count < 2 && pres.food >= 100 && bc.has_box {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.box_pos, map, &bc.building_positions);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::FishMarket,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_cat_tree && pres.food >= 150 && bc.has_box {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.box_pos, map, &bc.building_positions);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::CatTree,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.litter_box_count) {
                    builder_used = Some(b);
                }
            }

            // Train army from CatTree
            if let Some(ct_e) = bc.cat_tree_entity {
                let nuisance_cost = cc_core::unit_stats::base_stats(UnitKind::Nuisance).food_cost;
                if pres.food >= nuisance_cost && pres.supply < pres.supply_cap {
                    cmd_queue.push(GameCommand::TrainUnit {
                        building: EntityId(ct_e.to_bits()),
                        unit_kind: UnitKind::Nuisance,
                    });
                }
            }

            if uc.army_count >= 4 {
                AiPhase::MidGame
            } else {
                AiPhase::BuildUp
            }
        }

        AiPhase::MidGame => {
            // Build one structure per tick to avoid same-worker conflicts
            if builder_used.is_none() && !bc.has_server_rack && pres.food >= 100 && pres.gpu_cores >= 75 && bc.has_box {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.box_pos, map, &bc.building_positions);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::ServerRack,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_scratching_post && pres.food >= 100 && pres.gpu_cores >= 50 && bc.has_box {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.box_pos, map, &bc.building_positions);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::ScratchingPost,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_laser_pointer && pres.food >= 75 && pres.gpu_cores >= 25 && bc.has_box {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.box_pos, map, &bc.building_positions);
                    cmd_queue.push(GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: BuildingKind::LaserPointer,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            // Queue research at ScratchingPost
            if let Some(sp_e) = bc.scratching_post_entity {
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
            if let Some(sr_e) = bc.server_rack_entity {
                if pres.supply < pres.supply_cap {
                    let kind = if pres.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                        && uc.army_count % 4 == 0
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
            if let Some(ct_e) = bc.cat_tree_entity {
                if pres.supply < pres.supply_cap {
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick);
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
                if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.litter_box_count) {
                    builder_used = Some(b);
                }
            }

            // Reactive economy: build additional FishMarkets if food is bottlenecked
            if builder_used.is_none() {
                if let Some(b) = maybe_build_economy(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.fish_market_count) {
                    builder_used = Some(b);
                }
            }

            // Continue training workers (cap at 6 to reserve supply for army)
            if uc.worker_count < 6 {
                if let Some(box_e) = bc.box_entity {
                    if pres.food >= 50 && pres.supply < pres.supply_cap {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }

            // Attack when army is ready, OR when enemy has no army (cleanup mode).
            let enemy_defenseless = uc.enemy_army_count == 0 && uc.army_count >= 2;
            if uc.army_count >= attack_threshold || enemy_defenseless {
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
                    if !uc.army_entities.is_empty() {
                        let ids: Vec<EntityId> = uc.army_entities
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
            if let Some(ct_e) = bc.cat_tree_entity {
                if pres.supply < pres.supply_cap {
                    let kind = if uc.army_count % 3 == 0 {
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

            if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.litter_box_count) {
                builder_used = Some(b);
            }

            // Reactive economy during attack — keep food production up
            if builder_used.is_none() {
                if let Some(b) = maybe_build_economy(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.fish_market_count) {
                    builder_used = Some(b);
                }
            }

            // Check if base is under attack (enemy units near our buildings)
            let base_threatened = is_base_threatened(ai_player, units, buildings);
            if base_threatened {
                AiPhase::Defend
            } else if uc.army_count < 4 && uc.enemy_army_count > 0 {
                // Lost most of army and enemy still has forces — rebuild
                AiPhase::MidGame
            } else {
                // Stay attacking (even with small army if enemy is defenseless)
                AiPhase::Attack
            }
        }

        AiPhase::Defend => {
            // Rally army back to base (use actual building position)
            let rally_pos = bc.box_pos.unwrap_or(GridPos::new(55, 55));
            if !uc.army_entities.is_empty() {
                let ids: Vec<EntityId> = uc.army_entities
                    .iter()
                    .map(|e| EntityId(e.to_bits()))
                    .collect();
                cmd_queue.push(GameCommand::AttackMove {
                    unit_ids: ids,
                    target: rally_pos,
                });
            }

            // Keep training reinforcements while defending
            if let Some(ct_e) = bc.cat_tree_entity {
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

            if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.litter_box_count) {
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

    // Send idle workers to gather (after FSM so builder_used is set).
    // When food is critically low relative to GPU, prioritize food deposits
    // to rebalance the economy.
    let prefer_food = pres.food < 100 && pres.gpu_cores > pres.food * 2;
    for &worker in &uc.idle_workers {
        if Some(worker) == builder_used {
            continue;
        }
        let deposit_result = if prefer_food {
            find_nearest_deposit_of_type(
                worker,
                units,
                deposits,
                Some(cc_core::components::ResourceType::Food),
            )
            .or_else(|| find_nearest_deposit(worker, units, deposits))
        } else {
            find_nearest_deposit(worker, units, deposits)
        };
        if let Some((deposit_entity, _)) = deposit_result {
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

/// Pick a unit kind to train using the profile's weighted preferences.
fn pick_unit_kind(profile: &AiPersonalityProfile, army_count: u32, tick: u64) -> UnitKind {
    if profile.unit_preferences.is_empty() {
        return UnitKind::Nuisance;
    }
    // Deterministic weighted selection using tick + army_count as pseudo-random seed
    let total_weight: u32 = profile.unit_preferences.iter().map(|(_, w)| w).sum();
    if total_weight == 0 {
        return UnitKind::Nuisance;
    }
    let hash = tick.wrapping_mul(6364136223846793005).wrapping_add(army_count as u64);
    let pick = (hash >> 33) as u32 % total_weight;
    let mut cumulative = 0u32;
    for &(kind, weight) in &profile.unit_preferences {
        cumulative += weight;
        if pick < cumulative {
            return kind;
        }
    }
    profile.unit_preferences.last().map_or(UnitKind::Nuisance, |&(k, _)| k)
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
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
) -> Option<(Entity, GridPos)> {
    find_nearest_deposit_of_type(worker, units, deposits, None)
}

/// Find nearest resource deposit of a specific type to a worker (skips depleted).
/// If `resource_type` is None, matches any deposit type.
fn find_nearest_deposit_of_type(
    worker: Entity,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
    resource_type: Option<cc_core::components::ResourceType>,
) -> Option<(Entity, GridPos)> {
    let worker_pos = units.get(worker).ok()?.1.world;
    let mut best = None;
    let mut best_dist = cc_core::math::Fixed::MAX;

    for (deposit_entity, deposit_pos, deposit) in deposits.iter() {
        if deposit.remaining == 0 {
            continue; // Skip depleted deposits
        }
        if let Some(rt) = resource_type {
            if deposit.resource_type != rt {
                continue; // Skip wrong resource type
            }
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
///
/// `pending_litter_boxes` counts LitterBoxes already under construction or
/// with a BuildOrder in-flight, so we don't spam redundant build orders.
fn maybe_build_supply(
    pres: &crate::resources::PlayerResourceState,
    idle_workers: &[Entity],
    all_workers: &[Entity],
    box_pos: Option<GridPos>,
    map: &GameMap,
    building_positions: &[(GridPos, BuildingKind)],
    cmd_queue: &mut CommandQueue,
    pending_litter_boxes: u32,
) -> Option<Entity> {
    // Build when supply is within 3 of the cap (was 2 — too tight, causing
    // the AI to hit the cap before the LitterBox finishes building).
    // Also skip if a LitterBox is already being built to avoid redundant orders.
    let supply_headroom = pres.supply_cap.saturating_sub(pres.supply);
    let needs_supply = supply_headroom <= 3;
    let litter_box_already_pending = pending_litter_boxes > 0;

    if needs_supply && !litter_box_already_pending && pres.food >= 75 {
        if let Some(builder) = pick_builder(idle_workers, all_workers) {
            let build_pos = find_build_position(box_pos, map, building_positions);
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

/// Build an additional FishMarket when food is critically low and GPU is high.
/// This addresses the economy imbalance where food bottlenecks while GPU piles up.
/// Additional FishMarkets serve as drop-off points closer to food deposits,
/// reducing worker travel time and increasing food throughput.
/// Returns the builder entity if a build command was issued.
fn maybe_build_economy(
    pres: &crate::resources::PlayerResourceState,
    idle_workers: &[Entity],
    all_workers: &[Entity],
    box_pos: Option<GridPos>,
    map: &GameMap,
    building_positions: &[(GridPos, BuildingKind)],
    cmd_queue: &mut CommandQueue,
    fish_market_count: u32,
) -> Option<Entity> {
    // Cap at 3 FishMarkets to avoid over-investing in economy buildings
    if fish_market_count >= 3 {
        return None;
    }

    // React to food crisis: food < 50 while GPU >= 100 means economy is imbalanced.
    // Also build a second FishMarket once food drops below 75, regardless of GPU,
    // to provide a closer drop-off point.
    let food_crisis = pres.food < 50 && pres.gpu_cores >= 100;
    let needs_second_market = fish_market_count < 2 && pres.food < 75;

    if (food_crisis || needs_second_market) && pres.food >= 100 {
        if let Some(builder) = pick_builder(idle_workers, all_workers) {
            let build_pos = find_build_position(box_pos, map, building_positions);
            cmd_queue.push(GameCommand::Build {
                builder: EntityId(builder.to_bits()),
                building_kind: BuildingKind::FishMarket,
                position: build_pos,
            });
            return Some(builder);
        }
    }
    None
}

/// Find a position near the base to place a building.
/// Uses a growing spiral search with terrain passability and collision checks.
fn find_build_position(
    box_pos: Option<GridPos>,
    map: &GameMap,
    existing_buildings: &[(GridPos, BuildingKind)],
) -> GridPos {
    let base = box_pos.unwrap_or(GridPos::new(32, 32));
    let mut fallback: Option<GridPos> = None;

    // Search concentric rings at increasing distances from the base
    for dist in AI_BUILD_SPACING..AI_BUILD_SPACING + 12 {
        // Walk the perimeter of a square at Chebyshev distance `dist`
        for offset in 0..(dist * 8) {
            let (dx, dy) = ring_offset(dist, offset);
            let candidate = GridPos::new(base.x + dx, base.y + dy);

            if !map.is_passable(candidate) {
                continue;
            }

            // Check collision: no existing building within AI_BUILD_SPACING
            let too_close = existing_buildings.iter().any(|(bp, _)| {
                let cx = (bp.x - candidate.x).abs();
                let cy = (bp.y - candidate.y).abs();
                cx < AI_BUILD_SPACING && cy < AI_BUILD_SPACING
            });
            if too_close {
                if fallback.is_none() {
                    fallback = Some(candidate);
                }
                continue;
            }

            return candidate;
        }
    }

    // All spiral positions occupied or blocked — return first passable fallback
    fallback.unwrap_or(base)
}

/// Map a linear offset to (dx, dy) on the perimeter of a Chebyshev-distance ring.
fn ring_offset(dist: i32, offset: i32) -> (i32, i32) {
    let side_len = dist * 2;
    let side = offset / side_len;
    let pos = offset % side_len;
    match side {
        0 => (-dist + pos, -dist),          // top edge, left to right
        1 => (dist, -dist + pos),           // right edge, top to bottom
        2 => (dist - pos, dist),            // bottom edge, right to left
        3 => (-dist, dist - pos),           // left edge, bottom to top
        _ => (0, 0),
    }
}

/// Check if enemy units are within 8 tiles of any of the AI's buildings.
fn is_base_threatened(
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
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
    for (_, pos, owner, _, _, _, _) in units.iter() {
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
        assert_eq!(state.profile.attack_threshold, 8);
    }

    #[test]
    fn personality_preset_thresholds() {
        assert_eq!(AiPersonalityProfile::aggressive().attack_threshold, 6);
        assert_eq!(AiPersonalityProfile::balanced().attack_threshold, 8);
        assert_eq!(AiPersonalityProfile::defensive().attack_threshold, 12);
    }

    #[test]
    fn all_factions_have_personality_profiles() {
        let factions = [
            Faction::CatGpt,
            Faction::TheClawed,
            Faction::SeekersOfTheDeep,
            Faction::TheMurder,
            Faction::Llama,
            Faction::Croak,
        ];
        for faction in factions {
            let profile = faction_personality(faction);
            assert!(!profile.name.is_empty(), "{faction:?} has empty name");
            assert!(profile.attack_threshold > 0, "{faction:?} attack_threshold is 0");
            assert!(!profile.unit_preferences.is_empty(), "{faction:?} has no unit preferences");
            assert!(profile.target_workers > 0, "{faction:?} target_workers is 0");
            assert!(profile.chaos_factor <= 100, "{faction:?} chaos_factor > 100");
            assert!(profile.leak_chance <= 100, "{faction:?} leak_chance > 100");
        }
    }

    #[test]
    fn personality_profiles_differ_between_factions() {
        let geppity = faction_personality(Faction::CatGpt);
        let deepseek = faction_personality(Faction::SeekersOfTheDeep);
        let llhama = faction_personality(Faction::Llama);

        // Deepseek is more cautious
        assert!(deepseek.attack_threshold > geppity.attack_threshold);
        // Llhama leaks intel
        assert!(llhama.leak_chance > 0);
        assert_eq!(geppity.leak_chance, 0);
        // Deepseek never makes mistakes
        assert_eq!(deepseek.chaos_factor, 0);
    }

    #[test]
    fn unknown_faction_string_returns_neutral_profile() {
        let unknown = faction_personality_by_name("nonexistent");
        assert_eq!(unknown.name, "Neutral");
        assert_eq!(unknown.attack_threshold, 8);
    }

    #[test]
    fn ai_state_unified_profile() {
        let state = AiState::default();
        // Profile is always present — no more Option<> or separate personality field
        assert_eq!(state.profile.attack_threshold, 8);
        assert_eq!(state.profile.name, "Balanced");
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

    #[test]
    fn find_build_position_avoids_water() {
        use cc_core::map::GameMap;
        use cc_core::terrain::TerrainType;

        let mut map = GameMap::new(20, 20);
        let base = GridPos::new(10, 10);

        // Flood the first ring (distance AI_BUILD_SPACING) with water
        for dx in -AI_BUILD_SPACING..=AI_BUILD_SPACING {
            for dy in -AI_BUILD_SPACING..=AI_BUILD_SPACING {
                let chebyshev = dx.abs().max(dy.abs());
                if chebyshev == AI_BUILD_SPACING {
                    let pos = GridPos::new(base.x + dx, base.y + dy);
                    if let Some(tile) = map.get_mut(pos) {
                        tile.terrain = TerrainType::Water;
                    }
                }
            }
        }

        // Place an existing building at (10, 14) to test collision avoidance
        let existing = vec![(GridPos::new(10, 14), BuildingKind::FishMarket)];

        let result = find_build_position(Some(base), &map, &existing);

        // Must be passable
        assert!(map.is_passable(result), "Position {result:?} should be passable");

        // Must not overlap with existing building
        assert_ne!(result, GridPos::new(10, 14), "Should not overlap existing building");
    }

    #[test]
    fn find_build_position_spiral_grows() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let base = GridPos::new(15, 15);

        // Place buildings covering the first ring
        let mut existing: Vec<(GridPos, BuildingKind)> = Vec::new();
        for dx in -AI_BUILD_SPACING..=AI_BUILD_SPACING {
            for dy in -AI_BUILD_SPACING..=AI_BUILD_SPACING {
                existing.push((GridPos::new(base.x + dx, base.y + dy), BuildingKind::LitterBox));
            }
        }

        let result = find_build_position(Some(base), &map, &existing);

        // Should place beyond the first ring
        let dist_x = (result.x - base.x).abs();
        let dist_y = (result.y - base.y).abs();
        let chebyshev = dist_x.max(dist_y);
        assert!(chebyshev >= AI_BUILD_SPACING, "Should place at distance >= {AI_BUILD_SPACING}, got {chebyshev}");
    }

    #[test]
    fn maybe_build_supply_triggers_near_cap() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        // Create a fake entity for the builder
        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        pres.supply = 18;
        pres.supply_cap = 20;
        pres.food = 200;

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        // No pending LitterBoxes — should trigger a build
        let result = maybe_build_supply(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 0,
        );
        assert!(result.is_some(), "Should build a LitterBox when supply is near cap");
        assert_eq!(cmd_queue.commands.len(), 1);
        match &cmd_queue.commands[0] {
            GameCommand::Build { building_kind, .. } => {
                assert_eq!(*building_kind, BuildingKind::LitterBox);
            }
            other => panic!("Expected Build command, got {:?}", other),
        }
    }

    #[test]
    fn maybe_build_supply_skips_when_litter_box_pending() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        pres.supply = 18;
        pres.supply_cap = 20;
        pres.food = 200;

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        // One LitterBox already pending — should NOT build another
        let result = maybe_build_supply(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 1,
        );
        assert!(result.is_none(), "Should skip when a LitterBox is already pending");
        assert!(cmd_queue.commands.is_empty());
    }

    #[test]
    fn maybe_build_supply_triggers_with_headroom_3() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        pres.supply = 17; // headroom = 3, should trigger (was 2 before fix)
        pres.supply_cap = 20;
        pres.food = 200;

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        let result = maybe_build_supply(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 0,
        );
        assert!(result.is_some(), "Should build when headroom is exactly 3");
    }

    #[test]
    fn maybe_build_supply_skips_with_headroom_4() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        pres.supply = 16; // headroom = 4, should NOT trigger
        pres.supply_cap = 20;
        pres.food = 200;

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        let result = maybe_build_supply(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 0,
        );
        assert!(result.is_none(), "Should not build when headroom is 4");
    }

    #[test]
    fn maybe_build_economy_triggers_on_food_crisis() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        // Food crisis scenario: food enough to build (>= 100) but historically low,
        // GPU piling up. In practice food oscillates — the check is on current food.
        pres.food = 100; // Can afford the 100 food cost
        pres.gpu_cores = 300; // GPU hoarded

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        // Needs second market (count < 2) and food < 75 — but food is 100 here.
        // Use food_crisis path: food < 50 && gpu >= 100 — set food to trigger.
        pres.food = 40;
        let result = maybe_build_economy(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 1,
        );
        // food < 50 triggers crisis, but food (40) < cost (100) — can't afford
        assert!(result.is_none(), "Can't build FishMarket when food < 100");

        pres.food = 120;
        pres.gpu_cores = 300;
        // food >= 50 but needs_second_market: fish_market_count < 2 && food < 75 — nope (120 >= 75)
        // food_crisis: food < 50 — nope (120 >= 50)
        let result = maybe_build_economy(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 1,
        );
        assert!(result.is_none(), "No economy issue when food is healthy");
    }

    #[test]
    fn maybe_build_economy_caps_at_three() {
        use cc_core::map::GameMap;

        let map = GameMap::new(30, 30);
        let box_pos = Some(GridPos::new(15, 15));
        let building_positions = vec![(GridPos::new(15, 15), BuildingKind::TheBox)];
        let mut cmd_queue = CommandQueue::default();

        let mut world = bevy::prelude::World::new();
        let builder = world.spawn_empty().id();

        let mut pres = crate::resources::PlayerResourceState::default();
        pres.food = 100;
        pres.gpu_cores = 300;

        let idle_workers = vec![builder];
        let all_workers = vec![builder];

        // Already have 3 FishMarkets — should not build more
        let result = maybe_build_economy(
            &pres, &idle_workers, &all_workers,
            box_pos, &map, &building_positions, &mut cmd_queue, 3,
        );
        assert!(result.is_none(), "Should cap FishMarkets at 3");
    }

    #[test]
    fn building_census_counts_litter_boxes() {
        // Verify the BuildingCensus correctly counts LitterBoxes
        let mut census = BuildingCensus {
            has_box: true,
            has_cat_tree: false,
            has_fish_market: false,
            has_server_rack: false,
            has_scratching_post: false,
            has_laser_pointer: false,
            fish_market_count: 0,
            litter_box_count: 0,
            box_entity: None,
            box_pos: Some(GridPos::new(10, 10)),
            cat_tree_entity: None,
            server_rack_entity: None,
            scratching_post_entity: None,
            building_positions: Vec::new(),
        };

        // Simulate adding LitterBoxes from pending builds
        let pending = vec![
            (BuildingKind::LitterBox, GridPos::new(13, 10)),
            (BuildingKind::FishMarket, GridPos::new(10, 13)),
        ];
        for (kind, pos) in &pending {
            census.building_positions.push((*pos, *kind));
            match kind {
                BuildingKind::LitterBox => census.litter_box_count += 1,
                BuildingKind::FishMarket => {
                    census.has_fish_market = true;
                    census.fish_market_count += 1;
                }
                _ => {}
            }
        }

        assert_eq!(census.litter_box_count, 1, "Should count 1 LitterBox");
        assert_eq!(census.fish_market_count, 1, "Should count 1 FishMarket");
        assert!(census.has_fish_market, "Should set has_fish_market flag");
    }
}
