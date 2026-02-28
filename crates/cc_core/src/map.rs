use crate::coords::GridPos;

/// Per-tile data.
#[derive(Debug, Clone, Copy)]
pub struct TileData {
    pub passable: bool,
    pub elevation: u8,
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            passable: true,
            elevation: 0,
        }
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
    /// Create a map filled with default (passable) tiles.
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

    pub fn is_passable(&self, pos: GridPos) -> bool {
        self.get(pos).is_some_and(|t| t.passable)
    }

    pub fn in_bounds(&self, pos: GridPos) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && (pos.x as u32) < self.width
            && (pos.y as u32) < self.height
    }

    /// Return all passable neighbors (8-directional).
    pub fn neighbors(&self, pos: GridPos) -> Vec<GridPos> {
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
        DIRS.iter()
            .map(|(dx, dy)| GridPos::new(pos.x + dx, pos.y + dy))
            .filter(|p| self.is_passable(*p))
            .collect()
    }
}

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
    fn set_impassable() {
        let mut map = GameMap::new(10, 10);
        let pos = GridPos::new(5, 5);
        map.get_mut(pos).unwrap().passable = false;
        assert!(!map.is_passable(pos));
    }

    #[test]
    fn neighbors_corner() {
        let map = GameMap::new(10, 10);
        let neighbors = map.neighbors(GridPos::new(0, 0));
        // Corner has 3 neighbors (right, down-right, down)
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
        map.get_mut(GridPos::new(6, 5)).unwrap().passable = false;
        map.get_mut(GridPos::new(4, 5)).unwrap().passable = false;
        let neighbors = map.neighbors(GridPos::new(5, 5));
        assert_eq!(neighbors.len(), 6);
    }
}
