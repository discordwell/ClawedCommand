use serde::{Deserialize, Serialize};

use crate::components::{Faction, UnitKind};
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

/// All static data for a hero character, consolidated from the four parallel
/// match blocks (base_kind, modifiers, name, faction) into one struct.
#[derive(Debug, Clone, Copy)]
pub struct HeroData {
    /// Base unit template for this hero.
    pub base_kind: UnitKind,
    /// Stat modifiers on top of the base unit stats.
    pub modifiers: HeroStatModifiers,
    /// Display name.
    pub name: &'static str,
    /// Faction affiliation.
    pub faction: Faction,
}

/// Returns all static data for a hero in a single lookup.
pub fn hero_data(hero: HeroId) -> HeroData {
    match hero {
        HeroId::Kelpie => HeroData {
            base_kind: UnitKind::Nuisance,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(40 << 16),    // +40 HP
                speed_multiplier: Fixed::from_bits(78643),    // 1.2x speed
                damage_bonus: Fixed::from_bits(4 << 16),      // +4 damage
                range_bonus: Fixed::ZERO,
            },
            name: "Kelpie",
            faction: Faction::Neutral,
        },
        HeroId::FelixNine => HeroData {
            base_kind: UnitKind::MechCommander,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(100 << 16),    // +100 HP
                speed_multiplier: Fixed::ONE,                  // normal speed
                damage_bonus: Fixed::from_bits(8 << 16),       // +8 damage
                range_bonus: Fixed::from_bits(1 << 16),        // +1 range
            },
            name: "Commander Felix Nine",
            faction: Faction::CatGpt,
        },
        HeroId::Thimble => HeroData {
            base_kind: UnitKind::Nuisance,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(30 << 16),
                speed_multiplier: Fixed::from_bits(85196),     // 1.3x speed
                damage_bonus: Fixed::from_bits(3 << 16),
                range_bonus: Fixed::ZERO,
            },
            name: "Thimble",
            faction: Faction::TheClawed,
        },
        HeroId::MotherGranite => HeroData {
            base_kind: UnitKind::Chonk,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(200 << 16),     // +200 HP (fortress)
                speed_multiplier: Fixed::from_bits(58982),      // 0.9x speed (slower)
                damage_bonus: Fixed::from_bits(6 << 16),
                range_bonus: Fixed::ZERO,
            },
            name: "Mother Granite",
            faction: Faction::SeekersOfTheDeep,
        },
        HeroId::RexSolstice => HeroData {
            base_kind: UnitKind::FlyingFox,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(30 << 16),
                speed_multiplier: Fixed::from_bits(72089),      // 1.1x speed
                damage_bonus: Fixed::from_bits(10 << 16),       // +10 damage
                range_bonus: Fixed::from_bits(2 << 16),          // +2 range
            },
            name: "Rex Solstice",
            faction: Faction::TheMurder,
        },
        HeroId::KingRingtail => HeroData {
            base_kind: UnitKind::FerretSapper,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(50 << 16),
                speed_multiplier: Fixed::from_bits(78643),      // 1.2x speed
                damage_bonus: Fixed::from_bits(15 << 16),       // +15 damage (explosive)
                range_bonus: Fixed::ZERO,
            },
            name: "King Ringtail",
            faction: Faction::Llama,
        },
        HeroId::TheEternal => HeroData {
            base_kind: UnitKind::Chonk,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(250 << 16),      // +250 HP (regenerator)
                speed_multiplier: Fixed::from_bits(52428),       // 0.8x speed
                damage_bonus: Fixed::from_bits(5 << 16),
                range_bonus: Fixed::ZERO,
            },
            name: "The Eternal",
            faction: Faction::Croak,
        },
        HeroId::Patches => HeroData {
            base_kind: UnitKind::Mouser,
            modifiers: HeroStatModifiers {
                health_bonus: Fixed::from_bits(25 << 16),
                speed_multiplier: Fixed::from_bits(78643),       // 1.2x speed
                damage_bonus: Fixed::from_bits(5 << 16),
                range_bonus: Fixed::ZERO,
            },
            name: "Patches",
            faction: Faction::CatGpt,
        },
    }
}

/// All hero IDs in canonical order.
pub const ALL_HEROES: [HeroId; 8] = [
    HeroId::Kelpie,
    HeroId::FelixNine,
    HeroId::Thimble,
    HeroId::MotherGranite,
    HeroId::RexSolstice,
    HeroId::KingRingtail,
    HeroId::TheEternal,
    HeroId::Patches,
];

/// Returns the file-name slug for a hero (e.g. "king_ringtail", "the_eternal").
pub fn hero_slug(hero: HeroId) -> &'static str {
    match hero {
        HeroId::Kelpie => "kelpie",
        HeroId::FelixNine => "felix_nine",
        HeroId::Thimble => "thimble",
        HeroId::MotherGranite => "mother_granite",
        HeroId::RexSolstice => "rex_solstice",
        HeroId::KingRingtail => "king_ringtail",
        HeroId::TheEternal => "the_eternal",
        HeroId::Patches => "patches",
    }
}

/// Maps a hero to their base unit template.
/// Convenience wrapper around `hero_data`.
pub fn hero_base_kind(hero: HeroId) -> UnitKind {
    hero_data(hero).base_kind
}

/// Returns the stat modifiers for a given hero.
/// Convenience wrapper around `hero_data`.
pub fn hero_modifiers(hero: HeroId) -> HeroStatModifiers {
    hero_data(hero).modifiers
}

/// Display name for hero characters.
/// Convenience wrapper around `hero_data`.
pub fn hero_name(hero: HeroId) -> &'static str {
    hero_data(hero).name
}

/// Faction affiliation for each hero (as Faction enum).
/// Convenience wrapper around `hero_data`.
pub fn hero_faction(hero: HeroId) -> Faction {
    hero_data(hero).faction
}

/// Faction affiliation for each hero as a string.
/// Convenience wrapper for backward compatibility.
pub fn hero_faction_str(hero: HeroId) -> &'static str {
    hero_data(hero).faction.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_stats::base_stats;

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
            assert!(!hero_faction(hero).as_str().is_empty(), "{hero:?} has empty faction");
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

    #[test]
    fn hero_slug_all_valid() {
        for hero in ALL_HEROES {
            let slug = hero_slug(hero);
            assert!(!slug.is_empty(), "{hero:?} has empty slug");
            assert!(
                slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{hero:?} slug '{slug}' contains invalid characters"
            );
        }
    }

    #[test]
    fn hero_slug_uniqueness() {
        let mut seen = std::collections::HashSet::new();
        for hero in ALL_HEROES {
            let slug = hero_slug(hero);
            assert!(seen.insert(slug), "Duplicate hero slug: '{slug}'");
        }
    }

    #[test]
    fn all_heroes_constant_has_eight_entries() {
        assert_eq!(ALL_HEROES.len(), 8);
    }

    #[test]
    fn hero_data_struct_matches_convenience_fns() {
        for hero in ALL_HEROES {
            let data = hero_data(hero);
            assert_eq!(data.base_kind, hero_base_kind(hero), "{hero:?} base_kind mismatch");
            assert_eq!(data.name, hero_name(hero), "{hero:?} name mismatch");
            assert_eq!(data.faction, hero_faction(hero), "{hero:?} faction mismatch");
            // Also verify the string form matches
            assert_eq!(data.faction.as_str(), hero_faction_str(hero), "{hero:?} faction_str mismatch");
            let mods = hero_modifiers(hero);
            assert_eq!(data.modifiers.health_bonus, mods.health_bonus, "{hero:?} health_bonus mismatch");
            assert_eq!(data.modifiers.speed_multiplier, mods.speed_multiplier, "{hero:?} speed_multiplier mismatch");
            assert_eq!(data.modifiers.damage_bonus, mods.damage_bonus, "{hero:?} damage_bonus mismatch");
            assert_eq!(data.modifiers.range_bonus, mods.range_bonus, "{hero:?} range_bonus mismatch");
        }
    }
}
