use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Owner, Position, ResourceDeposit, Selected, UnitType};
use cc_core::coords::{ScreenPos, screen_to_world};
use cc_sim::resources::CommandQueue;

use super::{DragSelectState, InputMode};

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: u8 = 0;

/// Minimum drag distance (pixels) before box select activates.
const DRAG_THRESHOLD: f32 = 5.0;

pub fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    units: Query<(Entity, &Position, &Owner, Option<&Selected>), With<UnitType>>,
    deposits: Query<(Entity, &Position), With<ResourceDeposit>>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut drag_state: ResMut<DragSelectState>,
    mut input_mode: ResMut<InputMode>,
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

    // --- Left click down: start drag ---
    if mouse_button.just_pressed(MouseButton::Left) {
        drag_state.start = Some(cursor_pos);
        drag_state.active = false;

        // In AttackMove mode, left-click issues attack-move and reverts
        if *input_mode == InputMode::AttackMove {
            let selected_ids: Vec<EntityId> = units
                .iter()
                .filter(|(_, _, _, sel)| sel.is_some())
                .map(|(entity, _, _, _)| EntityId(entity.to_bits()))
                .collect();
            if !selected_ids.is_empty() {
                let target = iso_world.to_grid();
                cmd_queue.push(GameCommand::AttackMove {
                    unit_ids: selected_ids,
                    target,
                });
            }
            *input_mode = InputMode::Normal;
            drag_state.start = None;
            return;
        }
    }

    // --- Left click held: check drag threshold ---
    if mouse_button.pressed(MouseButton::Left) {
        if let Some(start) = drag_state.start {
            let delta = cursor_pos - start;
            if delta.length() > DRAG_THRESHOLD {
                drag_state.active = true;
            }
        }
    }

    // --- Left click released ---
    if mouse_button.just_released(MouseButton::Left) {
        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        if drag_state.active {
            // Box select: select all own units within the screen-space rectangle
            if let Some(start) = drag_state.start {
                let min_x = start.x.min(cursor_pos.x);
                let max_x = start.x.max(cursor_pos.x);
                let min_y = start.y.min(cursor_pos.y);
                let max_y = start.y.max(cursor_pos.y);

                if !shift {
                    cmd_queue.push(GameCommand::Deselect);
                }

                let mut box_selected = Vec::new();
                for (entity, pos, owner, _) in units.iter() {
                    if owner.player_id != LOCAL_PLAYER {
                        continue;
                    }
                    // Convert unit world pos to screen (viewport) coords
                    let unit_screen = unit_to_viewport(pos, camera, camera_transform);
                    if let Some(sp) = unit_screen {
                        if sp.x >= min_x && sp.x <= max_x && sp.y >= min_y && sp.y <= max_y {
                            box_selected.push(EntityId(entity.to_bits()));
                        }
                    }
                }
                if !box_selected.is_empty() {
                    cmd_queue.push(GameCommand::Select {
                        unit_ids: box_selected,
                    });
                }
            }
        } else {
            // Click select: pick nearest unit
            let mut clicked_unit = None;
            let mut best_dist = f32::MAX;

            for (entity, pos, _owner, _) in units.iter() {
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

        // Reset drag state
        drag_state.start = None;
        drag_state.active = false;
    }

    // --- Right click: smart command ---
    if mouse_button.just_pressed(MouseButton::Right) {
        // Cancel attack-move mode on right-click
        if *input_mode == InputMode::AttackMove {
            *input_mode = InputMode::Normal;
            return;
        }

        let selected_ids: Vec<EntityId> = units
            .iter()
            .filter(|(_, _, _, sel)| sel.is_some())
            .map(|(entity, _, _, _)| EntityId(entity.to_bits()))
            .collect();

        if selected_ids.is_empty() {
            return;
        }

        // Check: right-click on enemy unit → Attack
        let mut clicked_enemy = None;
        let mut best_dist = f32::MAX;

        for (entity, pos, owner, _) in units.iter() {
            if owner.player_id == LOCAL_PLAYER {
                continue;
            }
            let ux: f32 = pos.world.x.to_num();
            let uy: f32 = pos.world.y.to_num();
            let iso_x: f32 = iso_world.x.to_num();
            let iso_y: f32 = iso_world.y.to_num();
            let dx = ux - iso_x;
            let dy = uy - iso_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 0.8 && dist < best_dist {
                best_dist = dist;
                clicked_enemy = Some(entity);
            }
        }

        if let Some(enemy) = clicked_enemy {
            cmd_queue.push(GameCommand::Attack {
                unit_ids: selected_ids,
                target: EntityId(enemy.to_bits()),
            });
            return;
        }

        // Check: right-click on resource deposit → GatherResource
        let mut clicked_deposit = None;
        best_dist = f32::MAX;

        for (entity, pos) in deposits.iter() {
            let ux: f32 = pos.world.x.to_num();
            let uy: f32 = pos.world.y.to_num();
            let iso_x: f32 = iso_world.x.to_num();
            let iso_y: f32 = iso_world.y.to_num();
            let dx = ux - iso_x;
            let dy = uy - iso_y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 1.0 && dist < best_dist {
                best_dist = dist;
                clicked_deposit = Some(entity);
            }
        }

        if let Some(deposit) = clicked_deposit {
            cmd_queue.push(GameCommand::GatherResource {
                unit_ids: selected_ids,
                deposit: EntityId(deposit.to_bits()),
            });
            return;
        }

        // Default: right-click on ground → Move
        let target = iso_world.to_grid();
        cmd_queue.push(GameCommand::Move {
            unit_ids: selected_ids,
            target,
        });
    }
}

/// Convert a unit's world position to viewport (screen) coordinates.
fn unit_to_viewport(
    pos: &Position,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Vec2> {
    use cc_core::coords::world_to_screen;

    let screen = world_to_screen(pos.world);
    // Apply isometric → Bevy coordinate (Y flip)
    let bevy_world = Vec3::new(screen.x, -screen.y, 0.0);

    camera.world_to_viewport(camera_transform, bevy_world).ok()
}
