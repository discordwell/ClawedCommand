use serde::{Deserialize, Serialize};

use crate::coords::GridPos;
use crate::map::{GameMap, TileData};
use crate::terrain::TerrainType;

/// Symmetry type for map generation and validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapSymmetry {
    /// 180-degree rotational symmetry (standard 1v1).
    Rotational180,
    /// 4-way rotational symmetry (FFA/2v2).
    Rotational90,
    /// Left-right mirror.
    MirrorHorizontal,
    /// Top-bottom mirror.
    MirrorVertical,
}

/// Resource type for map placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    FishPond,
    BerryBush,
    GpuDeposit,
    MonkeyMine,
}

/// Neutral camp difficulty tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampTier {
    Green,
    Orange,
    Red,
}

/// A spawn point for a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub player: u8,
    pub pos: (i32, i32),
}

/// A resource placement on the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePlacement {
    pub kind: ResourceKind,
    pub pos: (i32, i32),
}

/// A neutral camp placement (future use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutralCamp {
    pub tier: CampTier,
    pub pos: (i32, i32),
}

/// Complete map definition (serializable to/from RON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDefinition {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Each tile stored as (terrain_type_u8, elevation_u8).
    pub tiles: Vec<(u8, u8)>,
    pub spawn_points: Vec<SpawnPoint>,
    pub resources: Vec<ResourcePlacement>,
    pub neutral_camps: Vec<NeutralCamp>,
    pub symmetry: MapSymmetry,
}

impl MapDefinition {
    /// Convert this definition into a GameMap for simulation.
    pub fn to_game_map(&self) -> GameMap {
        let mut map = GameMap::new(self.width, self.height);

        for (i, &(terrain_u8, elevation)) in self.tiles.iter().enumerate() {
            let x = (i as u32 % self.width) as i32;
            let y = (i as u32 / self.width) as i32;
            let pos = GridPos::new(x, y);
            if let Some(tile) = map.get_mut(pos) {
                tile.terrain = TerrainType::from_u8(terrain_u8).unwrap_or_default();
                tile.elevation = elevation;
            }
        }

        map
    }

    /// Create a MapDefinition from an existing GameMap.
    pub fn from_game_map(map: &GameMap, name: String) -> Self {
        let tiles: Vec<(u8, u8)> = map
            .tiles()
            .iter()
            .map(|t| (t.terrain as u8, t.elevation))
            .collect();

        Self {
            name,
            width: map.width,
            height: map.height,
            tiles,
            spawn_points: Vec::new(),
            resources: Vec::new(),
            neutral_camps: Vec::new(),
            symmetry: MapSymmetry::Rotational180,
        }
    }

    /// Serialize to RON string.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Deserialize from RON string.
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }

    /// Validate tile data dimensions.
    pub fn validate(&self) -> Result<(), String> {
        let expected = (self.width * self.height) as usize;
        if self.tiles.len() != expected {
            return Err(format!(
                "Tile count {} doesn't match {}x{} = {}",
                self.tiles.len(),
                self.width,
                self.height,
                expected
            ));
        }

        for (i, &(terrain_u8, elevation)) in self.tiles.iter().enumerate() {
            if TerrainType::from_u8(terrain_u8).is_none() {
                return Err(format!("Invalid terrain type {} at tile index {}", terrain_u8, i));
            }
            if elevation > 2 {
                return Err(format!("Elevation {} > 2 at tile index {}", elevation, i));
            }
        }

        if self.spawn_points.is_empty() {
            return Err("No spawn points defined".into());
        }

        Ok(())
    }
}

/// Create a minimal TileData from terrain and elevation.
impl TileData {
    pub fn new(terrain: TerrainType, elevation: u8) -> Self {
        Self {
            terrain,
            elevation,
            dynamic_flags: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_game_map() {
        let mut map = GameMap::new(8, 8);
        map.get_mut(GridPos::new(3, 3)).unwrap().terrain = TerrainType::Water;
        map.get_mut(GridPos::new(3, 3)).unwrap().elevation = 0;
        map.get_mut(GridPos::new(5, 5)).unwrap().terrain = TerrainType::Forest;
        map.get_mut(GridPos::new(5, 5)).unwrap().elevation = 1;

        let def = MapDefinition::from_game_map(&map, "test".into());
        let restored = def.to_game_map();

        assert_eq!(
            restored.terrain_at(GridPos::new(3, 3)),
            Some(TerrainType::Water)
        );
        assert_eq!(
            restored.terrain_at(GridPos::new(5, 5)),
            Some(TerrainType::Forest)
        );
        assert_eq!(restored.elevation_at(GridPos::new(5, 5)), 1);
    }

    #[test]
    fn ron_serialization_round_trip() {
        let def = MapDefinition {
            name: "Test Map".into(),
            width: 4,
            height: 4,
            tiles: vec![(0, 0); 16],
            spawn_points: vec![SpawnPoint {
                player: 0,
                pos: (1, 1),
            }],
            resources: vec![ResourcePlacement {
                kind: ResourceKind::FishPond,
                pos: (2, 2),
            }],
            neutral_camps: vec![],
            symmetry: MapSymmetry::Rotational180,
        };

        let ron_str = def.to_ron().unwrap();
        let restored = MapDefinition::from_ron(&ron_str).unwrap();

        assert_eq!(restored.name, "Test Map");
        assert_eq!(restored.width, 4);
        assert_eq!(restored.height, 4);
        assert_eq!(restored.tiles.len(), 16);
        assert_eq!(restored.spawn_points.len(), 1);
        assert_eq!(restored.resources.len(), 1);
    }

    #[test]
    fn validate_correct_map() {
        let def = MapDefinition {
            name: "Valid".into(),
            width: 4,
            height: 4,
            tiles: vec![(0, 0); 16],
            spawn_points: vec![SpawnPoint {
                player: 0,
                pos: (0, 0),
            }],
            resources: vec![],
            neutral_camps: vec![],
            symmetry: MapSymmetry::Rotational180,
        };
        assert!(def.validate().is_ok());
    }

    #[test]
    fn validate_wrong_tile_count() {
        let def = MapDefinition {
            name: "Bad".into(),
            width: 4,
            height: 4,
            tiles: vec![(0, 0); 10], // Wrong count
            spawn_points: vec![SpawnPoint {
                player: 0,
                pos: (0, 0),
            }],
            resources: vec![],
            neutral_camps: vec![],
            symmetry: MapSymmetry::Rotational180,
        };
        assert!(def.validate().is_err());
    }

    #[test]
    fn validate_invalid_terrain() {
        let def = MapDefinition {
            name: "Bad".into(),
            width: 2,
            height: 2,
            tiles: vec![(255, 0); 4], // Invalid terrain type
            spawn_points: vec![SpawnPoint {
                player: 0,
                pos: (0, 0),
            }],
            resources: vec![],
            neutral_camps: vec![],
            symmetry: MapSymmetry::Rotational180,
        };
        assert!(def.validate().is_err());
    }

    #[test]
    fn validate_no_spawn_points() {
        let def = MapDefinition {
            name: "Bad".into(),
            width: 2,
            height: 2,
            tiles: vec![(0, 0); 4],
            spawn_points: vec![],
            resources: vec![],
            neutral_camps: vec![],
            symmetry: MapSymmetry::Rotational180,
        };
        assert!(def.validate().is_err());
    }
}
