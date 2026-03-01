pub mod mutator_state;
pub mod mutator_systems;
pub mod state;
pub mod triggers;
pub mod wave_spawner;

use bevy::prelude::*;

use crate::systems::cleanup_system;

/// Campaign plugin — registers resources, messages, and systems for mission play.
pub struct CampaignPlugin;

impl Plugin for CampaignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<state::CampaignState>()
            .init_resource::<wave_spawner::WaveTracker>()
            .init_resource::<wave_spawner::MissionStarted>()
            .init_resource::<mutator_state::ControlRestrictions>()
            .init_resource::<mutator_state::MutatorState>()
            .init_resource::<mutator_state::FogState>()
            .add_message::<triggers::DialogueEvent>()
            .add_message::<triggers::TriggerFiredEvent>()
            .add_message::<triggers::ObjectiveCompleteEvent>()
            .add_message::<state::MissionFailedEvent>()
            .add_message::<state::MissionVictoryEvent>()
            .add_systems(
                FixedUpdate,
                (
                    wave_spawner::wave_tracking_system,
                    triggers::trigger_check_system,
                    wave_spawner::wave_spawner_system,
                    state::mission_objective_system,
                    mutator_systems::environmental_hazard_system,
                    mutator_systems::hazard_damage_system,
                    mutator_systems::mutator_tick_system,
                )
                    .chain()
                    .after(cleanup_system::cleanup_system),
            );
    }
}
