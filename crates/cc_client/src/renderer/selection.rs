use bevy::prelude::*;

use cc_core::components::{Owner, Selected};

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: u8 = 0;

/// Update the sprite color of units based on ownership and selection state.
/// Selected units get a brighter tint; deselected units show team color.
pub fn render_selection_indicators(
    mut query: Query<(&mut Sprite, &Owner, Option<&Selected>), With<cc_core::components::UnitType>>,
) {
    for (mut sprite, owner, selected) in query.iter_mut() {
        if selected.is_some() {
            sprite.color = Color::srgb(0.3, 0.8, 1.0); // Bright cyan when selected
        } else if owner.player_id == LOCAL_PLAYER {
            sprite.color = Color::srgb(0.2, 0.4, 0.9); // Blue for player
        } else {
            sprite.color = Color::srgb(0.9, 0.2, 0.2); // Red for enemy
        }
    }
}
