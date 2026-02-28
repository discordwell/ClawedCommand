use serde::{Deserialize, Serialize};

use crate::components::UnitKind;
use crate::math::Fixed;

/// Named hero characters in the campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeroId {
    /// Kelpie — otter protagonist, Polyglot Protocol wielder.
    Kelpie,
    /// Commander Felix Nine — catGPT military leader, MechCommander.
    FelixNine,
    /// Thimble — Clawed (mice) hero, Nuisance-type scout.
    Thimble,
    /// Mother Granite — Seekers of the Deep (badgers) hero, heavy tank.
    MotherGranite,
    /// Rex Solstice — The Murder (corvids) hero, air striker.
    RexSolstice,
    /// King Ringtail — LLAMA (raccoons) hero, chaotic demo expert.
    KingRingtail,
    /// The Eternal — Croak (axolotls) hero, regenerating tank.
    TheEternal,
    /// Patches — catGPT Mouser scout, secondary hero.
    Patches,
}

/// Additive/multiplicative stat modifiers for heroes on top of base unit stats.
#[derive(Debug, Clone, Copy)]
pub struct HeroStatModifiers {
    /// Added to base HP.
    pub health_bonus: Fixed,
    /// Multiplied with base speed (1.0 = no change).
    pub speed_multiplier: Fixed,
    /// Added to base damage.
    pub damage_bonus: Fixed,
    /// Added to base range.
    pub range_bonus: Fixed,
}

/// Maps a hero to their base unit template.
pub fn hero_base_kind(hero: HeroId) -> UnitKind {
    match hero {
        HeroId::Kelpie => UnitKind::Nuisance,
        HeroId::FelixNine => UnitKind::MechCommander,
        HeroId::Thimble => UnitKind::Nuisance,
        HeroId::MotherGranite => UnitKind::Chonk,
        HeroId::RexSolstice => UnitKind::FlyingFox,
        HeroId::KingRingtail => UnitKind::FerretSapper,
        HeroId::TheEternal => UnitKind::Chonk,
        HeroId::Patches => UnitKind::Mouser,
    }
}

/// Returns the stat modifiers for a given hero.
pub fn hero_modifiers(hero: HeroId) -> HeroStatModifiers {
    match hero {
        HeroId::Kelpie => HeroStatModifiers {
            health_bonus: Fixed::from_bits(40 << 16),    // +40 HP
            speed_multiplier: Fixed::from_bits(78643),    // 1.2x speed
            damage_bonus: Fixed::from_bits(4 << 16),      // +4 damage
            range_bonus: Fixed::ZERO,
        },
        HeroId::FelixNine => HeroStatModifiers {
            health_bonus: Fixed::from_bits(100 << 16),    // +100 HP
            speed_multiplier: Fixed::ONE,                  // normal speed
            damage_bonus: Fixed::from_bits(8 << 16),       // +8 damage
            range_bonus: Fixed::from_bits(1 << 16),        // +1 range
        },
        HeroId::Thimble => HeroStatModifiers {
            health_bonus: Fixed::from_bits(30 << 16),
            speed_multiplier: Fixed::from_bits(85196),     // 1.3x speed
            damage_bonus: Fixed::from_bits(3 << 16),
            range_bonus: Fixed::ZERO,
        },
        HeroId::MotherGranite => HeroStatModifiers {
            health_bonus: Fixed::from_bits(200 << 16),     // +200 HP (fortress)
            speed_multiplier: Fixed::from_bits(58982),      // 0.9x speed (slower)
            damage_bonus: Fixed::from_bits(6 << 16),
            range_bonus: Fixed::ZERO,
        },
        HeroId::RexSolstice => HeroStatModifiers {
            health_bonus: Fixed::from_bits(30 << 16),
            speed_multiplier: Fixed::from_bits(72089),      // 1.1x speed
            damage_bonus: Fixed::from_bits(10 << 16),       // +10 damage
            range_bonus: Fixed::from_bits(2 << 16),          // +2 range
        },
        HeroId::KingRingtail => HeroStatModifiers {
            health_bonus: Fixed::from_bits(50 << 16),
            speed_multiplier: Fixed::from_bits(78643),      // 1.2x speed
            damage_bonus: Fixed::from_bits(15 << 16),       // +15 damage (explosive)
            range_bonus: Fixed::ZERO,
        },
        HeroId::TheEternal => HeroStatModifiers {
            health_bonus: Fixed::from_bits(250 << 16),      // +250 HP (regenerator)
            speed_multiplier: Fixed::from_bits(52428),       // 0.8x speed
            damage_bonus: Fixed::from_bits(5 << 16),
            range_bonus: Fixed::ZERO,
        },
        HeroId::Patches => HeroStatModifiers {
            health_bonus: Fixed::from_bits(25 << 16),
            speed_multiplier: Fixed::from_bits(78643),       // 1.2x speed
            damage_bonus: Fixed::from_bits(5 << 16),
            range_bonus: Fixed::ZERO,
        },
    }
}

/// Display name for hero characters.
pub fn hero_name(hero: HeroId) -> &'static str {
    match hero {
        HeroId::Kelpie => "Kelpie",
        HeroId::FelixNine => "Commander Felix Nine",
        HeroId::Thimble => "Thimble",
        HeroId::MotherGranite => "Mother Granite",
        HeroId::RexSolstice => "Rex Solstice",
        HeroId::KingRingtail => "King Ringtail",
        HeroId::TheEternal => "The Eternal",
        HeroId::Patches => "Patches",
    }
}

/// Faction affiliation for each hero.
pub fn hero_faction(hero: HeroId) -> &'static str {
    match hero {
        HeroId::Kelpie => "neutral",
        HeroId::FelixNine | HeroId::Patches => "catGPT",
        HeroId::Thimble => "The Clawed",
        HeroId::MotherGranite => "Seekers of the Deep",
        HeroId::RexSolstice => "The Murder",
        HeroId::KingRingtail => "LLAMA",
        HeroId::TheEternal => "Croak",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_stats::base_stats;

    /// All heroes defined in HeroId.
    const ALL_HEROES: [HeroId; 8] = [
        HeroId::Kelpie,
        HeroId::FelixNine,
        HeroId::Thimble,
        HeroId::MotherGranite,
        HeroId::RexSolstice,
        HeroId::KingRingtail,
        HeroId::TheEternal,
        HeroId::Patches,
    ];

    #[test]
    fn all_heroes_have_valid_base_kind() {
        for hero in ALL_HEROES {
            let kind = hero_base_kind(hero);
            let stats = base_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{hero:?} base kind {kind:?} has no HP");
        }
    }

    #[test]
    fn hero_modifiers_are_non_negative() {
        for hero in ALL_HEROES {
            let mods = hero_modifiers(hero);
            assert!(mods.health_bonus >= Fixed::ZERO, "{hero:?} health_bonus negative");
            assert!(mods.speed_multiplier > Fixed::ZERO, "{hero:?} speed_multiplier non-positive");
            assert!(mods.damage_bonus >= Fixed::ZERO, "{hero:?} damage_bonus negative");
            assert!(mods.range_bonus >= Fixed::ZERO, "{hero:?} range_bonus negative");
        }
    }

    #[test]
    fn hero_boosted_stats_exceed_base() {
        for hero in ALL_HEROES {
            let kind = hero_base_kind(hero);
            let base = base_stats(kind);
            let mods = hero_modifiers(hero);

            let boosted_hp = base.health + mods.health_bonus;
            assert!(boosted_hp >= base.health, "{hero:?} boosted HP < base");

            let boosted_dmg = base.damage + mods.damage_bonus;
            assert!(boosted_dmg >= base.damage, "{hero:?} boosted damage < base");
        }
    }

    #[test]
    fn all_heroes_have_names() {
        for hero in ALL_HEROES {
            assert!(!hero_name(hero).is_empty(), "{hero:?} has empty name");
        }
    }

    #[test]
    fn all_heroes_have_factions() {
        for hero in ALL_HEROES {
            assert!(!hero_faction(hero).is_empty(), "{hero:?} has empty faction");
        }
    }

    #[test]
    fn kelpie_is_faster_than_base_nuisance() {
        let base_speed = base_stats(UnitKind::Nuisance).speed;
        let mods = hero_modifiers(HeroId::Kelpie);
        // Kelpie's speed multiplier is 1.2x
        let boosted = base_speed * mods.speed_multiplier;
        assert!(boosted > base_speed);
    }

    #[test]
    fn felix_has_more_hp_than_base_mech() {
        let base_hp = base_stats(UnitKind::MechCommander).health;
        let mods = hero_modifiers(HeroId::FelixNine);
        let boosted = base_hp + mods.health_bonus;
        assert_eq!(boosted, base_hp + Fixed::from_bits(100 << 16));
    }
}
