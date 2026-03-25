pub mod ability_bar;
pub mod act_title_card;
pub mod briefing;
pub mod build_menu;
pub mod building_info;
pub mod campaign_menu;
pub mod campaign_save;
pub mod cinematic;
pub mod command_card;
pub mod debrief;
pub mod dialogue;
pub mod game_over;
pub mod notifications;
pub mod resource_bar;
pub mod unit_info;
pub mod world_map;

// Agent-dependent UI modules — need cc_agent crate
#[cfg(any(feature = "native", feature = "wasm-agent"))]
pub mod agent_chat;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
pub mod construct_mode;
#[cfg(any(feature = "native", feature = "wasm-agent"))]
pub mod prompt_overlay;

use bevy::prelude::*;

/// Identifies which player the local client controls.
#[derive(Resource, Default)]
pub struct LocalPlayer(pub u8);

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
            .init_resource::<dialogue::PortraitHandles>()
            .init_resource::<campaign_menu::AvailableMissions>()
            .init_resource::<campaign_menu::CampaignMenuOpen>()
            .init_resource::<debrief::DebriefState>()
            .init_resource::<act_title_card::ActTitleTimer>()
            .init_resource::<briefing::BriefingTypewriter>()
            .init_resource::<campaign_save::PreviousCampaignPhase>()
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
                    debrief::spawn_debrief,
                    act_title_card::spawn_act_title_card,
                    world_map::spawn_world_map,
                    campaign_save::load_campaign_save,
                    load_campaign_missions,
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
            )
            .add_systems(
                Update,
                (
                    debrief::update_debrief,
                    debrief::debrief_typewriter,
                    debrief::debrief_interaction,
                    act_title_card::update_act_title_card,
                    act_title_card::act_title_input.after(act_title_card::update_act_title_card),
                    world_map::update_world_map,
                    world_map::world_map_interaction,
                    world_map::world_map_input,
                    cinematic::animate_fade_in,
                    cinematic::animate_slide_in,
                    campaign_save::auto_save_campaign,
                ),
            );

        // Agent-dependent UI systems
        #[cfg(any(feature = "native", feature = "wasm-agent"))]
        {
            app.init_resource::<prompt_overlay::ScriptManagerExpanded>()
                .add_systems(
                    Startup,
                    (
                        agent_chat::spawn_agent_chat,
                        construct_mode::spawn_construct_mode,
                        prompt_overlay::spawn_prompt_overlay,
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
                        prompt_overlay::prompt_overlay_visibility,
                        prompt_overlay::prompt_text_input,
                        prompt_overlay::update_prompt_display,
                        prompt_overlay::update_undo_toast,
                    ),
                );
        }
    }
}

/// Startup system: scan the campaign directory and load all RON mission files.
fn load_campaign_missions(mut available: ResMut<campaign_menu::AvailableMissions>) {
    let campaign_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/campaign");

    let Ok(entries) = std::fs::read_dir(&campaign_dir) else {
        warn!(
            "Could not read campaign directory: {}",
            campaign_dir.display()
        );
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "ron") {
            // Skip demo files
            let filename = path.file_stem().unwrap_or_default().to_string_lossy();
            if filename.starts_with("demo_") {
                continue;
            }

            match std::fs::read_to_string(&path) {
                Ok(ron_str) => {
                    match ron::from_str::<cc_core::mission::MissionDefinition>(&ron_str) {
                        Ok(mission) => {
                            available.missions.push(mission);
                        }
                        Err(e) => {
                            warn!("Failed to parse {}: {e}", path.display());
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read {}: {e}", path.display());
                }
            }
        }
    }

    // Sort by act then mission_index
    available.missions.sort_by(|a, b| {
        a.act
            .cmp(&b.act)
            .then(a.mission_index.cmp(&b.mission_index))
    });

    info!("Loaded {} campaign missions", available.missions.len());
}
