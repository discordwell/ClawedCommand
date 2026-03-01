mod input;
mod renderer;
mod setup;
mod showcase;
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

    // Check for CLI flags
    // --demo or --demo N (N=1/2/3, default 1)
    let demo_scenario = {
        let args: Vec<String> = std::env::args().collect();
        let mut scenario: Option<u8> = None;
        for (i, arg) in args.iter().enumerate() {
            if arg == "--demo" {
                // Check if next arg is a number
                if let Some(next) = args.get(i + 1) {
                    if let Ok(n) = next.parse::<u8>() {
                        scenario = Some(n.clamp(1, 3));
                    } else {
                        scenario = Some(1);
                    }
                } else {
                    scenario = Some(1);
                }
            }
        }
        scenario
    };
    let demo_mode = demo_scenario.is_some();
    let showcase_mode = std::env::args().any(|arg| arg == "--showcase");

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

        // Load scenario-specific scripts
        // S1: P0 → cat_formation, P1 → no script (just AttackMove from wave spawner)
        // S2: P0 → cat_formation, P1 → clawed_formation
        // S3: P0 → cat_formation, P1 → clawed_advanced
        let scenario = demo_scenario.unwrap_or(1);
        eprintln!("Demo scenario {scenario}");

        use cc_agent::events::ScriptRegistration;
        use cc_agent::runner::ScriptRegistry;

        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/scripts");
        let mut registry = ScriptRegistry::default();

        // P0 always gets cat_formation
        let p0_script_path = scripts_dir.join("cat_formation.lua");
        match std::fs::read_to_string(&p0_script_path) {
            Ok(source) => {
                let mut reg = ScriptRegistration::new(
                    "cat_formation_p0".to_string(),
                    source,
                    vec!["on_tick".to_string()],
                    0,
                );
                reg.tick_interval = 3;
                registry.register(reg);
            }
            Err(e) => {
                eprintln!("Warning: failed to load cat_formation.lua: {e}");
            }
        }

        // P1 script depends on scenario
        let p1_script_name = match scenario {
            1 => None, // No script — P1 relies on AttackMove from wave spawner
            2 => Some("clawed_formation.lua"),
            3 => Some("clawed_advanced.lua"),
            _ => None,
        };

        if let Some(script_file) = p1_script_name {
            let p1_script_path = scripts_dir.join(script_file);
            match std::fs::read_to_string(&p1_script_path) {
                Ok(source) => {
                    let mut reg = ScriptRegistration::new(
                        format!("{}_p1", script_file.trim_end_matches(".lua")),
                        source,
                        vec!["on_tick".to_string()],
                        1,
                    );
                    reg.tick_interval = 3;
                    registry.register(reg);
                }
                Err(e) => {
                    eprintln!("Warning: failed to load {script_file}: {e}");
                }
            }
        }

        app.insert_resource(registry);

        // Scenario 3: give P1 extra GPU for abilities
        if scenario == 3 {
            let player_res = cc_sim::resources::PlayerResources {
                players: vec![
                    cc_sim::resources::PlayerResourceState::default(),
                    {
                        let mut p = cc_sim::resources::PlayerResourceState::default();
                        p.gpu_cores = 200;
                        p
                    },
                ],
            };
            app.insert_resource(player_res);
        }
    }

    // If --showcase, build all-factions building showcase mission
    if showcase_mode && !demo_mode {
        let mission = showcase::build_showcase_mission();
        if let Err(errors) = mission.validate() {
            panic!("Showcase mission validation failed: {errors:?}");
        }
        let mut campaign = CampaignState::default();
        campaign.load_mission(mission);
        campaign.phase = CampaignPhase::InMission;
        app.insert_resource(campaign);

        // 6-player resources (one per faction) — inserted before SimPlugin
        // so that init_resource in SimSystemsPlugin is a no-op.
        let player_res = cc_sim::resources::PlayerResources {
            players: (0..6)
                .map(|_| {
                    let mut p = cc_sim::resources::PlayerResourceState::default();
                    p.food = 9999;
                    p.gpu_cores = 9999;
                    p.supply_cap = 100;
                    p
                })
                .collect(),
        };
        app.insert_resource(player_res);
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
