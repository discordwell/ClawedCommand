use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::components::Dead;
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};
use cc_core::tuning::CC_IMMUNITY_TICKS;

/// Tick down status effect durations, remove expired, handle CC immunity.
/// Also converts 5 Annoyed stacks into Tilted CC (T1 fix).
pub fn status_effect_system(
    mut query: Query<&mut StatusEffects, Without<Dead>>,
) {
    for mut effects in query.iter_mut() {
        let had_cc = effects.has_active_cc();

        // Tick remaining_ticks on all effects
        for instance in effects.effects.iter_mut() {
            if instance.remaining_ticks > 0 {
                instance.remaining_ticks -= 1;
            }

            // Decay Corroded stacks: lose 1 stack per 80 ticks
            if instance.effect == StatusEffectId::Corroded && instance.remaining_ticks % 80 == 0 {
                instance.stacks = instance.stacks.saturating_sub(1);
            }
        }

        // T1 Fix: Convert 5 Annoyed stacks -> Tilted CC
        let annoyed_stacks: u32 = effects
            .effects
            .iter()
            .filter(|e| e.effect == StatusEffectId::Annoyed && e.remaining_ticks > 0)
            .map(|e| e.stacks)
            .sum();
        if annoyed_stacks >= 5 {
            effects
                .effects
                .retain(|e| e.effect != StatusEffectId::Annoyed);
            if !effects.is_cc_immune() {
                effects.effects.push(StatusInstance {
                    effect: StatusEffectId::Tilted,
                    remaining_ticks: 40,
                    stacks: 1,
                    source: EntityId(0),
                });
            }
        }

        // Remove expired effects
        effects.effects.retain(|e| e.remaining_ticks > 0);

        // Tick CC immunity
        if effects.cc_immunity_remaining > 0 {
            effects.cc_immunity_remaining -= 1;
        }

        // Grant CC immunity when CC expires
        let has_cc_now = effects.has_active_cc();
        if had_cc && !has_cc_now {
            effects.cc_immunity_remaining = CC_IMMUNITY_TICKS;
        }
    }
}
