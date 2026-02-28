use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::terrain::TerrainType;

/// 8-bit transition mask representing which neighbors have lower tiling priority.
/// Bits: 0=NW, 1=N, 2=NE, 3=W, 4=E, 5=SW, 6=S, 7=SE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionMask(pub u8);

/// Bit positions for each neighbor direction.
pub const BIT_NW: u8 = 0;
pub const BIT_N: u8 = 1;
pub const BIT_NE: u8 = 2;
pub const BIT_W: u8 = 3;
pub const BIT_E: u8 = 4;
pub const BIT_SW: u8 = 5;
pub const BIT_S: u8 = 6;
pub const BIT_SE: u8 = 7;

/// Direction offsets matching bit positions.
const NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (-1, -1), // NW = bit 0
    (0, -1),  // N  = bit 1
    (1, -1),  // NE = bit 2
    (-1, 0),  // W  = bit 3
    (1, 0),   // E  = bit 4
    (-1, 1),  // SW = bit 5
    (0, 1),   // S  = bit 6
    (1, 1),   // SE = bit 7
];

/// An overlay piece to render on top of the base tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPiece {
    /// Edge overlay for a cardinal direction.
    Edge(CardinalDir),
    /// Inner corner: diagonal where both adjacent cardinals also transition.
    InnerCorner(DiagonalDir),
    /// Outer corner: diagonal only (adjacent cardinals don't transition).
    OuterCorner(DiagonalDir),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalDir {
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagonalDir {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl TransitionMask {
    /// Compute the transition mask for a tile at (x, y).
    /// Bits are set where neighbors have LOWER tiling priority.
    pub fn compute(map: &GameMap, x: i32, y: i32) -> Self {
        let center_terrain = map
            .terrain_at(GridPos::new(x, y))
            .unwrap_or(TerrainType::Grass);
        let center_priority = center_terrain.tiling_priority();

        let mut mask: u8 = 0;
        for (bit, &(dx, dy)) in NEIGHBOR_OFFSETS.iter().enumerate() {
            let np = GridPos::new(x + dx, y + dy);
            let neighbor_priority = map
                .terrain_at(np)
                .unwrap_or(center_terrain) // Out-of-bounds treated as same terrain
                .tiling_priority();
            if neighbor_priority < center_priority {
                mask |= 1 << bit;
            }
        }

        Self(mask)
    }

    /// Returns true if no transitions needed (uniform terrain).
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn has_bit(self, bit: u8) -> bool {
        self.0 & (1 << bit) != 0
    }

    /// Decompose the mask into individual overlay pieces.
    pub fn to_overlay_pieces(self) -> Vec<OverlayPiece> {
        let mut pieces = Vec::new();

        // Cardinal edges
        if self.has_bit(BIT_N) {
            pieces.push(OverlayPiece::Edge(CardinalDir::North));
        }
        if self.has_bit(BIT_E) {
            pieces.push(OverlayPiece::Edge(CardinalDir::East));
        }
        if self.has_bit(BIT_S) {
            pieces.push(OverlayPiece::Edge(CardinalDir::South));
        }
        if self.has_bit(BIT_W) {
            pieces.push(OverlayPiece::Edge(CardinalDir::West));
        }

        // Diagonal corners
        // NE corner: inner if both N and E transition; outer if only diagonal
        self.add_corner(
            &mut pieces,
            BIT_NE,
            BIT_N,
            BIT_E,
            DiagonalDir::NorthEast,
        );
        self.add_corner(
            &mut pieces,
            BIT_SE,
            BIT_S,
            BIT_E,
            DiagonalDir::SouthEast,
        );
        self.add_corner(
            &mut pieces,
            BIT_SW,
            BIT_S,
            BIT_W,
            DiagonalDir::SouthWest,
        );
        self.add_corner(
            &mut pieces,
            BIT_NW,
            BIT_N,
            BIT_W,
            DiagonalDir::NorthWest,
        );

        pieces
    }

    fn add_corner(
        self,
        pieces: &mut Vec<OverlayPiece>,
        diag_bit: u8,
        card1_bit: u8,
        card2_bit: u8,
        dir: DiagonalDir,
    ) {
        if self.has_bit(diag_bit) {
            if self.has_bit(card1_bit) && self.has_bit(card2_bit) {
                pieces.push(OverlayPiece::InnerCorner(dir));
            } else if !self.has_bit(card1_bit) && !self.has_bit(card2_bit) {
                pieces.push(OverlayPiece::OuterCorner(dir));
            }
            // If only one cardinal transitions, the edge overlay covers it
        }
    }
}

/// Map an overlay piece to its atlas index within a transition sprite sheet.
/// Sheet layout: 4 cols x 3 rows (12 cells)
/// Row 0: Edge N, Edge E, Edge S, Edge W
/// Row 1: Inner NE, Inner SE, Inner SW, Inner NW
/// Row 2: Outer NE, Outer SE, Outer SW, Outer NW
impl OverlayPiece {
    pub fn atlas_index(self) -> usize {
        match self {
            Self::Edge(CardinalDir::North) => 0,
            Self::Edge(CardinalDir::East) => 1,
            Self::Edge(CardinalDir::South) => 2,
            Self::Edge(CardinalDir::West) => 3,
            Self::InnerCorner(DiagonalDir::NorthEast) => 4,
            Self::InnerCorner(DiagonalDir::SouthEast) => 5,
            Self::InnerCorner(DiagonalDir::SouthWest) => 6,
            Self::InnerCorner(DiagonalDir::NorthWest) => 7,
            Self::OuterCorner(DiagonalDir::NorthEast) => 8,
            Self::OuterCorner(DiagonalDir::SouthEast) => 9,
            Self::OuterCorner(DiagonalDir::SouthWest) => 10,
            Self::OuterCorner(DiagonalDir::NorthWest) => 11,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_map_with_terrain(w: u32, h: u32, default: TerrainType) -> GameMap {
        let mut map = GameMap::new(w, h);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                map.get_mut(GridPos::new(x, y)).unwrap().terrain = default;
            }
        }
        map
    }

    #[test]
    fn uniform_terrain_no_transitions() {
        let map = GameMap::new(10, 10); // All grass
        let mask = TransitionMask::compute(&map, 5, 5);
        assert!(mask.is_empty());
        assert_eq!(mask.to_overlay_pieces().len(), 0);
    }

    #[test]
    fn single_water_neighbor() {
        let mut map = GameMap::new(10, 10); // All grass (priority 4)
        map.get_mut(GridPos::new(5, 4)).unwrap().terrain = TerrainType::Water; // North, priority 0
        let mask = TransitionMask::compute(&map, 5, 5);
        assert!(mask.has_bit(BIT_N));
        // Should have a north edge overlay
        let pieces = mask.to_overlay_pieces();
        assert!(pieces.contains(&OverlayPiece::Edge(CardinalDir::North)));
    }

    #[test]
    fn edge_of_map_no_crash() {
        let map = GameMap::new(5, 5);
        // Corner tile — out-of-bounds neighbors treated as same terrain
        let mask = TransitionMask::compute(&map, 0, 0);
        assert!(mask.is_empty());
    }

    #[test]
    fn outer_corner_diagonal_only() {
        let mut map = GameMap::new(10, 10); // All grass
        // Only the NE diagonal neighbor is water
        map.get_mut(GridPos::new(6, 4)).unwrap().terrain = TerrainType::Water;
        let mask = TransitionMask::compute(&map, 5, 5);
        assert!(mask.has_bit(BIT_NE));
        assert!(!mask.has_bit(BIT_N));
        assert!(!mask.has_bit(BIT_E));
        let pieces = mask.to_overlay_pieces();
        assert!(pieces.contains(&OverlayPiece::OuterCorner(DiagonalDir::NorthEast)));
    }

    #[test]
    fn inner_corner_all_three() {
        let mut map = GameMap::new(10, 10); // All grass
        // N, E, and NE all water
        map.get_mut(GridPos::new(5, 4)).unwrap().terrain = TerrainType::Water;
        map.get_mut(GridPos::new(6, 5)).unwrap().terrain = TerrainType::Water;
        map.get_mut(GridPos::new(6, 4)).unwrap().terrain = TerrainType::Water;
        let mask = TransitionMask::compute(&map, 5, 5);
        let pieces = mask.to_overlay_pieces();
        assert!(pieces.contains(&OverlayPiece::Edge(CardinalDir::North)));
        assert!(pieces.contains(&OverlayPiece::Edge(CardinalDir::East)));
        assert!(pieces.contains(&OverlayPiece::InnerCorner(DiagonalDir::NorthEast)));
    }

    #[test]
    fn overlay_piece_atlas_indices_unique() {
        use std::collections::HashSet;
        let all_pieces = [
            OverlayPiece::Edge(CardinalDir::North),
            OverlayPiece::Edge(CardinalDir::East),
            OverlayPiece::Edge(CardinalDir::South),
            OverlayPiece::Edge(CardinalDir::West),
            OverlayPiece::InnerCorner(DiagonalDir::NorthEast),
            OverlayPiece::InnerCorner(DiagonalDir::SouthEast),
            OverlayPiece::InnerCorner(DiagonalDir::SouthWest),
            OverlayPiece::InnerCorner(DiagonalDir::NorthWest),
            OverlayPiece::OuterCorner(DiagonalDir::NorthEast),
            OverlayPiece::OuterCorner(DiagonalDir::SouthEast),
            OverlayPiece::OuterCorner(DiagonalDir::SouthWest),
            OverlayPiece::OuterCorner(DiagonalDir::NorthWest),
        ];
        let indices: HashSet<usize> = all_pieces.iter().map(|p| p.atlas_index()).collect();
        assert_eq!(indices.len(), 12, "All 12 overlay pieces should have unique indices");
    }

    #[test]
    fn forest_over_grass_no_transition() {
        // Forest has higher priority than grass, so grass center won't see forest as lower
        let mut map = GameMap::new(10, 10); // All grass
        map.get_mut(GridPos::new(5, 4)).unwrap().terrain = TerrainType::Forest; // North, higher priority
        let mask = TransitionMask::compute(&map, 5, 5);
        // Forest has higher priority than grass, so no transition FROM grass perspective
        assert!(mask.is_empty());
    }

    #[test]
    fn higher_priority_terrain_sees_lower_neighbors() {
        // Forest tile surrounded by grass: forest (priority 6) sees grass (priority 4) as lower
        let mut map = GameMap::new(10, 10); // All grass
        map.get_mut(GridPos::new(5, 5)).unwrap().terrain = TerrainType::Forest;
        let mask = TransitionMask::compute(&map, 5, 5);
        assert!(!mask.is_empty(), "Forest should see transitions to surrounding grass");
    }
}
