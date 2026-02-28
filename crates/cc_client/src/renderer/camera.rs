use bevy::prelude::*;

const PAN_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
const EDGE_SCROLL_MARGIN: f32 = 20.0;
const EDGE_SCROLL_SPEED: f32 = 200.0;
const MIN_ZOOM: f32 = 0.3;
const MAX_ZOOM: f32 = 3.0;

pub fn camera_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    window: Single<&Window>,
    mut camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let (ref mut transform, ref mut projection) = *camera;
    let Projection::Orthographic(ref mut ortho) = **projection else {
        return;
    };
    let dt = time.delta_secs();
    let mut pan = Vec2::ZERO;

    // Keyboard panning (WASD + arrows)
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        pan.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        pan.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        pan.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        pan.x += 1.0;
    }

    if pan != Vec2::ZERO {
        pan = pan.normalize() * PAN_SPEED * dt;
    }

    // Edge scrolling
    if let Some(cursor) = window.cursor_position() {
        let w = window.width();
        let h = window.height();
        let mut edge_pan = Vec2::ZERO;

        if cursor.x < EDGE_SCROLL_MARGIN {
            edge_pan.x -= 1.0;
        }
        if cursor.x > w - EDGE_SCROLL_MARGIN {
            edge_pan.x += 1.0;
        }
        if cursor.y < EDGE_SCROLL_MARGIN {
            edge_pan.y += 1.0;
        }
        if cursor.y > h - EDGE_SCROLL_MARGIN {
            edge_pan.y -= 1.0;
        }

        if edge_pan != Vec2::ZERO {
            pan += edge_pan.normalize() * EDGE_SCROLL_SPEED * dt;
        }
    }

    // Scale pan speed by zoom level so it feels consistent
    let scale = ortho.scale;
    transform.translation.x += pan.x * scale;
    transform.translation.y += pan.y * scale;

    // Zoom (scroll wheel)
    for event in scroll_events.read() {
        ortho.scale -= event.y * ZOOM_SPEED;
        ortho.scale = ortho.scale.clamp(MIN_ZOOM, MAX_ZOOM);
    }
}
