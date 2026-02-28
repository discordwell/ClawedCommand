use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::components::{
    AttackStats, Building, Health, ProductionQueue, RallyPoint, Selected, UnderConstruction,
    UnitType,
};
use cc_core::unit_stats::base_stats;

/// Bottom panel: selected unit/building stats.
pub fn unit_info_system(
    mut contexts: EguiContexts,
    selected_units: Query<(&UnitType, &Health, &AttackStats), With<Selected>>,
    selected_buildings: Query<
        (
            &Building,
            &Health,
            Option<&UnderConstruction>,
            Option<&ProductionQueue>,
            Option<&RallyPoint>,
        ),
        With<Selected>,
    >,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let unit_count = selected_units.iter().count();
    let building_count = selected_buildings.iter().count();

    egui::TopBottomPanel::bottom("unit_info").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if unit_count == 0 && building_count == 0 {
                ui.colored_label(egui::Color32::GRAY, "No selection");
                return;
            }

            // Single unit selected
            if unit_count == 1 && building_count == 0 {
                if let Ok((unit_type, health, attack)) = selected_units.single() {
                    let hp_cur: f32 = health.current.to_num();
                    let hp_max: f32 = health.max.to_num();
                    let hp_pct = hp_cur / hp_max;
                    let dmg: f32 = attack.damage.to_num();
                    let rng: f32 = attack.range.to_num();

                    ui.colored_label(egui::Color32::WHITE, format!("{:?}", unit_type.kind));
                    ui.separator();

                    let hp_color = if hp_pct > 0.5 {
                        egui::Color32::GREEN
                    } else if hp_pct > 0.25 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(hp_color, format!("HP: {:.0}/{:.0}", hp_cur, hp_max));
                    ui.separator();
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 150, 100),
                        format!("ATK: {:.0}", dmg),
                    );
                    ui.colored_label(
                        egui::Color32::from_rgb(150, 200, 255),
                        format!("RNG: {:.1}", rng),
                    );
                }
                return;
            }

            // Single building selected
            if building_count == 1 && unit_count == 0 {
                if let Ok((building, health, under_construction, prod_queue, rally)) =
                    selected_buildings.single()
                {
                    let hp_cur: f32 = health.current.to_num();
                    let hp_max: f32 = health.max.to_num();
                    let hp_pct = hp_cur / hp_max;

                    ui.colored_label(
                        egui::Color32::from_rgb(200, 200, 255),
                        format!("{:?}", building.kind),
                    );
                    ui.separator();

                    let hp_color = if hp_pct > 0.5 {
                        egui::Color32::GREEN
                    } else if hp_pct > 0.25 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(hp_color, format!("HP: {:.0}/{:.0}", hp_cur, hp_max));

                    // Construction progress
                    if let Some(uc) = under_construction {
                        let progress = if uc.total_ticks > 0 {
                            1.0 - (uc.remaining_ticks as f32 / uc.total_ticks as f32)
                        } else {
                            1.0
                        };
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 50),
                            format!("Building... {:.0}%", progress * 100.0),
                        );
                        return;
                    }

                    // Production queue
                    if let Some(queue) = prod_queue {
                        if let Some((kind, ticks_remaining)) = queue.queue.front() {
                            let stats = base_stats(*kind);
                            let total_secs = stats.train_time as f32 / 10.0;
                            let remaining_secs = *ticks_remaining as f32 / 10.0;
                            let elapsed_secs = total_secs - remaining_secs;
                            ui.separator();
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 200, 255),
                                format!(
                                    "Training: {:?} {:.0}/{:.0}s",
                                    kind, elapsed_secs, total_secs
                                ),
                            );
                            let queued = queue.queue.len() - 1;
                            if queued > 0 {
                                ui.colored_label(
                                    egui::Color32::LIGHT_GRAY,
                                    format!("(+{} queued)", queued),
                                );
                            }
                        } else {
                            ui.separator();
                            ui.colored_label(egui::Color32::GRAY, "Idle");
                        }
                    }

                    // Rally point indicator
                    if let Some(rally) = rally {
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 255, 150),
                            format!("Rally: ({},{})", rally.target.x, rally.target.y),
                        );
                    }
                }
                return;
            }

            // Multi-select: show count breakdown
            if unit_count > 1 && building_count == 0 {
                // Unit type breakdown
                use std::collections::HashMap;
                let mut type_counts: HashMap<cc_core::components::UnitKind, u32> = HashMap::new();
                for (unit_type, _, _) in selected_units.iter() {
                    *type_counts.entry(unit_type.kind).or_insert(0) += 1;
                }
                let mut parts: Vec<String> = type_counts
                    .iter()
                    .map(|(kind, count)| format!("{}x {:?}", count, kind))
                    .collect();
                parts.sort();
                ui.colored_label(
                    egui::Color32::WHITE,
                    format!("{} selected: {}", unit_count, parts.join(", ")),
                );
            } else {
                // Mixed or multi-building selection
                let mut label_parts = Vec::new();
                if unit_count > 0 {
                    label_parts.push(format!("{} unit{}", unit_count, if unit_count > 1 { "s" } else { "" }));
                }
                if building_count > 0 {
                    label_parts.push(format!(
                        "{} building{}",
                        building_count,
                        if building_count > 1 { "s" } else { "" }
                    ));
                }
                ui.colored_label(egui::Color32::WHITE, label_parts.join(" + "));
            }
        });
    });
}
