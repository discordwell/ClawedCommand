use bevy::prelude::*;

use cc_core::components::Dead;

/// Fade out sprite alpha on entities marked Dead, giving a brief visual death effect.
pub fn death_fade_system(mut query: Query<&mut Sprite, With<Dead>>) {
    for mut sprite in query.iter_mut() {
        let a = sprite.color.alpha();
        sprite.color.set_alpha((a - 0.15).max(0.0));
    }
}
