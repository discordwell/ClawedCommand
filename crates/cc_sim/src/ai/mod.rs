pub mod fsm;

use bevy::prelude::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<fsm::AiState>()
            .add_systems(FixedUpdate, fsm::ai_decision_system.after(crate::systems::cleanup_system::cleanup_system));
    }
}
