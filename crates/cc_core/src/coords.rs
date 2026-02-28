use serde::{Deserialize, Serialize};

use crate::math::{Fixed, FIXED_ZERO, fixed_from_i32, fixed_to_f32};

/// Tile width/height in world units for isometric projection.
pub const TILE_WIDTH: f32 = 64.0;
pub const TILE_HEIGHT: f32 = 32.0;
pub const TILE_HALF_WIDTH: f32 = TILE_WIDTH / 2.0;
pub const TILE_HALF_HEIGHT: f32 = TILE_HEIGHT / 2.0;

/// Logical tile position on the isometric grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Sub-tile world position used in the deterministic simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldPos {
    pub x: Fixed,
    pub y: Fixed,
}

impl WorldPos {
    pub fn new(x: Fixed, y: Fixed) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self {
            x: FIXED_ZERO,
            y: FIXED_ZERO,
        }
    }

    pub fn from_grid(grid: GridPos) -> Self {
        Self {
            x: fixed_from_i32(grid.x),
            y: fixed_from_i32(grid.y),
        }
    }

    pub fn to_grid(self) -> GridPos {
        GridPos {
            x: self.x.to_num::<i32>(),
            y: self.y.to_num::<i32>(),
        }
    }

    /// Distance squared (avoids sqrt for comparisons).
    pub fn distance_squared(self, other: WorldPos) -> Fixed {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

/// Pixel position on screen (rendering only, not deterministic).
#[derive(Debug, Clone, Copy)]
pub struct ScreenPos {
    pub x: f32,
    pub y: f32,
}

/// Convert a world position to screen pixel coordinates (isometric projection).
pub fn world_to_screen(world: WorldPos) -> ScreenPos {
    let wx = fixed_to_f32(world.x);
    let wy = fixed_to_f32(world.y);
    ScreenPos {
        x: (wx - wy) * TILE_HALF_WIDTH,
        y: (wx + wy) * TILE_HALF_HEIGHT,
    }
}

/// Convert screen pixel coordinates back to world position (inverse isometric).
pub fn screen_to_world(screen: ScreenPos) -> WorldPos {
    let sx = screen.x / TILE_HALF_WIDTH;
    let sy = screen.y / TILE_HALF_HEIGHT;
    let wx = (sx + sy) / 2.0;
    let wy = (sy - sx) / 2.0;
    WorldPos {
        x: Fixed::from_num(wx),
        y: Fixed::from_num(wy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_world_round_trip() {
        for (gx, gy) in [(0, 0), (5, 3), (-2, 7), (10, 10)] {
            let grid = GridPos::new(gx, gy);
            let world = WorldPos::from_grid(grid);
            let back = world.to_grid();
            assert_eq!(grid, back, "round trip failed for ({gx}, {gy})");
        }
    }

    #[test]
    fn screen_world_round_trip() {
        let original = WorldPos::new(fixed_from_i32(5), fixed_from_i32(3));
        let screen = world_to_screen(original);
        let back = screen_to_world(screen);
        let epsilon = Fixed::from_num(0.01f32);
        assert!((original.x - back.x).abs() < epsilon);
        assert!((original.y - back.y).abs() < epsilon);
    }

    #[test]
    fn origin_maps_to_screen_origin() {
        let screen = world_to_screen(WorldPos::zero());
        assert!((screen.x).abs() < 0.001);
        assert!((screen.y).abs() < 0.001);
    }
}
