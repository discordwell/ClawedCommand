use bevy::prelude::*;

use cc_core::components::{Dead, Health};
use cc_core::math::FIXED_ZERO;

/// Mark units with zero health as Dead.
/// Actual despawn is handled by the client's death_fade_system after the visual fade completes.
pub fn cleanup_system(
    mut commands: Commands,
    newly_dead: Query<(Entity, &Health), Without<Dead>>,
) {
    for (entity, health) in newly_dead.iter() {
        if health.current <= FIXED_ZERO {
            commands.entity(entity).insert(Dead);
        }
    }
}
