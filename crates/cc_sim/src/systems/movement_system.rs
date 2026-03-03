use bevy::prelude::*;

use crate::resources::MapResource;
use cc_core::components::{MoveTarget, MovementSpeed, Path, Position, StatModifiers, Velocity};
use cc_core::coords::WorldPos;
use cc_core::math::{FIXED_ONE, FIXED_ZERO, Fixed, approx_distance};

/// Check if a unit should snap to its target (within one tick's movement).
pub fn should_snap_to_target(pos: WorldPos, target: WorldPos, speed: Fixed) -> bool {
    let dx = target.x - pos.x;
    let dy = target.y - pos.y;
    let dist_sq = dx * dx + dy * dy;
    let threshold_sq = speed * speed;
    dist_sq <= threshold_sq
}

pub fn movement_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut query: Query<(
        Entity,
        &mut Position,
        &mut Velocity,
        &MovementSpeed,
        Option<&MoveTarget>,
        Option<&mut Path>,
        Option<&StatModifiers>,
    )>,
) {
    for (entity, mut pos, mut vel, speed, move_target, path, modifiers) in query.iter_mut() {
        // Check immobilized
        if modifiers.is_some_and(|m| m.immobilized) {
            vel.dx = FIXED_ZERO;
            vel.dy = FIXED_ZERO;
            continue;
        }

        let Some(target) = move_target else {
            // No target -- zero velocity
            vel.dx = FIXED_ZERO;
            vel.dy = FIXED_ZERO;
            continue;
        };

        // Get terrain movement cost at current position
        let grid_pos = pos.world.to_grid();
        let terrain_cost = map_res.map.movement_cost(grid_pos).unwrap_or(FIXED_ONE);

        // Apply speed_multiplier from StatModifiers
        let base_speed = if let Some(mods) = modifiers {
            speed.speed * mods.speed_multiplier
        } else {
            speed.speed
        };

        // Effective speed = base_speed / terrain_movement_cost
        let effective_speed = if terrain_cost > FIXED_ZERO {
            base_speed / terrain_cost
        } else {
            base_speed
        };

        let dx = target.target.x - pos.world.x;
        let dy = target.target.y - pos.world.y;
        let dist_sq = dx * dx + dy * dy;

        // Snap when within one tick's movement -- prevents oscillation at any speed
        let threshold_sq = effective_speed * effective_speed;
        if dist_sq <= threshold_sq {
            // Arrived at current waypoint
            pos.world = target.target;
            vel.dx = FIXED_ZERO;
            vel.dy = FIXED_ZERO;

            // Try to advance to next waypoint in path
            let mut advance_to_next = false;
            let mut next_waypoint = None;

            if let Some(mut path) = path {
                path.waypoints.pop_front(); // Remove the one we just reached
                if let Some(next) = path.waypoints.front() {
                    next_waypoint = Some(WorldPos::from_grid(*next));
                    advance_to_next = true;
                }
            }

            if advance_to_next {
                if let Some(wp) = next_waypoint {
                    commands.entity(entity).insert(MoveTarget { target: wp });
                }
            } else {
                commands.entity(entity).remove::<MoveTarget>();
                commands.entity(entity).remove::<Path>();
            }
        } else {
            // Move toward target using fast approximate distance normalization
            let approx_dist = approx_distance(dx, dy);

            if approx_dist > FIXED_ZERO {
                vel.dx = dx * effective_speed / approx_dist;
                vel.dy = dy * effective_speed / approx_dist;
            }

            // Apply velocity
            pos.world.x += vel.dx;
            pos.world.y += vel.dy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::math::fixed_from_i32;

    #[test]
    fn snap_when_within_one_step() {
        let pos = WorldPos::new(Fixed::from_num(0.0f32), Fixed::from_num(0.0f32));
        let target = WorldPos::new(Fixed::from_num(0.1f32), Fixed::from_num(0.0f32));
        let speed = Fixed::from_num(0.15f32);
        // Distance 0.1 < speed 0.15 -> should snap
        assert!(should_snap_to_target(pos, target, speed));
    }

    #[test]
    fn no_snap_when_far_away() {
        let pos = WorldPos::new(fixed_from_i32(0), fixed_from_i32(0));
        let target = WorldPos::new(fixed_from_i32(5), fixed_from_i32(5));
        let speed = Fixed::from_num(0.15f32);
        assert!(!should_snap_to_target(pos, target, speed));
    }

    #[test]
    fn snap_prevents_oscillation_high_speed() {
        // High speed unit very close to target should snap, not overshoot
        let pos = WorldPos::new(Fixed::from_num(4.9f32), Fixed::from_num(4.9f32));
        let target = WorldPos::new(fixed_from_i32(5), fixed_from_i32(5));
        let speed = Fixed::from_num(5.0f32); // Very fast
        // Distance ~0.14, speed 5.0 -> should snap
        assert!(should_snap_to_target(pos, target, speed));
    }
}
