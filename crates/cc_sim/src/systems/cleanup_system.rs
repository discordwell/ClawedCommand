use bevy::prelude::*;

use cc_core::components::{Dead, Health};
use cc_core::math::FIXED_ZERO;

/// Two-phase death cleanup:
/// Phase 1: Health <= 0 → add Dead marker (gives client one tick for death visuals).
/// Phase 2: Entities with Dead → despawn.
pub fn cleanup_system(
    mut commands: Commands,
    newly_dead: Query<(Entity, &Health), Without<Dead>>,
    dead_entities: Query<Entity, With<Dead>>,
) {
    // Phase 1: Mark newly dead
    for (entity, health) in newly_dead.iter() {
        if health.current <= FIXED_ZERO {
            commands.entity(entity).insert(Dead);
        }
    }

    // Phase 2: Despawn entities that were marked Dead last tick
    // (They already had Dead for at least one tick — safe to remove)
    // Note: entities just marked in Phase 1 won't appear in dead_entities
    // this tick because the insert is deferred via Commands.
    for entity in dead_entities.iter() {
        commands.entity(entity).despawn();
    }
}
