use bevy::prelude::*;

use cc_core::coords::{GridPos, TILE_HALF_HEIGHT, TILE_HALF_WIDTH, WorldPos, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

const HALF_W: f32 = TILE_HALF_WIDTH;
const HALF_H: f32 = TILE_HALF_HEIGHT;
const BORDER_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// Draw dark lines at terrain type boundaries using Gizmos.
/// Border line positions are cached after first computation since terrain is static.
pub fn draw_terrain_borders(
    mut gizmos: Gizmos,
    map_res: Res<MapResource>,
    mut cached_lines: Local<Option<Vec<(Vec2, Vec2)>>>,
) {
    let lines = cached_lines.get_or_insert_with(|| {
        let map = &map_res.map;
        let mut result = Vec::new();

        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let grid = GridPos::new(x, y);
                let tile = map.get(grid).unwrap();
                let world = WorldPos::from_grid(grid);
                let screen = world_to_screen(world);
                let elev_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

                let sx = screen.x;
                let sy = -screen.y + elev_offset;

                // Check east neighbor (+1, 0 in grid)
                let east = GridPos::new(x + 1, y);
                if let Some(east_tile) = map.get(east) {
                    if east_tile.terrain != tile.terrain {
                        result.push((Vec2::new(sx + HALF_W, sy), Vec2::new(sx, sy - HALF_H)));
                    }
                }

                // Check south neighbor (0, +1 in grid)
                let south = GridPos::new(x, y + 1);
                if let Some(south_tile) = map.get(south) {
                    if south_tile.terrain != tile.terrain {
                        result.push((Vec2::new(sx, sy - HALF_H), Vec2::new(sx - HALF_W, sy)));
                    }
                }
            }
        }

        result
    });

    for &(start, end) in lines.iter() {
        gizmos.line_2d(start, end, BORDER_COLOR);
    }
}
