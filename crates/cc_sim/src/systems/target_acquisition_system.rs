use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{
    AttackMoveTarget, AttackStats, AttackTarget, Building, ChasingTarget, Dead, HoldPosition,
    MoveTarget, Owner, Path, Position, UnitType,
};
use cc_core::math::Fixed;
use cc_core::tuning::ATTACK_MOVE_SIGHT_RANGE;

/// Auto-acquire enemy targets and clean up stale ones.
pub fn target_acquisition_system(
    mut commands: Commands,
    units: Query<
        (
            Entity,
            &Position,
            &Owner,
            &AttackStats,
            Option<&AttackTarget>,
            Option<&HoldPosition>,
            Option<&AttackMoveTarget>,
            Option<&MoveTarget>,
            Option<&ChasingTarget>,
        ),
        (With<UnitType>, Without<Dead>),
    >,
    potential_targets: Query<
        (Entity, &Position, &Owner),
        (Or<(With<UnitType>, With<Building>)>, Without<Dead>),
    >,
) {
    for (entity, pos, owner, stats, current_target, hold, atk_move, move_target, chasing) in
        units.iter()
    {
        // Check if current target is still alive
        if let Some(target) = current_target {
            let target_entity = Entity::from_bits(target.target.0);
            if potential_targets.get(target_entity).is_err() {
                // Target is dead or despawned — clear it
                commands.entity(entity).remove::<AttackTarget>();
            } else {
                // Already have a valid target
                continue;
            }
        }

        // Units executing a pure Move command (right-click ground) should not
        // auto-acquire targets. Only idle, hold-position, attack-move, and
        // chasing units should scan for enemies.
        if move_target.is_some() && atk_move.is_none() && chasing.is_none() {
            continue;
        }

        // Determine scan radius: weapon range for idle/hold, sight range for AttackMove
        let scan_range_sq = if atk_move.is_some() && hold.is_none() {
            let sight = Fixed::from_num(ATTACK_MOVE_SIGHT_RANGE);
            sight * sight
        } else {
            stats.range * stats.range
        };

        let mut best_dist_sq = scan_range_sq;
        let mut best_target = None;

        for (candidate, candidate_pos, candidate_owner) in potential_targets.iter() {
            // Skip friendlies
            if candidate_owner.player_id == owner.player_id {
                continue;
            }
            // Skip self
            if candidate == entity {
                continue;
            }

            let dist_sq = pos.world.distance_squared(candidate_pos.world);
            if dist_sq <= best_dist_sq {
                best_dist_sq = dist_sq;
                best_target = Some(candidate);
            }
        }

        if let Some(target_entity) = best_target {
            commands.entity(entity).insert(AttackTarget {
                target: EntityId::from_entity(target_entity),
            });
            // For AttackMove units, also chase the target (clear stale path first)
            if atk_move.is_some()
                && hold.is_none()
                && let Ok((_, target_pos, _)) = potential_targets.get(target_entity)
            {
                commands.entity(entity).remove::<Path>();
                commands.entity(entity).insert(ChasingTarget {
                    target: EntityId::from_entity(target_entity),
                });
                commands.entity(entity).insert(MoveTarget {
                    target: target_pos.world,
                });
            }
        }
    }
}
