pub mod command_system;
pub mod grid_sync_system;
pub mod movement_system;
pub mod tick_system;

use bevy::prelude::*;

use crate::resources::{CommandQueue, MapResource, SimClock};
use cc_core::map::GameMap;

pub struct SimSystemsPlugin;

impl Plugin for SimSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandQueue>()
            .init_resource::<SimClock>()
            .insert_resource(MapResource {
                map: GameMap::new(64, 64),
            })
            .add_systems(
                FixedUpdate,
                (
                    tick_system::tick_system,
                    command_system::process_commands,
                    movement_system::movement_system,
                    grid_sync_system::grid_sync_system,
                )
                    .chain(),
            );
    }
}
