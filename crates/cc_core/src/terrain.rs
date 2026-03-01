use serde::{Deserialize, Serialize};

use crate::math::{Fixed, FIXED_ONE, FIXED_ZERO};
use crate::mutator::HazardDirection;

/// Terrain type for each tile on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TerrainType {
    Grass = 0,
    Dirt = 1,
    Sand = 2,
    Forest = 3,
    Water = 4,
    Shallows = 5,
    Rock = 6,
    Ramp = 7,
    Road = 8,
    TechRuins = 9,
}

impl TerrainType {
    /// Movement cost multiplier. Higher = slower.
    pub fn movement_cost(self) -> Fixed {
        match self {
            Self::Grass => FIXED_ONE,                    // 1.0x
            Self::Dirt => Fixed::from_bits(62259),       // 0.95x
            Self::Sand => Fixed::from_bits(78643),       // 1.2x
            Self::Forest => Fixed::from_bits(85196),     // 1.3x
            Self::Water => FIXED_ONE,                    // 1.0x for Croak
            Self::Shallows => Fixed::from_bits(98304),   // 1.5x
            Self::Rock => FIXED_ZERO,                    // impassable
            Self::Ramp => Fixed::from_bits(72089),       // 1.1x
            Self::Road => Fixed::from_bits(45875),       // 0.7x
            Self::TechRuins => Fixed::from_bits(75366),  // 1.15x
        }
    }

    /// Whether this terrain is passable by default (ignoring faction rules).
    pub fn base_passable(self) -> bool {
        !matches!(self, Self::Rock | Self::Water)
    }

    /// Whether this terrain is water (for faction-specific traversal).
    pub fn is_water(self) -> bool {
        matches!(self, Self::Water | Self::Shallows)
    }

    /// Cover level provided by this terrain.
    pub fn cover(self) -> CoverLevel {
        match self {
            Self::Forest => CoverLevel::Light,
            Self::TechRuins => CoverLevel::Heavy,
            _ => CoverLevel::None,
        }
    }

    /// Whether this terrain provides concealment (for stealth units).
    pub fn provides_concealment(self) -> bool {
        matches!(self, Self::Forest)
    }

    /// Priority for auto-tiling transitions. Higher priority terrain
    /// renders on top of lower priority terrain at boundaries.
    pub fn tiling_priority(self) -> u8 {
        match self {
            Self::Water => 0,
            Self::Shallows => 1,
            Self::Sand => 2,
            Self::Dirt => 3,
            Self::Grass => 4,
            Self::Road => 5,
            Self::Forest => 6,
            Self::TechRuins => 7,
            Self::Rock => 8,
            Self::Ramp => 4, // Same as grass for blending
        }
    }

    /// Convert from u8 representation.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Grass),
            1 => Some(Self::Dirt),
            2 => Some(Self::Sand),
            3 => Some(Self::Forest),
            4 => Some(Self::Water),
            5 => Some(Self::Shallows),
            6 => Some(Self::Rock),
            7 => Some(Self::Ramp),
            8 => Some(Self::Road),
            9 => Some(Self::TechRuins),
            _ => None,
        }
    }

    /// All terrain type variants.
    pub const ALL: [TerrainType; 10] = [
        Self::Grass,
        Self::Dirt,
        Self::Sand,
        Self::Forest,
        Self::Water,
        Self::Shallows,
        Self::Rock,
        Self::Ramp,
        Self::Road,
        Self::TechRuins,
    ];
}

impl Default for TerrainType {
    fn default() -> Self {
        Self::Grass
    }
}

impl std::fmt::Display for TerrainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for TerrainType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Grass" => Ok(Self::Grass),
            "Dirt" => Ok(Self::Dirt),
            "Sand" => Ok(Self::Sand),
            "Forest" => Ok(Self::Forest),
            "Water" => Ok(Self::Water),
            "Shallows" => Ok(Self::Shallows),
            "Rock" => Ok(Self::Rock),
            "Ramp" => Ok(Self::Ramp),
            "Road" => Ok(Self::Road),
            "TechRuins" => Ok(Self::TechRuins),
            _ => Err(()),
        }
    }
}

/// Damage reduction from terrain cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverLevel {
    None,
    Light,  // -15% damage taken
    Heavy,  // -30% damage taken
}

impl CoverLevel {
    pub const ALL: [CoverLevel; 3] = [Self::None, Self::Light, Self::Heavy];

    /// Damage multiplier (1.0 = full damage, 0.85 = light cover, 0.70 = heavy cover).
    pub fn damage_multiplier(self) -> Fixed {
        match self {
            Self::None => FIXED_ONE,
            Self::Light => Fixed::from_bits(55705),  // 0.85
            Self::Heavy => Fixed::from_bits(45875),  // 0.70
        }
    }
}

impl std::fmt::Display for CoverLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for CoverLevel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "None" => Ok(Self::None),
            "Light" => Ok(Self::Light),
            "Heavy" => Ok(Self::Heavy),
            _ => Err(()),
        }
    }
}

/// Faction identifier for faction-aware terrain rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FactionId {
    CatGPT = 0,
    TheClawed = 1,
    SeekersOfTheDeep = 2,
    TheMurder = 3,
    LLAMA = 4,
    Croak = 5,
}

impl FactionId {
    /// Whether this faction can traverse water tiles.
    pub fn can_traverse_water(self) -> bool {
        matches!(self, Self::Croak)
    }

    /// Convert from u8.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::CatGPT),
            1 => Some(Self::TheClawed),
            2 => Some(Self::SeekersOfTheDeep),
            3 => Some(Self::TheMurder),
            4 => Some(Self::LLAMA),
            5 => Some(Self::Croak),
            _ => None,
        }
    }
}

/// Check if a terrain type is passable for a given faction.
pub fn is_passable_for_faction(terrain: TerrainType, faction: FactionId) -> bool {
    match terrain {
        TerrainType::Water => faction.can_traverse_water(),
        other => other.base_passable(),
    }
}

/// Dynamic terrain overlay effect created by abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayEffect {
    /// Blocks movement (e.g., Hairball, Loaf Mode).
    Block,
    /// Converts tile to water (e.g., Tidal Gate).
    WaterConvert,
    /// Creates a speed-modifying trail.
    Trail { speed_modifier: Fixed },
    /// Lava hazard — damages units on this tile.
    Lava { damage_per_tick: u32 },
    /// Toxic hazard — damages units on this tile.
    Toxic { damage_per_tick: u32 },
    /// Wind zone — displaces units in a direction.
    WindZone { direction: HazardDirection, force: u32 },
}

/// A temporary terrain modification applied by an ability.
#[derive(Debug, Clone)]
pub struct TerrainOverlay {
    pub x: i32,
    pub y: i32,
    pub effect: OverlayEffect,
    pub remaining_ticks: u32,
}

// Dynamic flags bit definitions
pub const FLAG_TEMP_BLOCKED: u8 = 0b0000_0001;
pub const FLAG_WATER_CONVERTED: u8 = 0b0000_0010;
pub const FLAG_LAVA: u8 = 0b0000_0100;
pub const FLAG_TOXIC: u8 = 0b0000_1000;

/// Elevation constants.
pub const MAX_ELEVATION: u8 = 2;
pub const ELEVATION_PIXEL_OFFSET: f32 = 8.0;

/// Vision range bonus per elevation level above ground (stub for Phase 2+).
pub const VISION_BONUS_PER_LEVEL: i32 = 1;

/// Damage bonus per elevation level above target: +15% per level (stub for Phase 2+).
pub fn elevation_damage_multiplier(advantage: i8) -> Fixed {
    if advantage > 0 {
        // +15% per level above
        FIXED_ONE + Fixed::from_bits(9830) * Fixed::from_num(advantage) // 0.15 * levels
    } else if advantage < 0 {
        // -15% per level below (minimum 0.55)
        let penalty = Fixed::from_bits(9830) * Fixed::from_num(-advantage);
        let result = FIXED_ONE - penalty;
        let min = Fixed::from_bits(36044); // 0.55
        if result < min { min } else { result }
    } else {
        FIXED_ONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_passability() {
        assert!(TerrainType::Grass.base_passable());
        assert!(TerrainType::Dirt.base_passable());
        assert!(TerrainType::Sand.base_passable());
        assert!(TerrainType::Forest.base_passable());
        assert!(!TerrainType::Water.base_passable());
        assert!(TerrainType::Shallows.base_passable());
        assert!(!TerrainType::Rock.base_passable());
        assert!(TerrainType::Ramp.base_passable());
        assert!(TerrainType::Road.base_passable());
        assert!(TerrainType::TechRuins.base_passable());
    }

    #[test]
    fn terrain_movement_costs() {
        // Road should be fastest
        assert!(TerrainType::Road.movement_cost() < TerrainType::Grass.movement_cost());
        // Sand should be slower than grass
        assert!(TerrainType::Sand.movement_cost() > TerrainType::Grass.movement_cost());
        // Forest slower than grass
        assert!(TerrainType::Forest.movement_cost() > TerrainType::Grass.movement_cost());
        // Shallows slowest passable
        assert!(TerrainType::Shallows.movement_cost() > TerrainType::Forest.movement_cost());
        // Dirt slightly faster than grass
        assert!(TerrainType::Dirt.movement_cost() < TerrainType::Grass.movement_cost());
    }

    #[test]
    fn terrain_cover_levels() {
        assert_eq!(TerrainType::Grass.cover(), CoverLevel::None);
        assert_eq!(TerrainType::Forest.cover(), CoverLevel::Light);
        assert_eq!(TerrainType::TechRuins.cover(), CoverLevel::Heavy);
        assert_eq!(TerrainType::Water.cover(), CoverLevel::None);
    }

    #[test]
    fn terrain_water_classification() {
        assert!(TerrainType::Water.is_water());
        assert!(TerrainType::Shallows.is_water());
        assert!(!TerrainType::Grass.is_water());
        assert!(!TerrainType::Sand.is_water());
    }

    #[test]
    fn croak_can_traverse_water() {
        assert!(is_passable_for_faction(TerrainType::Water, FactionId::Croak));
        assert!(!is_passable_for_faction(TerrainType::Water, FactionId::CatGPT));
        assert!(!is_passable_for_faction(TerrainType::Water, FactionId::TheClawed));
        assert!(!is_passable_for_faction(TerrainType::Water, FactionId::TheMurder));
        assert!(!is_passable_for_faction(TerrainType::Water, FactionId::LLAMA));
    }

    #[test]
    fn all_factions_blocked_by_rock() {
        for faction_id in 0..6u8 {
            let faction = FactionId::from_u8(faction_id).unwrap();
            assert!(!is_passable_for_faction(TerrainType::Rock, faction));
        }
    }

    #[test]
    fn faction_aware_neighbor_filtering() {
        // Croak should traverse water, CatGPT should not
        assert!(is_passable_for_faction(TerrainType::Shallows, FactionId::CatGPT));
        assert!(is_passable_for_faction(TerrainType::Shallows, FactionId::Croak));
        assert!(!is_passable_for_faction(TerrainType::Water, FactionId::CatGPT));
        assert!(is_passable_for_faction(TerrainType::Water, FactionId::Croak));
    }

    #[test]
    fn tiling_priority_ordering() {
        // Water < Shallows < Sand < Dirt < Grass < Road < Forest < TechRuins < Rock
        assert!(TerrainType::Water.tiling_priority() < TerrainType::Shallows.tiling_priority());
        assert!(TerrainType::Shallows.tiling_priority() < TerrainType::Sand.tiling_priority());
        assert!(TerrainType::Sand.tiling_priority() < TerrainType::Dirt.tiling_priority());
        assert!(TerrainType::Dirt.tiling_priority() < TerrainType::Grass.tiling_priority());
        assert!(TerrainType::Grass.tiling_priority() < TerrainType::Road.tiling_priority());
        assert!(TerrainType::Road.tiling_priority() < TerrainType::Forest.tiling_priority());
        assert!(TerrainType::Forest.tiling_priority() < TerrainType::TechRuins.tiling_priority());
        assert!(TerrainType::TechRuins.tiling_priority() < TerrainType::Rock.tiling_priority());
    }

    #[test]
    fn terrain_from_u8_round_trip() {
        for terrain in TerrainType::ALL {
            let v = terrain as u8;
            assert_eq!(TerrainType::from_u8(v), Some(terrain));
        }
        assert_eq!(TerrainType::from_u8(255), None);
    }

    #[test]
    fn cover_damage_multiplier() {
        assert_eq!(CoverLevel::None.damage_multiplier(), FIXED_ONE);
        assert!(CoverLevel::Light.damage_multiplier() < FIXED_ONE);
        assert!(CoverLevel::Heavy.damage_multiplier() < CoverLevel::Light.damage_multiplier());
    }

    #[test]
    fn concealment_only_in_forest() {
        for terrain in TerrainType::ALL {
            if terrain == TerrainType::Forest {
                assert!(terrain.provides_concealment());
            } else {
                assert!(!terrain.provides_concealment());
            }
        }
    }

    #[test]
    fn elevation_damage_multiplier_values() {
        // No advantage = 1.0
        assert_eq!(elevation_damage_multiplier(0), FIXED_ONE);
        // +1 level = 1.15
        assert!(elevation_damage_multiplier(1) > FIXED_ONE);
        // +2 levels = 1.30
        assert!(elevation_damage_multiplier(2) > elevation_damage_multiplier(1));
        // -1 level = 0.85
        assert!(elevation_damage_multiplier(-1) < FIXED_ONE);
        // -2 levels = 0.70
        assert!(elevation_damage_multiplier(-2) < elevation_damage_multiplier(-1));
        // Floor at 0.55
        assert!(elevation_damage_multiplier(-5) >= Fixed::from_bits(36044));
    }

    #[test]
    fn terrain_type_display_from_str_round_trip() {
        for terrain in TerrainType::ALL {
            let s = terrain.to_string();
            let parsed: TerrainType = s.parse().unwrap();
            assert_eq!(parsed, terrain);
        }
        assert!("Bogus".parse::<TerrainType>().is_err());
    }

    #[test]
    fn cover_level_display_from_str_round_trip() {
        for cover in CoverLevel::ALL {
            let s = cover.to_string();
            let parsed: CoverLevel = s.parse().unwrap();
            assert_eq!(parsed, cover);
        }
        assert!("Bogus".parse::<CoverLevel>().is_err());
    }

    #[test]
    fn dynamic_flags_bits() {
        let flags: u8 = FLAG_TEMP_BLOCKED | FLAG_WATER_CONVERTED;
        assert_ne!(flags & FLAG_TEMP_BLOCKED, 0);
        assert_ne!(flags & FLAG_WATER_CONVERTED, 0);
        assert_eq!(FLAG_TEMP_BLOCKED & FLAG_WATER_CONVERTED, 0); // no overlap
    }

    #[test]
    fn hazard_flags_no_overlap() {
        // All four dynamic flags must use distinct bits
        let all_flags = [FLAG_TEMP_BLOCKED, FLAG_WATER_CONVERTED, FLAG_LAVA, FLAG_TOXIC];
        for i in 0..all_flags.len() {
            for j in (i + 1)..all_flags.len() {
                assert_eq!(
                    all_flags[i] & all_flags[j],
                    0,
                    "Flags at index {} and {} overlap",
                    i,
                    j
                );
            }
        }
        // Combined should have exactly 4 bits set
        let combined = FLAG_TEMP_BLOCKED | FLAG_WATER_CONVERTED | FLAG_LAVA | FLAG_TOXIC;
        assert_eq!(combined.count_ones(), 4);
    }
}
