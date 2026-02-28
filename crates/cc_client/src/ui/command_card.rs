use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Building, BuildingKind, Owner, Producer, Selected, UnitKind, UnitType};
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
    selected_buildings: Query<(Entity, &Building, &Owner, Option<&Producer>), With<Selected>>,
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
                    ];
                    for (kind, label) in buildable {
                        let bstats = building_stats(kind);
                        let can_afford = pres
                            .map(|p| p.food >= bstats.food_cost && p.gpu_cores >= bstats.gpu_cost)
                            .unwrap_or(false);

                        let btn_text = format!("{} ({}F)", label, bstats.food_cost);
                        let btn = egui::Button::new(&btn_text);
                        if ui.add_enabled(can_afford, btn).clicked() {
                            *input_mode = InputMode::BuildPlacement { kind };
                        }
                    }
                }

                if has_buildings {
                    // Building commands — show trainable units
                    for (entity, building, owner, producer) in selected_buildings.iter() {
                        if owner.player_id != LOCAL_PLAYER || producer.is_none() {
                            continue;
                        }

                        ui.separator();
                        let trainable = trainable_units(building.kind);
                        for kind in trainable {
                            if ui.button(format!("Train {:?}", kind)).clicked() {
                                cmd_queue.push(GameCommand::TrainUnit {
                                    building: EntityId(entity.to_bits()),
                                    unit_kind: *kind,
                                });
                            }
                        }
                    }
                }
            });
        });
}

fn trainable_units(kind: BuildingKind) -> &'static [UnitKind] {
    match kind {
        BuildingKind::TheBox => &[UnitKind::Pawdler],
        BuildingKind::CatTree => &[
            UnitKind::Nuisance,
            UnitKind::Hisser,
            UnitKind::Chonk,
            UnitKind::Yowler,
        ],
        _ => &[],
    }
}
