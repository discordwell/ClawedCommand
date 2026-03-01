#[cfg(feature = "native")]
pub mod ability_bar;
#[cfg(feature = "native")]
pub mod agent_chat;
#[cfg(feature = "native")]
pub mod briefing;
#[cfg(feature = "native")]
pub mod campaign_menu;
#[cfg(feature = "native")]
pub mod command_card;
#[cfg(feature = "native")]
pub mod construct_mode;
#[cfg(feature = "native")]
pub mod dialogue;
#[cfg(feature = "native")]
pub mod game_over;
#[cfg(feature = "native")]
pub mod notifications;
#[cfg(feature = "native")]
pub mod resource_bar;
#[cfg(feature = "native")]
pub mod unit_info;

use bevy::prelude::*;
#[cfg(feature = "native")]
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
        app.init_resource::<UiState>();

        #[cfg(feature = "native")]
        {
            app.add_plugins(EguiPlugin::default())
                .init_resource::<dialogue::DialogueState>()
                .init_resource::<campaign_menu::AvailableMissions>()
                .init_resource::<campaign_menu::CampaignMenuOpen>()
                .add_systems(
                    Update,
                    (
                        construct_mode::construct_mode_toggle,
                        dialogue::dialogue_event_reader,
                        briefing::briefing_input_system,
                        campaign_menu::campaign_menu_toggle,
                    ),
                )
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
                        dialogue::dialogue_system,
                        briefing::briefing_system,
                        campaign_menu::campaign_menu_system,
                    ),
                );
        }
    }
}
