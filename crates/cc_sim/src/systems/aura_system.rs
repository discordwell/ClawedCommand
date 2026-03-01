use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{Aura, AuraType, Dead, Owner, Position};
use cc_core::status_effects::{StatusEffectId, StatusEffects};
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
                        effects.refresh_or_insert(
                            StatusEffectId::HarmonicBuff,
                            2,
                            EntityId::from_entity(source_entity),
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
                        effects.refresh_or_insert(
                            StatusEffectId::LullabyDebuff,
                            2,
                            EntityId::from_entity(source_entity),
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
                        effects.refresh_or_insert(
                            StatusEffectId::TacticalLink,
                            2,
                            EntityId::from_entity(source_entity),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

