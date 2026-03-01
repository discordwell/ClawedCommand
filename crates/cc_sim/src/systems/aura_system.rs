use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{Aura, AuraType, Dead, Owner, Position};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};

/// Aura system: applies 1-tick status effects from active aura sources to nearby entities.
/// Effects naturally expire in 1 tick when the source dies or toggles off.
pub fn aura_system(
    aura_sources: Query<(Entity, &Position, &Aura, &Owner), Without<Dead>>,
    mut targets: Query<(Entity, &Position, &mut StatusEffects, &Owner), Without<Dead>>,
) {
    for (source_entity, source_pos, aura, source_owner) in aura_sources.iter() {
        if !aura.active {
            continue;
        }

        let radius_sq = aura.radius * aura.radius;

        match aura.aura_type {
            AuraType::HarmonicResonance => {
                // Buff allies within radius (same player, not self)
                for (entity, target_pos, mut effects, target_owner) in targets.iter_mut() {
                    if entity == source_entity {
                        continue;
                    }
                    if target_owner.player_id != source_owner.player_id {
                        continue;
                    }
                    let dist_sq = source_pos.world.distance_squared(target_pos.world);
                    if dist_sq <= radius_sq {
                        refresh_or_add(
                            &mut effects,
                            StatusEffectId::HarmonicBuff,
                            2, // survives 1 tick of decrement
                            source_entity,
                        );
                    }
                }
            }
            AuraType::Lullaby => {
                // Debuff enemies within radius (different player)
                for (entity, target_pos, mut effects, target_owner) in targets.iter_mut() {
                    if entity == source_entity {
                        continue;
                    }
                    if target_owner.player_id == source_owner.player_id {
                        continue;
                    }
                    let dist_sq = source_pos.world.distance_squared(target_pos.world);
                    if dist_sq <= radius_sq {
                        refresh_or_add(
                            &mut effects,
                            StatusEffectId::LullabyDebuff,
                            2,
                            source_entity,
                        );
                    }
                }
            }
            // Other aura types deferred to Phase 4C/D
            _ => {}
        }
    }
}

/// Refresh an existing effect's duration or add a new one.
fn refresh_or_add(
    effects: &mut StatusEffects,
    id: StatusEffectId,
    duration: u32,
    source_entity: Entity,
) {
    if let Some(existing) = effects.effects.iter_mut().find(|e| e.effect == id) {
        existing.remaining_ticks = existing.remaining_ticks.max(duration);
    } else {
        effects.effects.push(StatusInstance {
            effect: id,
            remaining_ticks: duration,
            stacks: 1,
            source: EntityId(source_entity.to_bits()),
        });
    }
}
