pub mod fsm;

use bevy::prelude::*;

use crate::resources::GameState;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<fsm::AiState>()
            .init_resource::<MultiAiState>()
            .add_systems(
                FixedUpdate,
                fsm::ai_decision_system
                    .after(crate::systems::cleanup_system::cleanup_system)
                    .run_if(|state: Res<GameState>| *state == GameState::Playing),
            );
    }
}

/// Multi-player AI state resource for AI-vs-AI matches.
/// Each entry is an independent FSM controlling one player.
#[derive(Resource, Default)]
pub struct MultiAiState {
    pub players: Vec<fsm::AiState>,
}
