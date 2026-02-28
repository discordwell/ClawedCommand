use bevy::prelude::*;

use cc_core::math::{Fixed, FIXED_ZERO};

/// Shared command to apply damage, avoiding borrow conflicts between attacker/target queries.
/// Used by both combat_system (melee) and projectile_system (ranged hits).
pub struct ApplyDamageCommand {
    pub target: Entity,
    pub damage: Fixed,
}

impl Command for ApplyDamageCommand {
    fn apply(self, world: &mut World) {
        if let Some(mut health) = world.get_mut::<cc_core::components::Health>(self.target) {
            health.current -= self.damage;
            if health.current < FIXED_ZERO {
                health.current = FIXED_ZERO;
            }
        }
    }
}
