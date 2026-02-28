use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::{
    AttackMoveTarget, AttackStats, AttackTarget, Building, Dead, HoldPosition, Owner, Position,
    UnitType,
};

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
        ),
        (With<UnitType>, Without<Dead>),
    >,
    potential_targets: Query<
        (Entity, &Position, &Owner),
        (Or<(With<UnitType>, With<Building>)>, Without<Dead>),
    >,
) {
    for (entity, pos, owner, stats, current_target, _hold, _atk_move) in units.iter() {
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

        // Auto-acquire: find nearest enemy within weapon range
        // (Explicit Attack command handles chasing; auto-acquire is passive)
        let range_sq = stats.range * stats.range;
        let mut best_dist_sq = range_sq;
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
                target: EntityId(target_entity.to_bits()),
            });
        }
    }
}
