mod cutscene;
mod input;
mod renderer;
mod setup;
mod showcase;
mod ui;
mod voice_demo;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
use cc_agent::AgentPlugin;
use cc_sim::SimPlugin;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};
#[cfg(feature = "native")]
use cc_voice::VoicePlugin;

/// Parsed demo mode from CLI arguments.
#[derive(Debug, Clone)]
enum DemoMode {
    /// Canyon battle (original --demo behavior).
    Canyon(u8),
    /// Building showcase (original --showcase).
    Showcase,
    /// Cutscene dialogue demo (1, 2, or 3).
    Cutscene(u8),
    /// Voice command demo.
    Voice,
    /// AI mirror match — two scripted AI armies fight each other.
    Match,
}

/// Parse `--demo <mode>` from CLI args.
/// Supports: `--demo` (canyon 1), `--demo canyon [N]`, `--demo showcase`,
/// `--demo cutscene [N]`, `--demo N` (canyon N for backward compat),
/// `--showcase` (legacy alias for `--demo showcase`).
fn parse_demo_mode() -> Option<DemoMode> {
    let args: Vec<String> = std::env::args().collect();

    for (i, arg) in args.iter().enumerate() {
        if arg == "--demo" {
            let next = args.get(i + 1).map(|s| s.as_str());
            return Some(match next {
                Some("canyon") => {
                    let n = args.get(i + 2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
                    DemoMode::Canyon(n.clamp(1, 3))
                }
                Some("showcase") => DemoMode::Showcase,
                Some("cutscene") => {
                    let n = args.get(i + 2).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
                    DemoMode::Cutscene(n.clamp(1, 3))
                }
                Some("voice") => DemoMode::Voice,
                Some("match") => DemoMode::Match,
                Some("4") => DemoMode::Match,
                Some(n) if n.parse::<u8>().is_ok() => {
                    DemoMode::Canyon(n.parse::<u8>().unwrap().clamp(1, 3))
                }
                _ => DemoMode::Canyon(1),
            });
        }
        if arg == "--showcase" {
            return Some(DemoMode::Showcase);
        }
    }

    None
}

fn main() {
    let mut app = App::new();

    let demo_mode = parse_demo_mode();

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

    match &demo_mode {
        Some(DemoMode::Canyon(scenario)) => {
            setup_canyon(&mut app, *scenario);
        }
        Some(DemoMode::Showcase) => {
            setup_showcase(&mut app);
        }
        Some(DemoMode::Cutscene(scenario)) => {
            setup_cutscene(&mut app, *scenario);
        }
        Some(DemoMode::Voice) => {
            setup_voice_demo(&mut app);
        }
        Some(DemoMode::Match) => {
            setup_match(&mut app);
        }
        None => {
            // No demo mode — normal game startup
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
        )
        .add_systems(
            Update,
            voice_demo::voice_demo_buff_tint
                .after(renderer::selection::render_selection_indicators),
        );

    #[cfg(any(feature = "native", feature = "wasm-agent"))]
    app.add_plugins(AgentPlugin);

    #[cfg(feature = "native")]
    app.add_plugins(VoicePlugin);

    app.run();
}

/// Set up the canyon battle demo (original --demo behavior).
fn setup_canyon(app: &mut App, scenario: u8) {
    let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/campaign/demo_canyon.ron");
    let ron_str = std::fs::read_to_string(&ron_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", ron_path.display()));
    let mut mission: cc_core::mission::MissionDefinition = ron::from_str(&ron_str)
        .unwrap_or_else(|e| panic!("Failed to parse demo_canyon.ron: {e}"));

    // Scenario 3: inject hero units for both players
    if scenario == 3 {
        use cc_core::hero::HeroId;
        use cc_core::mission::HeroSpawn;
        use cc_core::coords::GridPos;

        // The Eternal (Croak hero) — near P0's base
        mission.player_setup.heroes.push(HeroSpawn {
            hero_id: HeroId::TheEternal,
            position: GridPos::new(12, 12),
            mission_critical: false,
            player_id: 0,
        });
        // King Ringtail (LLAMA hero) — near P1's base
        mission.player_setup.heroes.push(HeroSpawn {
            hero_id: HeroId::KingRingtail,
            position: GridPos::new(68, 36),
            mission_critical: false,
            player_id: 1,
        });
    }

    load_demo_mission(app, mission, "Canyon demo");

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
        1 => None,
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

/// Validate a mission, load it into a new CampaignState, and insert it as a resource.
fn load_demo_mission(app: &mut App, mission: cc_core::mission::MissionDefinition, label: &str) {
    if let Err(errors) = mission.validate() {
        panic!("{label} mission validation failed: {errors:?}");
    }
    let mut campaign = CampaignState::default();
    campaign.load_mission(mission);
    campaign.phase = CampaignPhase::InMission;
    app.insert_resource(campaign);
}

/// Build a 6-player resource set with max resources (for showcase/cutscene demos).
fn demo_player_resources() -> cc_sim::resources::PlayerResources {
    cc_sim::resources::PlayerResources {
        players: (0..6)
            .map(|_| {
                let mut p = cc_sim::resources::PlayerResourceState::default();
                p.food = 9999;
                p.gpu_cores = 9999;
                p.supply_cap = 100;
                p
            })
            .collect(),
    }
}

/// Set up the building showcase demo.
fn setup_showcase(app: &mut App) {
    let mission = showcase::build_showcase_mission();
    load_demo_mission(app, mission, "Showcase");

    app.insert_resource(demo_player_resources());
}

/// Set up a cutscene dialogue demo.
fn setup_cutscene(app: &mut App, scenario: u8) {
    eprintln!("Cutscene scenario {scenario}");

    let mission = cutscene::build_cutscene_mission(scenario);
    load_demo_mission(app, mission, "Cutscene");

    // Insert camera override for tight cutscene framing
    app.insert_resource(cutscene::cutscene_camera());

    app.insert_resource(demo_player_resources());
}

/// Set up the voice command demo.
fn setup_voice_demo(app: &mut App) {
    eprintln!("Voice command demo");

    let mission = voice_demo::build_voice_demo_mission();
    load_demo_mission(app, mission, "Voice demo");

    // Camera override for the voice demo
    app.insert_resource(voice_demo::voice_demo_camera());

    app.insert_resource(demo_player_resources());
    app.insert_resource(voice_demo::VoiceDemoState::default());

    // Register demo systems (buff tint registered globally)
    app.add_systems(Update, voice_demo::voice_demo_system);
}

/// Set up the AI mirror match demo — two CatGPT armies with Gen 42 scripts.
fn setup_match(app: &mut App) {
    eprintln!("AI Mirror Match demo");

    let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/campaign/demo_match.ron");
    let ron_str = std::fs::read_to_string(&ron_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", ron_path.display()));
    let mission: cc_core::mission::MissionDefinition = ron::from_str(&ron_str)
        .unwrap_or_else(|e| panic!("Failed to parse demo_match.ron: {e}"));

    load_demo_mission(app, mission, "AI Mirror Match");

    // Load Gen 42 script for both players
    use cc_agent::events::ScriptRegistration;
    use cc_agent::runner::ScriptRegistry;

    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../training/arena/gen_042/player_1/combat_micro.lua");
    let script_source = std::fs::read_to_string(&script_path)
        .unwrap_or_else(|e| panic!("Failed to read gen_042 combat_micro.lua: {e}"));

    let mut registry = ScriptRegistry::default();

    // Register for P0
    let mut p0_reg = ScriptRegistration::new(
        "combat_micro_p0".to_string(),
        script_source.clone(),
        vec!["on_tick".to_string()],
        0,
    );
    p0_reg.tick_interval = 3;
    registry.register(p0_reg);

    // Register for P1
    let mut p1_reg = ScriptRegistration::new(
        "combat_micro_p1".to_string(),
        script_source,
        vec!["on_tick".to_string()],
        1,
    );
    p1_reg.tick_interval = 3;
    registry.register(p1_reg);

    app.insert_resource(registry);

    // Populate MultiAiState with two CatGPT AIs so the FSM runs for both players
    use cc_core::components::Faction;
    use cc_core::coords::GridPos;
    use cc_sim::ai::MultiAiState;
    use cc_sim::ai::fsm::{AiState, faction_map};

    let multi_ai = MultiAiState {
        players: vec![
            AiState {
                player_id: 0,
                fmap: faction_map(Faction::CatGpt),
                enemy_spawn: Some(GridPos::new(70, 38)),
                ..Default::default()
            },
            AiState {
                player_id: 1,
                fmap: faction_map(Faction::CatGpt),
                enemy_spawn: Some(GridPos::new(10, 10)),
                ..Default::default()
            },
        ],
    };
    app.insert_resource(multi_ai);

    // Give both players resources for the FSM to work with
    let player_res = cc_sim::resources::PlayerResources {
        players: vec![
            {
                let mut p = cc_sim::resources::PlayerResourceState::default();
                p.food = 500;
                p.gpu_cores = 200;
                p.supply_cap = 50;
                p
            },
            {
                let mut p = cc_sim::resources::PlayerResourceState::default();
                p.food = 500;
                p.gpu_cores = 200;
                p.supply_cap = 50;
                p
            },
        ],
    };
    app.insert_resource(player_res);
}
