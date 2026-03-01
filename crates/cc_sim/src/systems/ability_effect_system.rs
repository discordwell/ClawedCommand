use bevy::prelude::*;

use cc_core::abilities::AbilityId;
use cc_core::commands::EntityId;
use cc_core::components::{AbilitySlots, Dead, DreamSiegeTimer, Health, Owner};
use cc_core::status_effects::{StatusEffectId, StatusEffects};
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

        let source = EntityId(entity.to_bits());

        for slot in &slots.slots {
            match slot.id {
                AbilityId::Zoomies => {
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::Zoomies,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::LoafMode => {
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::LoafModeActive,
                            2,
                            source,
                        );
                    }
                }
                AbilityId::SpiteCarry => {
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpiteCarryBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::PowerNap => {
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::PowerNapping,
                            slot.duration_remaining.max(1),
                            source,
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
                // =============================================
                // The Clawed (Mice) — self-buff bridges
                // =============================================
                AbilityId::ChewThrough => {
                    // Toggle: building damage bonus (using DamageBuff)
                    if slot.active {
                        effects.refresh_or_insert(StatusEffectId::DamageBuff, 2, source);
                    }
                }
                AbilityId::SpineWall => {
                    // Toggle: damage reduction (using ArmorBuff)
                    if slot.active {
                        effects.refresh_or_insert(StatusEffectId::ArmorBuff, 2, source);
                    }
                }
                AbilityId::PileOn => {
                    // Activated: damage boost for duration
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::DamageBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::Scatter => {
                    // Activated: speed boost for duration
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpeedBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::StubbornAdvance => {
                    // Activated: damage boost + armor for duration
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::DamageBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                        effects.refresh_or_insert(
                            StatusEffectId::ArmorBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }

                // =============================================
                // Seekers of the Deep (Badgers) — self-buff bridges
                // =============================================
                AbilityId::Entrench => {
                    // Toggle: immobile + damage reduction + damage boost
                    if slot.active {
                        effects.refresh_or_insert(StatusEffectId::Entrenched, 2, source);
                    }
                }
                AbilityId::ShieldWall => {
                    // Activated: damage reduction for duration
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::ArmorBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::GrudgeCharge | AbilityId::RecklessLunge => {
                    // Activated: speed + damage boost for charge
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpeedBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                        effects.refresh_or_insert(
                            StatusEffectId::DamageBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }

                // =============================================
                // The Murder (Corvids) — self-buff bridges
                // =============================================
                AbilityId::Overwatch => {
                    // Toggle: increased attack range (using ArmorBuff as proxy)
                    if slot.active {
                        effects.refresh_or_insert(StatusEffectId::ArmorBuff, 2, source);
                    }
                }

                // =============================================
                // LLAMA (Raccoons) — self-buff bridges
                // =============================================
                AbilityId::Getaway => {
                    // Activated: speed boost to escape
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpeedBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::PlayDead => {
                    // Activated: invulnerable + immobile (playing dead)
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::PlayingDead,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::Scavenge => {
                    // Activated: gather speed boost
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpiteCarryBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }

                // =============================================
                // Croak (Axolotls) — self-buff bridges
                // =============================================
                AbilityId::HunkerAbility => {
                    // Toggle: immobile + damage reduction (like LoafMode)
                    if slot.active {
                        effects.refresh_or_insert(StatusEffectId::Entrenched, 2, source);
                    }
                }
                AbilityId::Inflate => {
                    // Activated: armor buff for duration
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::ArmorBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }
                AbilityId::Hop => {
                    // Activated: speed burst
                    if slot.active {
                        effects.refresh_or_insert(
                            StatusEffectId::SpeedBuff,
                            slot.duration_remaining.max(1),
                            source,
                        );
                    }
                }

                _ => {}
            }
        }
    }
}
