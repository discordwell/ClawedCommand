mod input;
mod renderer;
mod setup;
mod ui;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
use cc_agent::AgentPlugin;
use cc_sim::SimPlugin;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};
#[cfg(feature = "native")]
use cc_voice::VoicePlugin;

fn main() {
    let mut app = App::new();

    // Check for --demo flag
    let demo_mode = std::env::args().any(|arg| arg == "--demo");

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
    .insert_resource(ClearColor(Color::srgb(0.06, 0.06, 0.10)));

    // If --demo, load mission RON and insert CampaignState BEFORE SimPlugin
    // so that init_resource in CampaignPlugin sees it already exists.
    if demo_mode {
        let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/campaign/demo_canyon.ron");
        let ron_str = std::fs::read_to_string(&ron_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", ron_path.display()));
        let mission: cc_core::mission::MissionDefinition = ron::from_str(&ron_str)
            .unwrap_or_else(|e| panic!("Failed to parse demo_canyon.ron: {e}"));
        if let Err(errors) = mission.validate() {
            panic!("Demo mission validation failed: {errors:?}");
        }
        let mut campaign = CampaignState::default();
        campaign.load_mission(mission);
        campaign.phase = CampaignPhase::InMission;
        app.insert_resource(campaign);

        // Load demo combat script for both players
        let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/scripts/demo_combat.lua");
        match std::fs::read_to_string(&script_path) {
            Ok(script_source) => {
                use cc_agent::events::ScriptRegistration;
                use cc_agent::runner::ScriptRegistry;

                let mut registry = ScriptRegistry::default();
                for player_id in 0..2u8 {
                    let mut reg = ScriptRegistration::new(
                        format!("demo_combat_p{player_id}"),
                        script_source.clone(),
                        vec!["on_tick".to_string()],
                        player_id,
                    );
                    reg.tick_interval = 3;
                    registry.register(reg);
                }
                app.insert_resource(registry);
            }
            Err(e) => {
                eprintln!("Warning: failed to load demo combat script at {}: {e}", script_path.display());
            }
        }
    }

    app.add_plugins(SimPlugin)
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
