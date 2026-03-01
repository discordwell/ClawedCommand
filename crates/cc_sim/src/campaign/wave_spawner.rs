use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use cc_core::components::*;
use cc_core::coords::WorldPos;
use cc_core::hero::{hero_base_kind, hero_modifiers};
use cc_core::mission::{WaveAiBehavior, WaveTrigger};
use cc_core::unit_stats::base_stats;

use crate::resources::SimClock;

use super::state::{CampaignPhase, CampaignState};

/// Tracks per-wave entity counts for WaveEliminated condition.
#[derive(Resource, Default)]
pub struct WaveTracker {
    /// wave_id -> (total_spawned, alive_count)
    pub waves: HashMap<String, (u32, u32)>,
    /// Wave IDs already processed by the spawner (prevent double-spawn).
    pub processed: HashSet<String>,
}

/// Tracks whether the current mission has been initialized (player setup spawned, immediate waves fired).
/// Resets automatically when a new mission is loaded (detected by mission ID change).
#[derive(Resource, Default)]
pub struct MissionStarted {
    pub started: bool,
    pub mission_id: Option<String>,
}

/// System: spawn entities from wave definitions.
///
/// On first tick of InMission: spawn player heroes/units/buildings, spawn Immediate waves.
/// Each tick: check AtTick waves against SimClock, check spawned_waves for trigger-spawned waves.
pub fn wave_spawner_system(
    mut commands: Commands,
    clock: Res<SimClock>,
    campaign: Res<CampaignState>,
    mut wave_tracker: ResMut<WaveTracker>,
    mut mission_started: ResMut<MissionStarted>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    let Some(mission) = campaign.current_mission.clone() else {
        return;
    };

    // Detect mission change and reset state for the new mission
    let current_id = mission.id.clone();
    if mission_started.mission_id.as_ref() != Some(&current_id) {
        mission_started.started = false;
        mission_started.mission_id = Some(current_id);
        wave_tracker.waves.clear();
        wave_tracker.processed.clear();
    }

    // First tick of InMission: spawn player setup + Immediate waves
    if !mission_started.started {
        mission_started.started = true;

        // Spawn player heroes
        for hero_spawn in &mission.player_setup.heroes {
            let kind = hero_base_kind(hero_spawn.hero_id);
            let base = base_stats(kind);
            let mods = hero_modifiers(hero_spawn.hero_id);

            let boosted_hp = base.health + mods.health_bonus;
            let boosted_speed = base.speed * mods.speed_multiplier;
            let boosted_damage = base.damage + mods.damage_bonus;
            let boosted_range = base.range + mods.range_bonus;

            commands.spawn((
                Position {
                    world: WorldPos::from_grid(hero_spawn.position),
                },
                Velocity::zero(),
                GridCell {
                    pos: hero_spawn.position,
                },
                MovementSpeed {
                    speed: boosted_speed,
                },
                Owner { player_id: 0 },
                UnitType { kind },
                Health {
                    current: boosted_hp,
                    max: boosted_hp,
                },
                AttackStats {
                    damage: boosted_damage,
                    range: boosted_range,
                    attack_speed: base.attack_speed,
                    cooldown_remaining: 0,
                },
                AttackTypeMarker {
                    attack_type: base.attack_type,
                },
                HeroIdentity {
                    hero_id: hero_spawn.hero_id,
                    mission_critical: hero_spawn.mission_critical,
                },
            ));
        }

        // Spawn player regular units
        for unit_spawn in &mission.player_setup.units {
            spawn_unit(&mut commands, unit_spawn.kind, unit_spawn.position, 0, None);
        }

        // Spawn Immediate waves
        for wave in &mission.enemy_waves {
            if matches!(wave.trigger, WaveTrigger::Immediate) {
                spawn_wave_entities(
                    &mut commands,
                    &wave.wave_id,
                    &wave.units,
                    &wave.ai_behavior,
                    &mut wave_tracker,
                );
                wave_tracker.processed.insert(wave.wave_id.clone());
            }
        }
    }

    // Check AtTick waves
    for wave in &mission.enemy_waves {
        if let WaveTrigger::AtTick(tick) = wave.trigger {
            if clock.tick == tick && !wave_tracker.processed.contains(&wave.wave_id) {
                spawn_wave_entities(
                    &mut commands,
                    &wave.wave_id,
                    &wave.units,
                    &wave.ai_behavior,
                    &mut wave_tracker,
                );
                wave_tracker.processed.insert(wave.wave_id.clone());
            }
        }
    }

    // Check trigger-spawned waves (OnTrigger waves marked in spawned_waves by trigger system)
    let spawned_waves = campaign.spawned_waves.clone();
    for wave in &mission.enemy_waves {
        if let WaveTrigger::OnTrigger(trigger_id) = &wave.trigger {
            if spawned_waves.contains(trigger_id)
                && !wave_tracker.processed.contains(&wave.wave_id)
            {
                spawn_wave_entities(
                    &mut commands,
                    &wave.wave_id,
                    &wave.units,
                    &wave.ai_behavior,
                    &mut wave_tracker,
                );
                wave_tracker.processed.insert(wave.wave_id.clone());
            }
        }
    }

    // Also check waves explicitly pushed to spawned_waves by wave_id
    for wave in &mission.enemy_waves {
        if campaign.spawned_waves.contains(&wave.wave_id)
            && !wave_tracker.processed.contains(&wave.wave_id)
        {
            spawn_wave_entities(
                &mut commands,
                &wave.wave_id,
                &wave.units,
                &wave.ai_behavior,
                &mut wave_tracker,
            );
            wave_tracker.processed.insert(wave.wave_id.clone());
        }
    }
}

/// Spawn all entities for a wave, tagging them with WaveMember.
fn spawn_wave_entities(
    commands: &mut Commands,
    wave_id: &str,
    units: &[cc_core::mission::UnitSpawn],
    ai_behavior: &WaveAiBehavior,
    wave_tracker: &mut WaveTracker,
) {
    let count = units.len() as u32;

    for unit_spawn in units {
        let wave_membership = WaveMember {
            wave_id: wave_id.to_string(),
        };
        let entity_id = spawn_unit(
            commands,
            unit_spawn.kind,
            unit_spawn.position,
            unit_spawn.player_id,
            Some(wave_membership),
        );

        // Apply AI behavior
        match ai_behavior {
            WaveAiBehavior::AttackMove(target) => {
                commands.entity(entity_id).insert(AttackMoveTarget {
                    target: *target,
                });
            }
            WaveAiBehavior::Defend => {
                commands.entity(entity_id).insert(HoldPosition);
            }
            WaveAiBehavior::Idle => {
                // No additional components needed
            }
            WaveAiBehavior::Patrol(_waypoints) => {
                // Patrol behavior not yet implemented — treat as hold for now
                commands.entity(entity_id).insert(HoldPosition);
            }
        }
    }

    wave_tracker
        .waves
        .insert(wave_id.to_string(), (count, count));
}

/// Spawn a single combat unit with base stats.
fn spawn_unit(
    commands: &mut Commands,
    kind: UnitKind,
    pos: cc_core::coords::GridPos,
    player_id: u8,
    wave_membership: Option<WaveMember>,
) -> Entity {
    let stats = base_stats(kind);
    let mut entity = commands.spawn((
        Position {
            world: WorldPos::from_grid(pos),
        },
        Velocity::zero(),
        GridCell { pos },
        MovementSpeed { speed: stats.speed },
        Owner { player_id },
        UnitType { kind },
        Health {
            current: stats.health,
            max: stats.health,
        },
        AttackStats {
            damage: stats.damage,
            range: stats.range,
            attack_speed: stats.attack_speed,
            cooldown_remaining: 0,
        },
        AttackTypeMarker {
            attack_type: stats.attack_type,
        },
    ));

    if let Some(wm) = wave_membership {
        entity.insert(wm);
    }

    entity.id()
}

/// System: track wave entity deaths and update WaveTracker counts.
///
/// Runs after cleanup_system so Dead markers are set. Decrements alive counts and
/// increments campaign enemy_kill_count.
pub fn wave_tracking_system(
    mut commands: Commands,
    mut campaign: ResMut<CampaignState>,
    mut wave_tracker: ResMut<WaveTracker>,
    dead_wave_units: Query<(Entity, &WaveMember), With<Dead>>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    for (entity, membership) in dead_wave_units.iter() {
        // Decrement alive count
        if let Some((_total, alive)) = wave_tracker.waves.get_mut(&membership.wave_id) {
            *alive = alive.saturating_sub(1);
        }

        // Increment global kill count
        campaign.enemy_kill_count += 1;

        // Remove WaveMember from dead entity to avoid re-counting
        commands.entity(entity).remove::<WaveMember>();
    }
}
