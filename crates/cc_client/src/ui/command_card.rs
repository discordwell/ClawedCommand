use bevy::prelude::*;

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

/// Marker for the command card root node.
#[derive(Component)]
pub struct CommandCardRoot;

/// Marker for the command card text (compact text display of available commands).
#[derive(Component)]
pub struct CommandCardText;

pub fn spawn_command_card(mut commands: Commands) {
    commands
        .spawn((
            CommandCardRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(30.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-220.0)),
                width: Val::Px(440.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                CommandCardText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
        });
}

pub fn update_command_card(
    mut cmd_queue: ResMut<CommandQueue>,
    mut input_mode: ResMut<InputMode>,
    player_resources: Res<PlayerResources>,
    keys: Res<ButtonInput<KeyCode>>,
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
    mut root_vis: Query<
        &mut Visibility,
        (
            With<CommandCardRoot>,
            Without<CommandCardText>,
        ),
    >,
    mut text_q: Query<&mut Text, With<CommandCardText>>,
) {
    let has_units = selected_units.iter().count() > 0;
    let has_buildings = selected_buildings.iter().count() > 0;

    let show = has_units || has_buildings;
    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        return;
    }

    let pres = player_resources.players.get(LOCAL_PLAYER as usize);
    let mut lines = Vec::new();

    // Unit commands — show available hotkeys
    if has_units {
        let atk_mode = if *input_mode == InputMode::AttackMove {
            "[ON]"
        } else {
            ""
        };
        lines.push(format!(
            "UNIT: [H] Stop  [Shift+H] Hold  [A] A-Move {}",
            atk_mode
        ));

        // Keyboard handling for command card
        if keys.just_pressed(KeyCode::KeyA) {
            *input_mode = if *input_mode == InputMode::AttackMove {
                InputMode::Normal
            } else {
                InputMode::AttackMove
            };
        }
    }

    // Pawdler build menu is handled by build_menu.rs, so just show hint
    let has_pawdler = selected_units
        .iter()
        .any(|(_, ut, owner)| ut.kind == UnitKind::Pawdler && owner.player_id == LOCAL_PLAYER);
    if has_pawdler {
        lines.push("[B] Open Build Menu".to_string());
    }

    // Building commands
    if has_buildings {
        for (entity, building, owner, producer, prod_queue, researcher, research_queue) in
            selected_buildings.iter()
        {
            if owner.player_id != LOCAL_PLAYER {
                continue;
            }

            // Production building
            if producer.is_some() {
                let trainable = building_stats(building.kind).can_produce;
                let keys_list = ["Q", "W", "E", "R"];
                let mut train_parts = Vec::new();
                for (i, &kind) in trainable.iter().enumerate() {
                    let key = keys_list.get(i).unwrap_or(&"?");
                    let ustats = base_stats(kind);
                    let prereq_met = pres
                        .map(|p| {
                            if kind == UnitKind::Catnapper {
                                p.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                            } else if kind == UnitKind::MechCommander {
                                p.completed_upgrades.contains(&UpgradeType::MechPrototype)
                            } else {
                                true
                            }
                        })
                        .unwrap_or(true);

                    let can_afford = prereq_met
                        && pres
                            .map(|p| {
                                p.food >= ustats.food_cost
                                    && p.gpu_cores >= ustats.gpu_cost
                                    && p.supply + ustats.supply_cost <= p.supply_cap
                            })
                            .unwrap_or(false);

                    let marker = if !prereq_met {
                        " LOCKED"
                    } else if !can_afford {
                        "*"
                    } else {
                        ""
                    };
                    train_parts.push(format!(
                        "[{key}] {:?} {}f {}sup{marker}",
                        kind, ustats.food_cost, ustats.supply_cost
                    ));
                }
                lines.push(format!("TRAIN: {}", train_parts.join("  ")));

                // Handle training hotkeys
                for (i, &kind) in trainable.iter().enumerate() {
                    let key = match i {
                        0 => KeyCode::KeyQ,
                        1 => KeyCode::KeyW,
                        2 => KeyCode::KeyE,
                        3 => KeyCode::KeyR,
                        _ => continue,
                    };
                    if keys.just_pressed(key) {
                        cmd_queue.push(GameCommand::TrainUnit {
                            building: EntityId(entity.to_bits()),
                            unit_kind: kind,
                        });
                    }
                }

                // Queue status
                if let Some(queue) = prod_queue {
                    if !queue.queue.is_empty() {
                        lines.push(format!("Queue: {} — [X] Cancel", queue.queue.len()));
                        if keys.just_pressed(KeyCode::KeyX) {
                            cmd_queue.push(GameCommand::CancelQueue {
                                building: EntityId(entity.to_bits()),
                            });
                        }
                    }
                }
            }

            // Research building
            if researcher.is_some() {
                let upgrades = [
                    (UpgradeType::SharperClaws, "Claws+2D", KeyCode::Digit1),
                    (UpgradeType::ThickerFur, "Fur+25HP", KeyCode::Digit2),
                    (UpgradeType::NimblePaws, "Paws+10%S", KeyCode::Digit3),
                    (UpgradeType::SiegeTraining, "SiegeTrn", KeyCode::Digit4),
                    (UpgradeType::MechPrototype, "MechProto", KeyCode::Digit5),
                ];
                let mut research_parts = Vec::new();
                for (upgrade, label, key) in &upgrades {
                    let already_done = pres
                        .map(|p| p.completed_upgrades.contains(upgrade))
                        .unwrap_or(false);
                    if already_done {
                        continue;
                    }
                    let ustats = upgrade_stats(*upgrade);
                    let can_afford = pres
                        .map(|p| p.food >= ustats.food_cost && p.gpu_cores >= ustats.gpu_cost)
                        .unwrap_or(false);
                    let marker = if !can_afford { "*" } else { "" };
                    let key_num = match key {
                        KeyCode::Digit1 => "1",
                        KeyCode::Digit2 => "2",
                        KeyCode::Digit3 => "3",
                        KeyCode::Digit4 => "4",
                        KeyCode::Digit5 => "5",
                        _ => "?",
                    };
                    research_parts
                        .push(format!("[{key_num}] {label} {}f/{}g{marker}", ustats.food_cost, ustats.gpu_cost));

                    if keys.just_pressed(*key) && can_afford {
                        cmd_queue.push(GameCommand::Research {
                            building: EntityId(entity.to_bits()),
                            upgrade: *upgrade,
                        });
                    }
                }
                if !research_parts.is_empty() {
                    lines.push(format!("RESEARCH: {}", research_parts.join("  ")));
                }

                if let Some(rqueue) = research_queue {
                    if !rqueue.queue.is_empty() {
                        lines.push(format!("Research Q: {}", rqueue.queue.len()));
                    }
                }
            }
        }
    }

    if let Ok(mut text) = text_q.single_mut() {
        text.0 = lines.join("\n");
    }
}
