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
                )
                    .chain()
                    .after(cleanup_system::cleanup_system),
            );
    }
}
