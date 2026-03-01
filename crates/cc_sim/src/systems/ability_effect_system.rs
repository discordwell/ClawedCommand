use bevy::prelude::*;

use cc_core::abilities::AbilityId;
use cc_core::commands::EntityId;
use cc_core::components::{AbilitySlots, Dead, DreamSiegeTimer, Health, Owner};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};
use cc_core::tuning::POWER_NAP_GPU_INTERVAL;

use crate::resources::PlayerResources;

/// Bridge system: reads AbilitySlots and applies/refreshes corresponding StatusEffects
/// when abilities are active. Also handles DreamSiege damage reset and PowerNap GPU gen.
pub fn ability_effect_system(
    mut query: Query<
        (
            Entity,
            &AbilitySlots,
            &mut StatusEffects,
            Option<&Health>,
            Option<&mut DreamSiegeTimer>,
            Option<&Owner>,
        ),
        Without<Dead>,
    >,
    mut player_res: ResMut<PlayerResources>,
) {
    for (entity, slots, mut effects, health, dream_siege_timer, owner) in query.iter_mut() {
        // T2 Fix: DreamSiege reset on Catnapper taking damage
        if let Some(mut siege_timer) = dream_siege_timer {
            if let Some(health) = health {
                if siege_timer.last_hp > cc_core::math::FIXED_ZERO
                    && health.current < siege_timer.last_hp
                {
                    siege_timer.ticks_on_target = 0;
                    siege_timer.current_target = None;
                }
                siege_timer.last_hp = health.current;
            }
        }

        for slot in &slots.slots {
            match slot.id {
                AbilityId::Zoomies => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::Zoomies,
                            slot.duration_remaining.max(1),
                            entity,
                        );
                    }
                }
                AbilityId::LoafMode => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::LoafModeActive,
                            2,
                            entity,
                        );
                    }
                }
                AbilityId::SpiteCarry => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::SpiteCarryBuff,
                            slot.duration_remaining.max(1),
                            entity,
                        );
                    }
                }
                AbilityId::PowerNap => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::PowerNapping,
                            slot.duration_remaining.max(1),
                            entity,
                        );
                        // Generate GPU: 1 GPU every POWER_NAP_GPU_INTERVAL ticks
                        if slot.duration_remaining > 0
                            && slot.duration_remaining % POWER_NAP_GPU_INTERVAL == 0
                        {
                            if let Some(owner) = owner {
                                if let Some(pres) =
                                    player_res.players.get_mut(owner.player_id as usize)
                                {
                                    pres.gpu_cores += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn ensure_effect(effects: &mut StatusEffects, id: StatusEffectId, duration: u32, entity: Entity) {
    if let Some(existing) = effects.effects.iter_mut().find(|e| e.effect == id) {
        existing.remaining_ticks = existing.remaining_ticks.max(duration);
    } else {
        effects.effects.push(StatusInstance {
            effect: id,
            remaining_ticks: duration,
            stacks: 1,
            source: EntityId(entity.to_bits()),
        });
    }
}
