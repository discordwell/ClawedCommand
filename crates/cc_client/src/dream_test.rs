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

use crate::dream::{DreamOfficeState, OfficePhase};
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
    /// Move Kell to a grid position.
    MoveTo(GridPos),
    /// Wait N real seconds for movement to complete.
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
            start_delay: 0.5, // short delay for first render frame
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
        TestAction::Log("Moving to desk (Work)"),
        TestAction::MoveTo(GridPos::new(10, 7)),
        TestAction::WaitSecs(5.0),
        TestAction::Screenshot("03_at_desk"),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("04_after_work_1"),
        TestAction::Log("Moving to vending machine"),
        TestAction::MoveTo(GridPos::new(14, 2)),
        TestAction::WaitSecs(8.0),
        TestAction::Screenshot("05_at_vending"),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("06_after_drink"),
        TestAction::Log("Moving to gym"),
        TestAction::MoveTo(GridPos::new(3, 12)),
        TestAction::WaitSecs(10.0),
        TestAction::Screenshot("07_at_gym"),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("08_after_workout"),
        TestAction::Log("Moving to cot (disabled — should get refusal)"),
        TestAction::MoveTo(GridPos::new(2, 2)),
        TestAction::WaitSecs(10.0),
        TestAction::Screenshot("09_at_cot"),
        TestAction::PressF,
        TestAction::WaitSecs(2.0),
        TestAction::Screenshot("10_refusal_dialogue"),
        // Do a few more work sessions
        TestAction::Log("Rapid work sessions — back to desk"),
        TestAction::MoveTo(GridPos::new(10, 7)),
        TestAction::WaitSecs(10.0),
        TestAction::PressF,
        TestAction::WaitForFreeRoam,
        TestAction::PressF, // should still be near desk
        TestAction::WaitForFreeRoam,
        TestAction::Screenshot("11_after_several_sessions"),
        TestAction::Log("=== Dream Test: complete ==="),
        TestAction::Screenshot("12_final"),
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

        TestAction::MoveTo(target) => {
            if let Some((entity, _, _, _)) = heroes.iter().find(|(_, hi, owner, _)| {
                hi.hero_id == HeroId::Kelpie && owner.player_id == 0
            }) {
                cmd_queue.push(GameCommand::Move {
                    unit_ids: vec![EntityId::from_entity(entity)],
                    target,
                });
                eprintln!("[dream-test] Move to ({}, {})", target.x, target.y);
            }
            driver.current += 1;
        }

        TestAction::WaitSecs(secs) => {
            driver.wait_timer = secs;
            driver.current += 1;
        }

        TestAction::PressF => {
            // Simulate F key press by directly triggering the interaction
            if let Some(nearby) = dream.nearby_action {
                if nearby.is_enabled() {
                    // Check forced action
                    if dream.forced_action.is_none() || dream.forced_action == Some(nearby) {
                        dream.current_action = Some(nearby);
                        dream.action_timer = 2.5; // ACTION_DURATION
                        dream.phase = OfficePhase::ActionInProgress;
                        eprintln!("[dream-test] PressF → {:?}", nearby);
                    } else {
                        eprintln!("[dream-test] PressF blocked by forced action {:?}", dream.forced_action);
                    }
                } else {
                    // Disabled action — trigger refusal
                    dream.refusal_timer = 3.0;
                    eprintln!("[dream-test] PressF → refusal for {:?}", nearby);
                }
            } else {
                eprintln!("[dream-test] PressF — nothing nearby");
            }
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
