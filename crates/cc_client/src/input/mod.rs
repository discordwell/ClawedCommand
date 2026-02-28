pub mod keyboard;
pub mod mouse;

use bevy::prelude::*;

/// Current input mode — changes how left-click behaves.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    AttackMove,
}

/// State for drag-box selection.
#[derive(Resource, Default, Debug, Clone)]
pub struct DragSelectState {
    /// Screen-space start position of the drag (set on left-click down).
    pub start: Option<Vec2>,
    /// Whether the drag has exceeded the threshold (5px).
    pub active: bool,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputMode>()
            .init_resource::<DragSelectState>()
            .add_systems(
                Update,
                (
                    mouse::handle_mouse_input,
                    keyboard::handle_keyboard,
                ),
            );
    }
}
