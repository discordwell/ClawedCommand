use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::{CommandQueue, ControlGroups, MapResource, PlayerResources};
use cc_core::abilities::{ability_def, AbilityActivation, AbilityId};
use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    AbilitySlots, AttackMoveTarget, AttackTarget, BuildOrder, Aura, AuraType, Building, ChasingTarget,
    Gathering, GatherState, HoldPosition, MoveTarget, Owner, Path, Position, Producer,
    ProductionQueue, RallyPoint, ResearchQueue, Researcher, ResourceDeposit, Selected,
    StatModifiers, UnderConstruction, UnitKind, UpgradeType,
};
use cc_core::coords::WorldPos;
use cc_core::math::{Fixed, FIXED_ONE, fixed_from_i32};
use cc_core::status_effects::StatusEffectId;
use cc_core::terrain::FactionId;
use cc_core::unit_stats::base_stats;
use cc_core::upgrade_stats::upgrade_stats;

use crate::systems::damage::{AoeCcCommand, RevulsionAoeCommand};

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
    mut ability_query: Query<(&mut AbilitySlots, Option<&StatModifiers>)>,
    mut research_queues: Query<(&Owner, &mut ResearchQueue), With<Researcher>>,
    build_orders: Query<(&BuildOrder, &Owner)>,
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
                    // Refund building cost if cancelling a build order
                    if let Ok((bo, bo_owner)) = build_orders.get(entity) {
                        let bstats = building_stats(bo.building_kind);
                        if let Some(pres) = player_resources.players.get_mut(bo_owner.player_id as usize) {
                            pres.food += bstats.food_cost;
                            pres.gpu_cores += bstats.gpu_cost;
                        }
                    }
                    commands.entity(entity).remove::<BuildOrder>();

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
                    // Refund building cost if cancelling a build order
                    if let Ok((build_order, owner)) = build_orders.get(entity) {
                        let bstats = building_stats(build_order.building_kind);
                        if let Some(pres) = player_resources.players.get_mut(owner.player_id as usize) {
                            pres.food += bstats.food_cost;
                            pres.gpu_cores += bstats.gpu_cost;
                        }
                    }
                    commands.entity(entity).remove::<BuildOrder>();
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
                        last_pos: (pos.world.x, pos.world.y),
                        stale_ticks: 0,
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
                if let Ok((_, pos, owner, move_target, path)) = query.get_mut(builder_entity) {
                    let player_id = owner.map(|o| o.player_id).unwrap_or(0);

                    let bstats = building_stats(building_kind);

                    // Validate resources
                    if let Some(pres) = player_resources.players.get_mut(player_id as usize) {
                        if pres.food < bstats.food_cost || pres.gpu_cores < bstats.gpu_cost {
                            continue; // Insufficient resources
                        }
                        // Deduct resources immediately (standard RTS convention)
                        pres.food -= bstats.food_cost;
                        pres.gpu_cores -= bstats.gpu_cost;
                    } else {
                        continue;
                    }

                    // Clear existing orders on builder
                    commands.entity(builder_entity).remove::<AttackTarget>();
                    commands.entity(builder_entity).remove::<ChasingTarget>();
                    commands.entity(builder_entity).remove::<AttackMoveTarget>();
                    commands.entity(builder_entity).remove::<HoldPosition>();
                    commands.entity(builder_entity).remove::<Gathering>();

                    // Attach BuildOrder -- builder_system will spawn the building on arrival
                    commands.entity(builder_entity).insert(BuildOrder { building_kind, position });

                    // Pathfind to build site
                    let faction = owner
                        .and_then(|o| FactionId::from_u8(o.player_id))
                        .unwrap_or(FactionId::CatGPT);
                    let start = pos.world.to_grid();

                    if let Some(waypoints) =
                        pathfinding::find_path(&map_res.map, start, position, faction)
                    {
                        let first_waypoint = waypoints[0];
                        let path_component = Path {
                            waypoints: VecDeque::from(waypoints),
                        };

                        if let Some(mut existing_path) = path {
                            *existing_path = path_component;
                        } else {
                            commands.entity(builder_entity).insert(path_component);
                        }

                        let first_wp = WorldPos::from_grid(first_waypoint);
                        if let Some(mut mt) = move_target {
                            mt.target = first_wp;
                        } else {
                            commands.entity(builder_entity).insert(MoveTarget { target: first_wp });
                        }
                    } else {
                        // Pathfinding failed -- refund resources and cancel build
                        if let Some(pres) = player_resources.players.get_mut(player_id as usize) {
                            pres.food += bstats.food_cost;
                            pres.gpu_cores += bstats.gpu_cost;
                        }
                        commands.entity(builder_entity).remove::<BuildOrder>();
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

                    // Gate advanced units behind upgrade prerequisites
                    if let Some(pres) = player_resources.players.get(player_id as usize) {
                        if unit_kind == UnitKind::Catnapper
                            && !pres.completed_upgrades.contains(&UpgradeType::SiegeTraining)
                        {
                            continue;
                        }
                        if unit_kind == UnitKind::MechCommander
                            && !pres.completed_upgrades.contains(&UpgradeType::MechPrototype)
                        {
                            continue;
                        }
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

            GameCommand::ActivateAbility {
                unit_id,
                slot,
                target: _ability_target,
            } => {
                let entity = Entity::from_bits(unit_id.0);

                // Get owner player_id for deferred commands
                let owner_player_id = query
                    .get(entity)
                    .ok()
                    .and_then(|(_, _, o, _, _)| o.map(|o| o.player_id))
                    .unwrap_or(0);

                if let Ok((mut ability_slots, stat_mods)) = ability_query.get_mut(entity) {
                    let slot_idx = slot as usize;
                    if slot_idx >= 3 {
                        continue;
                    }

                    let ability_state = &mut ability_slots.slots[slot_idx];
                    let def = ability_def(ability_state.id);
                    let ability_id = ability_state.id;

                    // Check silenced
                    if stat_mods.is_some_and(|m| m.silenced) {
                        continue;
                    }

                    // Check cooldown
                    if ability_state.cooldown_remaining > 0 {
                        continue;
                    }

                    // Check charges (for charge-based abilities)
                    if def.max_charges > 0 && ability_state.charges == 0 {
                        continue;
                    }

                    // Check GPU cost
                    if def.gpu_cost > 0 {
                        if let Some(pres) = player_resources.players.get_mut(owner_player_id as usize) {
                            if pres.gpu_cores < def.gpu_cost {
                                continue;
                            }
                            pres.gpu_cores -= def.gpu_cost;
                        } else {
                            continue;
                        }
                    }

                    // Apply cooldown_multiplier from StatModifiers (e.g. TacticalUplink)
                    let cd_mult = stat_mods.map(|m| m.cooldown_multiplier).unwrap_or(FIXED_ONE);
                    let effective_cooldown = (fixed_from_i32(def.cooldown_ticks as i32) * cd_mult).to_num::<u32>();

                    // Activate
                    match def.activation {
                        AbilityActivation::Toggle => {
                            ability_state.active = !ability_state.active;
                            ability_state.cooldown_remaining = effective_cooldown;

                            let is_now_active = ability_state.active;

                            // Mutual exclusivity: HarmonicResonance ↔ Lullaby
                            if ability_id == AbilityId::HarmonicResonance && is_now_active {
                                for other_slot in &mut ability_slots.slots {
                                    if other_slot.id == AbilityId::Lullaby {
                                        other_slot.active = false;
                                    }
                                }
                            }
                            if ability_id == AbilityId::Lullaby && is_now_active {
                                for other_slot in &mut ability_slots.slots {
                                    if other_slot.id == AbilityId::HarmonicResonance {
                                        other_slot.active = false;
                                    }
                                }
                            }

                            // Aura component management for toggle auras
                            match ability_id {
                                AbilityId::HarmonicResonance => {
                                    commands.entity(entity).insert(Aura {
                                        aura_type: AuraType::HarmonicResonance,
                                        radius: def.range,
                                        active: is_now_active,
                                    });
                                }
                                AbilityId::Lullaby => {
                                    commands.entity(entity).insert(Aura {
                                        aura_type: AuraType::Lullaby,
                                        radius: def.range,
                                        active: is_now_active,
                                    });
                                }
                                _ => {}
                            }
                        }
                        AbilityActivation::Activated => {
                            ability_state.active = true;
                            ability_state.cooldown_remaining = effective_cooldown;
                            ability_state.duration_remaining = def.duration_ticks;
                            if def.max_charges > 0 {
                                ability_state.charges -= 1;
                            }

                            // Instant effects on activation
                            match ability_id {
                                AbilityId::DissonantScreech => {
                                    // AoE CC: apply Disoriented to enemies in range
                                    commands.queue(AoeCcCommand {
                                        source_entity: entity,
                                        source_pos: WorldPos::zero(),
                                        radius: def.range,
                                        effect: StatusEffectId::Disoriented,
                                        duration: def.duration_ticks,
                                        source_owner: owner_player_id,
                                    });
                                }
                                AbilityId::Revulsion => {
                                    // AoE pushback: push enemies away
                                    commands.queue(RevulsionAoeCommand {
                                        source_entity: entity,
                                        source_pos: WorldPos::zero(),
                                        radius: def.range,
                                        push_distance: Fixed::from_num(2),
                                        source_owner: owner_player_id,
                                    });
                                }
                                _ => {}
                            }
                        }
                        AbilityActivation::Passive => {
                            // Passives can't be manually activated
                        }
                    }
                }
            }

            GameCommand::Research { building, upgrade } => {
                let building_entity = Entity::from_bits(building.0);

                // Check if upgrade is already queued at ANY ScratchingPost for this player
                // (prevents double-research across multiple buildings)
                {
                    let mut already_queued = false;
                    let target_player = research_queues
                        .get(building_entity)
                        .map(|(o, _)| o.player_id)
                        .ok();

                    if let Some(pid) = target_player {
                        for (rq_owner, rq_queue) in research_queues.iter() {
                            if rq_owner.player_id == pid
                                && rq_queue.queue.iter().any(|(u, _)| *u == upgrade)
                            {
                                already_queued = true;
                                break;
                            }
                        }
                    }
                    if already_queued {
                        continue;
                    }
                }

                if let Ok((owner, mut queue)) = research_queues.get_mut(building_entity) {
                    let player_id = owner.player_id as usize;

                    // Check not already researched
                    if let Some(pres) = player_resources.players.get(player_id) {
                        if pres.completed_upgrades.contains(&upgrade) {
                            continue;
                        }
                    }

                    let ustats = upgrade_stats(upgrade);

                    // Validate resources
                    if let Some(pres) = player_resources.players.get_mut(player_id) {
                        if pres.food < ustats.food_cost || pres.gpu_cores < ustats.gpu_cost {
                            continue;
                        }
                        pres.food -= ustats.food_cost;
                        pres.gpu_cores -= ustats.gpu_cost;
                    } else {
                        continue;
                    }

                    queue.queue.push_back((upgrade, ustats.research_time));
                }
            }

            GameCommand::CancelResearch { building } => {
                let building_entity = Entity::from_bits(building.0);
                if let Ok((owner, mut queue)) = research_queues.get_mut(building_entity) {
                    if let Some((upgrade, _)) = queue.queue.pop_front() {
                        let player_id = owner.player_id as usize;
                        let ustats = upgrade_stats(upgrade);
                        if let Some(pres) = player_resources.players.get_mut(player_id) {
                            pres.food += ustats.food_cost;
                            pres.gpu_cores += ustats.gpu_cost;
                        }
                    }
                }
            }
        }
    }
}
