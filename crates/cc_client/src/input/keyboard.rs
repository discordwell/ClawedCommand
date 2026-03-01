use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Building, BuildingKind, Owner, Producer, Selected, UnitType};
use cc_sim::resources::CommandQueue;

use super::{DoubleClickState, InputMode};

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: u8 = 0;

pub fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_units: Query<Entity, (With<UnitType>, With<Selected>)>,
    selected_buildings: Query<(Entity, &Building, &Owner), (With<Producer>, With<Selected>)>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut input_mode: ResMut<InputMode>,
    mut dbl_click: ResMut<DoubleClickState>,
    restrictions: Option<Res<cc_sim::campaign::mutator_state::ControlRestrictions>>,
    #[cfg(any(feature = "native", feature = "wasm-agent"))]
    construct_state: Res<cc_agent::construct_mode::ConstructModeState>,
) {
    // Gate: skip all keyboard commands (except camera, handled separately) if restricted
    if restrictions.as_ref().is_some_and(|r| !r.mouse_keyboard_enabled) {
        return;
    }

    // Prompt mode: block all game hotkeys (text input handled by prompt_overlay UI)
    if *input_mode == InputMode::Prompt {
        // Only Escape exits prompt mode (handled by prompt_overlay system)
        return;
    }

    // `/` key — open AI prompt overlay (only from Normal mode, blocked during construct mode)
    if keyboard.just_pressed(KeyCode::Slash) && *input_mode == InputMode::Normal {
        #[cfg(any(feature = "native", feature = "wasm-agent"))]
        if construct_state.active {
            return; // Don't open prompt while construct mode panel is open
        }
        *input_mode = InputMode::Prompt;
        return;
    }

    // --- Build menu sub-key handling ---
    if *input_mode == InputMode::BuildMenu {
        let building = if keyboard.just_pressed(KeyCode::KeyT) {
            Some(BuildingKind::CatTree)
        } else if keyboard.just_pressed(KeyCode::KeyF) {
            Some(BuildingKind::FishMarket)
        } else if keyboard.just_pressed(KeyCode::KeyL) {
            Some(BuildingKind::LitterBox)
        } else if keyboard.just_pressed(KeyCode::KeyS) {
            Some(BuildingKind::ServerRack)
        } else if keyboard.just_pressed(KeyCode::KeyP) {
            Some(BuildingKind::ScratchingPost)
        } else if keyboard.just_pressed(KeyCode::KeyC) {
            Some(BuildingKind::CatFlap)
        } else if keyboard.just_pressed(KeyCode::KeyR) {
            Some(BuildingKind::LaserPointer)
        } else {
            None
        };

        if let Some(kind) = building {
            *input_mode = InputMode::BuildPlacement { kind };
        } else if keyboard.just_pressed(KeyCode::Escape) {
            *input_mode = InputMode::Normal;
        }
        return;
    }

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

    // G — Enter attack-move mode (A conflicts with WASD camera pan)
    if keyboard.just_pressed(KeyCode::KeyG) {
        *input_mode = InputMode::AttackMove;
    }

    // M — Enter explicit move mode
    if keyboard.just_pressed(KeyCode::KeyM) {
        *input_mode = InputMode::Move;
    }

    // B — Enter build menu (when a unit is selected)
    if keyboard.just_pressed(KeyCode::KeyB) {
        if !selected_units.is_empty() {
            *input_mode = InputMode::BuildMenu;
        }
    }

    // Q/W/E/R — Train units from selected producer building
    // X — Cancel front of production queue
    let train_key = if keyboard.just_pressed(KeyCode::KeyQ) {
        Some(0usize)
    } else if keyboard.just_pressed(KeyCode::KeyW) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::KeyE) {
        Some(2)
    } else if keyboard.just_pressed(KeyCode::KeyR) {
        Some(3)
    } else {
        None
    };

    if let Some(slot) = train_key {
        for (entity, bld, owner) in selected_buildings.iter() {
            if owner.player_id != LOCAL_PLAYER {
                continue;
            }
            let bstats = building_stats(bld.kind);
            if let Some(&unit_kind) = bstats.can_produce.get(slot) {
                cmd_queue.push(GameCommand::TrainUnit {
                    building: EntityId(entity.to_bits()),
                    unit_kind,
                });
            }
        }
    }

    if keyboard.just_pressed(KeyCode::KeyX) {
        for (entity, _, owner) in selected_buildings.iter() {
            if owner.player_id != LOCAL_PLAYER {
                continue;
            }
            cmd_queue.push(GameCommand::CancelQueue {
                building: EntityId(entity.to_bits()),
            });
        }
    }

    // Escape — Cancel special modes or deselect all
    if keyboard.just_pressed(KeyCode::Escape) {
        dbl_click.last_click_kind = None;
        if *input_mode != InputMode::Normal {
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
