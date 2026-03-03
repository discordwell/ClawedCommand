use bevy::prelude::*;

use cc_core::components::{Dead, Health, MoveTarget, Owner, Path, Position};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;
use cc_core::mission::MissionDefinition;
use cc_core::mutator::{HazardDirection, MissionMutator};
use cc_core::terrain::{FLAG_LAVA, FLAG_TEMP_BLOCKED, FLAG_TOXIC, FLAG_WATER_CONVERTED};

use crate::campaign::mutator_state::{ControlRestrictions, FogState, MutatorState, should_fire};
use crate::campaign::state::{
    CampaignPhase, CampaignState, MissionFailedEvent, TimeLimitWarningEvent,
};
use crate::resources::{MapResource, SimClock, SimRng};
use crate::systems::damage::ApplyDamageCommand;

/// Initialize mutator state and control restrictions from the current mission definition.
/// Called when a mission is loaded.
pub fn mutator_init(
    _campaign: &CampaignState,
    mission: &MissionDefinition,
    restrictions: &mut ControlRestrictions,
    mutator_state: &mut MutatorState,
    fog: &mut FogState,
) {
    // Reset to defaults
    *restrictions = ControlRestrictions::default();
    *fog = FogState::default();

    let mut active = vec![true; mission.mutators.len()];

    for (i, mutator) in mission.mutators.iter().enumerate() {
        // DamageZone with active_from_start=false starts inactive
        if let MissionMutator::DamageZone {
            active_from_start: false,
            ..
        } = mutator
        {
            active[i] = false;
        }

        match mutator {
            MissionMutator::VoiceOnlyControl {
                ai_enabled,
                enemy_difficulty_multiplier,
            } => {
                restrictions.mouse_keyboard_enabled = false;
                restrictions.voice_enabled = true;
                restrictions.ai_enabled = *ai_enabled;
                restrictions.enemy_difficulty_multiplier = *enemy_difficulty_multiplier;
            }
            MissionMutator::NoAiControl => {
                restrictions.ai_enabled = false;
            }
            MissionMutator::NoBuildMode => {
                restrictions.building_enabled = false;
            }
            MissionMutator::AiOnlyControl { .. } => {
                restrictions.mouse_keyboard_enabled = false;
                restrictions.voice_enabled = false;
                restrictions.ai_enabled = true;
            }
            MissionMutator::RestrictedUnits {
                allowed_kinds,
                max_unit_count,
            } => {
                restrictions.allowed_unit_kinds = Some(allowed_kinds.clone());
                restrictions.max_unit_count = *max_unit_count;
            }
            MissionMutator::DenseFog {
                vision_reduction, ..
            } => {
                fog.vision_reduction = *vision_reduction;
                fog.currently_clear = false;
            }
            MissionMutator::Flooding {
                initial_water_level,
                ..
            } => {
                mutator_state.current_water_level = *initial_water_level;
            }
            _ => {}
        }
    }

    mutator_state.active = active;
    mutator_state.lava_advance_count = 0;
    mutator_state.toxic_advance_count = 0;
    mutator_state.wind_active = false;
    mutator_state.fog_cleared = false;
    mutator_state.time_warning_fired = false;
}

/// Set dynamic flags on a tile by grid coordinates, bounds-checked.
fn set_tile_flags(map: &mut cc_core::map::GameMap, x: i32, y: i32, flags: u8) {
    let pos = GridPos::new(x, y);
    if let Some(tile) = map.get_mut(pos) {
        tile.dynamic_flags |= flags;
    }
}

/// Environmental hazard system — processes active hazard mutators each tick.
/// Modifies terrain flags, applies overlays, and tracks hazard progression.
pub fn environmental_hazard_system(
    campaign: Res<CampaignState>,
    mut mutator_state: ResMut<MutatorState>,
    mut map_res: ResMut<MapResource>,
    sim_clock: Res<SimClock>,
    mut sim_rng: ResMut<SimRng>,
    mut fog: ResMut<FogState>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    let mission = match &campaign.current_mission {
        Some(m) => m,
        None => return,
    };

    let tick = sim_clock.tick;
    let width = map_res.map.width as i32;
    let height = map_res.map.height as i32;

    for (i, mutator) in mission.mutators.iter().enumerate() {
        if !mutator_state.is_active(i) {
            continue;
        }

        match mutator {
            MissionMutator::LavaRise {
                interval_ticks,
                direction,
                rows_per_wave,
                initial_delay_ticks,
                ..
            } => {
                if !should_fire(tick, *initial_delay_ticks, *interval_ticks) {
                    continue;
                }
                let wave = mutator_state.lava_advance_count;
                let flags = FLAG_LAVA | FLAG_TEMP_BLOCKED;

                for row_offset in 0..*rows_per_wave {
                    let row = wave * rows_per_wave + row_offset;
                    match direction {
                        HazardDirection::East => {
                            let x = row as i32;
                            if x < width {
                                for y in 0..height {
                                    set_tile_flags(&mut map_res.map, x, y, flags);
                                }
                            }
                        }
                        HazardDirection::West => {
                            let x = width - 1 - row as i32;
                            if x >= 0 {
                                for y in 0..height {
                                    set_tile_flags(&mut map_res.map, x, y, flags);
                                }
                            }
                        }
                        HazardDirection::North => {
                            let y = row as i32;
                            if y < height {
                                for x in 0..width {
                                    set_tile_flags(&mut map_res.map, x, y, flags);
                                }
                            }
                        }
                        HazardDirection::South => {
                            let y = height - 1 - row as i32;
                            if y >= 0 {
                                for x in 0..width {
                                    set_tile_flags(&mut map_res.map, x, y, flags);
                                }
                            }
                        }
                        HazardDirection::AllEdges => {
                            let r = row as i32;
                            for x in 0..width {
                                if r < height {
                                    set_tile_flags(&mut map_res.map, x, r, flags);
                                }
                                let sy = height - 1 - r;
                                if sy >= 0 {
                                    set_tile_flags(&mut map_res.map, x, sy, flags);
                                }
                            }
                            for y in 0..height {
                                if r < width {
                                    set_tile_flags(&mut map_res.map, r, y, flags);
                                }
                                let ex = width - 1 - r;
                                if ex >= 0 {
                                    set_tile_flags(&mut map_res.map, ex, y, flags);
                                }
                            }
                        }
                    }
                }
                mutator_state.lava_advance_count += 1;
            }

            MissionMutator::ToxicTide {
                interval_ticks,
                rows_per_wave,
                initial_delay_ticks,
                safe_zone_center,
                min_safe_radius,
                ..
            } => {
                if !should_fire(tick, *initial_delay_ticks, *interval_ticks) {
                    continue;
                }

                let center = safe_zone_center.unwrap_or(GridPos::new(width / 2, height / 2));
                let ring = mutator_state.toxic_advance_count;

                for row_offset in 0..*rows_per_wave {
                    let depth = ring * rows_per_wave + row_offset;
                    for x in 0..width {
                        for y in 0..height {
                            let dx = (x - center.x).unsigned_abs();
                            let dy = (y - center.y).unsigned_abs();
                            let dist = dx.max(dy);
                            let edge_dist_x = x.min(width - 1 - x) as u32;
                            let edge_dist_y = y.min(height - 1 - y) as u32;
                            let edge_dist = edge_dist_x.min(edge_dist_y);

                            if edge_dist <= depth && dist > *min_safe_radius {
                                set_tile_flags(&mut map_res.map, x, y, FLAG_TOXIC);
                            }
                        }
                    }
                }
                mutator_state.toxic_advance_count += 1;
            }

            MissionMutator::Tremors {
                interval_ticks,
                terrain_change_chance,
                epicenter_radius,
                initial_delay_ticks,
                ..
            } => {
                if !should_fire(tick, *initial_delay_ticks, *interval_ticks) {
                    continue;
                }

                let cx = sim_rng.next_bounded(width as u32) as i32;
                let cy = sim_rng.next_bounded(height as u32) as i32;
                let radius = *epicenter_radius as i32;

                for dx in -radius..=radius {
                    for dy in -radius..=radius {
                        let x = cx + dx;
                        let y = cy + dy;
                        if x < 0 || y < 0 || x >= width || y >= height {
                            continue;
                        }
                        if sim_rng.next_bounded(100) < *terrain_change_chance {
                            set_tile_flags(&mut map_res.map, x, y, FLAG_TEMP_BLOCKED);
                        }
                    }
                }
            }

            MissionMutator::Flooding {
                interval_ticks,
                max_water_level,
                initial_delay_ticks,
                ..
            } => {
                if !should_fire(tick, *initial_delay_ticks, *interval_ticks) {
                    continue;
                }

                if mutator_state.current_water_level < *max_water_level {
                    mutator_state.current_water_level += 1;
                    let level = mutator_state.current_water_level;

                    for x in 0..width {
                        for y in 0..height {
                            let pos = GridPos::new(x, y);
                            if let Some(tile) = map_res.map.get_mut(pos)
                                && tile.elevation < level
                            {
                                tile.dynamic_flags |= FLAG_WATER_CONVERTED;
                            }
                        }
                    }
                }
            }

            MissionMutator::WindStorm {
                interval_ticks,
                duration_ticks,
                initial_delay_ticks,
                ..
            } => {
                if *interval_ticks == 0 || tick < *initial_delay_ticks {
                    continue;
                }
                let elapsed = tick - initial_delay_ticks;
                let cycle = elapsed % interval_ticks;
                mutator_state.wind_active = cycle < *duration_ticks;
            }

            MissionMutator::DenseFog {
                periodic_clearing, ..
            } => {
                if let Some(clearing) = periodic_clearing
                    && clearing.interval_ticks > 0
                {
                    let cycle = tick % clearing.interval_ticks;
                    let is_clear = cycle < clearing.clear_duration_ticks;
                    fog.currently_clear = is_clear;
                    mutator_state.fog_cleared = is_clear;
                }
            }

            _ => {} // Non-environmental mutators handled elsewhere
        }
    }
}

/// Damage units standing on hazardous tiles (lava, toxic).
pub fn hazard_damage_system(
    campaign: Res<CampaignState>,
    mutator_state: Res<MutatorState>,
    map_res: Res<MapResource>,
    mut commands: Commands,
    units: Query<(Entity, &Position, &Health), With<Owner>>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    let mission = match &campaign.current_mission {
        Some(m) => m,
        None => return,
    };

    // Collect damage-per-tick values for lava and toxic from active mutators
    let mut lava_dpt: u32 = 0;
    let mut toxic_dpt: u32 = 0;
    let mut damage_zones: Vec<(&[GridPos], u32)> = Vec::new();

    for (i, mutator) in mission.mutators.iter().enumerate() {
        if !mutator_state.is_active(i) {
            continue;
        }
        match mutator {
            MissionMutator::LavaRise {
                damage_per_tick, ..
            } => {
                lava_dpt = lava_dpt.max(*damage_per_tick);
            }
            MissionMutator::ToxicTide {
                damage_per_tick, ..
            } => {
                toxic_dpt = toxic_dpt.max(*damage_per_tick);
            }
            MissionMutator::DamageZone {
                tiles,
                damage_per_tick,
                toggle_flag,
                ..
            } => {
                let flag_active = match toggle_flag {
                    Some(flag_name) => campaign.flags.contains(flag_name),
                    None => true,
                };
                if flag_active {
                    damage_zones.push((tiles.as_slice(), *damage_per_tick));
                }
            }
            _ => {}
        }
    }

    let map = &map_res.map;

    for (entity, pos, _health) in units.iter() {
        let gx = pos.world.x.to_num::<i32>();
        let gy = pos.world.y.to_num::<i32>();
        let grid_pos = GridPos::new(gx, gy);

        let tile = match map.get(grid_pos) {
            Some(t) => t,
            None => continue,
        };

        let flags = tile.dynamic_flags;

        // Lava damage
        if flags & FLAG_LAVA != 0 && lava_dpt > 0 {
            commands.queue(ApplyDamageCommand {
                target: entity,
                damage: Fixed::from_num(lava_dpt),
            });
        }

        // Toxic damage
        if flags & FLAG_TOXIC != 0 && toxic_dpt > 0 {
            commands.queue(ApplyDamageCommand {
                target: entity,
                damage: Fixed::from_num(toxic_dpt),
            });
        }

        // Damage zones
        for (tiles, dpt) in &damage_zones {
            if tiles.contains(&grid_pos) {
                commands.queue(ApplyDamageCommand {
                    target: entity,
                    damage: Fixed::from_num(*dpt),
                });
            }
        }
    }
}

/// Tick system for time-based mutators (TimeLimit countdown, etc).
pub fn mutator_tick_system(
    campaign: Res<CampaignState>,
    mut mutator_state: ResMut<MutatorState>,
    sim_clock: Res<SimClock>,
    mut fail_events: MessageWriter<MissionFailedEvent>,
    mut warning_events: MessageWriter<TimeLimitWarningEvent>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    let mission = match &campaign.current_mission {
        Some(m) => m,
        None => return,
    };

    let tick = sim_clock.tick;

    for (i, mutator) in mission.mutators.iter().enumerate() {
        if !mutator_state.is_active(i) {
            continue;
        }
        if let MissionMutator::TimeLimit {
            max_ticks,
            warning_at,
        } = mutator
        {
            // Fire warning event once when reaching warning_at threshold
            if !mutator_state.time_warning_fired && tick >= *warning_at {
                mutator_state.time_warning_fired = true;
                warning_events.write(TimeLimitWarningEvent);
            }
            if tick >= *max_ticks {
                fail_events.write(MissionFailedEvent {
                    reason: "Time limit exceeded".to_string(),
                });
                return;
            }
        }
    }
}

/// Displace units when wind is active (WindStorm mutator).
/// Pushes all units in the wind direction; kills those pushed off-map when `can_push_off_map` is true.
pub fn wind_displacement_system(
    campaign: Res<CampaignState>,
    mutator_state: Res<MutatorState>,
    map_res: Res<MapResource>,
    mut commands: Commands,
    mut units: Query<(Entity, &mut Position, &Health), (With<Owner>, Without<Dead>)>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }
    if !mutator_state.wind_active {
        return;
    }

    let mission = match &campaign.current_mission {
        Some(m) => m,
        None => return,
    };

    // Find active WindStorm mutator
    let mut wind_dir = None;
    let mut wind_force = 0u32;
    let mut push_off = false;

    for (i, mutator) in mission.mutators.iter().enumerate() {
        if !mutator_state.is_active(i) {
            continue;
        }
        if let MissionMutator::WindStorm {
            direction,
            force,
            can_push_off_map,
            ..
        } = mutator
        {
            wind_dir = Some(*direction);
            wind_force = *force;
            push_off = *can_push_off_map;
            break;
        }
    }

    let Some(direction) = wind_dir else { return };

    // AllEdges has no single displacement direction — skip
    if direction == HazardDirection::AllEdges {
        return;
    }

    let force = wind_force as i32;
    let (dx, dy): (i32, i32) = match direction {
        HazardDirection::North => (0, -force),
        HazardDirection::South => (0, force),
        HazardDirection::East => (force, 0),
        HazardDirection::West => (-force, 0),
        HazardDirection::AllEdges => unreachable!(),
    };

    let map_w = map_res.map.width as i32;
    let map_h = map_res.map.height as i32;
    let max_x = Fixed::from_num(map_w - 1);
    let max_y = Fixed::from_num(map_h - 1);
    let zero = Fixed::from_num(0);

    for (entity, mut pos, health) in units.iter_mut() {
        let new_x = pos.world.x + Fixed::from_num(dx);
        let new_y = pos.world.y + Fixed::from_num(dy);

        let out_of_bounds = new_x < zero || new_x > max_x || new_y < zero || new_y > max_y;

        if out_of_bounds && push_off {
            // Lethal damage — pushed off the map
            commands.queue(ApplyDamageCommand {
                target: entity,
                damage: health.max,
            });
        } else {
            // Clamp to map bounds
            pos.world.x = new_x.max(zero).min(max_x);
            pos.world.y = new_y.max(zero).min(max_y);
        }

        // Clear movement targets to prevent walk-back
        commands
            .entity(entity)
            .remove::<MoveTarget>()
            .remove::<Path>();
    }
}
