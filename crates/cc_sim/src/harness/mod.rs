//! Wet test harness for AI-vs-AI matches.
//!
//! Runs headless simulations with two AI bots, capturing snapshots and minimaps,
//! checking invariants every tick, and producing a JSON report.

pub mod invariants;
pub mod minimap;
pub mod report;
pub mod snapshot;

use std::path::PathBuf;
use std::time::Instant;

use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::map_gen::{self, MapGenParams};
use cc_core::unit_stats::{base_stats, spawn_base_unit};

// Re-export BotConfig so `use cc_sim::harness::*` brings it into scope
// (used by wet_test.rs and harness consumers).
pub use crate::ai::fsm::BotConfig;
use crate::ai::fsm::{AiDifficulty, AiPersonalityProfile, AiPhase, AiState};
use crate::ai::MultiAiState;
use crate::resources::{
    CombatStats, CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock,
    SpawnPositions,
};
use crate::systems::{
    ability_effect_system::ability_effect_system,
    ability_system::ability_cooldown_system, aura_system::aura_system,
    builder_system::builder_system,
    cleanup_system::cleanup_system, combat_system::combat_system,
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, production_system::production_system,
    projectile_system::projectile_system, research_system::research_system,
    resource_system::gathering_system, stat_modifier_system::stat_modifier_system,
    status_effect_system::status_effect_system,
    target_acquisition_system::target_acquisition_system, tick_system::tick_system,
    tower_combat_system::tower_combat_system, victory_system::victory_system,
};

use invariants::{InvariantChecker, Severity};
use snapshot::GameStateSnapshot;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// A synthetic voice command to inject at a specific tick.
#[derive(Debug, Clone)]
pub struct VoiceInjection {
    pub tick: u64,
    pub keyword: String,
    pub confidence: f32,
}

/// Configuration for a wet test match.
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    pub seed: u64,
    pub map_size: (u32, u32),
    pub max_ticks: u64,
    pub snapshot_interval: u64,
    pub minimap_interval: u64,
    pub invariant_interval: u64,
    pub output_dir: Option<PathBuf>,
    pub bots: [BotConfig; 2],
    pub voice_script: Option<Vec<VoiceInjection>>,
    pub fail_on_warning: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            map_size: (64, 64),
            max_ticks: 6000,        // 10 min at 10hz
            snapshot_interval: 100, // every 10s
            minimap_interval: 100,
            invariant_interval: 10, // every second
            output_dir: None,
            bots: [
                BotConfig {
                    player_id: 0,
                    difficulty: AiDifficulty::Medium,
                    profile: AiPersonalityProfile::balanced(),
                    faction: cc_core::components::Faction::CatGpt,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    profile: AiPersonalityProfile::balanced(),
                    faction: cc_core::components::Faction::CatGpt,
                },
            ],
            voice_script: None,
            fail_on_warning: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Match outcome
// ---------------------------------------------------------------------------

/// Outcome of a completed match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOutcome {
    Victory { winner: u8, tick: u64 },
    Draw { tick: u64 },
    Timeout { tick: u64, leading_player: Option<u8> },
    Error { tick: u64, message: String },
}

impl std::fmt::Display for MatchOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchOutcome::Victory { winner, tick } => {
                write!(f, "Victory(player {winner} at tick {tick})")
            }
            MatchOutcome::Draw { tick } => write!(f, "Draw(tick {tick})"),
            MatchOutcome::Timeout { tick, leading_player } => {
                write!(f, "Timeout(tick {tick}, leading: {leading_player:?})")
            }
            MatchOutcome::Error { tick, message } => {
                write!(f, "Error(tick {tick}: {message})")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Match result
// ---------------------------------------------------------------------------

/// Full result of a match run.
pub struct MatchResult {
    pub outcome: MatchOutcome,
    pub final_tick: u64,
    pub wall_time_ms: u64,
    pub snapshots: Vec<GameStateSnapshot>,
    pub minimap_frames: Vec<(u64, Vec<u8>)>,
    pub violations: Vec<invariants::InvariantViolation>,
    pub voice_commands_injected: u32,
    pub voice_commands_resolved: u32,
}

impl MatchResult {
    /// A match passes if there are no Error/Fatal violations and no Error outcome.
    pub fn passed(&self) -> bool {
        let is_error = matches!(self.outcome, MatchOutcome::Error { .. });
        self.fatal_violations().is_empty() && !is_error
    }

    /// Returns only Error/Fatal severity violations.
    pub fn fatal_violations(&self) -> Vec<&invariants::InvariantViolation> {
        self.violations
            .iter()
            .filter(|v| matches!(v.severity, Severity::Error | Severity::Fatal))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Voice command resolution (inline, avoids cc_voice circular dep)
// ---------------------------------------------------------------------------

/// Resolve a voice keyword into a GameCommand for the given units.
/// SYNC: simplified copy of cc_voice::intent::resolve_agent_command
/// (duplicated here to avoid circular dependency cc_sim ↔ cc_voice)
fn resolve_voice_keyword(
    keyword: &str,
    player_units: &[(Entity, UnitKind)],
) -> Option<GameCommand> {
    let unit_ids: Vec<EntityId> = player_units
        .iter()
        .map(|(e, _)| EntityId(e.to_bits()))
        .collect();

    if unit_ids.is_empty() {
        return None;
    }

    match keyword {
        "stop" => Some(GameCommand::Stop { unit_ids }),
        "hold" | "defend" | "guard" => Some(GameCommand::HoldPosition { unit_ids }),
        "gather" => {
            let worker_ids: Vec<EntityId> = player_units
                .iter()
                .filter(|(_, kind)| *kind == UnitKind::Pawdler)
                .map(|(e, _)| EntityId(e.to_bits()))
                .collect();
            if worker_ids.is_empty() {
                None
            } else {
                Some(GameCommand::Stop {
                    unit_ids: worker_ids,
                })
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Main match runner
// ---------------------------------------------------------------------------

/// Run a complete AI-vs-AI match with the given configuration.
pub fn run_match(config: &HarnessConfig) -> MatchResult {
    let wall_start = Instant::now();

    // Generate map
    let params = MapGenParams {
        width: config.map_size.0,
        height: config.map_size.1,
        seed: config.seed,
        ..Default::default()
    };
    let map_def = map_gen::generate_map(&params);
    let game_map = map_def.to_game_map();
    let map_width = game_map.width;
    let map_height = game_map.height;

    // Create World + Schedule
    let (mut world, mut schedule) = make_harness_sim(game_map, config, &map_def);

    let mut checker = InvariantChecker::new(map_width, map_height);
    let mut snapshots: Vec<GameStateSnapshot> = Vec::new();
    let mut minimap_frames: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut voice_injected = 0u32;
    let mut voice_resolved = 0u32;

    // Sort voice script by tick
    let voice_script: Vec<VoiceInjection> = config
        .voice_script
        .as_ref()
        .map(|v| {
            let mut sorted = v.clone();
            sorted.sort_by_key(|vi| vi.tick);
            sorted
        })
        .unwrap_or_default();
    let mut voice_idx = 0usize;

    let mut outcome = MatchOutcome::Timeout {
        tick: config.max_ticks,
        leading_player: None,
    };

    // Create output directories if needed
    if let Some(ref dir) = config.output_dir {
        let _ = std::fs::create_dir_all(dir.join("snapshots"));
        let _ = std::fs::create_dir_all(dir.join("minimaps"));
    }

    // Main simulation loop
    for _ in 0..config.max_ticks {
        let tick = world.resource::<SimClock>().tick;

        // Inject voice commands at the right tick
        while voice_idx < voice_script.len() && voice_script[voice_idx].tick <= tick {
            let injection = &voice_script[voice_idx];
            if injection.tick == tick {
                let player_units: Vec<(Entity, UnitKind)> = world
                    .query::<(Entity, &Owner, &UnitType)>()
                    .iter(&mut world)
                    .filter(|(_, owner, _)| owner.player_id == 0)
                    .map(|(e, _, ut)| (e, ut.kind))
                    .collect();

                if let Some(cmd) = resolve_voice_keyword(&injection.keyword, &player_units) {
                    world.resource_mut::<CommandQueue>().push(cmd);
                    voice_resolved += 1;
                }
                voice_injected += 1;
            }
            voice_idx += 1;
        }

        // Run one tick with panic catching
        let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            schedule.run(&mut world);
        }));

        let tick_after = world.resource::<SimClock>().tick;

        if let Err(panic_info) = tick_result {
            let msg = extract_panic_message(panic_info);
            checker.record_panic(tick_after, &msg);
            outcome = MatchOutcome::Error {
                tick: tick_after,
                message: msg,
            };
            break;
        }

        // Invariant checks
        if tick_after % config.invariant_interval == 0 {
            checker.check_all(&mut world, tick_after);
        }

        // Snapshot capture
        if tick_after % config.snapshot_interval == 0 {
            let snap = snapshot::capture_snapshot(&mut world, tick_after);

            if let Some(ref dir) = config.output_dir {
                if let Ok(json) = serde_json::to_string_pretty(&snap) {
                    let path = dir
                        .join("snapshots")
                        .join(format!("tick_{tick_after:04}.json"));
                    let _ = std::fs::write(path, json);
                }
            }

            snapshots.push(snap);
        }

        // Minimap capture
        if tick_after % config.minimap_interval == 0 {
            let png_data = minimap::render_minimap(&mut world, map_width, map_height);

            if let Some(ref dir) = config.output_dir {
                let path = dir
                    .join("minimaps")
                    .join(format!("tick_{tick_after:04}.png"));
                let _ = std::fs::write(&path, &png_data);
            }

            minimap_frames.push((tick_after, png_data));
        }

        // Check victory via GameState resource
        let game_state = *world.resource::<GameState>();
        if let GameState::Victory { winner } = game_state {
            outcome = MatchOutcome::Victory {
                winner,
                tick: tick_after,
            };
            break;
        }

        // Broader elimination check (attacker advantage on mutual elimination)
        if let Some(winner) = check_elimination(&mut world, Some(0)) {
            outcome = MatchOutcome::Victory {
                winner,
                tick: tick_after,
            };
            break;
        }
    }

    let final_tick = world.resource::<SimClock>().tick;
    if matches!(outcome, MatchOutcome::Timeout { .. }) {
        let leading = determine_leader(&mut world);
        outcome = MatchOutcome::Timeout {
            tick: final_tick,
            leading_player: leading,
        };
        checker.record_timeout(final_tick);
    }

    let wall_time = wall_start.elapsed().as_millis() as u64;

    MatchResult {
        outcome,
        final_tick,
        wall_time_ms: wall_time,
        snapshots,
        minimap_frames,
        violations: checker.violations,
        voice_commands_injected: voice_injected,
        voice_commands_resolved: voice_resolved,
    }
}

// ---------------------------------------------------------------------------
// Setup helpers (pub for reuse by cc_agent::arena)
// ---------------------------------------------------------------------------

/// Extract a panic message from a `catch_unwind` payload.
///
/// Handles the common `&str` and `String` downcast cases, falling back
/// to `"unknown panic"` for other payload types.
pub fn extract_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Create a headless Bevy `World` with the standard simulation resources
/// (CommandQueue, SimClock, GameState, PlayerResources, etc.) and the
/// spawn positions extracted from the map definition.
///
/// Returns `(world, spawn_positions)` so callers can further customize
/// the `MultiAiState`, schedule, and additional resources before running.
pub fn make_headless_world(
    map: GameMap,
    map_def: &cc_core::map_format::MapDefinition,
) -> (World, Vec<(u8, GridPos)>) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(GameState::Playing);
    world.insert_resource(CombatStats::default());

    let mut player_res = PlayerResources::default();
    while player_res.players.len() < 2 {
        player_res.players.push(Default::default());
    }
    world.insert_resource(player_res);
    world.insert_resource(MapResource { map });

    let spawn_positions: Vec<(u8, GridPos)> = map_def
        .spawn_points
        .iter()
        .map(|sp| (sp.player, GridPos::new(sp.pos.0, sp.pos.1)))
        .collect();
    world.insert_resource(SpawnPositions {
        positions: spawn_positions.clone(),
    });

    world.insert_resource(AiState::default());

    (world, spawn_positions)
}

fn make_harness_sim(
    map: GameMap,
    config: &HarnessConfig,
    map_def: &cc_core::map_format::MapDefinition,
) -> (World, Schedule) {
    let (mut world, spawn_positions) = make_headless_world(map, map_def);

    let multi_ai = MultiAiState {
        players: config
            .bots
            .iter()
            .map(|bot| AiState {
                player_id: bot.player_id,
                phase: AiPhase::EarlyGame,
                difficulty: bot.difficulty,
                profile: bot.profile.clone(),
                fmap: crate::ai::fsm::faction_map(bot.faction),
                enemy_spawn: None,
                attack_ordered: false,
                last_attack_tick: 0,
                tier: crate::ai::fsm::AiTier::Basic,
            })
            .collect(),
    };
    world.insert_resource(multi_ai);

    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            crate::ai::fsm::multi_ai_decision_system,
            process_commands,
            ability_cooldown_system,
            ability_effect_system,
            status_effect_system,
            aura_system,
            stat_modifier_system,
            production_system,
            research_system,
            gathering_system,
            target_acquisition_system,
            combat_system,
            tower_combat_system,
            projectile_system,
            movement_system,
            builder_system,
            grid_sync_system,
            cleanup_system,
            headless_despawn_system,
        )
            .chain(),
    );
    schedule.add_systems(victory_system.after(headless_despawn_system));

    for (player_id, spawn_pos) in &spawn_positions {
        let faction = config.bots[*player_id as usize].faction;
        spawn_starting_entities(&mut world, *player_id, *spawn_pos, faction, map_def);
    }

    (world, schedule)
}

/// Spawn the HQ building, starting workers, and resource deposits for a player.
pub fn spawn_starting_entities(
    world: &mut World,
    player_id: u8,
    spawn_pos: GridPos,
    faction: cc_core::components::Faction,
    map_def: &cc_core::map_format::MapDefinition,
) {
    let fmap = crate::ai::fsm::faction_map(faction);
    let hq_stats = building_stats(fmap.hq);
    world.spawn((
        Position {
            world: WorldPos::from_grid(spawn_pos),
        },
        GridCell { pos: spawn_pos },
        Owner { player_id },
        Building {
            kind: fmap.hq,
        },
        Health {
            current: hq_stats.health,
            max: hq_stats.health,
        },
        Producer,
        ProductionQueue::default(),
    ));

    // Grant supply_cap from HQ and starting resources
    {
        let mut player_res = world.resource_mut::<PlayerResources>();
        if let Some(pres) = player_res.players.get_mut(player_id as usize) {
            pres.supply_cap += hq_stats.supply_provided;
            pres.food = 200; // Starting resources
        }
    }

    let unit_supply_cost = base_stats(fmap.worker).supply_cost;
    for i in 0..2 {
        let offset = GridPos::new(spawn_pos.x + 1 + i, spawn_pos.y);
        spawn_combat_unit(world, offset, player_id, fmap.worker);
    }

    // Track supply used by starting units
    {
        let mut player_res = world.resource_mut::<PlayerResources>();
        if let Some(pres) = player_res.players.get_mut(player_id as usize) {
            pres.supply += unit_supply_cost * 2;
        }
    }

    // Spawn resource deposits only once (player 0)
    if player_id == 0 {
        for res in &map_def.resources {
            let pos = GridPos::new(res.pos.0, res.pos.1);
            let (resource_type, amount) = match res.kind {
                cc_core::map_format::ResourceKind::FishPond => (ResourceType::Food, 1500),
                cc_core::map_format::ResourceKind::BerryBush => (ResourceType::Food, 800),
                cc_core::map_format::ResourceKind::GpuDeposit => (ResourceType::GpuCores, 500),
                cc_core::map_format::ResourceKind::MonkeyMine => (ResourceType::Nft, 200),
            };
            world.spawn((
                Position {
                    world: WorldPos::from_grid(pos),
                },
                GridCell { pos },
                ResourceDeposit {
                    resource_type,
                    remaining: amount,
                },
            ));
        }
    }
}

/// Spawn a single combat unit with base stats at the given grid position.
pub fn spawn_combat_unit(world: &mut World, grid: GridPos, player_id: u8, kind: UnitKind) -> Entity {
    spawn_base_unit(world, kind, grid, player_id)
}

/// Headless despawn: in the harness there's no client death_fade_system,
/// so we despawn Dead entities immediately after cleanup marks them.
pub fn headless_despawn_system(mut commands: Commands, dead: Query<Entity, With<Dead>>) {
    for entity in dead.iter() {
        commands.entity(entity).despawn();
    }
}

/// Count living (non-Dead) entities per player.
pub(crate) fn count_living_entities(world: &mut World) -> [u32; 2] {
    let mut counts = [0u32; 2];
    for (owner,) in world
        .query_filtered::<(&Owner,), Without<Dead>>()
        .iter(world)
    {
        if (owner.player_id as usize) < 2 {
            counts[owner.player_id as usize] += 1;
        }
    }
    counts
}

/// Check if either player has been eliminated.
///
/// `mutual_elimination_winner` controls the result when both players have
/// zero entities: `Some(id)` gives victory to that player (attacker advantage),
/// `None` signals a draw.
pub fn check_elimination(world: &mut World, mutual_elimination_winner: Option<u8>) -> Option<u8> {
    let counts = count_living_entities(world);
    match (counts[0] > 0, counts[1] > 0) {
        (false, false) => mutual_elimination_winner,
        (false, true) => Some(1),
        (true, false) => Some(0),
        (true, true) => None,
    }
}

/// Determine which player is leading based on living entity count.
pub fn determine_leader(world: &mut World) -> Option<u8> {
    let counts = count_living_entities(world);
    match counts[0].cmp(&counts[1]) {
        std::cmp::Ordering::Greater => Some(0),
        std::cmp::Ordering::Less => Some(1),
        std::cmp::Ordering::Equal => None,
    }
}

/// Generate a MatchReport from a MatchResult.
pub fn generate_report(result: &MatchResult, config: &HarnessConfig) -> report::MatchReport {
    report::MatchReport::from_result(result, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal World with PlayerResources for 2 players.
    fn test_world() -> World {
        let mut world = World::new();
        let mut player_res = PlayerResources::default();
        while player_res.players.len() < 2 {
            player_res.players.push(Default::default());
        }
        world.insert_resource(player_res);
        world
    }

    #[test]
    fn extract_panic_message_from_str() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("test panic");
        assert_eq!(extract_panic_message(payload), "test panic");
    }

    #[test]
    fn extract_panic_message_from_string() {
        let payload: Box<dyn std::any::Any + Send> =
            Box::new(String::from("string panic"));
        assert_eq!(extract_panic_message(payload), "string panic");
    }

    #[test]
    fn extract_panic_message_from_unknown() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42u32);
        assert_eq!(extract_panic_message(payload), "unknown panic");
    }

    #[test]
    fn make_headless_world_inserts_core_resources() {
        let params = cc_core::map_gen::MapGenParams {
            width: 16,
            height: 16,
            seed: 1,
            ..Default::default()
        };
        let map_def = cc_core::map_gen::generate_map(&params);
        let game_map = map_def.to_game_map();

        let (world, spawn_positions) = make_headless_world(game_map, &map_def);

        // Core resources should exist
        assert!(world.get_resource::<CommandQueue>().is_some());
        assert!(world.get_resource::<SimClock>().is_some());
        assert!(world.get_resource::<GameState>().is_some());
        assert!(world.get_resource::<PlayerResources>().is_some());
        assert!(world.get_resource::<MapResource>().is_some());
        assert!(world.get_resource::<SpawnPositions>().is_some());
        assert!(world.get_resource::<CombatStats>().is_some());
        assert!(world.get_resource::<ControlGroups>().is_some());

        // Spawn positions should be extracted
        assert!(!spawn_positions.is_empty());
    }

    #[test]
    fn spawn_combat_unit_creates_entity_with_stats() {
        let mut world = test_world();
        let grid = GridPos::new(5, 5);
        let entity = spawn_combat_unit(&mut world, grid, 0, UnitKind::Chonk);

        let owner = world.get::<Owner>(entity).unwrap();
        assert_eq!(owner.player_id, 0);

        let unit_type = world.get::<UnitType>(entity).unwrap();
        assert_eq!(unit_type.kind, UnitKind::Chonk);

        let health = world.get::<Health>(entity).unwrap();
        let expected = base_stats(UnitKind::Chonk);
        assert_eq!(health.max, expected.health);
        assert_eq!(health.current, expected.health);

        let attack = world.get::<AttackStats>(entity).unwrap();
        assert_eq!(attack.damage, expected.damage);
        assert_eq!(attack.range, expected.range);
    }

    #[test]
    fn count_living_entities_ignores_dead() {
        let mut world = test_world();
        let grid = GridPos::new(5, 5);

        // Spawn 2 for player 0, 1 for player 1
        spawn_combat_unit(&mut world, grid, 0, UnitKind::Chonk);
        let dead_entity = spawn_combat_unit(&mut world, GridPos::new(6, 5), 0, UnitKind::Nuisance);
        spawn_combat_unit(&mut world, GridPos::new(7, 5), 1, UnitKind::Hisser);

        // Mark one as dead
        world.entity_mut(dead_entity).insert(Dead);

        let counts = count_living_entities(&mut world);
        assert_eq!(counts[0], 1); // 2 spawned, 1 dead
        assert_eq!(counts[1], 1);
    }

    #[test]
    fn check_elimination_p0_eliminated() {
        let mut world = test_world();
        // Only player 1 has entities
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 1, UnitKind::Chonk);

        assert_eq!(check_elimination(&mut world, Some(0)), Some(1));
        assert_eq!(check_elimination(&mut world, None), Some(1));
    }

    #[test]
    fn check_elimination_p1_eliminated() {
        let mut world = test_world();
        // Only player 0 has entities
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);

        assert_eq!(check_elimination(&mut world, Some(0)), Some(0));
        assert_eq!(check_elimination(&mut world, None), Some(0));
    }

    #[test]
    fn check_elimination_mutual_with_attacker_advantage() {
        let mut world = test_world();
        // No entities at all
        assert_eq!(check_elimination(&mut world, Some(0)), Some(0));
    }

    #[test]
    fn check_elimination_mutual_draw() {
        let mut world = test_world();
        // No entities at all
        assert_eq!(check_elimination(&mut world, None), None);
    }

    #[test]
    fn check_elimination_both_alive() {
        let mut world = test_world();
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
        spawn_combat_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Hisser);

        assert_eq!(check_elimination(&mut world, Some(0)), None);
        assert_eq!(check_elimination(&mut world, None), None);
    }

    #[test]
    fn determine_leader_p0_leads() {
        let mut world = test_world();
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
        spawn_combat_unit(&mut world, GridPos::new(6, 5), 0, UnitKind::Nuisance);
        spawn_combat_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Hisser);

        assert_eq!(determine_leader(&mut world), Some(0));
    }

    #[test]
    fn determine_leader_p1_leads() {
        let mut world = test_world();
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
        spawn_combat_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Hisser);
        spawn_combat_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

        assert_eq!(determine_leader(&mut world), Some(1));
    }

    #[test]
    fn determine_leader_tie() {
        let mut world = test_world();
        spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
        spawn_combat_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Hisser);

        assert_eq!(determine_leader(&mut world), None);
    }

    #[test]
    fn spawn_starting_entities_creates_hq_and_workers() {
        let params = cc_core::map_gen::MapGenParams {
            width: 16,
            height: 16,
            seed: 1,
            ..Default::default()
        };
        let map_def = cc_core::map_gen::generate_map(&params);
        let game_map = map_def.to_game_map();

        let (mut world, spawn_positions) = make_headless_world(game_map, &map_def);

        if let Some(&(player_id, spawn_pos)) = spawn_positions.first() {
            spawn_starting_entities(
                &mut world,
                player_id,
                spawn_pos,
                Faction::CatGpt,
                &map_def,
            );

            // Should have 1 building (HQ) and 2 workers for this player
            let mut building_count = 0u32;
            let mut unit_count = 0u32;
            for (owner,) in world
                .query_filtered::<(&Owner,), With<Building>>()
                .iter(&world)
            {
                if owner.player_id == player_id {
                    building_count += 1;
                }
            }
            for (owner,) in world
                .query_filtered::<(&Owner,), With<UnitType>>()
                .iter(&world)
            {
                if owner.player_id == player_id {
                    unit_count += 1;
                }
            }
            assert_eq!(building_count, 1, "should have 1 HQ building");
            assert_eq!(unit_count, 2, "should have 2 starting workers");

            // Check resources were granted
            let player_res = world.resource::<PlayerResources>();
            if let Some(pres) = player_res.players.get(player_id as usize) {
                assert!(pres.supply_cap > 0, "supply_cap should be > 0 from HQ");
                assert_eq!(pres.food, 200, "starting food should be 200");
                assert!(pres.supply > 0, "supply used should be > 0 from workers");
            }
        }
    }
}
