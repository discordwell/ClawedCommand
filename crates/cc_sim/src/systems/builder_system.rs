use bevy::prelude::*;

use crate::resources::{MapResource, PlayerResources};
use cc_core::building_stats::building_stats;
use cc_core::components::{
    BuildOrder, Building, GridCell, Health, Owner, Position, Producer, ProductionQueue,
    UnderConstruction, Velocity,
};
use cc_core::coords::WorldPos;

/// Checks each builder with a `BuildOrder` for adjacency to the build site.
/// When the builder arrives (Chebyshev distance <= 1), spawns the building and
/// removes the `BuildOrder`.
///
/// Runs after `movement_system` so the builder has moved this tick before we check.
pub fn builder_system(
    mut commands: Commands,
    builders: Query<(Entity, &GridCell, &BuildOrder, &Owner)>,
    mut player_resources: ResMut<PlayerResources>,
) {
    for (entity, grid, build_order, owner) in builders.iter() {
        let dx = (grid.pos.x - build_order.position.x).abs();
        let dy = (grid.pos.y - build_order.position.y).abs();
        // Adjacent = Chebyshev distance <= 1 (includes standing on the tile)
        if dx <= 1 && dy <= 1 {
            let bstats = building_stats(build_order.building_kind);
            let world = WorldPos::from_grid(build_order.position);

            // Spawn the building
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
                // Pre-built: add producer + queue immediately
                if !bstats.can_produce.is_empty() {
                    building.insert((Producer, ProductionQueue::default()));
                }
            }

            // Grant supply_cap now that the building is actually placed
            if bstats.supply_provided > 0 {
                if let Some(pres) = player_resources
                    .players
                    .get_mut(owner.player_id as usize)
                {
                    pres.supply_cap += bstats.supply_provided;
                }
            }

            // Remove BuildOrder from builder
            commands.entity(entity).remove::<BuildOrder>();
        }
    }
}
