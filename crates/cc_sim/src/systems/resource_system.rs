use bevy::prelude::*;
use std::collections::VecDeque;

use cc_core::components::{
    Building, BuildingKind, Dead, GatherState, Gathering, MoveTarget, Owner, Path, Position,
    ResourceDeposit,
};
use cc_core::coords::WorldPos;
use cc_core::math::Fixed;
use cc_core::terrain::FactionId;
use cc_core::tuning::{CARRY_AMOUNT, DROPOFF_PROXIMITY_SQ, GATHERER_STALE_TICKS, HARVEST_TICKS};

use crate::pathfinding;
use crate::resources::{MapResource, PlayerResources};

/// Pawdler gather loop: MovingToDeposit → Harvesting → ReturningToBase → deposit → repeat.
pub fn gathering_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut gatherers: Query<
        (
            Entity,
            &Position,
            &Owner,
            &mut Gathering,
            Option<&MoveTarget>,
        ),
        Without<Dead>,
    >,
    mut deposits: Query<(Entity, &Position, &mut ResourceDeposit)>,
    drop_offs: Query<(Entity, &Position, &Owner, &Building)>,
    mut player_resources: ResMut<PlayerResources>,
) {
    let dropoff_proximity_sq = Fixed::from_num(DROPOFF_PROXIMITY_SQ);

    for (entity, pos, owner, mut gathering, move_target) in gatherers.iter_mut() {
        // --- Staleness detection (Bug 1 fix) ---
        // For movement states (MovingToDeposit, ReturningToBase) with an active
        // MoveTarget, check whether the worker has made positional progress.
        // If stuck for GATHERER_STALE_TICKS, remove Gathering so it can be reassigned.
        match gathering.state {
            GatherState::MovingToDeposit | GatherState::ReturningToBase
                if move_target.is_some() =>
            {
                let cur = (pos.world.x, pos.world.y);
                if cur == gathering.last_pos {
                    gathering.stale_ticks += 1;
                    if gathering.stale_ticks >= GATHERER_STALE_TICKS {
                        commands.entity(entity).remove::<Gathering>();
                        commands.entity(entity).remove::<MoveTarget>();
                        commands.entity(entity).remove::<Path>();
                        continue;
                    }
                } else {
                    gathering.stale_ticks = 0;
                    gathering.last_pos = cur;
                }
            }
            _ => {}
        }

        match gathering.state {
            GatherState::MovingToDeposit => {
                // Check if we've arrived at the deposit (no more MoveTarget = arrived)
                if move_target.is_none() {
                    // Check proximity to deposit
                    let deposit_entity = Entity::from_bits(gathering.deposit_entity.0);
                    if let Ok((_, deposit_pos, deposit)) = deposits.get(deposit_entity) {
                        if deposit.remaining == 0 {
                            // Deposit depleted — stop gathering
                            commands.entity(entity).remove::<Gathering>();
                            continue;
                        }
                        let dist = pos.world.distance_squared(deposit_pos.world);
                        let threshold = Fixed::from_num(2); // within ~1.4 tiles
                        if dist <= threshold {
                            // Start harvesting
                            gathering.state = GatherState::Harvesting {
                                ticks_remaining: HARVEST_TICKS,
                            };
                            gathering.carried_type = deposit.resource_type;
                        } else {
                            // Arrived but not close enough — release so AI can reassign
                            commands.entity(entity).remove::<Gathering>();
                        }
                    } else {
                        // Deposit gone — stop gathering
                        commands.entity(entity).remove::<Gathering>();
                    }
                }
            }

            GatherState::Harvesting {
                ref mut ticks_remaining,
            } => {
                if *ticks_remaining > 0 {
                    *ticks_remaining -= 1;
                } else {
                    // Finished harvesting — pick up resources, deplete deposit
                    let deposit_entity = Entity::from_bits(gathering.deposit_entity.0);
                    let actual_carry = if let Ok((_, _, mut deposit)) =
                        deposits.get_mut(deposit_entity)
                    {
                        let actual = CARRY_AMOUNT.min(deposit.remaining);
                        deposit.remaining -= actual;
                        actual
                    } else {
                        // Deposit gone — stop gathering
                        commands.entity(entity).remove::<Gathering>();
                        continue;
                    };
                    gathering.carried_amount = actual_carry;

                    // Find nearest drop-off (TheBox or FishMarket owned by same player)
                    let nearest_dropoff = find_nearest_dropoff(
                        pos.world,
                        owner.player_id,
                        &drop_offs,
                    );

                    if let Some(dropoff_pos) = nearest_dropoff {
                        let faction = FactionId::from_u8(owner.player_id)
                            .unwrap_or(FactionId::CatGPT);
                        let start = pos.world.to_grid();
                        let target = dropoff_pos.to_grid();

                        if let Some(waypoints) =
                            pathfinding::find_path(&map_res.map, start, target, faction)
                        {
                            let first_waypoint = waypoints[0];
                            commands.entity(entity).insert(Path {
                                waypoints: VecDeque::from(waypoints),
                            });
                            commands.entity(entity).insert(MoveTarget {
                                target: WorldPos::from_grid(first_waypoint),
                            });
                        }

                        gathering.state = GatherState::ReturningToBase;
                        // Reset staleness tracking for the new movement phase
                        gathering.last_pos = (pos.world.x, pos.world.y);
                        gathering.stale_ticks = 0;
                    } else {
                        // No drop-off found — stop gathering
                        commands.entity(entity).remove::<Gathering>();
                    }
                }
            }

            GatherState::ReturningToBase => {
                // Check if we've arrived at the drop-off
                if move_target.is_none() {
                    // --- Proximity check (Bug 2 fix) ---
                    // Only deposit resources if actually near a friendly drop-off.
                    let near_dropoff = is_near_dropoff(
                        pos.world,
                        owner.player_id,
                        dropoff_proximity_sq,
                        &drop_offs,
                    );

                    if near_dropoff {
                        // Deposit resources
                        let player_id = owner.player_id as usize;
                        if let Some(pres) = player_resources.players.get_mut(player_id) {
                            match gathering.carried_type {
                                cc_core::components::ResourceType::Food => {
                                    pres.food += gathering.carried_amount;
                                }
                                cc_core::components::ResourceType::GpuCores => {
                                    pres.gpu_cores += gathering.carried_amount;
                                }
                                cc_core::components::ResourceType::Nft => {
                                    pres.nfts += gathering.carried_amount;
                                }
                            }
                        }
                        gathering.carried_amount = 0;

                        // Return to deposit for another trip
                        let deposit_entity = Entity::from_bits(gathering.deposit_entity.0);
                        if let Ok((_, deposit_pos, _)) = deposits.get(deposit_entity) {
                            let faction = FactionId::from_u8(owner.player_id)
                                .unwrap_or(FactionId::CatGPT);
                            let start = pos.world.to_grid();
                            let target = deposit_pos.world.to_grid();

                            if let Some(waypoints) =
                                pathfinding::find_path(&map_res.map, start, target, faction)
                            {
                                let first_waypoint = waypoints[0];
                                commands.entity(entity).insert(Path {
                                    waypoints: VecDeque::from(waypoints),
                                });
                                commands.entity(entity).insert(MoveTarget {
                                    target: WorldPos::from_grid(first_waypoint),
                                });
                                gathering.state = GatherState::MovingToDeposit;
                                // Reset staleness tracking for the new movement phase
                                gathering.last_pos = (pos.world.x, pos.world.y);
                                gathering.stale_ticks = 0;
                            } else {
                                // Re-pathfinding failed — release so AI can reassign
                                commands.entity(entity).remove::<Gathering>();
                            }
                        } else {
                            // Deposit gone
                            commands.entity(entity).remove::<Gathering>();
                        }
                    } else {
                        // MoveTarget removed but not near a drop-off — release so AI
                        // can reassign (resources stay on the worker for next trip).
                        commands.entity(entity).remove::<Gathering>();
                    }
                }
            }
        }
    }
}

/// Find the nearest TheBox or FishMarket owned by the given player.
fn find_nearest_dropoff(
    from: WorldPos,
    player_id: u8,
    drop_offs: &Query<(Entity, &Position, &Owner, &Building)>,
) -> Option<WorldPos> {
    let mut best_dist = Fixed::MAX;
    let mut best_pos = None;

    for (_, bpos, bowner, building) in drop_offs.iter() {
        if bowner.player_id != player_id {
            continue;
        }
        match building.kind {
            BuildingKind::TheBox | BuildingKind::FishMarket => {}
            _ => continue,
        }

        let dist = from.distance_squared(bpos.world);
        if dist < best_dist {
            best_dist = dist;
            best_pos = Some(bpos.world);
        }
    }

    best_pos
}

/// Check if the worker is within `max_dist_sq` of any friendly drop-off building.
fn is_near_dropoff(
    from: WorldPos,
    player_id: u8,
    max_dist_sq: Fixed,
    drop_offs: &Query<(Entity, &Position, &Owner, &Building)>,
) -> bool {
    for (_, bpos, bowner, building) in drop_offs.iter() {
        if bowner.player_id != player_id {
            continue;
        }
        match building.kind {
            BuildingKind::TheBox | BuildingKind::FishMarket => {}
            _ => continue,
        }

        let dist = from.distance_squared(bpos.world);
        if dist <= max_dist_sq {
            return true;
        }
    }

    false
}
