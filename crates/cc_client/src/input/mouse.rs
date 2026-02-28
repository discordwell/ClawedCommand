use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Position, Selected, UnitType};
use cc_core::coords::{ScreenPos, screen_to_world};
use cc_sim::resources::CommandQueue;

pub fn handle_mouse_click(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    units: Query<(Entity, &Position, Option<&Selected>), With<UnitType>>,
    mut cmd_queue: ResMut<CommandQueue>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera_q;

    // Convert cursor to world coordinates via camera
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Bevy world has Y-up, our isometric has Y-down, so flip Y for screen_to_world
    let iso_world = screen_to_world(ScreenPos {
        x: world_pos.x,
        y: -world_pos.y,
    });

    // Left click: select units
    if mouse_button.just_pressed(MouseButton::Left) {
        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        // Check if clicking on a unit
        let mut clicked_unit = None;
        let mut best_dist = f32::MAX;

        for (entity, pos, _) in units.iter() {
            let ux: f32 = pos.world.x.to_num();
            let uy: f32 = pos.world.y.to_num();
            let iso_x: f32 = iso_world.x.to_num();
            let iso_y: f32 = iso_world.y.to_num();
            let dx = ux - iso_x;
            let dy = uy - iso_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 0.8 && dist < best_dist {
                best_dist = dist;
                clicked_unit = Some(entity);
            }
        }

        if let Some(entity) = clicked_unit {
            if !shift {
                cmd_queue.push(GameCommand::Deselect);
            }
            cmd_queue.push(GameCommand::Select {
                unit_ids: vec![EntityId(entity.to_bits())],
            });
        } else if !shift {
            cmd_queue.push(GameCommand::Deselect);
        }
    }

    // Right click: move selected units
    if mouse_button.just_pressed(MouseButton::Right) {
        let selected_ids: Vec<EntityId> = units
            .iter()
            .filter(|(_, _, sel)| sel.is_some())
            .map(|(entity, _, _)| EntityId(entity.to_bits()))
            .collect();

        if !selected_ids.is_empty() {
            let target = iso_world.to_grid();
            cmd_queue.push(GameCommand::Move {
                unit_ids: selected_ids,
                target,
            });
        }
    }
}
