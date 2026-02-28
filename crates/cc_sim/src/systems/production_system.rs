use bevy::prelude::*;

use cc_core::abilities::unit_abilities;
use cc_core::building_stats::building_stats;
use cc_core::components::{
    AbilitySlots, AttackStats, AttackType, AttackTypeMarker, Building, BuildingKind, GridCell,
    Health, MoveTarget, MovementSpeed, Owner, Position, Producer, ProductionQueue, RallyPoint,
    ResearchQueue, Researcher, StatModifiers, UnderConstruction, UnitType, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::math::Fixed;
use cc_core::status_effects::StatusEffects;
use cc_core::unit_stats::base_stats;

use crate::resources::PlayerResources;
use crate::systems::research_system::apply_upgrades_to_new_unit;

/// Ticks UnderConstruction, ticks ProductionQueue, spawns units on completion.
pub fn production_system(
    mut commands: Commands,
    mut buildings: Query<(
        Entity,
        &Building,
        &Owner,
        &Position,
        Option<&mut UnderConstruction>,
        Option<&mut ProductionQueue>,
        Option<&RallyPoint>,
    )>,
    _player_resources: ResMut<PlayerResources>,
) {
    for (entity, building, owner, pos, under_construction, prod_queue, rally) in
        buildings.iter_mut()
    {
        // Phase 1: Tick construction countdown
        if let Some(mut uc) = under_construction {
            if uc.remaining_ticks > 0 {
                uc.remaining_ticks -= 1;
            }
            if uc.remaining_ticks == 0 {
                // Construction complete — promote to producer if applicable
                commands.entity(entity).remove::<UnderConstruction>();
                let bstats = building_stats(building.kind);
                if !bstats.can_produce.is_empty() {
                    commands
                        .entity(entity)
                        .insert((Producer, ProductionQueue::default()));
                }

                // ScratchingPost gets Researcher + ResearchQueue
                if building.kind == BuildingKind::ScratchingPost {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // LaserPointer gets AttackStats for tower combat
                if building.kind == BuildingKind::LaserPointer {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: Fixed::from_bits(10 << 16), // 10 damage
                            range: Fixed::from_bits(6 << 16),   // 6 range
                            attack_speed: 15,                    // 1.5s between attacks
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }
            }
            continue; // Don't process production while under construction
        }

        // Phase 2: Tick production queue
        if let Some(mut queue) = prod_queue {
            if let Some((unit_kind, ticks_remaining)) = queue.queue.front_mut() {
                if *ticks_remaining > 0 {
                    *ticks_remaining -= 1;
                }
                if *ticks_remaining == 0 {
                    let kind = *unit_kind;
                    queue.queue.pop_front();

                    // Spawn the trained unit at the building's position
                    let stats = base_stats(kind);
                    let spawn_grid = pos.world.to_grid();
                    let spawn_world = WorldPos::from_grid(spawn_grid);

                    let mut health = Health {
                        current: stats.health,
                        max: stats.health,
                    };
                    let mut attack_stats = AttackStats {
                        damage: stats.damage,
                        range: stats.range,
                        attack_speed: stats.attack_speed,
                        cooldown_remaining: 0,
                    };
                    let mut move_speed = MovementSpeed { speed: stats.speed };

                    // Apply completed upgrades to newly spawned unit
                    let player_id = owner.player_id as usize;
                    if let Some(pres) = _player_resources.players.get(player_id) {
                        apply_upgrades_to_new_unit(
                            kind,
                            &pres.completed_upgrades,
                            &mut health,
                            &mut attack_stats,
                            &mut move_speed,
                        );
                    }

                    let new_entity = commands
                        .spawn((
                            Position { world: spawn_world },
                            Velocity::zero(),
                            GridCell { pos: spawn_grid },
                            Owner {
                                player_id: owner.player_id,
                            },
                            UnitType { kind },
                            health,
                            move_speed,
                            attack_stats,
                            AttackTypeMarker {
                                attack_type: stats.attack_type,
                            },
                            AbilitySlots::from_abilities(unit_abilities(kind)),
                            StatusEffects::default(),
                            StatModifiers::default(),
                        ))
                        .id();

                    // Auto-move to rally point if set
                    if let Some(rally) = rally {
                        let rally_world = WorldPos::from_grid(rally.target);
                        commands
                            .entity(new_entity)
                            .insert(MoveTarget { target: rally_world });
                    }
                }
            }
        }
    }
}
