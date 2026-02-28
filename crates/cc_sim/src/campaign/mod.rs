pub mod state;
pub mod triggers;

use bevy::prelude::*;

use crate::systems::cleanup_system;

/// Campaign plugin — registers resources, messages, and systems for mission play.
pub struct CampaignPlugin;

impl Plugin for CampaignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<state::CampaignState>()
            .add_message::<triggers::DialogueEvent>()
            .add_message::<triggers::TriggerFiredEvent>()
            .add_message::<triggers::ObjectiveCompleteEvent>()
            .add_message::<state::MissionFailedEvent>()
            .add_message::<state::MissionVictoryEvent>()
            .add_systems(
                FixedUpdate,
                (
                    triggers::trigger_check_system
                        .after(crate::systems::combat_system::combat_system)
                        .before(cleanup_system::cleanup_system),
                    state::mission_objective_system
                        .after(cleanup_system::cleanup_system)
                        .before(crate::systems::victory_system::victory_system),
                ),
            );
    }
}
