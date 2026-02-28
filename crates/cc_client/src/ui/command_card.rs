use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    Building, BuildingKind, Owner, Producer, ProductionQueue, ResearchQueue, Researcher, Selected,
    UnitKind, UnitType, UpgradeType,
};
use cc_core::unit_stats::base_stats;
use cc_core::upgrade_stats::upgrade_stats;
use cc_sim::resources::{CommandQueue, PlayerResources};

use crate::input::InputMode;

const LOCAL_PLAYER: u8 = 0;

/// Bottom-center command card: context-sensitive buttons.
pub fn command_card_system(
    mut contexts: EguiContexts,
    mut cmd_queue: ResMut<CommandQueue>,
    mut input_mode: ResMut<InputMode>,
    player_resources: Res<PlayerResources>,
    selected_units: Query<(Entity, &UnitType, &Owner), With<Selected>>,
    selected_buildings: Query<
        (
            Entity,
            &Building,
            &Owner,
            Option<&Producer>,
            Option<&ProductionQueue>,
            Option<&Researcher>,
            Option<&ResearchQueue>,
        ),
        With<Selected>,
    >,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let has_units = selected_units.iter().count() > 0;
    let has_buildings = selected_buildings.iter().count() > 0;

    if !has_units && !has_buildings {
        return;
    }

    // Check if any selected units are Pawdlers owned by local player
    let has_pawdler = selected_units
        .iter()
        .any(|(_, ut, owner)| ut.kind == UnitKind::Pawdler && owner.player_id == LOCAL_PLAYER);

    let pres = player_resources.players.get(LOCAL_PLAYER as usize);

    egui::Window::new("Commands")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0))
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if has_units {
                    // Unit commands
                    if ui.button("Stop (H)").clicked() {
                        let ids: Vec<EntityId> = selected_units
                            .iter()
                            .map(|(e, _, _)| EntityId(e.to_bits()))
                            .collect();
                        cmd_queue.push(GameCommand::Stop { unit_ids: ids });
                    }

                    if ui.button("Hold (Shift+H)").clicked() {
                        let ids: Vec<EntityId> = selected_units
                            .iter()
                            .map(|(e, _, _)| EntityId(e.to_bits()))
                            .collect();
                        cmd_queue.push(GameCommand::HoldPosition { unit_ids: ids });
                    }

                    let atk_label = if *input_mode == InputMode::AttackMove {
                        "A-Move [ON]"
                    } else {
                        "A-Move (A)"
                    };
                    if ui.button(atk_label).clicked() {
                        *input_mode = if *input_mode == InputMode::AttackMove {
                            InputMode::Normal
                        } else {
                            InputMode::AttackMove
                        };
                    }
                }

                // Build buttons when Pawdler is selected
                if has_pawdler {
                    ui.separator();
                    let buildable = [
                        (BuildingKind::CatTree, "CatTree"),
                        (BuildingKind::FishMarket, "FishMkt"),
                        (BuildingKind::LitterBox, "LitterBox"),
                        (BuildingKind::ServerRack, "SrvRack"),
                        (BuildingKind::ScratchingPost, "ScrPost"),
                        (BuildingKind::CatFlap, "CatFlap"),
                        (BuildingKind::LaserPointer, "Laser"),
                    ];
                    for (kind, label) in buildable {
                        let bstats = building_stats(kind);
                        let can_afford = pres
                            .map(|p| p.food >= bstats.food_cost && p.gpu_cores >= bstats.gpu_cost)
                            .unwrap_or(false);

                        let btn_text = if bstats.gpu_cost > 0 {
                            format!("{} ({}F/{}G)", label, bstats.food_cost, bstats.gpu_cost)
                        } else {
                            format!("{} ({}F)", label, bstats.food_cost)
                        };
                        let btn = egui::Button::new(&btn_text);
                        if ui.add_enabled(can_afford, btn).clicked() {
                            *input_mode = InputMode::BuildPlacement { kind };
                        }
                    }
                }

                if has_buildings {
                    // Building commands — show trainable units, research, cancel
                    for (entity, building, owner, producer, prod_queue, researcher, research_queue) in
                        selected_buildings.iter()
                    {
                        if owner.player_id != LOCAL_PLAYER {
                            continue;
                        }

                        // Production building — show trainable units
                        if producer.is_some() {
                            ui.separator();
                            let trainable = trainable_units(building.kind);
                            for kind in trainable {
                                let ustats = base_stats(*kind);

                                // Check upgrade prerequisites
                                let prereq_met = pres
                                    .map(|p| {
                                        if *kind == UnitKind::Catnapper {
                                            p.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                                        } else if *kind == UnitKind::MechCommander {
                                            p.completed_upgrades.contains(&UpgradeType::MechPrototype)
                                        } else {
                                            true
                                        }
                                    })
                                    .unwrap_or(true);

                                let can_afford = prereq_met && pres
                                    .map(|p| {
                                        p.food >= ustats.food_cost
                                            && p.gpu_cores >= ustats.gpu_cost
                                            && p.supply + ustats.supply_cost <= p.supply_cap
                                    })
                                    .unwrap_or(false);

                                let btn_text = if !prereq_met {
                                    format!("{:?} [LOCKED]", kind)
                                } else {
                                    format!("Train {:?} ({}F/{}S)", kind, ustats.food_cost, ustats.supply_cost)
                                };
                                let btn = egui::Button::new(&btn_text);
                                if ui.add_enabled(can_afford, btn).clicked() {
                                    cmd_queue.push(GameCommand::TrainUnit {
                                        building: EntityId(entity.to_bits()),
                                        unit_kind: *kind,
                                    });
                                }
                            }

                            // Queue status + cancel button
                            if let Some(queue) = prod_queue {
                                if !queue.queue.is_empty() {
                                    ui.separator();
                                    ui.colored_label(
                                        egui::Color32::LIGHT_GRAY,
                                        format!("Q: {}", queue.queue.len()),
                                    );
                                    if ui
                                        .button("Cancel")
                                        .on_hover_text("Cancel front of queue (refunds resources)")
                                        .clicked()
                                    {
                                        cmd_queue.push(GameCommand::CancelQueue {
                                            building: EntityId(entity.to_bits()),
                                        });
                                    }
                                }
                            }
                        }

                        // Research building (ScratchingPost) — show research buttons
                        if researcher.is_some() {
                            ui.separator();
                            let upgrades = [
                                (UpgradeType::SharperClaws, "Claws +2D"),
                                (UpgradeType::ThickerFur, "Fur +25HP"),
                                (UpgradeType::NimblePaws, "Paws +10%S"),
                                (UpgradeType::SiegeTraining, "Siege Trn"),
                                (UpgradeType::MechPrototype, "Mech Proto"),
                            ];
                            for (upgrade, label) in upgrades {
                                let ustats = upgrade_stats(upgrade);
                                let already_done = pres
                                    .map(|p| p.completed_upgrades.contains(&upgrade))
                                    .unwrap_or(false);

                                if already_done {
                                    continue;
                                }

                                let can_afford = pres
                                    .map(|p| {
                                        p.food >= ustats.food_cost
                                            && p.gpu_cores >= ustats.gpu_cost
                                    })
                                    .unwrap_or(false);

                                let btn_text =
                                    format!("{} ({}F/{}G)", label, ustats.food_cost, ustats.gpu_cost);
                                let btn = egui::Button::new(&btn_text);
                                if ui.add_enabled(can_afford, btn).clicked() {
                                    cmd_queue.push(GameCommand::Research {
                                        building: EntityId(entity.to_bits()),
                                        upgrade,
                                    });
                                }
                            }

                            // Research queue status
                            if let Some(rqueue) = research_queue {
                                if !rqueue.queue.is_empty() {
                                    ui.separator();
                                    ui.colored_label(
                                        egui::Color32::LIGHT_GRAY,
                                        format!("R: {}", rqueue.queue.len()),
                                    );
                                    if ui
                                        .button("CancelR")
                                        .on_hover_text("Cancel front of research queue (refunds)")
                                        .clicked()
                                    {
                                        cmd_queue.push(GameCommand::CancelResearch {
                                            building: EntityId(entity.to_bits()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
}

fn trainable_units(kind: BuildingKind) -> &'static [UnitKind] {
    building_stats(kind).can_produce
}
