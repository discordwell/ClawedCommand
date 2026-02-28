use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::MapResource;
use crate::systems::damage::ApplyDamageCommand;
use cc_core::commands::EntityId;
use cc_core::components::{
    AttackStats, AttackTarget, AttackType, AttackTypeMarker, Building, ChasingTarget, Dead,
    HoldPosition, MoveTarget, Owner, Path, Position, Projectile, ProjectileTarget, UnitType,
    Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::math::{Fixed, FIXED_ONE};
use cc_core::terrain::FactionId;

/// Projectile speed in 16.16 fixed-point. 0.5 = 1 << 15 = 32768 bits.
const PROJECTILE_SPEED: Fixed = Fixed::from_bits(1 << 15);

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
        ),
        (With<UnitType>, Without<Dead>),
    >,
    targets: Query<(Entity, &Position), (Or<(With<UnitType>, With<Building>)>, Without<Dead>)>,
) {
    for (entity, pos, mut stats, atk_type, attack_target, owner, hold) in attackers.iter_mut() {
        // Tick cooldown
        if stats.cooldown_remaining > 0 {
            stats.cooldown_remaining -= 1;
        }

        let Some(target) = attack_target else {
            continue;
        };

        let target_entity = Entity::from_bits(target.target.0);
        let Ok((_, target_pos)) = targets.get(target_entity) else {
            // Target doesn't exist or is dead
            continue;
        };

        let dist_sq = pos.world.distance_squared(target_pos.world);
        let range_sq = stats.range * stats.range;

        if dist_sq <= range_sq {
            // In range — attack if cooldown is ready
            if stats.cooldown_remaining == 0 {
                stats.cooldown_remaining = stats.attack_speed;

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

                let final_damage = stats.damage * cover_mult * elev_mult;

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
