pub mod hud;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, hud::setup_hud)
            .add_systems(Update, hud::update_hud);
    }
}
