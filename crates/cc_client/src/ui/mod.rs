pub mod command_card;
pub mod game_over;
pub mod hud;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Startup, hud::setup_hud)
            .add_systems(
                Update,
                (
                    hud::update_hud,
                    command_card::command_card_system,
                    game_over::game_over_system,
                ),
            );
    }
}
