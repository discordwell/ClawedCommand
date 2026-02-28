use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Selected, UnitType};
use cc_sim::resources::CommandQueue;

pub fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_units: Query<Entity, (With<UnitType>, With<Selected>)>,
    mut cmd_queue: ResMut<CommandQueue>,
) {
    // H — Halt/stop selected units (S is used for camera pan)
    if keyboard.just_pressed(KeyCode::KeyH) {
        let unit_ids: Vec<EntityId> = selected_units
            .iter()
            .map(|e| EntityId(e.to_bits()))
            .collect();
        if !unit_ids.is_empty() {
            cmd_queue.push(GameCommand::Stop { unit_ids });
        }
    }

    // Escape — Deselect all
    if keyboard.just_pressed(KeyCode::Escape) {
        cmd_queue.push(GameCommand::Deselect);
    }
}
