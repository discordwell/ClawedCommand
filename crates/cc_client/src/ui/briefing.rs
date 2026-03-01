use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_sim::campaign::state::{CampaignPhase, CampaignState};

/// Full-screen mission briefing overlay.
/// Shown during CampaignPhase::Briefing, dismissed by clicking "Begin Mission".
pub fn briefing_system(
    mut contexts: EguiContexts,
    mut campaign: ResMut<CampaignState>,
) {
    if campaign.phase != CampaignPhase::Briefing {
        return;
    }

    let Some(mission) = &campaign.current_mission else {
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mission_name = mission.name.clone();
    let act = mission.act;
    let briefing = mission.briefing_text.clone();
    let objectives: Vec<(String, bool)> = mission
        .objectives
        .iter()
        .map(|o| (o.description.clone(), o.primary))
        .collect();

    let mut begin_clicked = false;

    egui::Area::new(egui::Id::new("briefing_overlay"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(10, 10, 20, 230))
                .inner_margin(egui::Margin::same(40))
                .corner_radius(12.0)
                .show(ui, |ui| {
                    ui.set_max_width(500.0);

                    // Act title
                    let act_text = if act == 0 {
                        "PROLOGUE".to_string()
                    } else {
                        format!("ACT {act}")
                    };
                    ui.label(
                        egui::RichText::new(act_text)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(150, 150, 180)),
                    );

                    ui.add_space(8.0);

                    // Mission name
                    ui.label(
                        egui::RichText::new(&mission_name)
                            .heading()
                            .size(24.0)
                            .color(egui::Color32::WHITE),
                    );

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Briefing text
                    ui.label(
                        egui::RichText::new(&briefing)
                            .size(14.0)
                            .color(egui::Color32::LIGHT_GRAY),
                    );

                    ui.add_space(16.0);

                    // Objectives
                    ui.label(
                        egui::RichText::new("OBJECTIVES")
                            .strong()
                            .size(14.0)
                            .color(egui::Color32::from_rgb(255, 200, 100)),
                    );
                    ui.add_space(4.0);

                    for (desc, primary) in &objectives {
                        let prefix = if *primary { ">> " } else { "   " };
                        let color = if *primary {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(150, 150, 150)
                        };
                        let suffix = if *primary { "" } else { " (optional)" };
                        ui.label(
                            egui::RichText::new(format!("{prefix}{desc}{suffix}"))
                                .size(13.0)
                                .color(color),
                        );
                    }

                    ui.add_space(24.0);

                    // Begin button
                    if ui.button(
                        egui::RichText::new("BEGIN MISSION")
                            .size(16.0)
                            .strong(),
                    ).clicked() {
                        begin_clicked = true;
                    }
                });
        });

    if begin_clicked {
        campaign.phase = CampaignPhase::InMission;
    }
}

/// Transition from Briefing to InMission when the player presses Enter or clicks Begin.
pub fn briefing_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut campaign: ResMut<CampaignState>,
) {
    if campaign.phase != CampaignPhase::Briefing {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        campaign.phase = CampaignPhase::InMission;
    }
}
