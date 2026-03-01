use bevy::prelude::*;

use cc_core::abilities::unit_abilities;
use cc_core::building_stats::building_stats;
use cc_core::commands::EntityId;
use cc_core::components::{
    AbilitySlots, AttackStats, AttackType, AttackTypeMarker, Aura, AuraType, Building,
    BuildingKind, DreamSiegeTimer, GatherState, Gathering, GridCell, Health, MoveTarget,
    MovementSpeed, NineLivesTracker, Owner, Position, Producer, ProductionQueue, RallyPoint,
    ResearchQueue, Researcher, ResourceDeposit, StatModifiers, UnderConstruction, UnitKind,
    UnitType, Velocity,
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
        &mut Health,
    )>,
    _player_resources: ResMut<PlayerResources>,
    deposits: Query<(Entity, &Position, &ResourceDeposit), Without<Building>>,
) {
    for (entity, building, owner, pos, under_construction, prod_queue, rally, mut health) in
        buildings.iter_mut()
    {
        // Phase 1: Tick construction countdown
        if let Some(mut uc) = under_construction {
            if uc.remaining_ticks > 0 {
                uc.remaining_ticks -= 1;
            }

            // Scale HP proportionally to construction progress (10% to 100%)
            // Use min() so combat damage is preserved — never heal above the formula value
            if uc.remaining_ticks > 0 && uc.total_ticks > 0 {
                let progress = Fixed::from_num(1.0f32)
                    - Fixed::from_num(uc.remaining_ticks as f32)
                        / Fixed::from_num(uc.total_ticks as f32);
                let formula_hp =
                    health.max * (Fixed::from_num(0.1f32) + Fixed::from_num(0.9f32) * progress);
                if health.current < formula_hp {
                    health.current = formula_hp;
                }
            }

            if uc.remaining_ticks == 0 {
                // Full HP on completion
                health.current = health.max;
                // Construction complete - promote to producer if applicable
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

                    let mut unit_health = Health {
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
                            &mut unit_health,
                            &mut attack_stats,
                            &mut move_speed,
                        );
                    }

                    let mut entity_cmds = commands.spawn((
                        Position { world: spawn_world },
                        Velocity::zero(),
                        GridCell { pos: spawn_grid },
                        Owner {
                            player_id: owner.player_id,
                        },
                        UnitType { kind },
                        unit_health,
                        move_speed,
                        attack_stats,
                        AttackTypeMarker {
                            attack_type: stats.attack_type,
                        },
                        AbilitySlots::from_abilities(unit_abilities(kind)),
                        StatusEffects::default(),
                        StatModifiers::default(),
                    ));

                    // DreamSiegeTimer for Catnappers
                    if kind == UnitKind::Catnapper {
                        entity_cmds.insert(DreamSiegeTimer::default());
                    }

                    // Chonk passive components: GravitationalChonk aura + NineLives tracker
                    if kind == UnitKind::Chonk {
                        entity_cmds.insert((
                            Aura {
                                aura_type: AuraType::GravitationalChonk,
                                radius: Fixed::from_bits(3 << 16),
                                active: true,
                            },
                            NineLivesTracker::default(),
                        ));
                    }

                    let new_entity = entity_cmds.id();

                    // Auto-move to rally point if set
                    if let Some(rally) = rally {
                        let rally_world = WorldPos::from_grid(rally.target);
                        commands
                            .entity(new_entity)
                            .insert(MoveTarget { target: rally_world });
                    } else if kind == UnitKind::Pawdler {
                        // Auto-gather: send newly produced Pawdlers to nearest deposit
                        let spawn_pos = spawn_world;
                        let mut best_dist_sq = i64::MAX;
                        let mut best_deposit: Option<(Entity, WorldPos)> = None;

                        for (dep_entity, dep_pos, dep) in deposits.iter() {
                            if dep.remaining == 0 {
                                continue;
                            }
                            let dx =
                                spawn_pos.x.to_bits() as i64 - dep_pos.world.x.to_bits() as i64;
                            let dy =
                                spawn_pos.y.to_bits() as i64 - dep_pos.world.y.to_bits() as i64;
                            let dist_sq = dx * dx + dy * dy;
                            if dist_sq < best_dist_sq {
                                best_dist_sq = dist_sq;
                                best_deposit = Some((dep_entity, dep_pos.world));
                            }
                        }

                        if let Some((dep_entity, dep_world)) = best_deposit {
                            let dep_resource = deposits.get(dep_entity).unwrap().2.resource_type;
                            commands.entity(new_entity).insert((
                                Gathering {
                                    deposit_entity: EntityId(dep_entity.to_bits()),
                                    carried_type: dep_resource,
                                    carried_amount: 0,
                                    state: GatherState::MovingToDeposit,
                                    last_pos: (spawn_pos.x, spawn_pos.y),
                                    stale_ticks: 0,
                                },
                                MoveTarget { target: dep_world },
                            ));
                        }
                    }
                }
            }
        }
    }
}
