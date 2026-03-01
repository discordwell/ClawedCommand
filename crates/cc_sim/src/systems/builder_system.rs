use bevy::prelude::*;

use crate::resources::PlayerResources;
use cc_core::building_stats::building_stats;
use cc_core::components::{
    BuildOrder, Building, Dead, GridCell, Health, Owner, Position, Producer, ProductionQueue,
    UnderConstruction, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::tuning::BUILDER_PROXIMITY;

/// Checks each builder with a `BuildOrder` for adjacency to the build site.
/// When the builder arrives (Chebyshev distance <= BUILDER_PROXIMITY), spawns
/// the building and removes the `BuildOrder`.
///
/// Uses Position (world coords) converted to grid, since GridCell may not yet be
/// synced after movement_system runs this tick.
pub fn builder_system(
    mut commands: Commands,
    builders: Query<(Entity, &Position, &BuildOrder, &Owner), Without<Dead>>,
    mut player_resources: ResMut<PlayerResources>,
) {
    for (entity, pos, build_order, owner) in builders.iter() {
        let builder_grid = pos.world.to_grid();
        let dx = (builder_grid.x - build_order.position.x).abs();
        let dy = (builder_grid.y - build_order.position.y).abs();
        if dx <= BUILDER_PROXIMITY && dy <= BUILDER_PROXIMITY {
            let bstats = building_stats(build_order.building_kind);
            let world = WorldPos::from_grid(build_order.position);

            let mut building = commands.spawn((
                Position { world },
                Velocity::zero(),
                GridCell {
                    pos: build_order.position,
                },
                Owner {
                    player_id: owner.player_id,
                },
                Building {
                    kind: build_order.building_kind,
                },
                Health {
                    current: bstats.health,
                    max: bstats.health,
                },
            ));

            if bstats.build_time > 0 {
                building.insert(UnderConstruction {
                    remaining_ticks: bstats.build_time,
                    total_ticks: bstats.build_time,
                });
            } else {
                if !bstats.can_produce.is_empty() {
                    building.insert((Producer, ProductionQueue::default()));
                }
            }

            if bstats.supply_provided > 0 {
                if let Some(pres) = player_resources
                    .players
                    .get_mut(owner.player_id as usize)
                {
                    pres.supply_cap += bstats.supply_provided;
                }
            }

            commands.entity(entity).remove::<BuildOrder>();
        }
    }
}
