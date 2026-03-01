mod input;
mod renderer;
mod setup;
mod ui;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
use cc_agent::AgentPlugin;
use cc_sim::SimPlugin;
#[cfg(feature = "native")]
use cc_voice::VoicePlugin;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ClawedCommand".into(),
                    resolution: (1280u32, 720u32).into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.10)))
    .add_plugins(SimPlugin)
    .add_plugins(renderer::RenderPlugin)
    .add_plugins(input::InputPlugin)
    .add_plugins(ui::UiPlugin)
    .add_systems(
        PreStartup,
        setup::setup_game
            .after(renderer::unit_gen::generate_unit_sprites)
            .after(renderer::resource_nodes::generate_resource_sprites),
    );

    #[cfg(any(feature = "native", feature = "wasm-agent"))]
    app.add_plugins(AgentPlugin);

    #[cfg(feature = "native")]
    app.add_plugins(VoicePlugin);

    app.run();
}
