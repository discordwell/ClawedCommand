use bevy::prelude::*;

use crate::setup::UnitMesh;
use cc_core::components::Position;
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Sync unit mesh Transform positions from their simulation Position each frame.
pub fn sync_unit_sprites(
    map_res: Res<MapResource>,
    mut query: Query<(&Position, &mut Transform), With<UnitMesh>>,
) {
    for (pos, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y + elevation_offset;
        transform.translation.z = depth_z(pos.world);
    }
}
