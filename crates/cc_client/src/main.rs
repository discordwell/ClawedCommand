mod input;
mod renderer;
mod setup;

use bevy::prelude::*;
use cc_sim::SimPlugin;
use cc_voice::VoicePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "ClawedCommand".into(),
                        resolution: (1280u32, 720u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(SimPlugin)
        .add_plugins(renderer::RenderPlugin)
        .add_plugins(input::InputPlugin)
        .add_plugins(VoicePlugin)
        .add_systems(Startup, setup::setup_game)
        .run();
}
