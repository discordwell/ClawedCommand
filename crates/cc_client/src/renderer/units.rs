use bevy::prelude::*;

use cc_core::components::Position;
use cc_core::coords::{depth_z, world_to_screen};

/// Sync unit sprite Transform positions from their simulation Position each frame.
pub fn sync_unit_sprites(mut query: Query<(&Position, &mut Transform), With<Sprite>>) {
    for (pos, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y; // Bevy Y is up, isometric Y goes down
        transform.translation.z = depth_z(pos.world);
    }
}
