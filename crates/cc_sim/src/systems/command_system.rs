use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::{CommandQueue, MapResource};
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{MoveTarget, Owner, Path, Position, Selected};
use cc_core::coords::WorldPos;
use cc_core::terrain::FactionId;

/// Process all queued commands for this tick.
pub fn process_commands(
    mut cmd_queue: ResMut<CommandQueue>,
    map_res: Res<MapResource>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Position,
        Option<&Owner>,
        Option<&mut MoveTarget>,
        Option<&mut Path>,
    )>,
) {
    let pending = cmd_queue.drain();

    for cmd in pending {
        match cmd {
            GameCommand::Move { unit_ids, target } => {
                for (entity, pos, owner, move_target, path) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }

                    // Determine faction from owner (default to CatGPT for unowned units)
                    let faction = owner
                        .and_then(|o| FactionId::from_u8(o.player_id))
                        .unwrap_or(FactionId::CatGPT);

                    let start = pos.world.to_grid();
                    if let Some(waypoints) =
                        pathfinding::find_path(&map_res.map, start, target, faction)
                    {
                        // Grab first waypoint before moving vec into Path
                        let first_waypoint = waypoints[0];
                        let path_component = Path {
                            waypoints: VecDeque::from(waypoints),
                        };

                        if let Some(mut existing_path) = path {
                            *existing_path = path_component;
                        } else {
                            commands.entity(entity).insert(path_component);
                        }

                        // Set immediate move target to first waypoint (not final destination)
                        let first_wp = WorldPos::from_grid(first_waypoint);
                        if let Some(mut mt) = move_target {
                            mt.target = first_wp;
                        } else {
                            commands.entity(entity).insert(MoveTarget { target: first_wp });
                        }
                    }
                }
            }
            GameCommand::Stop { unit_ids } => {
                for (entity, _, _, _, _) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }
                    commands.entity(entity).remove::<MoveTarget>();
                    commands.entity(entity).remove::<Path>();
                    // Velocity will be zeroed by movement_system when no MoveTarget
                }
            }
            GameCommand::Select { unit_ids } => {
                for (entity, _, _, _, _) in query.iter() {
                    let eid = EntityId(entity.to_bits());
                    if unit_ids.contains(&eid) {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }
            GameCommand::Deselect => {
                for (entity, _, _, _, _) in query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
            }
        }
    }
}
