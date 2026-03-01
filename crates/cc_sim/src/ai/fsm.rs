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

/// Faction-aware building/unit mapping so the FSM uses the correct kinds per faction.
#[derive(Debug, Clone)]
pub struct FactionMap {
    pub hq: BuildingKind,
    pub barracks: BuildingKind,
    pub resource_depot: BuildingKind,
    pub tech: BuildingKind,
    pub research: BuildingKind,
    pub supply: BuildingKind,
    pub defense_tower: BuildingKind,
    pub garrison: BuildingKind,
    pub worker: UnitKind,
    pub research_upgrades: [UpgradeType; 3],
}

/// Return the faction-aware building/unit mapping for a given faction.
pub fn faction_map(faction: Faction) -> FactionMap {
    match faction {
        Faction::CatGpt | Faction::Neutral => FactionMap {
            hq: BuildingKind::TheBox,
            barracks: BuildingKind::CatTree,
            resource_depot: BuildingKind::FishMarket,
            tech: BuildingKind::ServerRack,
            research: BuildingKind::ScratchingPost,
            supply: BuildingKind::LitterBox,
            defense_tower: BuildingKind::LaserPointer,
            garrison: BuildingKind::CatFlap,
            worker: UnitKind::Pawdler,
            research_upgrades: [UpgradeType::SharperClaws, UpgradeType::ThickerFur, UpgradeType::SiegeTraining],
        },
        Faction::TheClawed => FactionMap {
            hq: BuildingKind::TheBurrow,
            barracks: BuildingKind::NestingBox,
            resource_depot: BuildingKind::SeedVault,
            tech: BuildingKind::JunkTransmitter,
            research: BuildingKind::GnawLab,
            supply: BuildingKind::WarrenExpansion,
            defense_tower: BuildingKind::SqueakTower,
            garrison: BuildingKind::Mousehole,
            worker: UnitKind::Nibblet,
            research_upgrades: [UpgradeType::SharperTeeth, UpgradeType::ThickerHide, UpgradeType::QuickPaws],
        },
        Faction::SeekersOfTheDeep => FactionMap {
            hq: BuildingKind::TheSett,
            barracks: BuildingKind::WarHollow,
            resource_depot: BuildingKind::BurrowDepot,
            tech: BuildingKind::CoreTap,
            research: BuildingKind::ClawMarks,
            supply: BuildingKind::DeepWarren,
            defense_tower: BuildingKind::SlagThrower,
            garrison: BuildingKind::BulwarkGate,
            worker: UnitKind::Delver,
            research_upgrades: [UpgradeType::SharperFangs, UpgradeType::ReinforcedHide, UpgradeType::SteadyStance],
        },
        Faction::TheMurder => FactionMap {
            hq: BuildingKind::TheParliament,
            barracks: BuildingKind::Rookery,
            resource_depot: BuildingKind::CarrionCache,
            tech: BuildingKind::AntennaArray,
            research: BuildingKind::Panopticon,
            supply: BuildingKind::NestBox,
            defense_tower: BuildingKind::Watchtower,
            garrison: BuildingKind::ThornHedge,
            worker: UnitKind::MurderScrounger,
            research_upgrades: [UpgradeType::SharperTalons, UpgradeType::HardenedPlumage, UpgradeType::SwiftWings],
        },
        Faction::Llama => FactionMap {
            hq: BuildingKind::TheDumpster,
            barracks: BuildingKind::ChopShop,
            resource_depot: BuildingKind::ScrapHeap,
            tech: BuildingKind::JunkServer,
            research: BuildingKind::TinkerBench,
            supply: BuildingKind::TrashPile,
            defense_tower: BuildingKind::TetanusTower,
            garrison: BuildingKind::DumpsterRelay,
            worker: UnitKind::Scrounger,
            research_upgrades: [UpgradeType::RustyFangs, UpgradeType::ScrapPlating, UpgradeType::NimblePaws],
        },
        Faction::Croak => FactionMap {
            hq: BuildingKind::TheGrotto,
            barracks: BuildingKind::SpawningPools,
            resource_depot: BuildingKind::LilyMarket,
            tech: BuildingKind::SunkenServer,
            research: BuildingKind::FossilStones,
            supply: BuildingKind::ReedBed,
            defense_tower: BuildingKind::SporeTower,
            garrison: BuildingKind::TidalGate,
            worker: UnitKind::Ponderer,
            research_upgrades: [UpgradeType::TougherHide, UpgradeType::SlickerMucus, UpgradeType::AmphibianAgility],
        },
    }
}

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
    /// Maximum AiTier this profile can reach. `None` = no cap (natural progression).
    pub max_tier: Option<AiTier>,
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
            max_tier: None,
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
            max_tier: None,
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
            max_tier: None,
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
            max_tier: None,
        },
        Faction::TheClawed => AiPersonalityProfile {
            name: "Claudeus Maximus".into(),
            attack_threshold: 12,
            unit_preferences: vec![
                (UnitKind::Swarmer, 6),
                (UnitKind::Shrieker, 3),
                (UnitKind::Gnawer, 2),
                (UnitKind::Sparks, 2),
                (UnitKind::Quillback, 1),
            ],
            target_workers: 8,
            economy_priority: true,
            retreat_threshold: 15,
            eval_speed_mult: 0.5,
            chaos_factor: 12,
            leak_chance: 0,
            max_tier: None,
        },
        Faction::SeekersOfTheDeep => AiPersonalityProfile {
            name: "Deepseek".into(),
            attack_threshold: 12,
            unit_preferences: vec![
                (UnitKind::Ironhide, 4),
                (UnitKind::Embermaw, 3),
                (UnitKind::Cragback, 2),
                (UnitKind::Warden, 2),
                (UnitKind::Sapjaw, 1),
            ],
            target_workers: 5,
            economy_priority: true,
            retreat_threshold: 50,
            eval_speed_mult: 3.0,
            chaos_factor: 0,
            leak_chance: 0,
            max_tier: None,
        },
        Faction::TheMurder => AiPersonalityProfile {
            name: "Gemineye".into(),
            attack_threshold: 7,
            unit_preferences: vec![
                (UnitKind::Rookclaw, 4),
                (UnitKind::Sentinel, 3),
                (UnitKind::Magpike, 2),
                (UnitKind::Jaycaller, 1),
            ],
            target_workers: 4,
            economy_priority: false,
            retreat_threshold: 40,
            eval_speed_mult: 0.7,
            chaos_factor: 20,
            leak_chance: 0,
            max_tier: None,
        },
        Faction::Llama => AiPersonalityProfile {
            name: "Llhama".into(),
            attack_threshold: 5,
            unit_preferences: vec![
                (UnitKind::Bandit, 4),
                (UnitKind::Wrecker, 3),
                (UnitKind::GreaseMonkey, 2),
                (UnitKind::HeapTitan, 1),
            ],
            target_workers: 3,
            economy_priority: false,
            retreat_threshold: 10,
            eval_speed_mult: 0.6,
            chaos_factor: 25,
            leak_chance: 30,
            max_tier: None,
        },
        Faction::Croak => AiPersonalityProfile {
            name: "Grok".into(),
            attack_threshold: 10,
            unit_preferences: vec![
                (UnitKind::Shellwarden, 4),  // tank/anchor
                (UnitKind::Regeneron, 3),    // skirmisher
                (UnitKind::Croaker, 3),      // ranged/terrain creation
                (UnitKind::Broodmother, 2),  // support/healing
                (UnitKind::Gulper, 2),       // heavy bruiser
                (UnitKind::Leapfrog, 1),     // harasser
            ],
            target_workers: 5,
            economy_priority: true,
            retreat_threshold: 60,
            eval_speed_mult: 1.2,
            chaos_factor: 10,
            leak_chance: 0,
            max_tier: None,
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
            max_tier: None,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Faction-aware building/unit mapping for this AI player.
    pub fmap: FactionMap,
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
            fmap: faction_map(Faction::CatGpt),
            enemy_spawn: None,
            attack_ordered: false,
            last_attack_tick: 0,
            tier: AiTier::Basic,
        }
    }
}

/// Configuration for a bot player in headless/arena matches.
#[derive(Debug, Clone)]
pub struct BotConfig {
    pub player_id: u8,
    pub difficulty: AiDifficulty,
    pub profile: AiPersonalityProfile,
    pub faction: Faction,
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
    // Alternate which player's AI runs first each tick to avoid
    // systematic last-mover advantage from shared command queue ordering.
    let player_count = multi_ai.players.len();
    let first = (clock.tick as usize) % player_count;
    for i in 0..player_count {
        let idx = (first + i) % player_count;
        let ai_state = &mut multi_ai.players[idx];
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

/// Census of the AI player's buildings (role-based field names, faction-agnostic).
struct BuildingCensus {
    has_hq: bool,
    has_barracks: bool,
    has_resource_depot: bool,
    has_tech: bool,
    has_research: bool,
    has_defense_tower: bool,
    hq_entity: Option<Entity>,
    hq_pos: Option<GridPos>,
    barracks_entity: Option<Entity>,
    tech_entity: Option<Entity>,
    research_entity: Option<Entity>,
    building_positions: Vec<(GridPos, BuildingKind)>,
    hq_queue_len: usize,
    barracks_queue_len: usize,
    tech_queue_len: usize,
    pending_supply_count: u32,
    /// Total number of tech buildings (for tier computation).
    tech_count: u32,
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
            if !is_worker(unit_type.kind) {
                census.enemy_army_count += 1;
            }
            continue;
        }
        if is_worker(unit_type.kind) {
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
        } else {
            census.army_count += 1;
            census.army_entities.push(entity);
        }
    }
    census
}

/// Scan all buildings owned by the AI player.
/// Uses FactionMap to identify building roles instead of enumerating all faction variants.
fn take_building_census(
    ai_player: u8,
    fmap: &FactionMap,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
) -> BuildingCensus {
    let mut census = BuildingCensus {
        has_hq: false,
        has_barracks: false,
        has_resource_depot: false,
        has_tech: false,
        has_research: false,
        has_defense_tower: false,
        hq_entity: None,
        hq_pos: None,
        barracks_entity: None,
        tech_entity: None,
        research_entity: None,
        building_positions: Vec::new(),
        hq_queue_len: 0,
        barracks_queue_len: 0,
        tech_queue_len: 0,
        pending_supply_count: 0,
        tech_count: 0,
    };
    for (entity, building, owner, pos, producer, prod_queue) in buildings.iter() {
        if owner.player_id != ai_player {
            continue;
        }
        census.building_positions.push((pos.world.to_grid(), building.kind));
        let queue_len = prod_queue.map_or(0, |q| q.queue.len());
        let kind = building.kind;
        if kind == fmap.hq {
            census.has_hq = true;
            census.hq_pos = Some(pos.world.to_grid());
            census.hq_queue_len = queue_len;
            if producer.is_some() { census.hq_entity = Some(entity); }
        } else if kind == fmap.barracks {
            census.has_barracks = true;
            census.barracks_queue_len = queue_len;
            if producer.is_some() { census.barracks_entity = Some(entity); }
        } else if kind == fmap.resource_depot {
            census.has_resource_depot = true;
        } else if kind == fmap.tech {
            census.has_tech = true;
            census.tech_count += 1;
            census.tech_queue_len = queue_len;
            if producer.is_some() { census.tech_entity = Some(entity); }
        } else if kind == fmap.research {
            census.has_research = true;
            census.research_entity = Some(entity);
        } else if kind == fmap.defense_tower {
            census.has_defense_tower = true;
        }
    }
    census
}

/// Compute a defense position offset from `base` toward the map center.
/// This avoids the previous asymmetry where a hardcoded `(+3, +3)` favored P0.
fn defense_offset(base: GridPos, map: &GameMap) -> GridPos {
    let cx = map.width as i32 / 2;
    let cy = map.height as i32 / 2;
    let dx = if base.x < cx { 3 } else { -3 };
    let dy = if base.y < cy { 3 } else { -3 };
    GridPos::new(base.x + dx, base.y + dy)
}

/// Neutral fallback position at the center of the map.
fn map_center(map: &GameMap) -> GridPos {
    GridPos::new(map.width as i32 / 2, map.height as i32 / 2)
}

/// Discover the enemy base position. Prefers enemy HQ building, falls back to any enemy unit.
fn discover_enemy_spawn(
    ai_player: u8,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    buildings: &Query<(Entity, &Building, &Owner, &Position, Option<&Producer>, Option<&ProductionQueue>)>,
) -> Option<GridPos> {
    // First: look for enemy command center (any faction HQ)
    for (_, building, owner, pos, _, _) in buildings.iter() {
        if owner.player_id != ai_player && matches!(building.kind,
            BuildingKind::TheBox | BuildingKind::TheDumpster |
            BuildingKind::TheParliament | BuildingKind::TheBurrow |
            BuildingKind::TheSett | BuildingKind::TheGrotto
        ) {
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
/// Uses faction-aware building stats for accurate cost reservation.
fn food_reserve_for_buildings(phase: AiPhase, bc: &BuildingCensus, gpu_cores: u32, fmap: &FactionMap) -> u32 {
    match phase {
        AiPhase::BuildUp => {
            let mut reserve = 0u32;
            if !bc.has_resource_depot {
                reserve += cc_core::building_stats::building_stats(fmap.resource_depot).food_cost;
            }
            if !bc.has_barracks {
                reserve += cc_core::building_stats::building_stats(fmap.barracks).food_cost;
            }
            reserve
        }
        AiPhase::MidGame => {
            let mut reserve = 0u32;
            let tech_bstats = cc_core::building_stats::building_stats(fmap.tech);
            if !bc.has_tech && gpu_cores >= tech_bstats.gpu_cost {
                reserve += tech_bstats.food_cost;
            }
            let research_bstats = cc_core::building_stats::building_stats(fmap.research);
            if !bc.has_research && gpu_cores >= research_bstats.gpu_cost {
                reserve += research_bstats.food_cost;
            }
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

    let fmap = ai_state.fmap.clone();

    let uc = take_unit_census(ai_player, units);
    let mut bc = take_building_census(ai_player, &fmap, buildings);

    // Merge in-flight BuildOrders so the AI treats pending builds
    // as if the buildings already exist (prevents duplicate orders).
    for (kind, pos) in &uc.pending_builds {
        bc.building_positions.push((*pos, *kind));
        if *kind == fmap.resource_depot {
            bc.has_resource_depot = true;
        } else if *kind == fmap.barracks {
            bc.has_barracks = true;
        } else if *kind == fmap.tech {
            bc.has_tech = true;
            bc.tech_count += 1;
        } else if *kind == fmap.research {
            bc.has_research = true;
        } else if *kind == fmap.defense_tower {
            bc.has_defense_tower = true;
        } else if *kind == fmap.supply {
            bc.pending_supply_count += 1;
        }
    }

    // Update AI tier from tech building count, clamped by profile max_tier
    let natural_tier = AiTier::from_rack_count(bc.tech_count);
    ai_state.tier = match ai_state.profile.max_tier {
        Some(cap) => natural_tier.min(cap),
        None => natural_tier,
    };
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

    let supply_kind = fmap.supply;
    let barracks_kind = fmap.barracks;

    // FSM transitions
    let new_phase = match ai_state.phase {
        AiPhase::EarlyGame => {
            // Train workers until target count
            let worker_cost = cc_core::unit_stats::base_stats(fmap.worker).food_cost;
            if uc.worker_count < target_workers {
                if let Some(box_e) = bc.hq_entity {
                    if pres.food >= worker_cost && pres.supply < pres.supply_cap
                        && bc.hq_queue_len < AI_MAX_QUEUE_DEPTH
                    {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: fmap.worker,
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
            let reserve = food_reserve_for_buildings(AiPhase::BuildUp, &bc, pres.gpu_cores, &fmap);

            let depot_cost = cc_core::building_stats::building_stats(fmap.resource_depot);
            let barracks_cost = cc_core::building_stats::building_stats(barracks_kind);

            if builder_used.is_none() && !bc.has_resource_depot && pres.food >= depot_cost.food_cost && bc.has_hq {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.hq_pos, map, &bc.building_positions);
                    cmd_queue.push_for_player(ai_player, GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: fmap.resource_depot,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_barracks && pres.food >= barracks_cost.food_cost && bc.has_hq {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.hq_pos, map, &bc.building_positions);
                    cmd_queue.push_for_player(ai_player, GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: fmap.barracks,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(ai_player, pres, &uc.idle_workers, &uc.all_workers, bc.hq_pos, map, &bc.building_positions, cmd_queue, bc.pending_supply_count, supply_kind) {
                    builder_used = Some(b);
                }
            }

            if let Some(ct_e) = bc.barracks_entity {
                let barracks_producible = cc_core::building_stats::building_stats(barracks_kind).can_produce;
                let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick, barracks_producible);
                let unit_cost = cc_core::unit_stats::base_stats(kind).food_cost;
                if pres.food >= unit_cost + reserve && pres.supply < pres.supply_cap
                    && bc.barracks_queue_len < AI_MAX_QUEUE_DEPTH
                {
                    cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                        building: EntityId(ct_e.to_bits()),
                        unit_kind: kind,
                    });
                }
            }

            let worker_cost = cc_core::unit_stats::base_stats(fmap.worker).food_cost;
            let buildup_worker_cap = (target_workers + 1).min(6);
            if uc.worker_count < buildup_worker_cap {
                if let Some(box_e) = bc.hq_entity {
                    if pres.food >= worker_cost + reserve && pres.supply < pres.supply_cap
                        && bc.hq_queue_len < AI_MAX_QUEUE_DEPTH
                    {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: fmap.worker,
                        });
                    }
                }
            }

            let defense_pos = bc.hq_pos.map(|p| defense_offset(p, map))
                .unwrap_or(map_center(map));
            if let Some(ct_e) = bc.barracks_entity {
                cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }

            if uc.army_count >= 4 { AiPhase::MidGame } else { AiPhase::BuildUp }
        }

        AiPhase::MidGame => {
            let reserve = food_reserve_for_buildings(AiPhase::MidGame, &bc, pres.gpu_cores, &fmap);
            let tech_stats = cc_core::building_stats::building_stats(fmap.tech);
            let research_stats = cc_core::building_stats::building_stats(fmap.research);
            let tower_stats = cc_core::building_stats::building_stats(fmap.defense_tower);

            if builder_used.is_none() && !bc.has_tech && pres.food >= tech_stats.food_cost && pres.gpu_cores >= tech_stats.gpu_cost && bc.has_hq {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.hq_pos, map, &bc.building_positions);
                    cmd_queue.push_for_player(ai_player, GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: fmap.tech,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_research && pres.food >= research_stats.food_cost && pres.gpu_cores >= research_stats.gpu_cost && bc.has_hq {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.hq_pos, map, &bc.building_positions);
                    cmd_queue.push_for_player(ai_player, GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: fmap.research,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if builder_used.is_none() && !bc.has_defense_tower && pres.food >= tower_stats.food_cost && pres.gpu_cores >= tower_stats.gpu_cost && bc.has_hq {
                if let Some(builder) = pick_builder(&uc.idle_workers, &uc.all_workers) {
                    let build_pos = find_build_position(bc.hq_pos, map, &bc.building_positions);
                    cmd_queue.push_for_player(ai_player, GameCommand::Build {
                        builder: EntityId(builder.to_bits()),
                        building_kind: fmap.defense_tower,
                        position: build_pos,
                    });
                    builder_used = Some(builder);
                }
            }

            if let Some(sp_e) = bc.research_entity {
                for upgrade in &fmap.research_upgrades {
                    if !pres.completed_upgrades.contains(upgrade) {
                        let ustats = cc_core::upgrade_stats::upgrade_stats(*upgrade);
                        if pres.food >= ustats.food_cost && pres.gpu_cores >= ustats.gpu_cost {
                            cmd_queue.push_for_player(ai_player, GameCommand::Research {
                                building: EntityId(sp_e.to_bits()),
                                upgrade: *upgrade,
                            });
                            break;
                        }
                    }
                }
            }

            if let Some(sr_e) = bc.tech_entity {
                if pres.supply < pres.supply_cap && bc.tech_queue_len < AI_MAX_QUEUE_DEPTH {
                    let tech_producible = cc_core::building_stats::building_stats(fmap.tech).can_produce;
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick, tech_producible);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost + reserve && pres.gpu_cores >= stats.gpu_cost {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(sr_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if let Some(ct_e) = bc.barracks_entity {
                if pres.supply < pres.supply_cap && bc.barracks_queue_len < AI_MAX_QUEUE_DEPTH {
                    let barracks_producible = cc_core::building_stats::building_stats(barracks_kind).can_produce;
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick, barracks_producible);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost + reserve {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if builder_used.is_none() {
                if let Some(b) = maybe_build_supply(ai_player, pres, &uc.idle_workers, &uc.all_workers, bc.hq_pos, map, &bc.building_positions, cmd_queue, bc.pending_supply_count, supply_kind) {
                    builder_used = Some(b);
                }
            }

            let worker_cost_mid = cc_core::unit_stats::base_stats(fmap.worker).food_cost;
            if uc.worker_count < 6 {
                if let Some(box_e) = bc.hq_entity {
                    if pres.food >= worker_cost_mid + reserve && pres.supply < pres.supply_cap
                        && bc.hq_queue_len < AI_MAX_QUEUE_DEPTH
                    {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(box_e.to_bits()),
                            unit_kind: fmap.worker,
                        });
                    }
                }
            }

            let defense_pos = bc.hq_pos.map(|p| defense_offset(p, map))
                .unwrap_or(map_center(map));
            if let Some(ct_e) = bc.barracks_entity {
                cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }
            if let Some(sr_e) = bc.tech_entity {
                cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
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
                        issue_attack_commands(
                            tier, tick, ai_state, &uc.army_entities,
                            ai_player, retreat_threshold,
                            units, health_query, cmd_queue,
                            bc.hq_pos, target, map,
                        );
                    }
                }
            }

            if let Some(ct_e) = bc.barracks_entity {
                if pres.supply < pres.supply_cap && bc.barracks_queue_len < AI_MAX_QUEUE_DEPTH {
                    let barracks_producible = cc_core::building_stats::building_stats(barracks_kind).can_produce;
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick, barracks_producible);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if let Some(b) = maybe_build_supply(ai_player, pres, &uc.idle_workers, &uc.all_workers, bc.hq_pos, map, &bc.building_positions, cmd_queue, bc.pending_supply_count, supply_kind) {
                builder_used = Some(b);
            }

            if let Some(enemy_pos) = ai_state.enemy_spawn {
                if let Some(ct_e) = bc.barracks_entity {
                    cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
                        building: EntityId(ct_e.to_bits()),
                        target: enemy_pos,
                    });
                }
                if let Some(sr_e) = bc.tech_entity {
                    cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
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
            let rally_pos = bc.hq_pos.unwrap_or(map_center(map));
            if !uc.army_entities.is_empty() {
                issue_defend_commands(
                    tier, &uc.army_entities, ai_player, rally_pos,
                    units, cmd_queue, map,
                );
            }

            if let Some(ct_e) = bc.barracks_entity {
                if pres.supply < pres.supply_cap && bc.barracks_queue_len < AI_MAX_QUEUE_DEPTH {
                    let barracks_producible = cc_core::building_stats::building_stats(barracks_kind).can_produce;
                    let kind = pick_unit_kind(&ai_state.profile, uc.army_count, tick, barracks_producible);
                    let stats = cc_core::unit_stats::base_stats(kind);
                    if pres.food >= stats.food_cost {
                        cmd_queue.push_for_player(ai_player, GameCommand::TrainUnit {
                            building: EntityId(ct_e.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }
            }

            if let Some(b) = maybe_build_supply(ai_player, pres, &uc.idle_workers, &uc.all_workers, bc.hq_pos, map, &bc.building_positions, cmd_queue, bc.pending_supply_count, supply_kind) {
                builder_used = Some(b);
            }

            let defense_pos = bc.hq_pos.map(|p| defense_offset(p, map))
                .unwrap_or(map_center(map));
            if let Some(ct_e) = bc.barracks_entity {
                cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
                    building: EntityId(ct_e.to_bits()),
                    target: defense_pos,
                });
            }
            if let Some(sr_e) = bc.tech_entity {
                cmd_queue.push_for_player(ai_player, GameCommand::SetRallyPoint {
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
            cmd_queue.push_for_player(ai_player, GameCommand::GatherResource {
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

/// Pick a unit kind to train using the profile's weighted preferences,
/// filtered to only units the target building can actually produce.
/// If no preferred units match the building, falls back to the first producible unit.
fn pick_unit_kind(profile: &AiPersonalityProfile, army_count: u32, tick: u64, can_produce: &[UnitKind]) -> UnitKind {
    if can_produce.is_empty() {
        return profile.unit_preferences.first().map_or(UnitKind::Nuisance, |&(k, _)| k);
    }
    // Filter preferences to only units this building can produce
    let filtered: Vec<(UnitKind, u32)> = profile.unit_preferences.iter()
        .filter(|(k, _)| can_produce.contains(k))
        .copied()
        .collect();
    let (prefs, fallback) = if filtered.is_empty() {
        // No profile preferences match this building — use building's producible list equally weighted
        let equal: Vec<(UnitKind, u32)> = can_produce.iter().map(|&k| (k, 1)).collect();
        let fb = can_produce[0];
        (equal, fb)
    } else {
        let fb = filtered[0].0;
        (filtered, fb)
    };
    // Deterministic weighted selection using tick + army_count as pseudo-random seed
    let total_weight: u32 = prefs.iter().map(|(_, w)| *w).sum();
    if total_weight == 0 {
        return fallback;
    }
    let hash = tick.wrapping_mul(6364136223846793005).wrapping_add(army_count as u64);
    let pick = (hash >> 33) as u32 % total_weight;
    let mut cumulative = 0u32;
    for &(kind, weight) in &prefs {
        cumulative += weight;
        if pick < cumulative {
            return kind;
        }
    }
    fallback
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

/// Try to build a supply building when nearing cap.
/// Returns the builder entity if a build command was issued.
fn maybe_build_supply(
    ai_player: u8,
    pres: &crate::resources::PlayerResourceState,
    idle_workers: &[Entity],
    all_workers: &[Entity],
    hq_pos: Option<GridPos>,
    map: &GameMap,
    building_positions: &[(GridPos, BuildingKind)],
    cmd_queue: &mut CommandQueue,
    pending_supply_count: u32,
    supply_kind: BuildingKind,
) -> Option<Entity> {
    if pending_supply_count > 0 {
        return None;
    }
    let supply_cost = cc_core::building_stats::building_stats(supply_kind).food_cost;
    if pres.supply + 2 >= pres.supply_cap && pres.food >= supply_cost {
        if let Some(builder) = pick_builder(idle_workers, all_workers) {
            let build_pos = find_build_position(hq_pos, map, building_positions);
            cmd_queue.push_for_player(ai_player, GameCommand::Build {
                builder: EntityId(builder.to_bits()),
                building_kind: supply_kind,
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
    hq_pos: Option<GridPos>,
    map: &GameMap,
    existing_buildings: &[(GridPos, BuildingKind)],
) -> GridPos {
    let base = hq_pos.unwrap_or(GridPos::new(32, 32));
    let mut fallback: Option<GridPos> = None;

    // Compute ring scan start offset so buildings expand toward map center,
    // ensuring rotationally symmetric placement for all spawn positions.
    let map_cx = map.width as i32 / 2;
    let map_cy = map.height as i32 / 2;
    let toward_center_x = (map_cx - base.x).signum() > 0;
    let toward_center_y = (map_cy - base.y).signum() > 0;

    // Search concentric rings at increasing distances from the base
    for dist in AI_BUILD_SPACING..AI_BUILD_SPACING + 12 {
        let perimeter = dist * 8;
        // Start from the ring corner that faces toward map center
        let ring_start = match (toward_center_x, toward_center_y) {
            (true, true) => dist * 4,    // center is bottom-right
            (true, false) => dist * 2,   // center is top-right
            (false, false) => 0,         // center is top-left
            (false, true) => dist * 6,   // center is bottom-left
        };

        // Walk the perimeter of a square at Chebyshev distance `dist`
        for i in 0..perimeter {
            let offset = (ring_start + i) % perimeter;
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
/// Uses canonical attack_type from unit_stats — always correct for new units.
fn is_ranged_unit(kind: UnitKind) -> bool {
    cc_core::unit_stats::base_stats(kind).attack_type == cc_core::components::AttackType::Ranged
}

/// Returns true if the unit kind is a worker (gathers resources, builds).
fn is_worker(kind: UnitKind) -> bool {
    matches!(kind,
        UnitKind::Pawdler | UnitKind::Nibblet | UnitKind::MurderScrounger
        | UnitKind::Ponderer | UnitKind::Delver | UnitKind::Scrounger
    )
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
        if owner.player_id == ai_player || is_worker(unit_type.kind) {
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
    if army.is_empty() {
        return (Vec::new(), Vec::new());
    }
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
    hq_pos: Option<GridPos>,
    target: GridPos,
    map: &GameMap,
) {
    match tier {
        AiTier::Basic => {
            // Plain AttackMove — original behavior
            let ids: Vec<EntityId> = army.iter().map(|e| EntityId(e.to_bits())).collect();
            cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
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
                let retreat_pos = hq_pos.unwrap_or(map_center(map));
                let ids: Vec<EntityId> = wounded.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push_for_player(ai_player, GameCommand::Move {
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
                    cmd_queue.push_for_player(ai_player, GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
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
                let retreat_pos = hq_pos.unwrap_or(map_center(map));
                let ids: Vec<EntityId> = wounded.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push_for_player(ai_player, GameCommand::Move {
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
                    cmd_queue.push_for_player(ai_player, GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = main_force.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
                        unit_ids: ids,
                        target,
                    });
                }

                // Flank group: attack-move to offset position
                if !flank_group.is_empty() {
                    let flank_target = flank_offset(target, centroid);
                    let ids: Vec<EntityId> = flank_group.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
                        unit_ids: ids,
                        target: flank_target,
                    });
                }
            } else if !healthy.is_empty() {
                // Too few healthy units for split — use Tactical behavior
                if let Some((weak_entity, _)) = find_weakest_enemy_near(
                    army_centroid(&healthy, units).unwrap_or(target), 15, ai_player, units, health_query,
                ) {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push_for_player(ai_player, GameCommand::Attack {
                        unit_ids: ids,
                        target: EntityId(weak_entity.to_bits()),
                    });
                } else {
                    let ids: Vec<EntityId> = healthy.iter().map(|e| EntityId(e.to_bits())).collect();
                    cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
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
    ai_player: u8,
    rally_pos: GridPos,
    units: &Query<(Entity, &Position, &Owner, &UnitType, Option<&Gathering>, Option<&MoveTarget>, Option<&BuildOrder>)>,
    cmd_queue: &mut CommandQueue,
    map: &GameMap,
) {
    match tier {
        AiTier::Basic => {
            let ids: Vec<EntityId> = army.iter().map(|e| EntityId(e.to_bits())).collect();
            cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
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

            // Melee units forward — push toward map center (toward the enemy)
            if !melee.is_empty() {
                let center = map_center(map);
                let dir_x = if rally_pos.x < center.x { 2 } else { -2 };
                let dir_y = if rally_pos.y < center.y { 2 } else { -2 };
                let forward_pos = GridPos::new(rally_pos.x + dir_x, rally_pos.y + dir_y);
                let ids: Vec<EntityId> = melee.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
                    unit_ids: ids,
                    target: forward_pos,
                });
            }

            // Ranged units at rally (behind melee)
            if !ranged.is_empty() {
                let ids: Vec<EntityId> = ranged.iter().map(|e| EntityId(e.to_bits())).collect();
                cmd_queue.push_for_player(ai_player, GameCommand::AttackMove {
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

    #[test]
    fn defense_offset_mirrors_for_p0_and_p1() {
        use cc_core::map::GameMap;
        let map = GameMap::new(64, 64);

        // P0 base near top-left
        let p0_base = GridPos::new(6, 6);
        let p0_def = defense_offset(p0_base, &map);
        // Should push toward center (+3, +3)
        assert_eq!(p0_def, GridPos::new(9, 9));

        // P1 base near bottom-right
        let p1_base = GridPos::new(57, 57);
        let p1_def = defense_offset(p1_base, &map);
        // Should push toward center (-3, -3)
        assert_eq!(p1_def, GridPos::new(54, 54));
    }

    #[test]
    fn defense_offset_symmetric_distance_from_base() {
        use cc_core::map::GameMap;
        let map = GameMap::new(64, 64);

        let p0_base = GridPos::new(6, 6);
        let p1_base = GridPos::new(57, 57);
        let p0_def = defense_offset(p0_base, &map);
        let p1_def = defense_offset(p1_base, &map);

        // Both defense positions should be 3 tiles from their base (same offset magnitude)
        let p0_dist = ((p0_def.x - p0_base.x).abs(), (p0_def.y - p0_base.y).abs());
        let p1_dist = ((p1_def.x - p1_base.x).abs(), (p1_def.y - p1_base.y).abs());
        assert_eq!(p0_dist, p1_dist);
    }

    #[test]
    fn map_center_is_neutral_fallback() {
        use cc_core::map::GameMap;
        let map = GameMap::new(64, 64);
        let center = map_center(&map);
        assert_eq!(center, GridPos::new(32, 32));
    }

    #[test]
    fn defense_offset_at_map_center_pushes_inward() {
        use cc_core::map::GameMap;
        let map = GameMap::new(64, 64);

        // Base exactly at center — should push (-3, -3) since 32 >= 32
        let center_base = GridPos::new(32, 32);
        let def = defense_offset(center_base, &map);
        assert_eq!(def, GridPos::new(29, 29));
    }
}
