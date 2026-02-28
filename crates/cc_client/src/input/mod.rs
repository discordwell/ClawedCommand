pub mod keyboard;
pub mod mouse;

use bevy::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                mouse::handle_mouse_click,
                keyboard::handle_keyboard,
            ),
        );
    }
}
