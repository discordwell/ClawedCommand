//! Headless validator for the Strait Prelude.
//!
//! Runs the actual `cc_client::dream_strait` simulation systems against
//! `assets/campaign/dream_strait_prelude.ron` with no window, no input,
//! and no rendering. Reports drone counts at intervals and asserts that
//! the failure trigger fires within a tick budget.
//!
//!     cargo run --bin prelude_harness
//!
//! Exits 0 on success, panics with diagnostics on failure.

use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::ecs::message::Messages;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use cc_core::mission::MissionDefinition;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};
use cc_sim::campaign::triggers::DialogueEvent;

use cc_client::dream_strait::{
    StraitMode, StraitState, register_strait_resources, register_strait_simulation_systems,
};

/// Hard ceiling on ticks — at 60 Hz this is ~166 seconds of game time.
/// If the prelude doesn't fail by this point, the AA tuning or allied AI
/// is broken and the harness should fail.
const MAX_TICKS: u64 = 10_000;

/// How often to print a heartbeat line.
const REPORT_INTERVAL: u64 = 300;

/// Failure trigger from `strait_check_win_lose`. Mirrors the constant the
/// engine uses (`drones_alive <= 5`).
const FAILURE_THRESHOLD_ALIVE: u32 = 5;

/// First-loss dialog should fire by this tick at the latest. Allies start
/// pushing at PRELUDE_PUSH_TICK = 900; first contact ~5s after that.
const FIRST_LOSS_DEADLINE: u64 = 1_500;

fn main() {
    let mut app = App::new();

    // Bare-minimum ECS plumbing for headless. No window, no rendering, no
    // input devices — just Time, FrameCount, and asset infrastructure.
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.init_asset::<ColorMaterial>();
    app.init_asset::<Image>();

    // Message channels written by `strait_prelude_dialog_system`. Without
    // this resource the system would panic on first write.
    app.init_resource::<Messages<DialogueEvent>>();

    // `strait_init_system` writes to `ClearColor` to set the DEFCON
    // background. ClearColor is normally inserted by Bevy's window/render
    // plugins; we don't have those, so insert a stub.
    app.insert_resource(ClearColor(Color::BLACK));

    // Critical for headless: without a render loop driving wall-clock
    // time, `Time::delta_secs()` is ~0 and any system that scales speed
    // by dt (like `strait_enemy_aa`) effectively freezes. Pin Bevy's
    // time advancement to a fixed 1/60s per `app.update()` so the engine
    // behaves the same as the live game.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_micros(
        16_667,
    )));

    // Strait simulation only — explicitly NOT input or visual systems.
    register_strait_resources(&mut app);
    register_strait_simulation_systems(&mut app);

    // Load the prelude mission RON and force the campaign into InMission
    // so the strait run conditions activate immediately. The harness
    // doesn't simulate the briefing/debrief flow.
    let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/campaign/dream_strait_prelude.ron");
    let ron_str = std::fs::read_to_string(&ron_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", ron_path.display()));
    let mission: MissionDefinition =
        ron::from_str(&ron_str).expect("dream_strait_prelude.ron failed to parse");

    let mut campaign = CampaignState::default();
    campaign.load_mission(mission);
    campaign.phase = CampaignPhase::InMission;
    app.insert_resource(campaign);

    println!("=== Strait Prelude headless validation ===");
    println!("max ticks: {MAX_TICKS} (~{:.0}s @ 60Hz)", MAX_TICKS as f32 / 60.0);
    println!();

    let mut last_alive: i32 = -1;
    let mut first_kill_tick: Option<u64> = None;

    for tick in 0..MAX_TICKS {
        app.update();

        let state = app.world().resource::<StraitState>();
        let alive = state.drones_alive as i32;

        // Track the first allied loss for the FIRST_LOSS_DEADLINE assertion.
        if first_kill_tick.is_none() && alive < 20 && state.initialized {
            first_kill_tick = Some(tick);
        }

        // Heartbeat: at every REPORT_INTERVAL or whenever the count changes.
        if (tick % REPORT_INTERVAL == 0 || alive != last_alive) && state.initialized {
            println!(
                "  tick {tick:>5}: alive={alive:>2}, mode={:?}, complete={}",
                state.mode, state.mission_complete
            );
            last_alive = alive;
        }

        if state.mission_complete {
            println!();
            println!("MISSION COMPLETE");
            println!("  ticks elapsed: {tick}");
            println!("  final drones_alive: {}", state.drones_alive);
            println!("  mode: {:?}", state.mode);
            println!(
                "  first ally loss: {}",
                first_kill_tick
                    .map(|t| format!("tick {t}"))
                    .unwrap_or_else(|| "<never>".to_string())
            );

            // Assertions ------------------------------------------------
            assert_eq!(
                state.mode,
                StraitMode::Prelude,
                "expected Prelude mode, got {:?}",
                state.mode
            );
            assert!(
                state.drones_alive <= FAILURE_THRESHOLD_ALIVE,
                "expected drones_alive <= {FAILURE_THRESHOLD_ALIVE}, got {}",
                state.drones_alive
            );
            assert!(
                first_kill_tick.is_some_and(|t| t <= FIRST_LOSS_DEADLINE),
                "expected first ally loss by tick {FIRST_LOSS_DEADLINE}, got {first_kill_tick:?}"
            );

            println!();
            println!("PASS");
            return;
        }
    }

    let state = app.world().resource::<StraitState>();
    panic!(
        "Prelude FAILED to complete within {MAX_TICKS} ticks (final alive={}, mode={:?})",
        state.drones_alive, state.mode
    );
}
