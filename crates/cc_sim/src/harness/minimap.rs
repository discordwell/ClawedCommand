//! Headless minimap renderer — produces RGBA PNG bytes.

use bevy::prelude::*;

use cc_core::components::{Owner, Position};
use cc_core::terrain::TerrainType;

use crate::resources::MapResource;

/// Map terrain type to RGB color (replicates cc_client minimap palette).
fn terrain_color(terrain: TerrainType) -> (u8, u8, u8) {
    match terrain {
        TerrainType::Grass => (72, 140, 64),
        TerrainType::Dirt => (140, 107, 72),
        TerrainType::Sand => (217, 199, 140),
        TerrainType::Forest => (46, 107, 38),
        TerrainType::Water => (38, 89, 166),
        TerrainType::Shallows => (107, 178, 230),
        TerrainType::Rock => (89, 82, 77),
        TerrainType::Ramp => (128, 115, 97),
        TerrainType::Road => (158, 138, 107),
        TerrainType::TechRuins => (110, 110, 122),
    }
}

/// Player team colors.
fn player_color(player_id: u8) -> (u8, u8, u8) {
    match player_id {
        0 => (50, 100, 230), // Blue
        _ => (230, 50, 50),  // Red
    }
}

/// Render a minimap to PNG bytes.
///
/// Paints terrain at 1:1 (one pixel per tile), overlays unit dots and building
/// squares in team colors, then scales 4x via nearest-neighbor.
pub fn render_minimap(world: &mut World, width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;

    // RGBA buffer at map resolution
    let mut buf = vec![255u8; w * h * 4];

    // Paint terrain from MapResource
    {
        let map_res = world.resource::<MapResource>();
        for y in 0..h {
            for x in 0..w {
                let pos = cc_core::coords::GridPos::new(x as i32, y as i32);
                let (r, g, b) = map_res
                    .map
                    .get(pos)
                    .map(|tile| terrain_color(tile.terrain))
                    .unwrap_or((0, 0, 0));

                let idx = (y * w + x) * 4;
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = 255;
            }
        }
    }

    // Paint units as single pixels
    let unit_dots: Vec<(i32, i32, u8)> = world
        .query::<(&Position, &Owner)>()
        .iter(world)
        .map(|(pos, owner)| {
            let grid = pos.world.to_grid();
            (grid.x, grid.y, owner.player_id)
        })
        .collect();

    for (gx, gy, pid) in &unit_dots {
        let x = *gx as usize;
        let y = *gy as usize;
        if x < w && y < h {
            let (r, g, b) = player_color(*pid);
            let idx = (y * w + x) * 4;
            buf[idx] = r;
            buf[idx + 1] = g;
            buf[idx + 2] = b;
        }
    }

    // Paint buildings as 2x2 squares
    let building_rects: Vec<(i32, i32, u8)> = world
        .query::<(&Position, &Owner, &cc_core::components::Building)>()
        .iter(world)
        .map(|(pos, owner, _)| {
            let grid = pos.world.to_grid();
            (grid.x, grid.y, owner.player_id)
        })
        .collect();

    for (gx, gy, pid) in &building_rects {
        let (r, g, b) = player_color(*pid);
        for dy in 0..2i32 {
            for dx in 0..2i32 {
                let x = (*gx + dx) as usize;
                let y = (*gy + dy) as usize;
                if x < w && y < h {
                    let idx = (y * w + x) * 4;
                    buf[idx] = r;
                    buf[idx + 1] = g;
                    buf[idx + 2] = b;
                }
            }
        }
    }

    // Scale 4x via nearest-neighbor
    let scale = 4;
    let sw = w * scale;
    let sh = h * scale;
    let mut scaled = vec![0u8; sw * sh * 4];
    for sy in 0..sh {
        for sx in 0..sw {
            let src_idx = ((sy / scale) * w + (sx / scale)) * 4;
            let dst_idx = (sy * sw + sx) * 4;
            scaled[dst_idx..dst_idx + 4].copy_from_slice(&buf[src_idx..src_idx + 4]);
        }
    }

    // Encode to PNG via image crate
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
        encoder
            .write_image(
                &scaled,
                sw as u32,
                sh as u32,
                image::ExtendedColorType::Rgba8,
            )
            .expect("PNG encode failed");
    }
    cursor.into_inner()
}
