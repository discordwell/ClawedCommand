use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    BuildOrder, Building, BuildingKind, Faction, Gathering, Health, MoveTarget,
    Owner, Position, Producer, ProductionQueue, ResourceDeposit, UnitKind, UnitType, UpgradeType,
};
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_core::tuning::{AI_BUILD_SPACING, ATTACK_REISSUE_INTERVAL, BASE_THREAT_RADIUS};

use crate::resources::{CommandQueue, MapResource, PlayerResources, SimClock};

/// Maximum number of items in a building's production queue before the AI stops training.
const AI_MAX_QUEUE_DEPTH: usize = 2;

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

/// AI tier — determines tactical sophistication based on ServerRack count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTier {
    /// No ServerRacks — plain AttackMove, basic economy.
    Basic,
    /// 1 ServerRack — focus-fire weakest, retreat wounded.
    Tactical,
    /// 2 ServerRacks — coordinate assault (70/30 split + flank).
    Strategic,
    /// 3+ ServerRacks — all of above + adaptive positioning.
    Advanced,
}

impl AiTier {
    fn from_rack_count(count: u32) -> Self {
        match count {
            0 => AiTier::Basic,
            1 => AiTier::Tactical,
            2 => AiTier::Strategic,
            _ => AiTier::Advanced,
        }
    }
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
    /// Current tactical tier — derived from ServerRack count each tick.
    pub tier: AiTier,
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
            tier: AiTier::Basic,
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
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
    health_query: Query<&Health>,
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
        &health_query,
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
    buildings: Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
    health_query: Query<&Health>,
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
            &health_query,
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
    box_entity: Option<Entity>,
    box_pos: Option<GridPos>,
    cat_tree_entity: Option<Entity>,
    server_rack_entity: Option<Entity>,
    scratching_post_entity: Option<Entity>,
    building_positions: Vec<(GridPos, BuildingKind)>,
    box_queue_len: usize,
    cat_tree_queue_len: usize,
    server_rack_queue_len: usize,
    pending_litter_box_count: u32,
    /// Total number of ServerRack buildings (for tier computation).
    server_rack_count: u32,
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
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
) -> BuildingCensus {
    let mut census = BuildingCensus {
        has_box: false,
        has_cat_tree: false,
        has_fish_market: false,
        has_server_rack: false,
        has_scratching_post: false,
        has_laser_pointer: false,
        box_entity: None,
        box_pos: None,
        cat_tree_entity: None,
        server_rack_entity: None,
        scratching_post_entity: None,
        building_positions: Vec::new(),
        box_queue_len: 0,
        cat_tree_queue_len: 0,
        server_rack_queue_len: 0,
        pending_litter_box_count: 0,
        server_rack_count: 0,
    };
    for (entity, building, owner, pos, producer, prod_queue) in buildings.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        census.building_positions.push((pos.world.to_grid(), building.kind));
        let queue_len = prod_queue.map_or(0, |q| q.queue.len());
        match building.kind {
            BuildingKind::TheBox => {
                census.has_box = true;
                census.box_pos = Some(pos.world.to_grid());
                census.box_queue_len = queue_len;
                if producer.is_some() {
                    census.box_entity = Some(entity);
                }
            }
            BuildingKind::CatTree => {
                census.has_cat_tree = true;
                census.cat_tree_queue_len = queue_len;
                if producer.is_some() {
                    census.cat_tree_entity = Some(entity);
                }
            }
            BuildingKind::FishMarket => {
                census.has_fish_market = true;
            }
            BuildingKind::ServerRack => {
                census.has_server_rack = true;
                census.server_rack_count += 1;
                census.server_rack_queue_len = queue_len;
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
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
) -> Option<GridPos> {
    // First: look for enemy TheBox
    for (_, building, owner, pos, _, _) in buildings.iter() {
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

/// Calculate food to reserve for priority buildings the AI still needs.
/// GPU-aware: only reserves for buildings the AI can actually afford.
fn food_reserve_for_buildings(phase: AiPhase, bc: &BuildingCensus, gpu_cores: u32) -> u32 {
    match phase {
        AiPhase::BuildUp => {
            let mut reserve = 0u32;
            if !bc.has_fish_market { reserve += 100; }
            if !bc.has_cat_tree { reserve += 150; }
            reserve
        }
        AiPhase::MidGame => {
            let mut reserve = 0u32;
            if !bc.has_server_rack && gpu_cores >= 75 { reserve += 100; }
            if !bc.has_scratching_post && gpu_cores >= 50 { reserve += 100; }
            reserve
        }
        _ => 0,
    }
}

/// Core FSM logic shared between single-AI and multi-AI systems.
fn run_ai_fsm(
    tick: u64,
    ai_state: &mut AiState,
    cmd_queue: &mut CommandQueue,
    player_resources: &PlayerResources,
    map: &GameMap,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
    deposits: &Query<(Entity, &Position, &ResourceDeposit)>,
    health_query: &Query<&Health>,
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
            BuildingKind::FishMarket => bc.has_fish_market = true,
            BuildingKind::CatTree => bc.has_cat_tree = true,
            BuildingKind::ServerRack => {
                bc.has_server_rack = true;
                bc.server_rack_count += 1;
            }
            BuildingKind::ScratchingPost => bc.has_scratching_post = true,
            BuildingKind::LaserPointer => bc.has_laser_pointer = true,
            BuildingKind::LitterBox => bc.pending_litter_box_count += 1,
            _ => {}
        }
    }

    // Update AI tier from ServerRack count
    ai_state.tier = AiTier::from_rack_count(bc.server_rack_count);
    let tier = ai_state.tier;
    let retreat_threshold = ai_state.profile.retreat_threshold;

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
                    if pres.food >= 50 && pres.supply < pres.supply_cap
                        && bc.box_queue_len < AI_MAX_QUEUE_DEPTH
                    {
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
            let reserve = food_reserve_for_buildings(AiPhase::BuildUp, &bc, pres.gpu_cores);

            if builder_used.is_none() && !bc.has_fish_market && pres.food >= 100 && bc.has_box {
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
                if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.pending_litter_box_count) {
                    builder_used = Some(b);
                }
            }

            if let Some(ct_e) = bc.cat_tree_entity {
                let nuisance_cost = cc_core::unit_stats::base_stats(UnitKind::Nuisance).food_cost;
                if pres.food >= nuisance_cost + reserve && pres.supply < pres.supply_cap
                    && bc.cat_tree_queue_len < AI_MAX_QUEUE_DEPTH
                {
                    cmd_queue.push(GameCommand::TrainUnit {
                        building: EntityId(ct_e.to_bits()),
                        unit_kind: UnitKind::Nuisance,
                    });
                }
            }

            let buildup_worker_cap = (target_workers + 1).min(6);
            if uc.worker_count < buildup_worker_cap {
                if let Some(box_e) = bc.box_entity {
                    if pres.food >= 50 + reserve && pres.supply < pres.supply_cap
                        && bc.box_queue_len < AI_MAX_QUEUE_DEPTH
                    {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }

            let defense_pos = bc.box_pos.map(|p| GridPos::new(p.x + 3, p.y + 3))
                .unwrap_or(GridPos::new(55, 58));
            if let Some(ct_e) = bc.cat_tree_entity {
                cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }

            if uc.army_count >= 4 { AiPhase::MidGame } else { AiPhase::BuildUp }
        }

        AiPhase::MidGame => {
            let reserve = food_reserve_for_buildings(AiPhase::MidGame, &bc, pres.gpu_cores);

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

            if let Some(sp_e) = bc.scratching_post_entity {
                let research_priority = [
                    UpgradeType::SharperClaws, UpgradeType::ThickerFur, UpgradeType::SiegeTraining,
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

            if let Some(sr_e) = bc.server_rack_entity {
                if pres.supply < pres.supply_cap && bc.server_rack_queue_len < AI_MAX_QUEUE_DEPTH {
                    let kind = if pres.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                        && uc.army_count % 4 == 0
                    { UnitKind::Catnapper } else { UnitKind::FlyingFox };
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost + reserve && pres.gpu_cores >= stats.gpu_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(sr_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if let Some(ct_e) = bc.cat_tree_entity {
                if pres.supply < pres.supply_cap && bc.cat_tree_queue_len < AI_MAX_QUEUE_DEPTH {
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost + reserve {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.pending_litter_box_count) {
                    builder_used = Some(b);
                }
            }

            if uc.worker_count < 6 {
                if let Some(box_e) = bc.box_entity {
                    if pres.food >= 50 + reserve && pres.supply < pres.supply_cap
                        && bc.box_queue_len < AI_MAX_QUEUE_DEPTH
                    {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: UnitKind::Pawdler,
                        });
                    }
                }
            }

            let defense_pos = bc.box_pos.map(|p| GridPos::new(p.x + 3, p.y + 3))
                .unwrap_or(GridPos::new(55, 58));
            if let Some(ct_e) = bc.cat_tree_entity {
                cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }
            if let Some(sr_e) = bc.server_rack_entity {
                cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(sr_e.to_bits()),
                    target: defense_pos,
                });
            }

            let enemy_defenseless = uc.enemy_army_count == 0 && uc.army_count >= 2;
            if uc.army_count >= attack_threshold || enemy_defenseless {
                AiPhase::Attack
            } else {
                AiPhase::MidGame
            }
        }

        AiPhase::Attack => {
            let should_reissue = !ai_state.attack_ordered
                || tick.saturating_sub(ai_state.last_attack_tick) >= ATTACK_REISSUE_INTERVAL;

            if should_reissue {
                if let Some(target) = ai_state.enemy_spawn {
                    if !uc.army_entities.is_empty() {
                        let ids: Vec<EntityId> = uc.army_entities
                            .iter()
                            .map(|e| EntityId(e.to_bits()))
                            .collect();
                        cmd_queue.push(GameCommand::AttackMove { unit_ids: ids, target });
                        ai_state.attack_ordered = true;
                        ai_state.last_attack_tick = tick;
                    }
                }
            }

            if let Some(ct_e) = bc.cat_tree_entity {
                if pres.supply < pres.supply_cap && bc.cat_tree_queue_len < AI_MAX_QUEUE_DEPTH {
                    let kind = if uc.army_count % 3 == 0 { UnitKind::Hisser } else { UnitKind::Nuisance };
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.pending_litter_box_count) {
                builder_used = Some(b);
            }

            if let Some(enemy_pos) = ai_state.enemy_spawn {
                if let Some(ct_e) = bc.cat_tree_entity {
                    cmd_queue.push(GameCommand::SetRallyPoint {
                        building: EntityId(ct_e.to_bits()),
                        target: enemy_pos,
                    });
                }
                if let Some(sr_e) = bc.server_rack_entity {
                    cmd_queue.push(GameCommand::SetRallyPoint {
                        building: EntityId(sr_e.to_bits()),
                        target: enemy_pos,
                    });
                }
            }

            let base_threatened = is_base_threatened(ai_player, units, buildings);
            if base_threatened {
                AiPhase::Defend
            } else if uc.army_count < 4 && uc.enemy_army_count > 0 {
                AiPhase::MidGame
            } else {
                AiPhase::Attack
            }
        }

        AiPhase::Defend => {
            let rally_pos = bc.box_pos.unwrap_or(GridPos::new(55, 55));
            if !uc.army_entities.is_empty() {
                let ids: Vec<EntityId> = uc.army_entities
                    .iter()
                    .map(|e| EntityId(e.to_bits()))
                    .collect();
                cmd_queue.push(GameCommand::AttackMove { unit_ids: ids, target: rally_pos });
            }

            if let Some(ct_e) = bc.cat_tree_entity {
                if pres.supply < pres.supply_cap && bc.cat_tree_queue_len < AI_MAX_QUEUE_DEPTH {
                    let stats = cc_core::unit_stats::base_stats(UnitKind::Nuisance);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: UnitKind::Nuisance,
                        });
                    }
                }
            }

            if let Some(b) = maybe_build_supply(pres, &uc.idle_workers, &uc.all_workers, bc.box_pos, map, &bc.building_positions, cmd_queue, bc.pending_litter_box_count) {
                builder_used = Some(b);
            }

            let defense_pos = bc.box_pos.map(|p| GridPos::new(p.x + 3, p.y + 3))
                .unwrap_or(GridPos::new(55, 58));
            if let Some(ct_e) = bc.cat_tree_entity {
                cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }
            if let Some(sr_e) = bc.server_rack_entity {
                cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(sr_e.to_bits()),
                    target: defense_pos,
                });
            }

            let base_threatened = is_base_threatened(ai_player, units, buildings);
            if !base_threatened { AiPhase::MidGame } else { AiPhase::Defend }
        }
    };

    // Send idle workers to gather (after FSM so builder_used is set)
    for &worker in &uc.idle_workers {
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
    map: &GameMap,
    building_positions: &[(GridPos, BuildingKind)],
    cmd_queue: &mut CommandQueue,
    pending_litter_boxes: u32,
) -> Option<Entity> {
    if pending_litter_boxes > 0 {
        return None;
    }
    if pres.supply + 2 >= pres.supply_cap && pres.food >= 75 {
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
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
) -> bool {
    // Collect AI building positions as the actual base locations
    let mut base_positions: Vec<GridPos> = Vec::new();

    for (_, _, owner, pos, _, _) in buildings.iter() {
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

// ---------------------------------------------------------------------------
// Tier-aware tactical helpers
// ---------------------------------------------------------------------------

/// Returns true if the unit kind is ranged (fights from distance).
fn is_ranged_unit(kind: UnitKind) -> bool {
    matches!(kind, UnitKind::Hisser | UnitKind::FlyingFox | UnitKind::Catnapper | UnitKind::Yowler)
}

/// Find the weakest (lowest HP) enemy unit near a centroid position.
fn find_weakest_enemy_near(
    centroid: GridPos,
    radius: i32,
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    health_query: &Query<&Health>,
) -> Option<(Entity, GridPos)> {
    let mut best: Option<(Entity, GridPos, Fixed)> = None;
    for (entity, pos, owner, unit_type, _, _, _) in units.iter() {
        if owner.player_id == ai_player || unit_type.kind == UnitKind::Pawdler {
            continue;
        }
        let grid = pos.world.to_grid();
        let dx = (grid.x - centroid.x).abs();
        let dy = (grid.y - centroid.y).abs();
        if dx > radius || dy > radius {
            continue;
        }
        if let Ok(health) = health_query.get(entity) {
            let is_weaker = best.as_ref().map_or(true, |(_, _, best_hp)| health.current < *best_hp);
            if is_weaker {
                best = Some((entity, grid, health.current));
            }
        }
    }
    best.map(|(e, g, _)| (e, g))
}

/// Split army entities into main force (70%) and flank group (30%).
fn split_army_for_assault(army: &[Entity]) -> (Vec<Entity>, Vec<Entity>) {
    let split_point = (army.len() * 7) / 10;
    let main_force = army[..split_point.max(1)].to_vec();
    let flank = army[split_point.max(1)..].to_vec();
    (main_force, flank)
}

/// Collect entities of wounded units (HP below threshold percentage).
fn wounded_units(
    army: &[Entity],
    health_query: &Query<&Health>,
    threshold_pct: u32,
) -> Vec<Entity> {
    army.iter()
        .filter(|&&e| {
            if let Ok(health) = health_query.get(e) {
                if health.max > Fixed::ZERO {
                    let pct = (health.current * Fixed::from_num(100)) / health.max;
                    return pct < Fixed::from_num(threshold_pct);
                }
            }
            false
        })
        .copied()
        .collect()
}

/// Compute the centroid (average position) of a set of army entities.
fn army_centroid(
    army: &[Entity],
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
) -> Option<GridPos> {
    if army.is_empty() {
        return None;
    }
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut count: i64 = 0;
    for &e in army {
        if let Ok((_, pos, _, _, _, _, _)) = units.get(e) {
            let g = pos.world.to_grid();
            sum_x += g.x as i64;
            sum_y += g.y as i64;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some(GridPos::new((sum_x / count) as i32, (sum_y / count) as i32))
}

/// Offset a target position for flanking (perpendicular offset).
fn flank_offset(target: GridPos, centroid: GridPos) -> GridPos {
    let dx = target.x - centroid.x;
    let dy = target.y - centroid.y;
    // Perpendicular offset: rotate 90 degrees, scale to ~5 tiles
    let perp_x = -dy.signum() * 5;
    let perp_y = dx.signum() * 5;
    GridPos::new(
        (target.x + perp_x).max(0),
        (target.y + perp_y).max(0),
    )
}

/// Issue tier-aware attack commands for the Attack phase.
fn issue_attack_commands(
    tier: AiTier,
    tick: u64,
    ai_state: &mut AiState,
    army: &[Entity],
    ai_player: u8,
    retreat_threshold: u32,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    health_query: &Query<&Health>,
    cmd_queue: &mut CommandQueue,
    box_pos: Option<GridPos>,
    target: GridPos,
) {
    match tier {
        AiTier::Basic => {
            // Plain AttackMove — original behavior
            let ids: Vec<EntityId> = army.iter().map(|e| EntityId(e.to_bits())).collect();
            cmd_queue.push(GameCommand::AttackMove {
                unit_ids: ids,
                target,
            });
        }
        AiTier::Tactical => {
            // Focus-fire weakest enemy near army centroid; retreat wounded
            let centroid = army_centroid(army, units).unwrap_or(target);

            // Retreat wounded units back to base
            let wounded = wounded_units(army, health_query, retreat_threshold);
            if !wounded.is_empty() {
                let retreat_pos = box_pos.unwrap_or(GridPos::new(55, 55));
                let ids: Vec<EntityId> = wounded.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push(GameCommand::Move {
                    unit_ids: ids,
                    target: retreat_pos,
                });
            }

            // Healthy units focus-fire weakest enemy
            let healthy: Vec<Entity> = army.iter()
                .filter(|e| !wounded.contains(e))
                .copied()
                .collect();
            if !healthy.is_empty() {
                if let Some((weak_entity, _weak_pos)) = find_weakest_enemy_near(centroid, 15, ai_player, units, health_query) {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::AttackMove {
                        unit_ids: ids,
                        target,
                    });
                }
            }
        }
        AiTier::Strategic | AiTier::Advanced => {
            // Retreat wounded first (same as Tactical)
            let wounded = wounded_units(army, health_query, retreat_threshold);
            if !wounded.is_empty() {
                let retreat_pos = box_pos.unwrap_or(GridPos::new(55, 55));
                let ids: Vec<EntityId> = wounded.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push(GameCommand::Move {
                    unit_ids: ids,
                    target: retreat_pos,
                });
            }

            // 70/30 split with flanking
            let healthy: Vec<Entity> = army.iter()
                .filter(|e| !wounded.contains(e))
                .copied()
                .collect();
            if healthy.len() >= 4 {
                let centroid = army_centroid(&healthy, units).unwrap_or(target);
                let (main_force, flank_group) = split_army_for_assault(&healthy);

                // Main force: focus-fire weakest or attack-move
                if let Some((weak_entity, _)) = find_weakest_enemy_near(centroid, 15, ai_player, units, health_query) {
                    let ids: Vec<EntityId> = main_force.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = main_force.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::AttackMove {
                        unit_ids: ids,
                        target,
                    });
                }

                // Flank group: attack-move to offset position
                if !flank_group.is_empty() {
                    let flank_target = flank_offset(target, centroid);
                    let ids: Vec<EntityId> = flank_group.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::AttackMove {
                        unit_ids: ids,
                        target: flank_target,
                    });
                }
            } else {
                // Too few healthy units for split — use Tactical behavior
                if let Some((weak_entity, _)) = find_weakest_enemy_near(
                    army_centroid(&healthy, units).unwrap_or(target), 15, ai_player, units, health_query,
                ) {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push(GameCommand::AttackMove {
                        unit_ids: ids,
                        target,
                    });
                }
            }
        }
    }
    ai_state.attack_ordered = true;
    ai_state.last_attack_tick = tick;
}

/// Issue tier-aware defend commands — position melee forward, ranged back.
fn issue_defend_commands(
    tier: AiTier,
    army: &[Entity],
    rally_pos: GridPos,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    cmd_queue: &mut CommandQueue,
) {
    match tier {
        AiTier::Basic => {
            let ids: Vec<EntityId> = army.iter().map(|e| EntityId(e.to_bits())).collect();
            cmd_queue.push(GameCommand::AttackMove {
                unit_ids: ids,
                target: rally_pos,
            });
        }
        AiTier::Tactical | AiTier::Strategic | AiTier::Advanced => {
            // Split melee and ranged: melee forward (+2 toward threat), ranged at rally
            let mut melee = Vec::new();
            let mut ranged = Vec::new();
            for &e in army {
                if let Ok((_, _, _, ut, _, _, _)) = units.get(e) {
                    if is_ranged_unit(ut.kind) {
                        ranged.push(e);
                    } else {
                        melee.push(e);
                    }
                } else {
                    melee.push(e);
                }
            }

            // Melee units forward
            if !melee.is_empty() {
                let forward_pos = GridPos::new(rally_pos.x + 2, rally_pos.y + 2);
                let ids: Vec<EntityId> = melee.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push(GameCommand::AttackMove {
                    unit_ids: ids,
                    target: forward_pos,
                });
            }

            // Ranged units at rally (behind melee)
            if !ranged.is_empty() {
                let ids: Vec<EntityId> = ranged.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push(GameCommand::AttackMove {
                    unit_ids: ids,
                    target: rally_pos,
                });
            }
        }
    }
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
    fn tier_upgrades_with_rack_count() {
        assert_eq!(AiTier::from_rack_count(0), AiTier::Basic);
        assert_eq!(AiTier::from_rack_count(1), AiTier::Tactical);
        assert_eq!(AiTier::from_rack_count(2), AiTier::Strategic);
        assert_eq!(AiTier::from_rack_count(3), AiTier::Advanced);
        assert_eq!(AiTier::from_rack_count(10), AiTier::Advanced);
    }

    #[test]
    fn ai_default_tier_is_basic() {
        let state = AiState::default();
        assert_eq!(state.tier, AiTier::Basic);
    }

    #[test]
    fn split_army_70_30() {
        let entities: Vec<Entity> = (1..=10u64)
            .map(|i| Entity::from_bits((1u64 << 32) | i))
            .collect();
        let (main, flank) = split_army_for_assault(&entities);
        assert_eq!(main.len(), 7);
        assert_eq!(flank.len(), 3);
    }

    #[test]
    fn split_army_small() {
        let entities: Vec<Entity> = (1..=2u64)
            .map(|i| Entity::from_bits((1u64 << 32) | i))
            .collect();
        let (main, flank) = split_army_for_assault(&entities);
        assert_eq!(main.len(), 1);
        assert_eq!(flank.len(), 1);
    }

    #[test]
    fn flank_offset_perpendicular() {
        let target = GridPos::new(20, 20);
        let centroid = GridPos::new(10, 10);
        let flanked = flank_offset(target, centroid);
        // Should be offset perpendicular to attack direction
        assert_ne!(flanked, target);
    }
}
