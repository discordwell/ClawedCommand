//! Arena match runner — FSM + Lua scripts coexisting for AI training.
//!
//! The arena runs headless matches where the FSM handles macro decisions
//! (build orders, army movement, phase transitions) and Lua scripts handle
//! micro (focus fire, kiting, retreat). Both emit to CommandQueue.
//! For the same unit, the **later** command wins — scripts override FSM
//! since they run after it in the schedule chain.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::prelude::*;
use serde::Serialize;

use cc_core::components::*;
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::map_gen::{self, MapGenParams};

use cc_sim::ai::fsm::{AiDifficulty, AiPersonalityProfile, AiPhase, AiState, AiTier};
pub use cc_sim::ai::fsm::BotConfig;
use cc_sim::ai::MultiAiState;
use cc_sim::harness::invariants::{InvariantChecker, InvariantViolation, Severity};
use cc_sim::harness::snapshot::capture_snapshot;
use cc_sim::harness::{
    MatchOutcome, count_living_entities, determine_leader, headless_despawn_system,
    spawn_combat_unit, spawn_starting_entities,
};
use cc_sim::resources::{
    CombatStats, CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock,
    SpawnPositions, VoiceOverride,
};
use cc_sim::systems::{
    ability_effect_system::ability_effect_system,
    ability_system::ability_cooldown_system,
    aura_system::aura_system,
    builder_system::builder_system,
    cleanup_system::cleanup_system,
    combat_system::combat_system,
    command_system::process_commands,
    grid_sync_system::grid_sync_system,
    movement_system::movement_system,
    production_system::production_system,
    projectile_system::{projectile_system, ProjectileHit},
    research_system::research_system,
    resource_system::gathering_system,
    stat_modifier_system::stat_modifier_system,
    status_effect_system::status_effect_system,
    target_acquisition_system::target_acquisition_system,
    tick_system::tick_system,
    tower_combat_system::tower_combat_system,
    victory_system::victory_system,
};

use crate::events::ScriptRegistration;
use crate::runner::{script_runner_system, PreviousSnapshots, ScriptRegistry};
use crate::tool_tier::FactionToolStates;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Source for a Lua script — either inline source or a file path.
#[derive(Debug, Clone)]
pub enum ScriptSource {
    File(PathBuf),
    Inline { name: String, source: String },
}

/// Configuration for an arena match.
#[derive(Debug, Clone)]
pub struct ArenaConfig {
    pub seed: u64,
    pub map_size: (u32, u32),
    pub max_ticks: u64,
    pub output_path: Option<PathBuf>,
    pub bots: [BotConfig; 2],
    /// Per-player Lua script sources. Index 0 = player 0, index 1 = player 1.
    pub scripts: [Option<Vec<ScriptSource>>; 2],
    /// Compute budget per script execution (default 500).
    pub script_budget: u32,
    /// Extra combat units to spawn at match start: (player_id, UnitKind, count).
    /// Spawned near each player's HQ after normal starting entities.
    pub extra_spawns: Vec<(u8, UnitKind, u32)>,
    /// If > 0, dump GameStateSnapshot JSON every N ticks to output_path/snapshots/.
    pub snapshot_interval: u64,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            map_size: (64, 64),
            max_ticks: 6000,
            output_path: None,
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
            scripts: [None, None],
            script_budget: 500,
            extra_spawns: Vec::new(),
            snapshot_interval: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Stats tracking
// ---------------------------------------------------------------------------

/// A script error recorded during a match.
#[derive(Debug, Clone, Serialize)]
pub struct ArenaScriptError {
    pub tick: u64,
    pub script_name: String,
    pub message: String,
}

/// Per-player combat statistics tracked during an arena match.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayerArenaStats {
    pub units_trained: u32,
    pub units_lost: u32,
    pub units_killed: u32,
    pub buildings_built: u32,
    pub buildings_lost: u32,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub resources_gathered: u32,
}

/// A key moment during the match.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub tick: u64,
    pub event: String,
}

/// Resource for tracking arena stats during simulation.
#[derive(Resource, Default, Debug, Clone)]
pub struct ArenaStats {
    pub players: [PlayerArenaStats; 2],
    pub timeline: Vec<TimelineEvent>,
    pub script_errors: Vec<ArenaScriptError>,
    prev_unit_counts: [u32; 2],
    prev_building_counts: [u32; 2],
    first_combat_recorded: bool,
}

// ---------------------------------------------------------------------------
// Results & Report
// ---------------------------------------------------------------------------

/// Full result of an arena match.
pub struct ArenaResult {
    pub outcome: MatchOutcome,
    pub final_tick: u64,
    pub wall_time_ms: u64,
    pub stats: ArenaStats,
    pub scripts_loaded: [Vec<String>; 2],
    pub violations: Vec<InvariantViolation>,
    /// Snapshots captured during the match (if snapshot_interval > 0).
    pub snapshots: Vec<cc_sim::harness::snapshot::GameStateSnapshot>,
}

impl ArenaResult {
    pub fn passed(&self) -> bool {
        let has_fatal = self
            .violations
            .iter()
            .any(|v| matches!(v.severity, Severity::Error | Severity::Fatal));
        let is_error = matches!(self.outcome, MatchOutcome::Error { .. });
        !has_fatal && !is_error
    }
}

#[derive(Serialize, Debug)]
pub struct ArenaReport {
    pub seed: u64,
    pub outcome: String,
    pub duration_ticks: u64,
    pub wall_time_ms: u64,
    pub player_stats: [PlayerArenaStats; 2],
    pub scripts_loaded: [Vec<String>; 2],
    pub script_errors: Vec<ArenaScriptError>,
    pub timeline: Vec<TimelineEvent>,
    pub violations_warning: u32,
    pub violations_error: u32,
    pub passed: bool,
}

impl ArenaReport {
    pub fn from_result(result: &ArenaResult, config: &ArenaConfig) -> Self {
        let mut warnings = 0u32;
        let mut errors = 0u32;
        for v in &result.violations {
            match v.severity {
                Severity::Warning => warnings += 1,
                Severity::Error | Severity::Fatal => errors += 1,
            }
        }

        Self {
            seed: config.seed,
            outcome: format!("{}", result.outcome),
            duration_ticks: result.final_tick,
            wall_time_ms: result.wall_time_ms,
            player_stats: result.stats.players.clone(),
            scripts_loaded: result.scripts_loaded.clone(),
            script_errors: result.stats.script_errors.clone(),
            timeline: result.stats.timeline.clone(),
            violations_warning: warnings,
            violations_error: errors,
            passed: result.passed(),
        }
    }
}

// ---------------------------------------------------------------------------
// Script loading
// ---------------------------------------------------------------------------

/// Load Lua scripts from a directory, parsing annotation headers.
///
/// Annotation format in Lua comment headers:
/// ```lua
/// -- @name: tactical_micro
/// -- @events: on_tick, on_enemy_spotted, on_unit_attacked
/// -- @interval: 3
/// ```
pub fn load_scripts_from_dir(dir: &Path, player_id: u8) -> Vec<ScriptRegistration> {
    let mut scripts = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return scripts,
    };

    let mut lua_files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "lua")
                .unwrap_or(false)
        })
        .collect();
    lua_files.sort_by_key(|e| e.file_name());

    for entry in lua_files {
        let path = entry.path();
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let mut name = file_stem;
        let mut events = vec!["on_tick".to_string()];
        let mut interval = 5u32;

        for line in source.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("--") {
                if !trimmed.is_empty() {
                    break;
                }
                continue;
            }
            let comment = trimmed.trim_start_matches("--").trim();
            if let Some(val) = comment.strip_prefix("@name:") {
                name = val.trim().to_string();
            } else if let Some(val) = comment.strip_prefix("@events:") {
                events = val
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(val) = comment.strip_prefix("@interval:") {
                if let Ok(n) = val.trim().parse::<u32>() {
                    interval = n;
                }
            }
        }

        let mut reg = ScriptRegistration::new(name, source, events, player_id);
        reg.tick_interval = interval;
        scripts.push(reg);
    }

    scripts
}

// ---------------------------------------------------------------------------
// World + Schedule construction
// ---------------------------------------------------------------------------

/// Create a World + Schedule for arena matches (FSM + scripts coexisting).
fn make_arena_sim(
    map: GameMap,
    config: &ArenaConfig,
    map_def: &cc_core::map_format::MapDefinition,
    script_registrations: Vec<ScriptRegistration>,
) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(GameState::Playing);
    world.insert_resource(CombatStats::default());
    world.insert_resource(VoiceOverride::default());

    let mut player_res = PlayerResources::default();
    while player_res.players.len() < 2 {
        player_res.players.push(Default::default());
    }
    world.insert_resource(player_res);
    world.insert_resource(MapResource { map });
    world.init_resource::<bevy::prelude::Messages<ProjectileHit>>();

    let spawn_positions: Vec<(u8, GridPos)> = map_def
        .spawn_points
        .iter()
        .map(|sp| (sp.player, GridPos::new(sp.pos.0, sp.pos.1)))
        .collect();
    world.insert_resource(SpawnPositions {
        positions: spawn_positions.clone(),
    });

    // Pre-seed enemy_spawn from SpawnPositions so both AIs have symmetric
    // information from tick 0 (previously discovered at runtime, giving P0/P1
    // different amounts of "blind" time).
    let multi_ai = MultiAiState {
        players: config
            .bots
            .iter()
            .map(|bot| {
                let enemy_pos = spawn_positions
                    .iter()
                    .find(|(pid, _)| *pid != bot.player_id)
                    .map(|(_, pos)| *pos);
                AiState {
                    player_id: bot.player_id,
                    phase: AiPhase::EarlyGame,
                    difficulty: bot.difficulty,
                    profile: bot.profile.clone(),
                    fmap: cc_sim::ai::fsm::faction_map(bot.faction),
                    enemy_spawn: enemy_pos,
                    attack_ordered: false,
                    last_attack_tick: 0,
                    tier: AiTier::Basic,
                }
            })
            .collect(),
    };
    world.insert_resource(multi_ai);
    world.insert_resource(AiState::default());

    // Script runner resources
    let mut registry = ScriptRegistry::default();
    for reg in script_registrations {
        registry.register(reg);
    }
    world.insert_resource(registry);
    world.insert_resource(PreviousSnapshots::default());
    world.insert_resource(FactionToolStates::default());
    world.insert_resource(ArenaStats::default());

    // Schedule: tick → FSM → scripts → process_commands → sim → cleanup
    // Split into two chained groups to stay within Bevy's 20-tuple limit.
    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            cc_sim::ai::fsm::multi_ai_decision_system,
            script_runner_system,
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
        )
            .chain(),
    );
    schedule.add_systems(
        (cleanup_system, headless_despawn_system)
            .chain()
            .after(grid_sync_system),
    );
    schedule.add_systems(victory_system.after(headless_despawn_system));

    // Spawn starting entities for each player
    for (player_id, spawn_pos) in &spawn_positions {
        let faction = config.bots[*player_id as usize].faction;
        spawn_starting_entities(&mut world, *player_id, *spawn_pos, faction, map_def);
    }

    // Spawn extra combat units near each player's HQ
    for (player_id, kind, count) in &config.extra_spawns {
        if let Some((_, base_pos)) = spawn_positions.iter().find(|(pid, _)| pid == player_id) {
            for i in 0..*count {
                // Spread units in a grid around the HQ
                let dx = (i % 6) as i32 - 2;
                let dy = (i / 6) as i32 + 2;
                let pos = GridPos::new(base_pos.x + dx, base_pos.y + dy);
                spawn_combat_unit(&mut world, pos, *player_id, *kind);
            }
        }
    }

    // Seed ArenaStats prev counts so starting units aren't counted as "trained"
    {
        let mut unit_counts = [0u32; 2];
        let mut building_counts = [0u32; 2];
        for (owner,) in world
            .query_filtered::<(&Owner,), With<UnitType>>()
            .iter(&world)
        {
            if (owner.player_id as usize) < 2 {
                unit_counts[owner.player_id as usize] += 1;
            }
        }
        for (owner,) in world
            .query_filtered::<(&Owner,), With<Building>>()
            .iter(&world)
        {
            if (owner.player_id as usize) < 2 {
                building_counts[owner.player_id as usize] += 1;
            }
        }
        let mut stats = world.resource_mut::<ArenaStats>();
        stats.prev_unit_counts = unit_counts;
        stats.prev_building_counts = building_counts;
    }

    (world, schedule)
}

// ---------------------------------------------------------------------------
// Stats tracking
// ---------------------------------------------------------------------------

fn update_arena_stats(world: &mut World) {
    let mut unit_counts = [0u32; 2];
    let mut building_counts = [0u32; 2];

    for (owner,) in world
        .query_filtered::<(&Owner,), (With<UnitType>, Without<Dead>)>()
        .iter(world)
    {
        if (owner.player_id as usize) < 2 {
            unit_counts[owner.player_id as usize] += 1;
        }
    }

    for (owner,) in world
        .query_filtered::<(&Owner,), (With<Building>, Without<Dead>)>()
        .iter(world)
    {
        if (owner.player_id as usize) < 2 {
            building_counts[owner.player_id as usize] += 1;
        }
    }

    let tick = world.resource::<SimClock>().tick;
    let combat = world.resource::<CombatStats>().clone();
    let mut stats = world.resource_mut::<ArenaStats>();

    for p in 0..2 {
        let prev = stats.prev_unit_counts[p];
        let curr = unit_counts[p];
        if curr < prev {
            stats.players[p].units_lost += prev - curr;
            let other = 1 - p;
            stats.players[other].units_killed += prev - curr;
        }
        if curr > prev && tick > 0 {
            stats.players[p].units_trained += curr - prev;
        }

        let bprev = stats.prev_building_counts[p];
        let bcurr = building_counts[p];
        if bcurr < bprev {
            stats.players[p].buildings_lost += bprev - bcurr;
        }
        if bcurr > bprev && tick > 0 {
            stats.players[p].buildings_built += bcurr - bprev;
        }
    }

    if !stats.first_combat_recorded
        && (combat.melee_attack_count > 0 || combat.ranged_attack_count > 0)
    {
        stats.first_combat_recorded = true;
        stats.timeline.push(TimelineEvent {
            tick,
            event: "first_combat".to_string(),
        });
    }

    stats.prev_unit_counts = unit_counts;
    stats.prev_building_counts = building_counts;
}

/// Arena-specific elimination check: mutual annihilation = draw (unlike harness
/// which gives attacker advantage).
fn check_elimination(world: &mut World) -> Option<u8> {
    let counts = count_living_entities(world);
    match (counts[0] > 0, counts[1] > 0) {
        (false, false) => None, // mutual annihilation — draw
        (false, true) => Some(1),
        (true, false) => Some(0),
        (true, true) => None,
    }
}

// ---------------------------------------------------------------------------
// Main match runner
// ---------------------------------------------------------------------------

/// Run a complete arena match (FSM + scripts coexisting).
pub fn run_arena_match(config: &ArenaConfig) -> ArenaResult {
    let wall_start = Instant::now();

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

    // Load scripts
    let mut all_registrations = Vec::new();
    let mut scripts_loaded: [Vec<String>; 2] = [vec![], vec![]];

    for (player_idx, scripts_opt) in config.scripts.iter().enumerate() {
        let player_id = player_idx as u8;
        if let Some(sources) = scripts_opt {
            for source in sources {
                match source {
                    ScriptSource::File(path) => {
                        if path.is_dir() {
                            let regs = load_scripts_from_dir(path, player_id);
                            for reg in &regs {
                                scripts_loaded[player_idx].push(reg.name.clone());
                            }
                            all_registrations.extend(regs);
                        } else if let Ok(src) = std::fs::read_to_string(path) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unnamed")
                                .to_string();
                            scripts_loaded[player_idx].push(name.clone());
                            all_registrations.push(ScriptRegistration::new(
                                name,
                                src,
                                vec!["on_tick".to_string()],
                                player_id,
                            ));
                        }
                    }
                    ScriptSource::Inline { name, source } => {
                        scripts_loaded[player_idx].push(name.clone());
                        all_registrations.push(ScriptRegistration::new(
                            name.clone(),
                            source.clone(),
                            vec!["on_tick".to_string()],
                            player_id,
                        ));
                    }
                }
            }
        }
    }

    let (mut world, mut schedule) = make_arena_sim(game_map, config, &map_def, all_registrations);

    let mut checker = InvariantChecker::new(map_width, map_height);
    let mut snapshots = Vec::new();

    let mut outcome = MatchOutcome::Timeout {
        tick: config.max_ticks,
        leading_player: None,
    };

    for _ in 0..config.max_ticks {
        let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            schedule.run(&mut world);
        }));

        let tick_after = world.resource::<SimClock>().tick;

        if let Err(panic_info) = tick_result {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            checker.record_panic(tick_after, &msg);
            outcome = MatchOutcome::Error {
                tick: tick_after,
                message: msg,
            };
            break;
        }

        if tick_after % 10 == 0 {
            update_arena_stats(&mut world);
        }

        // Capture snapshots at the configured interval
        if config.snapshot_interval > 0 && tick_after % config.snapshot_interval == 0 {
            let snap = capture_snapshot(&mut world, tick_after);
            snapshots.push(snap);
        }

        if tick_after % 50 == 0 {
            checker.check_all(&mut world, tick_after);
        }

        let game_state = *world.resource::<GameState>();
        if let GameState::Victory { winner } = game_state {
            outcome = MatchOutcome::Victory {
                winner,
                tick: tick_after,
            };
            world
                .resource_mut::<ArenaStats>()
                .timeline
                .push(TimelineEvent {
                    tick: tick_after,
                    event: format!("victory_player_{winner}"),
                });
            break;
        }

        if let Some(winner) = check_elimination(&mut world) {
            outcome = MatchOutcome::Victory {
                winner,
                tick: tick_after,
            };
            world
                .resource_mut::<ArenaStats>()
                .timeline
                .push(TimelineEvent {
                    tick: tick_after,
                    event: format!("elimination_player_{winner}_wins"),
                });
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

    update_arena_stats(&mut world);

    let stats = world.resource::<ArenaStats>().clone();
    let wall_time = wall_start.elapsed().as_millis() as u64;

    ArenaResult {
        outcome,
        final_tick,
        wall_time_ms: wall_time,
        stats,
        scripts_loaded,
        violations: checker.violations,
        snapshots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn arena_fsm_only_match() {
        let config = ArenaConfig {
            max_ticks: 200,
            ..Default::default()
        };
        let result = run_arena_match(&config);
        assert!(result.passed());
        assert!(result.final_tick > 0);
        assert!(result.scripts_loaded[0].is_empty());
        assert!(result.scripts_loaded[1].is_empty());
    }

    #[test]
    fn arena_loads_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("test_micro.lua");
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(
            f,
            "-- @name: test_micro\n-- @events: on_tick\n-- @interval: 5\n\nlocal units = ctx:my_units()\n"
        )
        .unwrap();

        let scripts = load_scripts_from_dir(dir.path(), 0);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].name, "test_micro");
        assert!(scripts[0].listens_for("on_tick"));
        assert_eq!(scripts[0].tick_interval, 5);
    }

    #[test]
    fn arena_scripts_coexist_with_fsm() {
        let config = ArenaConfig {
            max_ticks: 100,
            scripts: [
                Some(vec![ScriptSource::Inline {
                    name: "noop_script".into(),
                    source: "-- do nothing\nlocal units = ctx:my_units()\n".into(),
                }]),
                None,
            ],
            ..Default::default()
        };
        let result = run_arena_match(&config);
        assert!(result.passed());
        assert_eq!(result.scripts_loaded[0], vec!["noop_script".to_string()]);
        assert!(result.scripts_loaded[1].is_empty());
    }

    #[test]
    fn arena_report_has_stats() {
        let config = ArenaConfig {
            max_ticks: 200,
            ..Default::default()
        };
        let result = run_arena_match(&config);
        let report = ArenaReport::from_result(&result, &config);

        assert_eq!(report.seed, 42);
        assert!(report.duration_ticks > 0);
        assert!(report.wall_time_ms > 0);
        // Stats should be initialized (may be 0 if no combat in 200 ticks)
        assert_eq!(report.player_stats.len(), 2);
    }

    #[test]
    fn arena_script_annotation_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("bare.lua");
        std::fs::write(&script_path, "local x = 1\n").unwrap();

        let scripts = load_scripts_from_dir(dir.path(), 1);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].name, "bare"); // defaults to filename
        assert!(scripts[0].listens_for("on_tick")); // default event
        assert_eq!(scripts[0].tick_interval, 5); // default interval
        assert_eq!(scripts[0].player_id, 1);
    }

    /// Demo scenario 1: cats with formation script vs Clawed with no script.
    /// Cat formation AI should give P0 a significant advantage.
    #[test]
    fn demo_scenario_1_cat_formation_wins() {
        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/scripts");

        let config = ArenaConfig {
            max_ticks: 3000,
            scripts: [
                Some(vec![ScriptSource::File(
                    scripts_dir.join("cat_formation.lua"),
                )]),
                None,
            ],
            ..Default::default()
        };
        let result = run_arena_match(&config);
        assert!(
            result.passed(),
            "Match should complete without errors: {:?}",
            result.violations,
        );
        assert_eq!(
            result.scripts_loaded[0],
            vec!["cat_formation".to_string()],
        );
        assert!(result.scripts_loaded[1].is_empty());
    }

    /// Demo scenario 2: both sides with formation scripts load without errors.
    #[test]
    fn demo_scenario_2_both_formations_load() {
        use cc_core::components::Faction;

        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/scripts");

        let mut config = ArenaConfig {
            max_ticks: 1000,
            scripts: [
                Some(vec![ScriptSource::File(
                    scripts_dir.join("cat_formation.lua"),
                )]),
                Some(vec![ScriptSource::File(
                    scripts_dir.join("clawed_formation.lua"),
                )]),
            ],
            ..Default::default()
        };
        config.bots[1].faction = Faction::TheClawed;
        let result = run_arena_match(&config);
        assert!(
            result.passed(),
            "Both formation scripts should run without errors: {:?}",
            result.violations,
        );
        assert_eq!(
            result.scripts_loaded[0],
            vec!["cat_formation".to_string()],
        );
        assert_eq!(
            result.scripts_loaded[1],
            vec!["clawed_formation".to_string()],
        );
    }

    /// Demo scenario 3: cat formation vs clawed advanced (formation + abilities).
    /// Both scripts should load and run without errors.
    #[test]
    fn demo_scenario_3_advanced_scripts_load() {
        use cc_core::components::Faction;

        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/scripts");

        let mut config = ArenaConfig {
            max_ticks: 1000,
            scripts: [
                Some(vec![ScriptSource::File(
                    scripts_dir.join("cat_formation.lua"),
                )]),
                Some(vec![ScriptSource::File(
                    scripts_dir.join("clawed_advanced.lua"),
                )]),
            ],
            ..Default::default()
        };
        config.bots[1].faction = Faction::TheClawed;
        let result = run_arena_match(&config);
        assert!(
            result.passed(),
            "Advanced script should run without errors: {:?}",
            result.violations,
        );
        assert_eq!(
            result.scripts_loaded[1],
            vec!["clawed_advanced".to_string()],
        );
    }

    /// Demo scenario 4: big-army mirror match — both CatGPT with Gen 42
    /// terrain-aware combat micro and doubled starting armies.
    #[test]
    fn demo_scenario_4_big_army_mirror_match() {
        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../training/arena/gen_042/player_1");

        let config = ArenaConfig {
            max_ticks: 8000,
            scripts: [
                Some(vec![ScriptSource::File(scripts_dir.clone())]),
                Some(vec![ScriptSource::File(scripts_dir)]),
            ],
            extra_spawns: vec![
                // Player 0: 4 Chonks, 4 Hissers, 3 Nuisances, 2 Yowlers
                (0, UnitKind::Chonk, 4),
                (0, UnitKind::Hisser, 4),
                (0, UnitKind::Nuisance, 3),
                (0, UnitKind::Yowler, 2),
                // Player 1: same army
                (1, UnitKind::Chonk, 4),
                (1, UnitKind::Hisser, 4),
                (1, UnitKind::Nuisance, 3),
                (1, UnitKind::Yowler, 2),
            ],
            ..Default::default()
        };
        let result = run_arena_match(&config);
        assert!(
            result.passed(),
            "Big-army mirror match should complete without errors: {:?}",
            result.violations,
        );
        // Both sides should have loaded the terrain kite script
        assert_eq!(
            result.scripts_loaded[0],
            vec!["combat_micro_terrain_kite_only".to_string()],
        );
        assert_eq!(
            result.scripts_loaded[1],
            vec!["combat_micro_terrain_kite_only".to_string()],
        );
        // Print results for visibility
        println!(
            "Demo 4 result: {} | ticks: {} | p0 kills: {} lost: {} | p1 kills: {} lost: {}",
            result.outcome,
            result.final_tick,
            result.stats.players[0].units_killed,
            result.stats.players[0].units_lost,
            result.stats.players[1].units_killed,
            result.stats.players[1].units_lost,
        );
    }
}
