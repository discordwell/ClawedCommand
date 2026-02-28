use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Selected, UnitType};
use cc_sim::resources::CommandQueue;

use super::InputMode;

pub fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_units: Query<Entity, (With<UnitType>, With<Selected>)>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut input_mode: ResMut<InputMode>,
) {
    // H — Halt/stop selected units (S is used for camera pan)
    if keyboard.just_pressed(KeyCode::KeyH) {
        let unit_ids: Vec<EntityId> = selected_units
            .iter()
            .map(|e| EntityId(e.to_bits()))
            .collect();
        if !unit_ids.is_empty() {
            // Shift+H = Hold Position (stop + attack in range only)
            if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
                cmd_queue.push(GameCommand::HoldPosition { unit_ids });
            } else {
                cmd_queue.push(GameCommand::Stop { unit_ids });
            }
        }
    }

    // A — Enter attack-move mode
    if keyboard.just_pressed(KeyCode::KeyA) {
        *input_mode = InputMode::AttackMove;
    }

    // Escape — Deselect all + cancel attack-move mode
    if keyboard.just_pressed(KeyCode::Escape) {
        if *input_mode == InputMode::AttackMove {
            *input_mode = InputMode::Normal;
        } else {
            cmd_queue.push(GameCommand::Deselect);
        }
    }

    // Control groups: Ctrl+0-9 assign, 0-9 recall
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    let group_keys = [
        (KeyCode::Digit0, 0u8),
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
        (KeyCode::Digit4, 4),
        (KeyCode::Digit5, 5),
        (KeyCode::Digit6, 6),
        (KeyCode::Digit7, 7),
        (KeyCode::Digit8, 8),
        (KeyCode::Digit9, 9),
    ];

    for (key, group) in group_keys {
        if keyboard.just_pressed(key) {
            if ctrl {
                // Assign selected units to control group
                let unit_ids: Vec<EntityId> = selected_units
                    .iter()
                    .map(|e| EntityId(e.to_bits()))
                    .collect();
                if !unit_ids.is_empty() {
                    cmd_queue.push(GameCommand::SetControlGroup { group, unit_ids });
                }
            } else {
                // Recall control group
                cmd_queue.push(GameCommand::RecallControlGroup { group });
            }
        }
    }
}
