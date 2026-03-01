use crate::commands::EntityId;

/// Unique identifier for each status effect in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusEffectId {
    /// Nuisance passive — stacking debuff on attacked target.
    Annoyed,
    /// Hisser passive — armor reduction over time.
    Corroded,
    /// FlyingFox ability — movement confusion (CC).
    Disoriented,
    /// Catnapper ability — falling asleep (CC).
    Drowsed,
    /// Yowler ability — attack/move disruption (CC).
    Tilted,
    /// Mouser ability — visible through fog.
    Tagged,
    /// Nuisance ability — massive speed buff.
    Zoomies,
    /// Chonk ability — immobile, damage reduction.
    LoafModeActive,
    /// Yowler aura — ally buff.
    Motivated,
    /// Yowler toggle aura — ally damage/speed buff.
    HarmonicBuff,
    /// Yowler toggle aura — enemy debuff.
    LullabyDebuff,
    /// MechCommander aura — ally cooldown reduction.
    TacticalLink,
    /// MechCommander ability — control enemy unit.
    Overridden,
    /// Chonk passive — reviving (CC immune during).
    NineLivesReviving,
    /// Pawdler SpiteCarry — gather speed boost.
    SpiteCarryBuff,
    /// Catnapper PowerNap — self-immobilize + GPU generation.
    PowerNapping,
    /// Post-CC immunity window.
    CcImmune,
    // --- Croak (Axolotls) ---
    /// Croak debuff — -10% move speed, applied by many Croak abilities.
    Waterlogged,

    // --- Cross-faction status effects ---
    /// Hard stun CC — immobile, can't attack, silenced.
    Stunned,
    /// Silenced — can't activate abilities, but can still move/attack.
    Silenced,
    /// Entrenched — immobile, 30% damage reduction, 20% damage boost.
    Entrenched,
    /// Generic speed buff — +50% speed (no attack penalty unlike Zoomies).
    SpeedBuff,
    /// Generic armor buff — 30% damage reduction.
    ArmorBuff,
    /// Generic damage buff — +25% damage.
    DamageBuff,
    /// Playing dead — invulnerable, immobile, can't attack/cast.
    PlayingDead,
}

/// Returns true if this status effect is crowd control (CC).
pub fn is_cc(id: StatusEffectId) -> bool {
    matches!(
        id,
        StatusEffectId::Disoriented
            | StatusEffectId::Drowsed
            | StatusEffectId::Tilted
            | StatusEffectId::Stunned
    )
}

/// A single instance of a status effect on an entity.
#[derive(Debug, Clone)]
pub struct StatusInstance {
    pub effect: StatusEffectId,
    /// Ticks remaining (0 = expired, will be cleaned up).
    pub remaining_ticks: u32,
    /// Stack count (for Annoyed, Corroded).
    pub stacks: u32,
    /// Who applied this effect.
    pub source: EntityId,
}

/// Component tracking all active status effects on an entity.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct StatusEffects {
    pub effects: Vec<StatusInstance>,
    /// Remaining ticks of CC immunity (granted after CC expires).
    pub cc_immunity_remaining: u32,
}

impl Default for StatusEffects {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            cc_immunity_remaining: 0,
        }
    }
}

impl StatusEffects {
    /// Check if entity has a specific status effect active.
    pub fn has(&self, id: StatusEffectId) -> bool {
        self.effects.iter().any(|e| e.effect == id && e.remaining_ticks > 0)
    }

    /// Get stack count for a specific status effect.
    pub fn stacks_of(&self, id: StatusEffectId) -> u32 {
        self.effects
            .iter()
            .filter(|e| e.effect == id && e.remaining_ticks > 0)
            .map(|e| e.stacks)
            .sum()
    }

    /// Check if entity has any active CC effect.
    pub fn has_active_cc(&self) -> bool {
        self.effects
            .iter()
            .any(|e| is_cc(e.effect) && e.remaining_ticks > 0)
    }

    /// Check if entity is currently CC immune.
    pub fn is_cc_immune(&self) -> bool {
        self.cc_immunity_remaining > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_cc_identifies_cc_effects() {
        assert!(is_cc(StatusEffectId::Disoriented));
        assert!(is_cc(StatusEffectId::Drowsed));
        assert!(is_cc(StatusEffectId::Tilted));
        assert!(!is_cc(StatusEffectId::Annoyed));
        assert!(!is_cc(StatusEffectId::Zoomies));
        assert!(!is_cc(StatusEffectId::Tagged));
        assert!(!is_cc(StatusEffectId::SpiteCarryBuff));
        assert!(!is_cc(StatusEffectId::PowerNapping));
    }

    #[test]
    fn status_effects_default_empty() {
        let se = StatusEffects::default();
        assert!(se.effects.is_empty());
        assert_eq!(se.cc_immunity_remaining, 0);
        assert!(!se.has(StatusEffectId::Annoyed));
        assert!(!se.has_active_cc());
        assert!(!se.is_cc_immune());
    }

    #[test]
    fn has_detects_active_effects() {
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Zoomies,
            remaining_ticks: 30,
            stacks: 1,
            source: EntityId(0),
        });
        assert!(se.has(StatusEffectId::Zoomies));
        assert!(!se.has(StatusEffectId::Annoyed));
    }

    #[test]
    fn has_ignores_expired_effects() {
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Zoomies,
            remaining_ticks: 0,
            stacks: 1,
            source: EntityId(0),
        });
        assert!(!se.has(StatusEffectId::Zoomies));
    }

    #[test]
    fn stacks_of_sums_correctly() {
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Annoyed,
            remaining_ticks: 50,
            stacks: 3,
            source: EntityId(1),
        });
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Annoyed,
            remaining_ticks: 30,
            stacks: 2,
            source: EntityId(2),
        });
        assert_eq!(se.stacks_of(StatusEffectId::Annoyed), 5);
        assert_eq!(se.stacks_of(StatusEffectId::Corroded), 0);
    }

    #[test]
    fn has_active_cc_detects_cc() {
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Disoriented,
            remaining_ticks: 10,
            stacks: 1,
            source: EntityId(0),
        });
        assert!(se.has_active_cc());
    }

    #[test]
    fn cc_immunity_works() {
        let mut se = StatusEffects::default();
        assert!(!se.is_cc_immune());
        se.cc_immunity_remaining = 10;
        assert!(se.is_cc_immune());
    }
}
