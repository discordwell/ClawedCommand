use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::components::{AttackStats, Health, Selected, UnitType};

/// Bottom-left panel: selected unit stats.
pub fn unit_info_system(
    mut contexts: EguiContexts,
    selected: Query<(&UnitType, &Health, &AttackStats), With<Selected>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::TopBottomPanel::bottom("unit_info").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let count = selected.iter().count();
            if count == 0 {
                ui.colored_label(egui::Color32::GRAY, "No unit selected");
                return;
            }

            if count == 1 {
                if let Ok((unit_type, health, attack)) = selected.single() {
                    let hp_cur: f32 = health.current.to_num();
                    let hp_max: f32 = health.max.to_num();
                    let hp_pct = hp_cur / hp_max;
                    let dmg: f32 = attack.damage.to_num();
                    let rng: f32 = attack.range.to_num();

                    ui.colored_label(egui::Color32::WHITE, format!("{:?}", unit_type.kind));
                    ui.separator();

                    // HP bar
                    let hp_color = if hp_pct > 0.5 {
                        egui::Color32::GREEN
                    } else if hp_pct > 0.25 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(hp_color, format!("HP: {:.0}/{:.0}", hp_cur, hp_max));
                    ui.separator();
                    ui.colored_label(egui::Color32::from_rgb(255, 150, 100), format!("ATK: {:.0}", dmg));
                    ui.colored_label(egui::Color32::from_rgb(150, 200, 255), format!("RNG: {:.1}", rng));
                }
            } else {
                ui.colored_label(egui::Color32::WHITE, format!("{} units selected", count));
            }
        });
    });
}
