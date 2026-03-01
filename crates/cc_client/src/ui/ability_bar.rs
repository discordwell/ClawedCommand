use bevy::prelude::*;

use cc_core::abilities::{ability_def, AbilityActivation};
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{AbilitySlots, Owner, Selected, UnitType};
use cc_sim::resources::CommandQueue;

const LOCAL_PLAYER: u8 = 0;

/// Marker for the ability bar root node.
#[derive(Component)]
pub struct AbilityBarRoot;

/// Marker for individual ability button text.
#[derive(Component)]
pub struct AbilityButton {
    pub slot: u8,
}

pub fn spawn_ability_bar(mut commands: Commands) {
    commands.spawn((
        AbilityBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(32.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-120.0)),
            width: Val::Px(240.0),
            padding: UiRect::all(Val::Px(6.0)),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        Visibility::Hidden,
    ));
}

pub fn update_ability_bar(
    mut commands: Commands,
    mut cmd_queue: ResMut<CommandQueue>,
    selected_units: Query<(Entity, &UnitType, &Owner, &AbilitySlots), With<Selected>>,
    root_q: Query<Entity, With<AbilityBarRoot>>,
    button_q: Query<Entity, With<AbilityButton>>,
    mut root_vis: Query<&mut Visibility, With<AbilityBarRoot>>,
    interactions: Query<(&AbilityButton, &Interaction), Changed<Interaction>>,
) {
    let Ok(root_entity) = root_q.single() else {
        return;
    };

    // Find first selected unit owned by local player with abilities
    let selected = selected_units
        .iter()
        .find(|(_, _, owner, _)| owner.player_id == LOCAL_PLAYER);

    let show = selected.is_some();
    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    let Some((entity, _unit_type, _owner, ability_slots)) = selected else {
        // Despawn old buttons
        for btn_entity in button_q.iter() {
            commands.entity(btn_entity).despawn();
        }
        return;
    };

    // Handle button clicks
    for (btn, interaction) in interactions.iter() {
        if *interaction == Interaction::Pressed {
            cmd_queue.push(GameCommand::ActivateAbility {
                unit_id: EntityId(entity.to_bits()),
                slot: btn.slot,
                target: AbilityTarget::SelfCast,
            });
        }
    }

    // Rebuild buttons each frame (abilities can change)
    for btn_entity in button_q.iter() {
        commands.entity(btn_entity).despawn();
    }

    for (slot_idx, state) in ability_slots.slots.iter().enumerate() {
        let def = ability_def(state.id);
        let label = match def.activation {
            AbilityActivation::Passive => format!("{:?} [P]", state.id),
            AbilityActivation::Toggle => {
                if state.active {
                    format!("{:?} [ON]", state.id)
                } else {
                    format!("{:?} [OFF]", state.id)
                }
            }
            AbilityActivation::Activated => {
                if state.cooldown_remaining > 0 {
                    format!("{:?} ({})", state.id, state.cooldown_remaining)
                } else {
                    format!("{:?}", state.id)
                }
            }
        };

        let is_passive = def.activation == AbilityActivation::Passive;
        let on_cooldown =
            state.cooldown_remaining > 0 && def.activation == AbilityActivation::Activated;
        let enabled = !is_passive && !on_cooldown;

        let text_color = if enabled {
            Color::srgb(0.9, 0.9, 0.9)
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        };

        commands.entity(root_entity).with_children(|parent| {
            parent
                .spawn((
                    AbilityButton {
                        slot: slot_idx as u8,
                    },
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(label),
                        TextColor(text_color),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });
        });
    }
}
