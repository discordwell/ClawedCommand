use bevy::prelude::*;

use crate::input::DragSelectState;

/// Marker for the box-select rectangle sprite.
#[derive(Component)]
pub struct BoxSelectRect;

/// Spawn a translucent box-select rectangle when drag is active, despawn when released.
pub fn render_box_select(
    mut commands: Commands,
    drag_state: Res<DragSelectState>,
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    existing: Query<Entity, With<BoxSelectRect>>,
) {
    if drag_state.active {
        if let Some(start) = drag_state.start {
            let Some(cursor_pos) = window.cursor_position() else {
                return;
            };
            let (camera, camera_transform) = *camera_q;

            // Convert screen corners to world space
            let Ok(start_world) = camera.viewport_to_world_2d(camera_transform, start) else {
                return;
            };
            let Ok(end_world) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
                return;
            };

            let center = (start_world + end_world) / 2.0;
            let size = (end_world - start_world).abs();

            // Despawn old rect, spawn new one
            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }

            commands.spawn((
                BoxSelectRect,
                Sprite {
                    color: Color::srgba(0.3, 0.8, 1.0, 0.15),
                    custom_size: Some(size),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, 999.0),
            ));
        }
    } else {
        // Despawn rect when drag ends
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
    }
}
