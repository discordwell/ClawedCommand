use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, world_to_screen, TILE_HALF_HEIGHT, TILE_HALF_WIDTH};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

const HALF_W: f32 = TILE_HALF_WIDTH;
const HALF_H: f32 = TILE_HALF_HEIGHT;
const BORDER_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// Draw dark lines at terrain type boundaries using Gizmos (immediate mode, zero entity cost).
pub fn draw_terrain_borders(mut gizmos: Gizmos, map_res: Res<MapResource>) {
    let map = &map_res.map;

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elev_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

            // Screen center of this tile (Bevy Y-up)
            let sx = screen.x;
            let sy = -screen.y + elev_offset;

            // Check east neighbor (+1, 0 in grid)
            let east = GridPos::new(x + 1, y);
            if let Some(east_tile) = map.get(east) {
                if east_tile.terrain != tile.terrain {
                    // East edge: (sx + HALF_W, sy) → (sx, sy - HALF_H)
                    gizmos.line_2d(
                        Vec2::new(sx + HALF_W, sy),
                        Vec2::new(sx, sy - HALF_H),
                        BORDER_COLOR,
                    );
                }
            }

            // Check south neighbor (0, +1 in grid)
            let south = GridPos::new(x, y + 1);
            if let Some(south_tile) = map.get(south) {
                if south_tile.terrain != tile.terrain {
                    // South edge: (sx, sy - HALF_H) → (sx - HALF_W, sy)
                    gizmos.line_2d(
                        Vec2::new(sx, sy - HALF_H),
                        Vec2::new(sx - HALF_W, sy),
                        BORDER_COLOR,
                    );
                }
            }
        }
    }
}
