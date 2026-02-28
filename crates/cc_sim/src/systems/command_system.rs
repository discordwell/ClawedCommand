use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::{CommandQueue, ControlGroups, MapResource, PlayerResources};
use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    AttackMoveTarget, AttackTarget, Building, ChasingTarget, Gathering, GatherState, Health,
    HoldPosition, MoveTarget, Owner, Path, Position, Producer, ProductionQueue, RallyPoint,
    ResourceDeposit, Selected, UnderConstruction,
};
use cc_core::coords::WorldPos;
use cc_core::terrain::FactionId;
use cc_core::unit_stats::base_stats;

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
    mut control_groups: ResMut<ControlGroups>,
    mut player_resources: ResMut<PlayerResources>,
    buildings: Query<(Entity, &Building, &Owner, Option<&Producer>, Option<&UnderConstruction>)>,
    mut prod_queues: Query<&mut ProductionQueue>,
    deposits: Query<&Position, With<ResourceDeposit>>,
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

                    // Clear combat state — player move overrides combat
                    commands.entity(entity).remove::<AttackTarget>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<AttackMoveTarget>();
                    commands.entity(entity).remove::<HoldPosition>();
                    commands.entity(entity).remove::<Gathering>();

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
                    // Clear combat state
                    commands.entity(entity).remove::<AttackTarget>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<AttackMoveTarget>();
                    commands.entity(entity).remove::<HoldPosition>();
                    commands.entity(entity).remove::<Gathering>();
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
            GameCommand::Attack {
                unit_ids,
                target: attack_target,
            } => {
                for (entity, _, _, _, _) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }
                    // Can't target self
                    if attack_target == eid {
                        continue;
                    }
                    // Set attack target, clear movement state
                    commands.entity(entity).insert(AttackTarget {
                        target: attack_target,
                    });
                    commands.entity(entity).remove::<MoveTarget>();
                    commands.entity(entity).remove::<Path>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<AttackMoveTarget>();
                    commands.entity(entity).remove::<HoldPosition>();
                    commands.entity(entity).remove::<Gathering>();
                }
            }
            GameCommand::AttackMove { unit_ids, target } => {
                for (entity, pos, owner, move_target, path) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }

                    // Clear previous combat state
                    commands.entity(entity).remove::<AttackTarget>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<HoldPosition>();
                    commands.entity(entity).remove::<Gathering>();

                    // Set attack-move marker
                    commands
                        .entity(entity)
                        .insert(AttackMoveTarget { target });

                    // Pathfind toward the destination
                    let faction = owner
                        .and_then(|o| FactionId::from_u8(o.player_id))
                        .unwrap_or(FactionId::CatGPT);

                    let start = pos.world.to_grid();
                    if let Some(waypoints) =
                        pathfinding::find_path(&map_res.map, start, target, faction)
                    {
                        let first_waypoint = waypoints[0];
                        let path_component = Path {
                            waypoints: VecDeque::from(waypoints),
                        };

                        if let Some(mut existing_path) = path {
                            *existing_path = path_component;
                        } else {
                            commands.entity(entity).insert(path_component);
                        }

                        let first_wp = WorldPos::from_grid(first_waypoint);
                        if let Some(mut mt) = move_target {
                            mt.target = first_wp;
                        } else {
                            commands.entity(entity).insert(MoveTarget { target: first_wp });
                        }
                    }
                }
            }
            GameCommand::HoldPosition { unit_ids } => {
                for (entity, _, _, _, _) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }
                    commands.entity(entity).insert(HoldPosition);
                    commands.entity(entity).remove::<MoveTarget>();
                    commands.entity(entity).remove::<Path>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<AttackMoveTarget>();
                }
            }

            // --- Economy / Production / Control Group commands ---

            GameCommand::GatherResource { unit_ids, deposit } => {
                // Look up deposit position for pathfinding
                let deposit_entity = Entity::from_bits(deposit.0);
                let deposit_pos = deposits.get(deposit_entity).ok().map(|p| p.world.to_grid());

                for (entity, pos, owner, move_target, path) in query.iter_mut() {
                    let eid = EntityId(entity.to_bits());
                    if !unit_ids.contains(&eid) {
                        continue;
                    }

                    // Clear combat state
                    commands.entity(entity).remove::<AttackTarget>();
                    commands.entity(entity).remove::<ChasingTarget>();
                    commands.entity(entity).remove::<AttackMoveTarget>();
                    commands.entity(entity).remove::<HoldPosition>();

                    // Set gathering component
                    commands.entity(entity).insert(Gathering {
                        deposit_entity: deposit,
                        carried_type: cc_core::components::ResourceType::Food,
                        carried_amount: 0,
                        state: GatherState::MovingToDeposit,
                    });

                    // Pathfind to deposit
                    if let Some(deposit_grid) = deposit_pos {
                        let faction = owner
                            .and_then(|o| FactionId::from_u8(o.player_id))
                            .unwrap_or(FactionId::CatGPT);

                        let start = pos.world.to_grid();
                        if let Some(waypoints) =
                            pathfinding::find_path(&map_res.map, start, deposit_grid, faction)
                        {
                            let first_waypoint = waypoints[0];
                            let path_component = Path {
                                waypoints: VecDeque::from(waypoints),
                            };

                            if let Some(mut existing_path) = path {
                                *existing_path = path_component;
                            } else {
                                commands.entity(entity).insert(path_component);
                            }

                            let first_wp = WorldPos::from_grid(first_waypoint);
                            if let Some(mut mt) = move_target {
                                mt.target = first_wp;
                            } else {
                                commands.entity(entity).insert(MoveTarget { target: first_wp });
                            }
                        }
                    }
                }
            }

            GameCommand::Build {
                builder,
                building_kind,
                position,
            } => {
                // Validate terrain is passable at build site (reject out-of-bounds too)
                let Some(terrain) = map_res.map.terrain_at(position) else {
                    continue; // Out of bounds
                };
                if !cc_core::terrain::is_passable_for_faction(terrain, cc_core::terrain::FactionId::CatGPT) {
                    continue; // Can't build on impassable terrain
                }

                let builder_entity = Entity::from_bits(builder.0);
                // Find builder's owner
                if let Ok((_, _, owner, _, _)) = query.get(builder_entity) {
                    let player_id = owner.map(|o| o.player_id).unwrap_or(0);

                    let bstats = building_stats(building_kind);

                    // Validate resources
                    if let Some(pres) = player_resources.players.get_mut(player_id as usize) {
                        if pres.food < bstats.food_cost || pres.gpu_cores < bstats.gpu_cost {
                            continue; // Insufficient resources
                        }
                        // Deduct resources
                        pres.food -= bstats.food_cost;
                        pres.gpu_cores -= bstats.gpu_cost;
                    } else {
                        continue;
                    }

                    let world = WorldPos::from_grid(position);

                    // Spawn building entity
                    let mut building_entity = commands.spawn((
                        Position { world },
                        cc_core::components::Velocity::zero(),
                        cc_core::components::GridCell { pos: position },
                        Owner { player_id },
                        Building { kind: building_kind },
                        Health {
                            current: bstats.health,
                            max: bstats.health,
                        },
                    ));

                    if bstats.build_time > 0 {
                        building_entity.insert(UnderConstruction {
                            remaining_ticks: bstats.build_time,
                            total_ticks: bstats.build_time,
                        });
                    } else {
                        // Pre-built: add producer + queue immediately
                        if !bstats.can_produce.is_empty() {
                            building_entity.insert((Producer, ProductionQueue::default()));
                        }
                    }

                    // Update supply cap
                    if bstats.supply_provided > 0 {
                        if let Some(pres) = player_resources.players.get_mut(player_id as usize) {
                            pres.supply_cap += bstats.supply_provided;
                        }
                    }
                }
            }

            GameCommand::TrainUnit {
                building,
                unit_kind,
            } => {
                let building_entity = Entity::from_bits(building.0);

                // Check building is a producer with correct owner
                if let Ok((_, bld, owner, producer, under_construction)) =
                    buildings.get(building_entity)
                {
                    // Can't train from unfinished or non-producer buildings
                    if under_construction.is_some() || producer.is_none() {
                        continue;
                    }

                    let player_id = owner.player_id;
                    let bstats = building_stats(bld.kind);

                    // Validate building can produce this unit kind
                    if !bstats.can_produce.contains(&unit_kind) {
                        continue;
                    }

                    let ustats = base_stats(unit_kind);

                    // Validate resources + supply
                    if let Some(pres) = player_resources.players.get_mut(player_id as usize) {
                        if pres.food < ustats.food_cost || pres.gpu_cores < ustats.gpu_cost {
                            continue;
                        }
                        if pres.supply + ustats.supply_cost > pres.supply_cap {
                            continue;
                        }
                        // Deduct resources, reserve supply
                        pres.food -= ustats.food_cost;
                        pres.gpu_cores -= ustats.gpu_cost;
                        pres.supply += ustats.supply_cost;
                    } else {
                        continue;
                    }

                    // Add to production queue
                    if let Ok(mut queue) = prod_queues.get_mut(building_entity) {
                        queue.queue.push_back((unit_kind, ustats.train_time));
                    }
                }
            }

            GameCommand::SetRallyPoint { building, target } => {
                let building_entity = Entity::from_bits(building.0);
                if buildings.get(building_entity).is_ok() {
                    commands.entity(building_entity).insert(RallyPoint { target });
                }
            }

            GameCommand::CancelQueue { building } => {
                let building_entity = Entity::from_bits(building.0);
                if let Ok(mut queue) = prod_queues.get_mut(building_entity) {
                    if let Some((unit_kind, _)) = queue.queue.pop_front() {
                        // Refund resources
                        if let Ok((_, _, owner, _, _)) = buildings.get(building_entity) {
                            let player_id = owner.player_id;
                            let ustats = base_stats(unit_kind);
                            if let Some(pres) =
                                player_resources.players.get_mut(player_id as usize)
                            {
                                pres.food += ustats.food_cost;
                                pres.gpu_cores += ustats.gpu_cost;
                                pres.supply = pres.supply.saturating_sub(ustats.supply_cost);
                            }
                        }
                    }
                }
            }

            GameCommand::SetControlGroup { group, unit_ids } => {
                if (group as usize) < control_groups.groups.len() {
                    control_groups.groups[group as usize] = unit_ids;
                }
            }

            GameCommand::RecallControlGroup { group } => {
                if let Some(group_ids) = control_groups.groups.get(group as usize) {
                    if !group_ids.is_empty() {
                        // Deselect all, then select control group
                        for (entity, _, _, _, _) in query.iter() {
                            commands.entity(entity).remove::<Selected>();
                        }
                        for (entity, _, _, _, _) in query.iter() {
                            let eid = EntityId(entity.to_bits());
                            if group_ids.contains(&eid) {
                                commands.entity(entity).insert(Selected);
                            }
                        }
                    }
                }
            }
        }
    }
}
