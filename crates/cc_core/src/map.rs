use crate::coords::GridPos;
use crate::math::Fixed;
use crate::terrain::{
    FactionId, TerrainType, is_passable_for_faction, FLAG_TEMP_BLOCKED, FLAG_WATER_CONVERTED,
};

/// Per-tile data.
#[derive(Debug, Clone, Copy)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: u8,       // 0-2 height levels
    pub dynamic_flags: u8,   // Bit 0 = temp blocked, Bit 1 = water converted
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            terrain: TerrainType::Grass,
            elevation: 0,
            dynamic_flags: 0,
        }
    }
}

impl TileData {
    /// Effective terrain type accounting for dynamic overlays.
    pub fn effective_terrain(&self) -> TerrainType {
        if self.dynamic_flags & FLAG_WATER_CONVERTED != 0 {
            TerrainType::Water
        } else {
            self.terrain
        }
    }

    /// Whether this tile is dynamically blocked by an overlay.
    pub fn is_dynamically_blocked(&self) -> bool {
        self.dynamic_flags & FLAG_TEMP_BLOCKED != 0
    }
}

/// The game map: a rectangular grid of tiles.
#[derive(Debug, Clone)]
pub struct GameMap {
    pub width: u32,
    pub height: u32,
    tiles: Vec<TileData>,
}

impl GameMap {
    /// Create a map filled with default (Grass, passable) tiles.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            tiles: vec![TileData::default(); (width * height) as usize],
        }
    }

    fn index(&self, pos: GridPos) -> Option<usize> {
        if pos.x >= 0
            && pos.y >= 0
            && (pos.x as u32) < self.width
            && (pos.y as u32) < self.height
        {
            Some((pos.y as u32 * self.width + pos.x as u32) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, pos: GridPos) -> Option<&TileData> {
        self.index(pos).map(|i| &self.tiles[i])
    }

    pub fn get_mut(&mut self, pos: GridPos) -> Option<&mut TileData> {
        self.index(pos).map(|i| &mut self.tiles[i])
    }

    /// Backward-compatible passability check (uses base_passable, ignores faction).
    pub fn is_passable(&self, pos: GridPos) -> bool {
        self.get(pos).is_some_and(|t| {
            !t.is_dynamically_blocked() && t.effective_terrain().base_passable()
        })
    }

    /// Faction-aware passability check.
    pub fn is_passable_for(&self, pos: GridPos, faction: FactionId) -> bool {
        self.get(pos).is_some_and(|t| {
            !t.is_dynamically_blocked()
                && is_passable_for_faction(t.effective_terrain(), faction)
        })
    }

    /// Movement cost at a position (None if impassable or dynamically blocked).
    pub fn movement_cost(&self, pos: GridPos) -> Option<Fixed> {
        self.get(pos).and_then(|t| {
            if t.is_dynamically_blocked() {
                return None;
            }
            let terrain = t.effective_terrain();
            if terrain.base_passable() || terrain == TerrainType::Water {
                Some(terrain.movement_cost())
            } else {
                None
            }
        })
    }

    /// Movement cost for a specific faction (accounts for water traversal).
    pub fn movement_cost_for(&self, pos: GridPos, faction: FactionId) -> Option<Fixed> {
        self.get(pos).and_then(|t| {
            if t.is_dynamically_blocked() {
                return None;
            }
            let terrain = t.effective_terrain();
            if is_passable_for_faction(terrain, faction) {
                Some(terrain.movement_cost())
            } else {
                None
            }
        })
    }

    pub fn in_bounds(&self, pos: GridPos) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && (pos.x as u32) < self.width
            && (pos.y as u32) < self.height
    }

    /// Return all passable neighbors (8-directional), using base passability.
    pub fn neighbors(&self, pos: GridPos) -> Vec<GridPos> {
        DIRS.iter()
            .map(|(dx, dy)| GridPos::new(pos.x + dx, pos.y + dy))
            .filter(|p| self.is_passable(*p))
            .collect()
    }

    /// Return all passable neighbors for a specific faction.
    pub fn neighbors_for_faction(&self, pos: GridPos, faction: FactionId) -> Vec<GridPos> {
        DIRS.iter()
            .map(|(dx, dy)| GridPos::new(pos.x + dx, pos.y + dy))
            .filter(|p| self.is_passable_for(*p, faction))
            .collect()
    }

    /// Elevation at a position (0 if out of bounds).
    pub fn elevation_at(&self, pos: GridPos) -> u8 {
        self.get(pos).map(|t| t.elevation).unwrap_or(0)
    }

    /// Check if movement between adjacent tiles is allowed by elevation rules.
    /// Movement between different elevation levels requires one tile to be a Ramp.
    pub fn can_move_between(&self, from: GridPos, to: GridPos) -> bool {
        let Some(from_tile) = self.get(from) else {
            return false;
        };
        let Some(to_tile) = self.get(to) else {
            return false;
        };

        if from_tile.elevation == to_tile.elevation {
            return true;
        }

        // Different elevations: one tile must be a Ramp
        from_tile.effective_terrain() == TerrainType::Ramp
            || to_tile.effective_terrain() == TerrainType::Ramp
    }

    /// Elevation advantage of attacker over target (positive = higher ground).
    pub fn elevation_advantage(&self, attacker_pos: GridPos, target_pos: GridPos) -> i8 {
        let a = self.elevation_at(attacker_pos) as i8;
        let t = self.elevation_at(target_pos) as i8;
        a - t
    }

    /// Terrain type at a position.
    pub fn terrain_at(&self, pos: GridPos) -> Option<TerrainType> {
        self.get(pos).map(|t| t.effective_terrain())
    }

    /// Raw tile data slice (for serialization/map gen).
    pub fn tiles(&self) -> &[TileData] {
        &self.tiles
    }

    /// Mutable tile data slice (for map gen).
    pub fn tiles_mut(&mut self) -> &mut [TileData] {
        &mut self.tiles
    }
}

const DIRS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_map_all_passable() {
        let map = GameMap::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                assert!(map.is_passable(GridPos::new(x, y)));
            }
        }
    }

    #[test]
    fn out_of_bounds_not_passable() {
        let map = GameMap::new(10, 10);
        assert!(!map.is_passable(GridPos::new(-1, 0)));
        assert!(!map.is_passable(GridPos::new(0, -1)));
        assert!(!map.is_passable(GridPos::new(10, 0)));
        assert!(!map.is_passable(GridPos::new(0, 10)));
    }

    #[test]
    fn set_impassable_rock() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        map.get_mut(pos).unwrap().terrain = TerrainType::Rock;
        assert!(!map.is_passable(pos));
    }

    #[test]
    fn set_water_impassable_for_catgpt() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        map.get_mut(pos).unwrap().terrain = TerrainType::Water;
        assert!(!map.is_passable(pos)); // base passable = false
        assert!(!map.is_passable_for(pos, FactionId::CatGPT));
        assert!(map.is_passable_for(pos, FactionId::Croak));
    }

    #[test]
    fn neighbors_corner() {
        let map = GameMap::new(10, 10);
        let neighbors = map.neighbors(GridPos::new(0, 0));
        assert_eq!(neighbors.len(), 3);
    }

    #[test]
    fn neighbors_center() {
        let map = GameMap::new(10, 10);
        let neighbors = map.neighbors(GridPos::new(5, 5));
        assert_eq!(neighbors.len(), 8);
    }

    #[test]
    fn neighbors_excludes_impassable() {
        let mut map = GameMap::new(10, 10);
        map.get_mut(GridPos::new(6, 5)).unwrap().terrain = TerrainType::Rock;
        map.get_mut(GridPos::new(4, 5)).unwrap().terrain = TerrainType::Rock;
        let neighbors = map.neighbors(GridPos::new(5, 5));
        assert_eq!(neighbors.len(), 6);
    }

    #[test]
    fn faction_neighbors_croak_includes_water() {
        let mut map = GameMap::new(10, 10);
        // Put water to the right
        map.get_mut(GridPos::new(6, 5)).unwrap().terrain = TerrainType::Water;
        let catgpt_neighbors = map.neighbors_for_faction(GridPos::new(5, 5), FactionId::CatGPT);
        let croak_neighbors = map.neighbors_for_faction(GridPos::new(5, 5), FactionId::Croak);
        assert_eq!(catgpt_neighbors.len(), 7); // water excluded
        assert_eq!(croak_neighbors.len(), 8);  // water included
    }

    #[test]
    fn dynamic_block_flag() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        assert!(map.is_passable(pos));
        map.get_mut(pos).unwrap().dynamic_flags |= FLAG_TEMP_BLOCKED;
        assert!(!map.is_passable(pos));
        assert!(!map.is_passable_for(pos, FactionId::Croak));
    }

    #[test]
    fn water_convert_flag() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        assert_eq!(map.terrain_at(pos), Some(TerrainType::Grass));
        map.get_mut(pos).unwrap().dynamic_flags |= FLAG_WATER_CONVERTED;
        assert_eq!(map.terrain_at(pos), Some(TerrainType::Water));
        assert!(!map.is_passable(pos)); // Water not base passable
        assert!(map.is_passable_for(pos, FactionId::Croak)); // But Croak can
    }

    #[test]
    fn movement_cost_values() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);

        // Grass = 1.0
        let grass_cost = map.movement_cost(pos).unwrap();
        assert_eq!(grass_cost, Fixed::ONE);

        // Road = 0.7 (faster)
        map.get_mut(pos).unwrap().terrain = TerrainType::Road;
        let road_cost = map.movement_cost(pos).unwrap();
        assert!(road_cost < grass_cost);

        // Rock = impassable (None)
        map.get_mut(pos).unwrap().terrain = TerrainType::Rock;
        assert!(map.movement_cost(pos).is_none());
    }

    #[test]
    fn movement_cost_faction_aware() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        map.get_mut(pos).unwrap().terrain = TerrainType::Water;

        assert!(map.movement_cost_for(pos, FactionId::CatGPT).is_none());
        assert!(map.movement_cost_for(pos, FactionId::Croak).is_some());
    }

    #[test]
    fn elevation_queries() {
        let mut map = GameMap::new(10, 10);
        map.get_mut(GridPos::new(3, 3)).unwrap().elevation = 2;
        map.get_mut(GridPos::new(5, 5)).unwrap().elevation = 0;

        assert_eq!(map.elevation_at(GridPos::new(3, 3)), 2);
        assert_eq!(map.elevation_at(GridPos::new(5, 5)), 0);
        assert_eq!(map.elevation_advantage(GridPos::new(3, 3), GridPos::new(5, 5)), 2);
        assert_eq!(map.elevation_advantage(GridPos::new(5, 5), GridPos::new(3, 3)), -2);
    }

    #[test]
    fn elevation_movement_requires_ramp() {
        let mut map = GameMap::new(10, 10);
        let low = GridPos::new(5, 5);
        let high = GridPos::new(6, 5);
        let ramp = GridPos::new(5, 6);

        map.get_mut(high).unwrap().elevation = 1;
        map.get_mut(ramp).unwrap().terrain = TerrainType::Ramp;
        map.get_mut(ramp).unwrap().elevation = 1;

        // Same elevation: allowed
        assert!(map.can_move_between(low, GridPos::new(4, 5)));
        // Different elevation, no ramp: blocked
        assert!(!map.can_move_between(low, high));
        // Ramp tile: allowed
        assert!(map.can_move_between(low, ramp));
    }

    #[test]
    fn movement_cost_blocked_by_dynamic_flag() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        assert!(map.movement_cost(pos).is_some());
        map.get_mut(pos).unwrap().dynamic_flags |= FLAG_TEMP_BLOCKED;
        assert!(map.movement_cost(pos).is_none());
    }

    #[test]
    fn default_tile_is_grass() {
        let map = GameMap::new(5, 5);
        let tile = map.get(GridPos::new(0, 0)).unwrap();
        assert_eq!(tile.terrain, TerrainType::Grass);
        assert_eq!(tile.elevation, 0);
        assert_eq!(tile.dynamic_flags, 0);
    }
}
