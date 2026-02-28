mod input;
mod renderer;
mod setup;
// mod ui; // TODO: re-enable once bevy_egui context init panic is fixed

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
        .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.10)))
        .add_plugins(SimPlugin)
        .add_plugins(renderer::RenderPlugin)
        .add_plugins(input::InputPlugin)
        // .add_plugins(ui::UiPlugin) // TODO: bevy_egui panics on available_rect() before Context::run()
        .add_plugins(VoicePlugin)
        .add_systems(PreStartup, setup::setup_game)
        .run();
}
