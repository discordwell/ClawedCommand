pub mod keyboard;
pub mod mouse;

use bevy::prelude::*;
use cc_core::components::{BuildingKind, CursorGridPos, UnitKind};

/// Current input mode — changes how left-click behaves.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    AttackMove,
    /// Waiting for a sub-key to select a building type.
    BuildMenu,
    BuildPlacement { kind: BuildingKind },
}

/// State for drag-box selection.
#[derive(Resource, Default, Debug, Clone)]
pub struct DragSelectState {
    /// Screen-space start position of the drag (set on left-click down).
    pub start: Option<Vec2>,
    /// Whether the drag has exceeded the threshold (5px).
    pub active: bool,
}

/// State for double-click detection.
#[derive(Resource, Default)]
pub struct DoubleClickState {
    /// Time of the last click-select (seconds since startup).
    pub last_click_time: f64,
    /// UnitKind of the last click-selected unit, if any.
    pub last_click_kind: Option<UnitKind>,
}

/// Resource tracking the ghost placement preview state.
#[derive(Resource, Default)]
pub struct PlacementPreview {
    pub grid_pos: Option<cc_core::coords::GridPos>,
    pub valid: bool,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputMode>()
            .init_resource::<DragSelectState>()
            .init_resource::<PlacementPreview>()
            .init_resource::<DoubleClickState>()
            .init_resource::<CursorGridPos>()
            .add_systems(
                Update,
                (
                    mouse::update_cursor_grid_pos,
                    mouse::handle_mouse_input,
                    keyboard::handle_keyboard,
                )
                    .chain(),
            );
    }
}
