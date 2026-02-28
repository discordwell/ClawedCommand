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
use cc_core::unit_stats::base_stats;

use crate::ai::fsm::{AiDifficulty, AiPhase, AiState, BotPersonality};
use crate::ai::MultiAiState;
use crate::resources::{
    CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock,
    SpawnPositions,
};
use crate::systems::{
    cleanup_system::cleanup_system, combat_system::combat_system,
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, production_system::production_system,
    projectile_system::projectile_system, resource_system::gathering_system,
    target_acquisition_system::target_acquisition_system, tick_system::tick_system,
    victory_system::victory_system,
};

use invariants::{InvariantChecker, Severity};
use snapshot::GameStateSnapshot;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a single bot player.
#[derive(Debug, Clone)]
pub struct BotConfig {
    pub player_id: u8,
    pub difficulty: AiDifficulty,
    pub personality: BotPersonality,
}

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
                    personality: BotPersonality::Balanced,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    personality: BotPersonality::Balanced,
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
        let has_fatal = self
            .violations
            .iter()
            .any(|v| matches!(v.severity, Severity::Error | Severity::Fatal));
        let is_error = matches!(self.outcome, MatchOutcome::Error { .. });
        !has_fatal && !is_error
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

        // Broader elimination check
        if let Some(winner) = check_elimination(&mut world) {
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
// Setup helpers
// ---------------------------------------------------------------------------

fn make_harness_sim(
    map: GameMap,
    config: &HarnessConfig,
    map_def: &cc_core::map_format::MapDefinition,
) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(GameState::Playing);

    let mut player_res = PlayerResources::default();
    while player_res.players.len() < 2 {
        player_res.players.push(Default::default());
    }
    world.insert_resource(player_res);
    world.insert_resource(MapResource { map });

    let multi_ai = MultiAiState {
        players: config
            .bots
            .iter()
            .map(|bot| AiState {
                player_id: bot.player_id,
                phase: AiPhase::EarlyGame,
                difficulty: bot.difficulty,
                personality: bot.personality,
                enemy_spawn: None,
                attack_ordered: false,
            })
            .collect(),
    };
    world.insert_resource(multi_ai);
    world.insert_resource(AiState::default());

    let spawn_positions: Vec<(u8, GridPos)> = map_def
        .spawn_points
        .iter()
        .map(|sp| (sp.player, GridPos::new(sp.pos.0, sp.pos.1)))
        .collect();
    world.insert_resource(SpawnPositions {
        positions: spawn_positions.clone(),
    });

    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            crate::ai::fsm::multi_ai_decision_system,
            process_commands,
            production_system,
            gathering_system,
            target_acquisition_system,
            combat_system,
            projectile_system,
            movement_system,
            grid_sync_system,
            cleanup_system,
        )
            .chain(),
    );
    schedule.add_systems(victory_system.after(cleanup_system));

    for (player_id, spawn_pos) in &spawn_positions {
        spawn_starting_entities(&mut world, *player_id, *spawn_pos, map_def);
    }

    (world, schedule)
}

fn spawn_starting_entities(
    world: &mut World,
    player_id: u8,
    spawn_pos: GridPos,
    map_def: &cc_core::map_format::MapDefinition,
) {
    let box_stats = building_stats(BuildingKind::TheBox);
    world.spawn((
        Position {
            world: WorldPos::from_grid(spawn_pos),
        },
        GridCell { pos: spawn_pos },
        Owner { player_id },
        Building {
            kind: BuildingKind::TheBox,
        },
        Health {
            current: box_stats.health,
            max: box_stats.health,
        },
        Producer,
        ProductionQueue::default(),
    ));

    // Grant supply_cap from TheBox and starting resources
    {
        let mut player_res = world.resource_mut::<PlayerResources>();
        if let Some(pres) = player_res.players.get_mut(player_id as usize) {
            pres.supply_cap += box_stats.supply_provided;
            pres.food = 200; // Starting resources
        }
    }

    let unit_supply_cost = base_stats(UnitKind::Pawdler).supply_cost;
    for i in 0..2 {
        let offset = GridPos::new(spawn_pos.x + 1 + i, spawn_pos.y);
        spawn_combat_unit(world, offset, player_id, UnitKind::Pawdler);
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

fn spawn_combat_unit(world: &mut World, grid: GridPos, player_id: u8, kind: UnitKind) -> Entity {
    let stats = base_stats(kind);
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            UnitType { kind },
            Health {
                current: stats.health,
                max: stats.health,
            },
            MovementSpeed { speed: stats.speed },
            AttackStats {
                damage: stats.damage,
                range: stats.range,
                attack_speed: stats.attack_speed,
                cooldown_remaining: 0,
            },
            AttackTypeMarker {
                attack_type: stats.attack_type,
            },
        ))
        .id()
}

fn check_elimination(world: &mut World) -> Option<u8> {
    let mut has_entities = [false; 2];
    for (owner,) in world.query::<(&Owner,)>().iter(world) {
        if (owner.player_id as usize) < 2 {
            has_entities[owner.player_id as usize] = true;
        }
    }
    if !has_entities[0] && !has_entities[1] {
        // Mutual elimination — attacker advantage (player 0 attacked first)
        Some(0)
    } else if !has_entities[0] && has_entities[1] {
        Some(1)
    } else if has_entities[0] && !has_entities[1] {
        Some(0)
    } else {
        None
    }
}

fn determine_leader(world: &mut World) -> Option<u8> {
    let mut counts = [0u32; 2];
    for (owner,) in world.query::<(&Owner,)>().iter(world) {
        if (owner.player_id as usize) < 2 {
            counts[owner.player_id as usize] += 1;
        }
    }
    if counts[0] > counts[1] {
        Some(0)
    } else if counts[1] > counts[0] {
        Some(1)
    } else {
        None
    }
}

/// Generate a MatchReport from a MatchResult.
pub fn generate_report(result: &MatchResult, config: &HarnessConfig) -> report::MatchReport {
    report::MatchReport::from_result(result, config)
}
