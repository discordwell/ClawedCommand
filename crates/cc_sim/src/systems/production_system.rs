use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::components::{
    AttackStats, AttackTypeMarker, Building, GridCell, Health, MovementSpeed, Owner, Position,
    Producer, ProductionQueue, RallyPoint, UnderConstruction, UnitType, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::unit_stats::base_stats;

use crate::resources::PlayerResources;

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

                    commands.spawn((
                        Position { world: spawn_world },
                        Velocity::zero(),
                        GridCell { pos: spawn_grid },
                        Owner {
                            player_id: owner.player_id,
                        },
                        UnitType { kind },
                        Health {
                            current: stats.health,
                            max: stats.health,
                        },
                        MovementSpeed { speed: stats.speed },
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

                    // Note: supply was already deducted in TrainUnit command
                    let _ = &rally;
                }
            }
        }
    }
}
