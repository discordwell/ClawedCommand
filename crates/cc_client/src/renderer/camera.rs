use bevy::prelude::*;

use cc_core::components::{Building, Owner, Producer, Selected};

use crate::LOCAL_PLAYER;
use crate::input::InputMode;

const PAN_SPEED: f32 = 300.0;
const ZOOM_SPEED: f32 = 0.1;
const KEY_ZOOM_SPEED: f32 = 1.0;
const EDGE_SCROLL_MARGIN: f32 = 20.0;
const EDGE_SCROLL_SPEED: f32 = 200.0;
const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 3.5;

pub fn camera_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    window: Single<&Window>,
    mut camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
    input_mode: Res<InputMode>,
    selected_producers: Query<&Owner, (With<Building>, With<Producer>, With<Selected>)>,
) {
    // Block camera pan/zoom during prompt overlay
    if *input_mode == InputMode::Prompt {
        // Drain scroll events so they don't accumulate
        for _ in scroll_events.read() {}
        return;
    }

    let (ref mut transform, ref mut projection) = *camera;
    let Projection::Orthographic(ref mut ortho) = **projection else {
        return;
    };
    let dt = time.delta_secs();

    // W doubles as the train-slot-1 hotkey while a local producer building is
    // selected (input/keyboard.rs), and S doubles as the ServerRack sub-key
    // while the build menu is open — don't also pan the camera on those keys.
    let suppress_w = selected_producers
        .iter()
        .any(|owner| owner.player_id == LOCAL_PLAYER);
    let suppress_s = *input_mode == InputMode::BuildMenu;
    let mut pan = keyboard_pan_vector(&keyboard, suppress_w, suppress_s);

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

    // Keyboard zoom: =/+ zooms in, -/_ zooms out (center-screen)
    if keyboard.pressed(KeyCode::Equal) {
        ortho.scale -= KEY_ZOOM_SPEED * dt;
        ortho.scale = ortho.scale.clamp(MIN_ZOOM, MAX_ZOOM);
    }
    if keyboard.pressed(KeyCode::Minus) {
        ortho.scale += KEY_ZOOM_SPEED * dt;
        ortho.scale = ortho.scale.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    // Zoom toward cursor (scroll wheel)
    for event in scroll_events.read() {
        let old_scale = ortho.scale;
        ortho.scale -= event.y * ZOOM_SPEED;
        ortho.scale = ortho.scale.clamp(MIN_ZOOM, MAX_ZOOM);
        let new_scale = ortho.scale;

        // Adjust camera position so the world point under the cursor stays fixed
        if let Some(cursor) = window.cursor_position() {
            let w = window.width();
            let h = window.height();
            // Cursor offset from window center in screen pixels
            let cursor_offset = Vec2::new(cursor.x - w / 2.0, -(cursor.y - h / 2.0));
            // World-space offset = cursor_offset * scale
            let world_before = cursor_offset * old_scale;
            let world_after = cursor_offset * new_scale;
            let delta = world_before - world_after;
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
        }
    }
}

/// Unnormalized WASD/arrow pan direction. `suppress_w`/`suppress_s` drop the
/// W/S letter keys when another binding claims them (W trains slot 1 while a
/// producer is selected; S picks ServerRack in the build menu); the arrow
/// keys always pan.
fn keyboard_pan_vector(
    keyboard: &ButtonInput<KeyCode>,
    suppress_w: bool,
    suppress_s: bool,
) -> Vec2 {
    let mut pan = Vec2::ZERO;

    if (keyboard.pressed(KeyCode::KeyW) && !suppress_w) || keyboard.pressed(KeyCode::ArrowUp) {
        pan.y += 1.0;
    }
    if (keyboard.pressed(KeyCode::KeyS) && !suppress_s) || keyboard.pressed(KeyCode::ArrowDown) {
        pan.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        pan.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        pan.x += 1.0;
    }

    pan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard_with(keys: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut kb = ButtonInput::default();
        for &key in keys {
            kb.press(key);
        }
        kb
    }

    #[test]
    fn wasd_pans_when_nothing_suppressed() {
        let kb = keyboard_with(&[KeyCode::KeyW, KeyCode::KeyD]);
        assert_eq!(
            keyboard_pan_vector(&kb, false, false),
            Vec2::new(1.0, 1.0)
        );
    }

    #[test]
    fn w_pan_suppressed_while_producer_selected() {
        // TDL HIGH fix: pressing W to train slot 1 must not also pan the camera
        let kb = keyboard_with(&[KeyCode::KeyW]);
        assert_eq!(keyboard_pan_vector(&kb, true, false), Vec2::ZERO);
    }

    #[test]
    fn arrow_up_still_pans_while_w_suppressed() {
        let kb = keyboard_with(&[KeyCode::ArrowUp]);
        assert_eq!(
            keyboard_pan_vector(&kb, true, false),
            Vec2::new(0.0, 1.0)
        );
    }

    #[test]
    fn s_pan_suppressed_in_build_menu() {
        // Pressing S to pick ServerRack in the build menu must not pan down
        let kb = keyboard_with(&[KeyCode::KeyS]);
        assert_eq!(keyboard_pan_vector(&kb, false, true), Vec2::ZERO);
    }

    #[test]
    fn other_pan_keys_unaffected_by_suppression() {
        let kb = keyboard_with(&[KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD]);
        // Only W suppressed: A/S/D still pan
        assert_eq!(
            keyboard_pan_vector(&kb, true, false),
            Vec2::new(0.0, -1.0)
        );
    }
}
