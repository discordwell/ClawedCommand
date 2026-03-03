pub mod ability_effect_system;
pub mod ability_system;
pub mod aura_system;
pub mod builder_system;
pub mod cleanup_system;
pub mod combat_system;
pub mod command_system;
pub mod damage;
pub mod grid_sync_system;
pub mod movement_system;
pub mod production_system;
pub mod projectile_system;
pub mod research_system;
pub mod resource_system;
pub mod stat_modifier_system;
pub mod status_effect_system;
pub mod target_acquisition_system;
pub mod tick_system;
pub mod tower_combat_system;
pub mod victory_system;

use bevy::prelude::*;

use crate::resources::{
    CombatStats, CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock,
    SimRng, SpawnPositions, VoiceOverride,
};
use cc_core::map::GameMap;

pub struct SimSystemsPlugin;

/// Run condition: only run the main sim chain while the game is still playing.
fn game_is_playing(state: Res<GameState>) -> bool {
    *state == GameState::Playing
}

impl Plugin for SimSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<projectile_system::ProjectileHit>()
            .init_resource::<CommandQueue>()
            .init_resource::<SimClock>()
            .init_resource::<ControlGroups>()
            .init_resource::<PlayerResources>()
            .init_resource::<GameState>()
            .init_resource::<SpawnPositions>()
            .init_resource::<SimRng>()
            .init_resource::<CombatStats>()
            .init_resource::<VoiceOverride>()
            .insert_resource(MapResource {
                map: GameMap::new(64, 64),
            })
            .add_systems(
                FixedUpdate,
                (
                    tick_system::tick_system,
                    command_system::process_commands,
                    ability_system::ability_cooldown_system,
                    ability_effect_system::ability_effect_system,
                    status_effect_system::status_effect_system,
                    aura_system::aura_system,
                    stat_modifier_system::stat_modifier_system,
                    production_system::production_system,
                    research_system::research_system,
                    resource_system::gathering_system,
                    target_acquisition_system::target_acquisition_system,
                    combat_system::combat_system,
                    tower_combat_system::tower_combat_system,
                    projectile_system::projectile_system,
                    movement_system::movement_system,
                    builder_system::builder_system,
                    grid_sync_system::grid_sync_system,
                    cleanup_system::cleanup_system,
                )
                    .chain()
                    .run_if(game_is_playing),
            )
            .add_systems(
                FixedUpdate,
                victory_system::victory_system.after(cleanup_system::cleanup_system),
            );
    }
}
