use bevy::prelude::*;

use bevy_egui::{EguiContexts, egui};

use cc_core::mission::MissionDefinition;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

/// Resource: available campaign missions loaded from RON files.
#[derive(Resource, Default)]
pub struct AvailableMissions {
    pub missions: Vec<MissionDefinition>,
}

/// Resource: controls whether the campaign menu is open.
#[derive(Resource)]
pub struct CampaignMenuOpen(pub bool);

impl Default for CampaignMenuOpen {
    fn default() -> Self {
        Self(false)
    }
}

/// Toggle campaign menu with Escape when no campaign is active.
pub fn campaign_menu_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu_open: ResMut<CampaignMenuOpen>,
    campaign: Res<CampaignState>,
) {
    if campaign.phase != CampaignPhase::Inactive {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        menu_open.0 = !menu_open.0;
    }
}

/// Render the campaign mission select screen.
pub fn campaign_menu_system(
    mut contexts: EguiContexts,
    menu_open: Res<CampaignMenuOpen>,
    available: Res<AvailableMissions>,
    mut campaign: ResMut<CampaignState>,
) {
    if !menu_open.0 {
        return;
    }

    if campaign.phase != CampaignPhase::Inactive {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mut selected_mission: Option<MissionDefinition> = None;

    egui::Area::new(egui::Id::new("campaign_menu"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(15, 15, 30, 240))
                .inner_margin(egui::Margin::same(30))
                .corner_radius(12.0)
                .show(ui, |ui| {
                    ui.set_max_width(400.0);

                    ui.label(
                        egui::RichText::new("CAMPAIGN")
                            .heading()
                            .size(20.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if available.missions.is_empty() {
                        ui.label(
                            egui::RichText::new("No missions available.")
                                .size(14.0)
                                .color(egui::Color32::GRAY),
                        );
                    }

                    for mission in &available.missions {
                        let completed = campaign.completed_missions.contains(&mission.id);
                        let label = if completed {
                            format!("[DONE] {} — {}", mission.name, mission.briefing_text.chars().take(60).collect::<String>())
                        } else {
                            format!("{} — {}", mission.name, mission.briefing_text.chars().take(60).collect::<String>())
                        };

                        let color = if completed {
                            egui::Color32::from_rgb(100, 200, 100)
                        } else {
                            egui::Color32::WHITE
                        };

                        let btn = ui.button(egui::RichText::new(&label).size(13.0).color(color));
                        if btn.clicked() {
                            selected_mission = Some(mission.clone());
                        }
                        ui.add_space(4.0);
                    }
                });
        });

    // Load selected mission
    if let Some(mission) = selected_mission {
        campaign.load_mission(mission);
    }
}
