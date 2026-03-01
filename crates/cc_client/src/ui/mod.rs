pub mod build_menu;
pub mod building_info;
pub mod ability_bar;
pub mod briefing;
pub mod campaign_menu;
pub mod command_card;
pub mod dialogue;
pub mod game_over;
pub mod notifications;
pub mod resource_bar;
pub mod unit_info;

// Agent-dependent UI modules — need cc_agent crate
#[cfg(any(feature = "native", feature = "wasm-agent"))]
pub mod agent_chat;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
pub mod construct_mode;

use bevy::prelude::*;

/// Identifies which player the local client controls.
#[derive(Resource)]
pub struct LocalPlayer(pub u8);

impl Default for LocalPlayer {
    fn default() -> Self {
        Self(0)
    }
}

/// Shared UI state -- notifications, etc.
#[derive(Resource, Default)]
pub struct UiState {
    /// Active toast notifications: (message, remaining_seconds).
    pub notifications: Vec<(String, f32)>,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalPlayer>()
            .init_resource::<UiState>()
            .init_resource::<dialogue::DialogueState>()
            .init_resource::<campaign_menu::AvailableMissions>()
            .init_resource::<campaign_menu::CampaignMenuOpen>()
            .add_systems(
                Startup,
                (
                    resource_bar::spawn_resource_bar,
                    build_menu::spawn_build_menu,
                    building_info::spawn_building_info,
                    unit_info::spawn_unit_info,
                    command_card::spawn_command_card,
                    ability_bar::spawn_ability_bar,
                    notifications::spawn_notifications,
                    game_over::spawn_game_over,
                    dialogue::spawn_dialogue,
                    briefing::spawn_briefing,
                    campaign_menu::spawn_campaign_menu,
                ),
            )
            .add_systems(
                Update,
                (
                    resource_bar::update_resource_bar,
                    build_menu::update_build_menu,
                    building_info::update_building_info,
                    unit_info::update_unit_info,
                    command_card::update_command_card,
                    ability_bar::update_ability_bar,
                    notifications::update_notifications,
                    game_over::update_game_over,
                    dialogue::dialogue_event_reader,
                    dialogue::update_dialogue,
                    briefing::update_briefing,
                    briefing::briefing_input_system,
                    campaign_menu::campaign_menu_toggle,
                    campaign_menu::update_campaign_menu,
                ),
            );

        // Agent-dependent UI systems
        #[cfg(any(feature = "native", feature = "wasm-agent"))]
        {
            app.add_systems(
                Startup,
                (
                    agent_chat::spawn_agent_chat,
                    construct_mode::spawn_construct_mode,
                ),
            )
            .add_systems(
                Update,
                (
                    agent_chat::update_agent_chat,
                    agent_chat::agent_quick_commands,
                    construct_mode::construct_mode_toggle,
                    construct_mode::update_construct_mode,
                    construct_mode::construct_mode_keys,
                ),
            );
        }
    }
}
