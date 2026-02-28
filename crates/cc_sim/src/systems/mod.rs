pub mod cleanup_system;
pub mod combat_system;
pub mod command_system;
pub mod damage;
pub mod grid_sync_system;
pub mod movement_system;
pub mod production_system;
pub mod projectile_system;
pub mod resource_system;
pub mod target_acquisition_system;
pub mod tick_system;

use bevy::prelude::*;

use crate::resources::{CommandQueue, ControlGroups, MapResource, PlayerResources, SimClock};
use cc_core::map::GameMap;

pub struct SimSystemsPlugin;

impl Plugin for SimSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandQueue>()
            .init_resource::<SimClock>()
            .init_resource::<ControlGroups>()
            .init_resource::<PlayerResources>()
            .insert_resource(MapResource {
                map: GameMap::new(64, 64),
            })
            .add_systems(
                FixedUpdate,
                (
                    tick_system::tick_system,
                    command_system::process_commands,
                    production_system::production_system,
                    resource_system::gathering_system,
                    target_acquisition_system::target_acquisition_system,
                    combat_system::combat_system,
                    projectile_system::projectile_system,
                    movement_system::movement_system,
                    grid_sync_system::grid_sync_system,
                    cleanup_system::cleanup_system,
                )
                    .chain(),
            );
    }
}
