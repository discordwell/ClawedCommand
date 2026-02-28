use bevy::prelude::*;

use cc_core::components::Selected;

/// Update the sprite color of selected units to highlight them.
/// Selected units get a brighter tint; deselected units revert.
pub fn render_selection_indicators(
    mut query: Query<(&mut Sprite, Option<&Selected>), With<cc_core::components::UnitType>>,
) {
    for (mut sprite, selected) in query.iter_mut() {
        if selected.is_some() {
            sprite.color = Color::srgb(0.3, 0.8, 1.0); // Bright cyan when selected
        } else {
            sprite.color = Color::srgb(0.2, 0.4, 0.9); // Default blue
        }
    }
}
