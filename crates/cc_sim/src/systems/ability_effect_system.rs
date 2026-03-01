use bevy::prelude::*;

use cc_core::abilities::AbilityId;
use cc_core::commands::EntityId;
use cc_core::components::{AbilitySlots, Dead};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};

/// Bridge system: reads AbilitySlots and applies/refreshes corresponding StatusEffects
/// when abilities are active. When inactive, effects are NOT forcibly removed — they
/// expire naturally through status_effect_system. This allows manually applied effects
/// (e.g., in tests or from other sources) to coexist.
///
/// Runs after ability_cooldown_system, before status_effect_system.
pub fn ability_effect_system(
    mut query: Query<(Entity, &AbilitySlots, &mut StatusEffects), Without<Dead>>,
) {
    for (entity, slots, mut effects) in query.iter_mut() {
        for slot in &slots.slots {
            match slot.id {
                AbilityId::Zoomies => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::Zoomies,
                            slot.duration_remaining.max(1),
                            entity,
                        );
                    }
                    // When inactive: effect expires naturally via status_effect_system
                }
                AbilityId::LoafMode => {
                    if slot.active {
                        // Refresh each tick (duration=2 so it survives 1 tick of decrement)
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::LoafModeActive,
                            2,
                            entity,
                        );
                    }
                    // When toggled off: 2-tick effect expires on next status_effect tick
                }
                AbilityId::SpiteCarry => {
                    if slot.active {
                        ensure_effect(
                            &mut effects,
                            StatusEffectId::SpiteCarryBuff,
                            slot.duration_remaining.max(1),
                            entity,
                        );
                    }
                }
                _ => {
                    // Other abilities: passive effects handled in combat, auras handled by aura_system
                }
            }
        }
    }
}

/// Ensure a status effect exists with at least the given duration; refresh if already present.
fn ensure_effect(effects: &mut StatusEffects, id: StatusEffectId, duration: u32, entity: Entity) {
    if let Some(existing) = effects.effects.iter_mut().find(|e| e.effect == id) {
        existing.remaining_ticks = existing.remaining_ticks.max(duration);
    } else {
        effects.effects.push(StatusInstance {
            effect: id,
            remaining_ticks: duration,
            stacks: 1,
            source: EntityId(entity.to_bits()),
        });
    }
}
