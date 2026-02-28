use bevy::prelude::*;

use cc_core::components::{GridCell, Position};

/// Recompute the cached GridCell from each entity's Position.
pub fn grid_sync_system(mut query: Query<(&Position, &mut GridCell)>) {
    for (pos, mut cell) in query.iter_mut() {
        cell.pos = pos.world.to_grid();
    }
}
