pub mod ability_bar;
pub mod agent_chat;
pub mod command_card;
pub mod construct_mode;
pub mod game_over;
pub mod notifications;
pub mod resource_bar;
pub mod unit_info;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

/// Shared UI state — notifications, etc.
#[derive(Resource, Default)]
pub struct UiState {
    /// Active toast notifications: (message, remaining_seconds).
    pub notifications: Vec<(String, f32)>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<UiState>()
            .add_systems(Update, construct_mode::construct_mode_toggle)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    resource_bar::resource_bar_system,
                    unit_info::unit_info_system,
                    command_card::command_card_system,
                    ability_bar::ability_bar_system,
                    notifications::notification_system,
                    construct_mode::construct_mode_system,
                    agent_chat::agent_chat_system,
                    game_over::game_over_system,
                ),
            );
    }
}
