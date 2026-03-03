use bevy::prelude::*;

use cc_core::components::{AbilitySlots, Dead, StatModifiers};

/// Tick ability cooldowns and durations. Deactivate expired activated abilities.
pub fn ability_cooldown_system(
    mut query: Query<(&mut AbilitySlots, Option<&StatModifiers>), Without<Dead>>,
) {
    for (mut slots, modifiers) in query.iter_mut() {
        let cooldown_mult = modifiers
            .map(|m| m.cooldown_multiplier)
            .unwrap_or(cc_core::math::FIXED_ONE);

        for slot in slots.slots.iter_mut() {
            // Tick cooldown
            if slot.cooldown_remaining > 0 {
                // cooldown_multiplier < 1.0 means faster cooldowns (e.g. TacticalUplink).
                // We apply it as: if multiplier <= 0.5, tick down by 2 instead of 1.
                // For simplicity at 10hz, we tick by 1 each tick and reduce cooldown_remaining
                // on activation based on multiplier. But the plan says read it here, so:
                // Use the simple approach: always tick by 1. The cooldown_multiplier
                // is applied when setting cooldown_remaining in command_system.
                slot.cooldown_remaining -= 1;
            }

            // Tick active duration for activated abilities
            if slot.active && slot.duration_remaining > 0 {
                slot.duration_remaining -= 1;
                if slot.duration_remaining == 0 {
                    // Duration expired — deactivate (but not toggles, they stay active)
                    let def = cc_core::abilities::ability_def(slot.id);
                    if def.activation == cc_core::abilities::AbilityActivation::Activated {
                        slot.active = false;
                    }
                }
            }

            // Recharge charges over time (for charge-based abilities)
            let def = cc_core::abilities::ability_def(slot.id);
            if def.max_charges > 0 && slot.charges < def.max_charges && slot.cooldown_remaining == 0
            {
                slot.charges += 1;
                if slot.charges < def.max_charges {
                    // Apply cooldown multiplier to charge regen cooldown
                    let base_cd = def.cooldown_ticks;
                    let adjusted = (cc_core::math::Fixed::from_num(base_cd as i32) * cooldown_mult)
                        .to_num::<u32>();
                    slot.cooldown_remaining = adjusted.max(1);
                }
            }
        }
    }
}
