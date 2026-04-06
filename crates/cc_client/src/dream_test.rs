//! Dream sequence test driver — programmatic input + Bevy screenshots.
//!
//! Activated by `--demo dream-test`. Drives the dream office scene through
//! a scripted sequence without requiring keyboard focus:
//! 1. Wait for dialogue to finish (or advance it)
//! 2. Move Kell to each interaction point
//! 3. Press F (interact) at each location
//! 4. Capture screenshots at key moments
//! 5. Run through several work sessions
//!
//! All input is injected via the ECS command queue. Screenshots use
//! Bevy's internal `Screenshot::primary_window()`.

use std::path::PathBuf;

use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{HeroIdentity, Owner, Position};
use cc_core::coords::GridPos;
use cc_core::hero::HeroId;
use cc_sim::resources::{CommandQueue, SimClock};

use crate::dream::{DreamOfficeState, OfficeAction, OfficePhase, kell_refusal};
use crate::ui::dialogue::DialogueState;

/// Output directory for test screenshots.
const TEST_OUTPUT_DIR: &str = "wet_tests/dream";

/// Scripted action for the test driver.
#[derive(Debug, Clone)]
enum TestAction {
    /// Wait for dialogue to finish, advancing it each tick.
    AdvanceDialogue,
    /// Take a screenshot with a label.
    Screenshot(&'static str),
    /// Move Kell to a grid position and wait until he arrives (within radius).
    MoveToAndWait(GridPos),
    /// Wait N real seconds.
    WaitSecs(f32),
    /// Simulate pressing F (trigger nearby interaction).
    PressF,
    /// Wait for the dream FSM to return to FreeRoam.
    WaitForFreeRoam,
    /// Log a message.
    Log(&'static str),
    /// Exit the app (test complete).
    Exit,
}

/// Test driver resource — holds the script and execution state.
#[derive(Resource)]
pub struct DreamTestDriver {
    script: Vec<TestAction>,
    current: usize,
    wait_timer: f32,
    screenshot_counter: u32,
    output_dir: PathBuf,
    /// Whether we've started (wait for first frame to render).
    started: bool,
    start_delay: f32,
    /// Grid position we're waiting for Kell to reach.
    move_target: Option<GridPos>,
    /// Timeout for movement (bail after this many seconds).
    move_timeout: f32,
}

impl Default for DreamTestDriver {
    fn default() -> Self {
        Self {
            script: build_test_script(),
            current: 0,
            wait_timer: 0.0,
            screenshot_counter: 0,
            output_dir: PathBuf::from(TEST_OUTPUT_DIR),
            started: false,
            start_delay: 2.0,
            move_target: None,
            move_timeout: 0.0,
        }
    }
}

/// Build the test script for the dream office sequence.
fn build_test_script() -> Vec<TestAction> {
    vec![
        TestAction::Log("=== Dream Test: starting ==="),
        TestAction::Screenshot("01_initial_load"),
        TestAction::AdvanceDialogue,
        TestAction::Screenshot("02_post_dialogue"),
        // === Work at desk (enabled action) ===
        TestAction::Log("Walking to desk"),
        TestAction::MoveToAndWait(GridPos::new(35, 24)),
        TestAction::Screenshot("03_at_desk"),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("04_after_work"),
        // === Call Ada — "I miss her, but I can't afford the distraction" ===
        TestAction::Log("Walking to phone (Call Ada)"),
        TestAction::MoveToAndWait(GridPos::new(24, 5)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("05_call_ada_refusal"),
        TestAction::WaitSecs(2.5),
        // === Sleep — "Plenty of time when I'm dead." ===
        TestAction::Log("Walking to barracks (Sleep)"),
        TestAction::MoveToAndWait(GridPos::new(55, 41)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("06_sleep_refusal"),
        TestAction::WaitSecs(2.5),
        // === Eat — "I'm not hungry" ===
        TestAction::Log("Walking to mess hall (Eat)"),
        TestAction::MoveToAndWait(GridPos::new(40, 41)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("07_eat_refusal"),
        TestAction::WaitSecs(2.5),
        // === Talk — "Not interested." ===
        TestAction::Log("Walking to break room (Talk)"),
        TestAction::MoveToAndWait(GridPos::new(16, 12)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("08_talk_refusal"),
        TestAction::WaitSecs(2.5),
        // === Storage/Armory — "My code is my weapon" ===
        TestAction::Log("Walking to armory (Storage)"),
        TestAction::MoveToAndWait(GridPos::new(55, 12)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("09_armory_refusal"),
        TestAction::WaitSecs(2.5),
        // === Water Fountain — "I'm not thirsty I just need the caffeine" ===
        TestAction::Log("Walking to water fountain"),
        TestAction::MoveToAndWait(GridPos::new(30, 9)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("10_water_refusal"),
        TestAction::WaitSecs(2.5),
        // === Medical Bay — "I'm fine" ===
        TestAction::Log("Walking to medical bay"),
        TestAction::MoveToAndWait(GridPos::new(28, 12)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("11_medical_refusal"),
        TestAction::WaitSecs(2.5),
        // === Photo Wall — first visit: "My unit." ===
        TestAction::Log("Walking to photo wall (first visit)"),
        TestAction::MoveToAndWait(GridPos::new(20, 9)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("12_photo_first"),
        TestAction::WaitSecs(2.5),
        // === Photo Wall — second visit: "I feel like I should feel something..." ===
        TestAction::Log("Photo wall again (second visit)"),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("13_photo_second"),
        TestAction::WaitSecs(2.5),
        // === Guard Post — time-based (at whatever hour we're at) ===
        TestAction::Log("Walking to guard post"),
        TestAction::MoveToAndWait(GridPos::new(15, 4)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("14_guard_refusal"),
        TestAction::WaitSecs(2.5),
        // === CO's Office — time-based ===
        TestAction::Log("Walking to CO's office"),
        TestAction::MoveToAndWait(GridPos::new(55, 5)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("15_co_refusal"),
        TestAction::WaitSecs(2.5),
        // === Leave Base (unchanged, sanity check) ===
        TestAction::Log("Walking to parking lot (Leave Base)"),
        TestAction::MoveToAndWait(GridPos::new(5, 24)),
        TestAction::PressF,
        TestAction::WaitSecs(1.0),
        TestAction::Screenshot("16_leave_refusal"),
        TestAction::WaitSecs(2.5),
        // === Gym (enabled action, exercise) ===
        TestAction::Log("Walking to gym (Work Out)"),
        TestAction::MoveToAndWait(GridPos::new(17, 41)),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("17_after_workout"),
        // === Final work session ===
        TestAction::Log("Back to desk"),
        TestAction::MoveToAndWait(GridPos::new(35, 24)),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("18_final"),
        TestAction::Log("=== Dream Test: complete ==="),
        TestAction::Exit,
    ]
}

/// Test driver system — executes one scripted action per frame when ready.
pub fn dream_test_driver_system(
    mut commands: Commands,
    time: Res<Time>,
    clock: Option<Res<SimClock>>,
    mut driver: ResMut<DreamTestDriver>,
    mut dream: ResMut<DreamOfficeState>,
    dialogue: Res<DialogueState>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    heroes: Query<(Entity, &HeroIdentity, &Owner, &Position)>,
) {
    // Wait for initial render
    if !driver.started {
        driver.start_delay -= time.delta_secs();
        if driver.start_delay > 0.0 {
            return;
        }
        driver.started = true;
        let _ = std::fs::create_dir_all(&driver.output_dir);
        eprintln!("[dream-test] Driver started, {} actions queued", driver.script.len());
    }

    // Check if script is done
    if driver.current >= driver.script.len() {
        if driver.wait_timer <= 0.0 {
            eprintln!("[dream-test] Exiting.");
            std::process::exit(0);
        }
        driver.wait_timer -= time.delta_secs();
        return;
    }

    // Handle wait timer
    if driver.wait_timer > 0.0 {
        driver.wait_timer -= time.delta_secs();
        if driver.wait_timer > 0.0 {
            return;
        }
    }

    let action = driver.script[driver.current].clone();

    match action {
        TestAction::Log(msg) => {
            let tick = clock.as_ref().map(|c| c.tick).unwrap_or(0);
            eprintln!("[dream-test] tick={tick} {msg}");
            driver.current += 1;
        }

        TestAction::Screenshot(label) => {
            let n = driver.screenshot_counter;
            driver.screenshot_counter += 1;
            let path = driver.output_dir.join(format!("{label}.png"));
            eprintln!("[dream-test] Screenshot: {}", path.display());

            #[cfg(not(target_arch = "wasm32"))]
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));

            driver.current += 1;
        }

        TestAction::AdvanceDialogue => {
            if dialogue.active {
                // Simulate space press by completing the current line instantly
                // The dialogue system checks chars_revealed vs total — we can't
                // directly mutate DialogueState here (it's Res not ResMut).
                // Instead, wait for the dialogue system to finish naturally,
                // or advance the dream FSM past OpeningDialogue phase.
                if dream.phase == OfficePhase::OpeningDialogue {
                    // Force transition past dialogue
                    dream.phase = OfficePhase::FreeRoam;
                    dream.action_timer = 0.0;
                    eprintln!("[dream-test] Forced past OpeningDialogue → FreeRoam");
                }
                // Still waiting for dialogue UI to clear
                driver.wait_timer = 1.0;
            } else {
                eprintln!("[dream-test] Dialogue already finished");
                driver.current += 1;
            }

            // If we forced FreeRoam, advance past this action
            if dream.phase == OfficePhase::FreeRoam {
                driver.current += 1;
            }
        }

        TestAction::MoveToAndWait(target) => {
            if driver.move_target.is_none() {
                // Issue the move command
                if let Some((entity, _, _, _)) = heroes.iter().find(|(_, hi, owner, _)| {
                    hi.hero_id == HeroId::KellFisher && owner.player_id == 0
                }) {
                    cmd_queue.push(GameCommand::Move {
                        unit_ids: vec![EntityId::from_entity(entity)],
                        target,
                    });
                    eprintln!("[dream-test] Move to ({}, {}) — waiting for arrival", target.x, target.y);
                }
                driver.move_target = Some(target);
                driver.move_timeout = 30.0; // bail after 30s
            } else {
                // Poll: check if Kell is near the target
                driver.move_timeout -= time.delta_secs();
                if let Some((_, _, _, pos)) = heroes.iter().find(|(_, hi, owner, _)| {
                    hi.hero_id == HeroId::KellFisher && owner.player_id == 0
                }) {
                    let grid = pos.world.to_grid();
                    let dx = (grid.x - target.x).abs();
                    let dy = (grid.y - target.y).abs();
                    if dx.max(dy) <= 3 {
                        eprintln!("[dream-test] Arrived at ({}, {})", target.x, target.y);
                        driver.move_target = None;
                        driver.current += 1;
                    } else if driver.move_timeout <= 0.0 {
                        eprintln!("[dream-test] Timeout reaching ({}, {}), at ({}, {})", target.x, target.y, grid.x, grid.y);
                        driver.move_target = None;
                        driver.current += 1;
                    }
                } else {
                    driver.move_target = None;
                    driver.current += 1;
                }
            }
        }

        TestAction::WaitSecs(secs) => {
            driver.wait_timer = secs;
            driver.current += 1;
        }

        TestAction::PressF => {
            // Log what we expect before injecting the key
            if let Some(nearby) = dream.nearby_action {
                if nearby.is_enabled() {
                    eprintln!("[dream-test] PressF → {:?} (enabled)", nearby);
                } else {
                    let expected = kell_refusal(nearby, &dream);
                    eprintln!("[dream-test] PressF → {:?} (refusal) hour={:.1}", nearby, dream.current_hour);
                    eprintln!("[dream-test]   expected: \"{}\"", expected);
                }
            } else {
                eprintln!("[dream-test] PressF — nothing nearby");
            }
            // Inject real key event so dream_interact_system handles it
            keys.press(KeyCode::KeyF);
            driver.current += 1;
        }

        TestAction::WaitForFreeRoam => {
            if dream.phase == OfficePhase::FreeRoam {
                driver.current += 1;
            }
            // else keep waiting (FSM will transition on its own)
        }

        TestAction::Exit => {
            eprintln!("[dream-test] Test complete. Screenshots in: {}", driver.output_dir.display());
            // Give a moment for final screenshot to flush, then exit
            driver.wait_timer = 2.0;
            driver.current += 1;
            // Exit will happen when wait_timer expires and current > script.len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_is_nonempty() {
        let script = build_test_script();
        assert!(script.len() > 5);
    }

    #[test]
    fn test_driver_defaults() {
        let driver = DreamTestDriver::default();
        assert_eq!(driver.current, 0);
        assert!(!driver.started);
        assert_eq!(driver.screenshot_counter, 0);
    }
}
