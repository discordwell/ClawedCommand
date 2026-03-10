use bevy::prelude::*;

use cc_core::components::{Dead, StatModifiers};
use cc_core::math::{FIXED_ONE, Fixed};
use cc_core::status_effects::{StatusEffectId, StatusEffects};

/// Recompute StatModifiers from StatusEffects every tick.
/// Clean slate each tick — no stale state.
pub fn stat_modifier_system(mut query: Query<(&StatusEffects, &mut StatModifiers), Without<Dead>>) {
    for (effects, mut modifiers) in query.iter_mut() {
        // Reset to defaults
        *modifiers = StatModifiers::default();

        for instance in &effects.effects {
            if instance.remaining_ticks == 0 {
                continue;
            }

            match instance.effect {
                StatusEffectId::Zoomies => {
                    // +100% speed, invulnerable, can't attack
                    modifiers.speed_multiplier *= Fixed::from_num(2);
                    modifiers.invulnerable = true;
                    modifiers.cannot_attack = true;
                }
                StatusEffectId::LoafModeActive => {
                    // Immobile + 50% damage reduction
                    modifiers.immobilized = true;
                    modifiers.damage_reduction *= Fixed::from_bits(32768); // 0.5
                }
                StatusEffectId::Motivated => {
                    // +15% damage
                    modifiers.damage_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 15 / 100); // 1.15
                }
                StatusEffectId::HarmonicBuff => {
                    // +20% damage, +10% speed
                    modifiers.damage_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 20 / 100); // 1.20
                    modifiers.speed_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 10 / 100); // 1.10
                }
                StatusEffectId::LullabyDebuff => {
                    // -30% speed, -15% attack speed
                    modifiers.speed_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                    modifiers.attack_speed_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 15 / 100); // 1.15 (slower)
                }
                StatusEffectId::TacticalLink => {
                    // -20% cooldowns
                    modifiers.cooldown_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 20 / 100); // 0.80
                }
                StatusEffectId::Annoyed => {
                    // -5% damage per stack (stacking debuff from Nuisance)
                    let reduction_per_stack = Fixed::from_bits((1 << 16) * 5 / 100); // 0.05
                    let total_reduction =
                        reduction_per_stack * Fixed::from_num(instance.stacks as i32);
                    let mult = (FIXED_ONE - total_reduction).max(Fixed::from_bits((1 << 16) / 2)); // floor at 0.5
                    modifiers.damage_multiplier *= mult;
                }
                StatusEffectId::Corroded => {
                    // -10% damage reduction per stack (takes more damage)
                    let increase_per_stack = Fixed::from_bits((1 << 16) * 10 / 100); // 0.10
                    let total_increase =
                        increase_per_stack * Fixed::from_num(instance.stacks as i32);
                    let mult = FIXED_ONE + total_increase; // > 1.0 means takes more damage
                    modifiers.damage_reduction *= mult;
                }
                StatusEffectId::Disoriented => {
                    // -50% speed (CC)
                    modifiers.speed_multiplier *= Fixed::from_bits(32768); // 0.5
                }
                StatusEffectId::Drowsed => {
                    // Immobile + silenced (CC)
                    modifiers.immobilized = true;
                    modifiers.silenced = true;
                }
                StatusEffectId::Tilted => {
                    // -30% speed, -20% damage (CC)
                    modifiers.speed_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                    modifiers.damage_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 20 / 100); // 0.80
                }
                StatusEffectId::NineLivesReviving => {
                    // Invulnerable during revive
                    modifiers.invulnerable = true;
                    modifiers.immobilized = true;
                }
                StatusEffectId::Overridden => {
                    // Silenced while overridden
                    modifiers.silenced = true;
                }
                StatusEffectId::SpiteCarryBuff => {
                    // +50% gather speed
                    modifiers.gather_speed_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 50 / 100); // 1.5
                }
                StatusEffectId::PowerNapping => {
                    // Self-immobilize + can't attack
                    modifiers.immobilized = true;
                    modifiers.cannot_attack = true;
                }

                StatusEffectId::Tagged | StatusEffectId::CcImmune => {
                    // These don't affect stats
                }
                StatusEffectId::Waterlogged => {
                    // -10% speed (Croak debuff)
                    modifiers.speed_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 10 / 100); // 0.90
                }
                StatusEffectId::Stunned => {
                    // Hard stun CC: immobile, can't attack, silenced
                    modifiers.immobilized = true;
                    modifiers.cannot_attack = true;
                    modifiers.silenced = true;
                }
                StatusEffectId::Silenced => {
                    // Can't use abilities
                    modifiers.silenced = true;
                }
                StatusEffectId::Entrenched => {
                    // Immobile, 30% damage reduction, 20% damage boost, +50% anti-static
                    modifiers.immobilized = true;
                    modifiers.damage_reduction *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                    modifiers.damage_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 20 / 100); // 1.20
                    // Patience of Stone: bonus damage vs stationary targets
                    modifiers.anti_static_bonus += Fixed::from_bits((1 << 16) * 50 / 100); // 0.5
                }
                StatusEffectId::SpeedBuff => {
                    // +50% speed (no attack penalty)
                    modifiers.speed_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 50 / 100); // 1.50
                }
                StatusEffectId::ArmorBuff => {
                    // 30% damage reduction
                    modifiers.damage_reduction *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                }
                StatusEffectId::DamageBuff => {
                    // +25% damage
                    modifiers.damage_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 25 / 100); // 1.25
                }
                StatusEffectId::PlayingDead => {
                    // Invulnerable, immobile, can't attack, silenced
                    modifiers.invulnerable = true;
                    modifiers.immobilized = true;
                    modifiers.cannot_attack = true;
                    modifiers.silenced = true;
                }
                StatusEffectId::SiegeNapDeployed => {
                    // Catnapper siege mode: immobilized, range ×1.43, 30% damage reduction
                    modifiers.immobilized = true;
                    modifiers.range_multiplier *= Fixed::from_bits((1 << 16) * 143 / 100); // 1.43
                    modifiers.damage_reduction *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                }
                StatusEffectId::JunkMortarDeployed => {
                    // Grease Monkey siege mode: immobilized, range ×2.0, attack speed ×0.7
                    modifiers.immobilized = true;
                    modifiers.range_multiplier *= Fixed::from_num(2); // 2.0
                    modifiers.attack_speed_multiplier *=
                        Fixed::from_bits((1 << 16) - (1 << 16) * 30 / 100); // 0.70
                }
                StatusEffectId::InflatedBombardment => {
                    // Croaker inflated: range ×1.667 (6→10), anti-static +0.4
                    modifiers.range_multiplier *= Fixed::from_bits((1 << 16) * 167 / 100); // 1.67
                    modifiers.anti_static_bonus += Fixed::from_bits((1 << 16) * 40 / 100); // 0.4
                }
                StatusEffectId::Rattled => {
                    // -10% attack speed (Shrieker Sonic Barrage debuff)
                    modifiers.attack_speed_multiplier *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 10 / 100); // 1.10 (slower)
                }
                StatusEffectId::Exposed => {
                    // +20% damage taken (Hootseer Death Omen debuff)
                    modifiers.damage_reduction *=
                        Fixed::from_bits((1 << 16) + (1 << 16) * 20 / 100); // 1.20
                }
            }
        }
    }
}
