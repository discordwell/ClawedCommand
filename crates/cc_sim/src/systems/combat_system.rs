use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::MapResource;
use crate::systems::damage::{ApplyDamageCommand, ApplyStatusCommand};
use cc_core::abilities::dream_siege_multiplier;
use cc_core::commands::EntityId;
use cc_core::components::{
    AttackStats, AttackTarget, AttackType, AttackTypeMarker, Building, ChasingTarget, Dead,
    DreamSiegeTimer, HoldPosition, MoveTarget, Owner, Path, Position, Projectile, ProjectileTarget,
    StatModifiers, UnitKind, UnitType, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::math::{Fixed, FIXED_ONE};
use cc_core::status_effects::StatusEffectId;
use cc_core::terrain::FactionId;
use cc_core::tuning::PROJECTILE_SPEED;

/// Core combat system: tick cooldowns, fire when ready, chase when out of range.
pub fn combat_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut attackers: Query<
        (
            Entity,
            &Position,
            &mut AttackStats,
            &AttackTypeMarker,
            Option<&AttackTarget>,
            Option<&Owner>,
            Option<&HoldPosition>,
            Option<&StatModifiers>,
            &UnitType,
            Option<&mut DreamSiegeTimer>,
        ),
        Without<Dead>,
    >,
    targets: Query<(Entity, &Position, Option<&StatModifiers>), (Or<(With<UnitType>, With<Building>)>, Without<Dead>)>,
) {
    for (entity, pos, mut stats, atk_type, attack_target, owner, hold, attacker_mods, unit_type, mut dream_siege_timer) in attackers.iter_mut() {
        // Tick cooldown
        if stats.cooldown_remaining > 0 {
            stats.cooldown_remaining -= 1;
        }

        // Cannot attack (e.g. Zoomies)
        if attacker_mods.is_some_and(|m| m.cannot_attack) {
            continue;
        }

        let Some(target) = attack_target else {
            continue;
        };

        let target_entity = Entity::from_bits(target.target.0);
        let Ok((_, target_pos, target_mods)) = targets.get(target_entity) else {
            // Target doesn't exist or is dead
            continue;
        };

        // Skip if target is invulnerable
        if target_mods.is_some_and(|m| m.invulnerable) {
            continue;
        }

        let dist_sq = pos.world.distance_squared(target_pos.world);
        let range_sq = stats.range * stats.range;

        if dist_sq <= range_sq {
            // In range — attack if cooldown is ready
            if stats.cooldown_remaining == 0 {
                // Apply attack_speed_multiplier to cooldown reset
                let base_cooldown = stats.attack_speed;
                let cooldown = if let Some(mods) = attacker_mods {
                    let adjusted = Fixed::from_num(base_cooldown as i32) * mods.attack_speed_multiplier;
                    adjusted.to_num::<u32>().max(1)
                } else {
                    base_cooldown
                };
                stats.cooldown_remaining = cooldown;

                // Calculate damage with cover + elevation modifiers
                let target_grid = target_pos.world.to_grid();
                let attacker_grid = pos.world.to_grid();

                let cover_mult = map_res
                    .map
                    .terrain_at(target_grid)
                    .map(|t| t.cover().damage_multiplier())
                    .unwrap_or(FIXED_ONE);

                let elev_advantage = map_res.map.elevation_advantage(attacker_grid, target_grid);
                let elev_mult =
                    cc_core::terrain::elevation_damage_multiplier(elev_advantage);

                let mut final_damage = stats.damage * cover_mult * elev_mult;

                // Apply attacker's damage_multiplier
                if let Some(mods) = attacker_mods {
                    final_damage = final_damage * mods.damage_multiplier;
                }

                // DreamSiege passive (Catnapper): ramp damage on same target
                if unit_type.kind == UnitKind::Catnapper {
                    if let Some(ref mut siege_timer) = dream_siege_timer {
                        if siege_timer.current_target == Some(EntityId(target_entity.to_bits())) {
                            siege_timer.ticks_on_target += 1;
                        } else {
                            siege_timer.current_target = Some(EntityId(target_entity.to_bits()));
                            siege_timer.ticks_on_target = 0;
                        }
                        let siege_mult = dream_siege_multiplier(siege_timer.ticks_on_target);
                        final_damage = final_damage * siege_mult;
                    }
                }

                // Apply target's damage_reduction
                if let Some(mods) = target_mods {
                    final_damage = final_damage * mods.damage_reduction;
                }

                match atk_type.attack_type {
                    AttackType::Melee => {
                        commands.queue(ApplyDamageCommand {
                            target: target_entity,
                            damage: final_damage,
                        });
                    }
                    AttackType::Ranged => {
                        commands.spawn((
                            Position { world: pos.world },
                            Velocity::zero(),
                            Projectile {
                                damage: final_damage,
                                speed: PROJECTILE_SPEED,
                            },
                            ProjectileTarget {
                                target: EntityId(target_entity.to_bits()),
                            },
                        ));
                    }
                }

                // AnnoyanceStacks passive (Nuisance): each attack applies Annoyed
                if unit_type.kind == UnitKind::Nuisance {
                    commands.queue(ApplyStatusCommand {
                        target: target_entity,
                        effect: StatusEffectId::Annoyed,
                        duration: 80, // 8s
                        stacks: 1,
                        max_stacks: 5,
                        source: EntityId(entity.to_bits()),
                    });
                }

                // CorrosiveSpit passive (Hisser): each attack applies Corroded
                if unit_type.kind == UnitKind::Hisser {
                    commands.queue(ApplyStatusCommand {
                        target: target_entity,
                        effect: StatusEffectId::Corroded,
                        duration: 80, // 8s
                        stacks: 1,
                        max_stacks: 6,
                        source: EntityId(entity.to_bits()),
                    });
                }
            }
        } else if hold.is_none() {
            // Out of range and not holding — chase
            let faction = owner
                .and_then(|o| FactionId::from_u8(o.player_id))
                .unwrap_or(FactionId::CatGPT);

            let start = pos.world.to_grid();
            let target_grid = target_pos.world.to_grid();

            if let Some(waypoints) =
                pathfinding::find_path(&map_res.map, start, target_grid, faction)
            {
                let first_waypoint = waypoints[0];
                commands.entity(entity).insert(ChasingTarget {
                    target: target.target,
                });
                commands.entity(entity).insert(Path {
                    waypoints: VecDeque::from(waypoints),
                });
                commands
                    .entity(entity)
                    .insert(MoveTarget {
                        target: WorldPos::from_grid(first_waypoint),
                    });
            }
        }
    }
}
