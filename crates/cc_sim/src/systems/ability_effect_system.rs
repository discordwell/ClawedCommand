use bevy::prelude::*;

use cc_core::abilities;
use cc_core::commands::EntityId;
use cc_core::components::{AbilitySlots, Dead, DreamSiegeTimer, Health, Owner};
use cc_core::status_effects::StatusEffects;

/// Bridge system: reads AbilitySlots and applies/refreshes corresponding StatusEffects
/// when abilities are active. Also handles DreamSiege damage reset.
pub fn ability_effect_system(
    mut query: Query<
        (
            Entity,
            &AbilitySlots,
            &mut StatusEffects,
            Option<&Health>,
            Option<&mut DreamSiegeTimer>,
            Option<&Owner>,
        ),
        Without<Dead>,
    >,
) {
    for (entity, slots, mut effects, health, dream_siege_timer, _owner) in query.iter_mut() {
        // T2 Fix: DreamSiege reset on Catnapper taking damage
        if let Some(mut siege_timer) = dream_siege_timer
            && let Some(health) = health
        {
            if siege_timer.last_hp > cc_core::math::FIXED_ZERO
                && health.current < siege_timer.last_hp
            {
                siege_timer.ticks_on_target = 0;
                siege_timer.current_target = None;
            }
            siege_timer.last_hp = health.current;
        }

        for slot in &slots.slots {
            if !slot.active {
                continue;
            }

            // Data-driven: look up self-buff effects from the ability definition
            let buff_effects = abilities::self_buff_effects(slot.id);
            for &(effect_id, use_remaining) in buff_effects {
                let duration = if use_remaining {
                    slot.duration_remaining.max(1)
                } else {
                    2
                };
                effects.refresh_or_insert(effect_id, duration, EntityId::from_entity(entity));
            }
        }
    }
}
