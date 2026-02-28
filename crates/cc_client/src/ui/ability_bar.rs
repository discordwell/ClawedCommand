use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::abilities::{ability_def, AbilityActivation};
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{AbilitySlots, Owner, Selected, UnitType};
use cc_sim::resources::CommandQueue;

const LOCAL_PLAYER: u8 = 0;

/// Bottom ability bar: shows 3 ability slots for the first selected unit.
pub fn ability_bar_system(
    mut contexts: EguiContexts,
    mut cmd_queue: ResMut<CommandQueue>,
    selected_units: Query<(Entity, &UnitType, &Owner, &AbilitySlots), With<Selected>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // Find the first selected unit owned by local player with abilities
    let Some((entity, _unit_type, _owner, ability_slots)) = selected_units
        .iter()
        .find(|(_, _, owner, _)| owner.player_id == LOCAL_PLAYER)
    else {
        return;
    };

    egui::Window::new("Abilities")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -5.0))
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (slot_idx, state) in ability_slots.slots.iter().enumerate() {
                    let def = ability_def(state.id);
                    let label = match def.activation {
                        AbilityActivation::Passive => {
                            format!("{:?} [P]", state.id)
                        }
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
                    let on_cooldown = state.cooldown_remaining > 0
                        && def.activation == AbilityActivation::Activated;
                    let enabled = !is_passive && !on_cooldown;

                    let btn = egui::Button::new(&label);
                    if ui.add_enabled(enabled, btn).clicked() {
                        cmd_queue.push(GameCommand::ActivateAbility {
                            unit_id: EntityId(entity.to_bits()),
                            slot: slot_idx as u8,
                            target: AbilityTarget::SelfCast,
                        });
                    }
                }
            });
        });
}
