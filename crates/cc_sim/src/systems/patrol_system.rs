use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::MapResource;
use cc_core::components::{Dead, MoveTarget, Owner, Path, PatrolWaypoints, Position};
use cc_core::coords::WorldPos;
use cc_core::terrain::FactionId;

/// Advance patrol units to their next waypoint when they finish their current path.
///
/// Runs after movement_system. When a unit with `PatrolWaypoints` has no `Path` or
/// `MoveTarget` (meaning it reached its waypoint or was idle after combat), this system
/// advances to the next waypoint and pathfinds toward it.
pub fn patrol_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut patrollers: Query<
        (Entity, &Position, &mut PatrolWaypoints, &Owner),
        (Without<Path>, Without<MoveTarget>, Without<Dead>),
    >,
) {
    for (entity, pos, mut patrol, owner) in patrollers.iter_mut() {
        if patrol.waypoints.is_empty() {
            continue;
        }

        // Advance to next waypoint (wrap around)
        patrol.current_index = (patrol.current_index + 1) % patrol.waypoints.len();
        let next_wp = patrol.waypoints[patrol.current_index];

        let faction = FactionId::from_u8(owner.player_id).unwrap_or(FactionId::CatGPT);
        let start = pos.world.to_grid();

        if let Some(path) = pathfinding::find_path(&map_res.map, start, next_wp, faction) {
            let first = path[0];
            commands.entity(entity).insert(Path {
                waypoints: VecDeque::from(path),
            });
            commands.entity(entity).insert(MoveTarget {
                target: WorldPos::from_grid(first),
            });
        }
    }
}
