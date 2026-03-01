use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{Aura, AuraType, Dead, Owner, Position};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};
use cc_core::tuning::GRAV_PULL_PER_TICK;

use crate::systems::damage::GravitationalPullCommand;

/// Aura system: applies 1-tick status effects from active aura sources to nearby entities.
pub fn aura_system(
    mut commands: Commands,
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
                            2,
                            source_entity,
                        );
                    }
                }
            }
            AuraType::Lullaby => {
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
            AuraType::GravitationalChonk => {
                for (entity, target_pos, _, target_owner) in targets.iter() {
                    if entity == source_entity {
                        continue;
                    }
                    if target_owner.player_id == source_owner.player_id {
                        continue;
                    }
                    let dist_sq = source_pos.world.distance_squared(target_pos.world);
                    if dist_sq <= radius_sq {
                        commands.queue(GravitationalPullCommand {
                            source_pos: source_pos.world,
                            target: entity,
                            pull_per_tick: GRAV_PULL_PER_TICK,
                        });
                    }
                }
            }
            AuraType::TacticalUplink => {
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
                            StatusEffectId::TacticalLink,
                            2,
                            source_entity,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

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
