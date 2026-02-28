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
pub mod victory_system;

use bevy::prelude::*;

use crate::resources::{CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock, SpawnPositions};
use cc_core::map::GameMap;

pub struct SimSystemsPlugin;

/// Run condition: only run the main sim chain while the game is still playing.
fn game_is_playing(state: Res<GameState>) -> bool {
    *state == GameState::Playing
}

impl Plugin for SimSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandQueue>()
            .init_resource::<SimClock>()
            .init_resource::<ControlGroups>()
            .init_resource::<PlayerResources>()
            .init_resource::<GameState>()
            .init_resource::<SpawnPositions>()
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
                    .chain()
                    .run_if(game_is_playing),
            )
            .add_systems(
                FixedUpdate,
                victory_system::victory_system,
            );
    }
}
