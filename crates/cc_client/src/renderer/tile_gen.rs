use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cc_core::terrain::TerrainType;

/// Width and height of each tile image in pixels.
const TILE_W: usize = 64;
const TILE_H: usize = 32;

/// Resource holding procedurally generated terrain tile images.
#[derive(Resource)]
pub struct ProceduralTiles {
    /// One image handle per TerrainType (indexed by `terrain as usize`).
    pub terrain: [Handle<Image>; 15],
    /// Second water variant for animation.
    pub water_alt: Handle<Image>,
    /// Second shallows variant for animation.
    pub shallows_alt: Handle<Image>,
}

/// Generate procedural terrain tile images at startup.
pub fn generate_terrain_tiles(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(15);

    for terrain in TerrainType::ALL {
        let img = generate_tile_image(terrain, 0);
        handles.push(images.add(img));
    }

    let water_alt = images.add(generate_tile_image(TerrainType::Water, 1));
    let shallows_alt = images.add(generate_tile_image(TerrainType::Shallows, 1));

    commands.insert_resource(ProceduralTiles {
        terrain: [
            handles[0].clone(),
            handles[1].clone(),
            handles[2].clone(),
            handles[3].clone(),
            handles[4].clone(),
            handles[5].clone(),
            handles[6].clone(),
            handles[7].clone(),
            handles[8].clone(),
            handles[9].clone(),
            handles[10].clone(),
            handles[11].clone(),
            handles[12].clone(),
            handles[13].clone(),
            handles[14].clone(),
        ],
        water_alt,
        shallows_alt,
    });
}

/// Generate a single 64x32 RGBA tile image for the given terrain type.
fn generate_tile_image(terrain: TerrainType, variant: u8) -> Image {
    let mut data = vec![0u8; TILE_W * TILE_H * 4];

    for py in 0..TILE_H {
        for px in 0..TILE_W {
            // Diamond mask: |px-32|/32 + |py-16|/16 <= 1.0
            let dx = (px as f32 - 32.0).abs() / 32.0;
            let dy = (py as f32 - 16.0).abs() / 16.0;
            if dx + dy > 1.0 {
                continue; // Outside diamond — leave transparent
            }

            let (r, g, b) = terrain_pixel(terrain, px, py, variant);
            let idx = (py * TILE_W + px) * 4;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    Image::new(
        Extent3d {
            width: TILE_W as u32,
            height: TILE_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// Compute the pixel color for a terrain type at position (px, py) within the tile.
fn terrain_pixel(terrain: TerrainType, px: usize, py: usize, variant: u8) -> (u8, u8, u8) {
    let noise = pseudo_noise(px, py);
    let fine = fine_noise(px, py);

    match terrain {
        TerrainType::Grass => {
            let base_r = 90u8;
            let base_g = 154u8;
            let base_b = 50u8;
            // Perlin-ish noise with darker blade clusters
            let blade = if noise > 200 { -20i16 } else { 0 };
            let variation = ((fine as i16) - 128) / 8;
            (
                clamp_u8(base_r as i16 + variation + blade),
                clamp_u8(base_g as i16 + variation * 2 + blade),
                clamp_u8(base_b as i16 + variation + blade),
            )
        }
        TerrainType::Dirt => {
            let base_r = 196u8;
            let base_g = 162u8;
            let base_b = 101u8;
            // Coarse noise with small pebble dots
            let variation = ((noise as i16) - 128) / 6;
            let pebble = if fine > 240 { -25i16 } else { 0 };
            (
                clamp_u8(base_r as i16 + variation + pebble),
                clamp_u8(base_g as i16 + variation + pebble),
                clamp_u8(base_b as i16 + variation / 2 + pebble),
            )
        }
        TerrainType::Sand => {
            let base_r = 232u8;
            let base_g = 213u8;
            let base_b = 163u8;
            // Fine noise with subtle wavy lines
            let variation = ((fine as i16) - 128) / 12;
            let wave = if (py + px / 4).is_multiple_of(6) {
                -8i16
            } else {
                0
            };
            (
                clamp_u8(base_r as i16 + variation + wave),
                clamp_u8(base_g as i16 + variation + wave),
                clamp_u8(base_b as i16 + variation / 2 + wave),
            )
        }
        TerrainType::Forest => {
            let base_r = 74u8;
            let base_g = 122u8;
            let base_b = 46u8;
            // Dense dark clusters for canopy shadows
            let shadow = if noise > 160 && fine > 100 { -30i16 } else { 0 };
            let variation = ((fine as i16) - 128) / 10;
            (
                clamp_u8(base_r as i16 + variation + shadow),
                clamp_u8(base_g as i16 + variation * 2 + shadow),
                clamp_u8(base_b as i16 + variation + shadow),
            )
        }
        TerrainType::Water => {
            let base_r = 74u8;
            let base_g = 144u8;
            let base_b = 217u8;
            // Wave line pattern with highlights
            let wave_offset = if variant == 0 { 0 } else { 3 };
            let wave = if (py + wave_offset + px / 8).is_multiple_of(5) {
                15i16
            } else {
                0
            };
            let ripple = if fine > 230 { 20i16 } else { 0 };
            let variation = ((noise as i16) - 128) / 12;
            (
                clamp_u8(base_r as i16 + variation + ripple),
                clamp_u8(base_g as i16 + variation + wave + ripple),
                clamp_u8(base_b as i16 + variation / 2 + wave + ripple),
            )
        }
        TerrainType::Shallows => {
            let base_r = 106u8;
            let base_g = 180u8;
            let base_b = 232u8;
            // Sparse waves, ground color bleed-through
            let wave_offset = if variant == 0 { 0 } else { 2 };
            let wave = if (py + wave_offset + px / 6).is_multiple_of(7) {
                10i16
            } else {
                0
            };
            let ground_bleed = if fine < 40 { 15i16 } else { 0 };
            let variation = ((noise as i16) - 128) / 14;
            (
                clamp_u8(base_r as i16 + variation + ground_bleed),
                clamp_u8(base_g as i16 + variation + wave - ground_bleed / 2),
                clamp_u8(base_b as i16 + variation / 2 + wave),
            )
        }
        TerrainType::Rock => {
            let base_r = 140u8;
            let base_g = 140u8;
            let base_b = 140u8;
            // Angular noise with dark crack lines
            let variation = ((noise as i16) - 128) / 5;
            let crack = if (px.wrapping_mul(3) ^ py.wrapping_mul(7)).is_multiple_of(17) {
                -40i16
            } else {
                0
            };
            (
                clamp_u8(base_r as i16 + variation + crack),
                clamp_u8(base_g as i16 + variation + crack),
                clamp_u8(base_b as i16 + variation + crack),
            )
        }
        TerrainType::Ramp => {
            let base_r = 170u8;
            let base_g = 155u8;
            let base_b = 130u8;
            // Diagonal chevron/stripe pattern
            let stripe = if (px + py) % 8 < 3 { -12i16 } else { 0 };
            let variation = ((fine as i16) - 128) / 10;
            (
                clamp_u8(base_r as i16 + variation + stripe),
                clamp_u8(base_g as i16 + variation + stripe),
                clamp_u8(base_b as i16 + variation / 2 + stripe),
            )
        }
        TerrainType::Road => {
            let base_r = 158u8;
            let base_g = 138u8;
            let base_b = 107u8;
            // Smooth with cobblestone dot pattern
            let variation = ((fine as i16) - 128) / 16;
            let cobble = if px % 8 < 2 && py % 6 < 2 { -15i16 } else { 0 };
            (
                clamp_u8(base_r as i16 + variation + cobble),
                clamp_u8(base_g as i16 + variation + cobble),
                clamp_u8(base_b as i16 + variation / 2 + cobble),
            )
        }
        TerrainType::TechRuins => {
            let base_r = 110u8;
            let base_g = 110u8;
            let base_b = 122u8;
            // Grid circuit lines with cyan accent dots
            let grid_line = if px.is_multiple_of(8) || py.is_multiple_of(8) {
                -15i16
            } else {
                0
            };
            let accent = px % 16 == 4 && py % 8 == 4;
            if accent {
                (80, 220, 220) // Cyan accent dot
            } else {
                let variation = ((noise as i16) - 128) / 10;
                (
                    clamp_u8(base_r as i16 + variation + grid_line),
                    clamp_u8(base_g as i16 + variation + grid_line),
                    clamp_u8(base_b as i16 + variation / 2 + grid_line),
                )
            }
        }
        TerrainType::Concrete => {
            let base_r = 184u8;
            let base_g = 179u8;
            let base_b = 173u8;
            // Subtle crack lines and aggregate texture
            let crack = if (px + py * 7) % 31 == 0 { -18i16 } else { 0 };
            let variation = ((noise as i16) - 128) / 12;
            (
                clamp_u8(base_r as i16 + variation + crack),
                clamp_u8(base_g as i16 + variation + crack),
                clamp_u8(base_b as i16 + variation + crack),
            )
        }
        TerrainType::Linoleum => {
            let base_r = 199u8;
            let base_g = 189u8;
            let base_b = 166u8;
            // Faint grid lines every 16px
            let grid = if px.is_multiple_of(16) || py.is_multiple_of(16) {
                -8i16
            } else {
                0
            };
            let variation = ((fine as i16) - 128) / 16;
            (
                clamp_u8(base_r as i16 + variation + grid),
                clamp_u8(base_g as i16 + variation + grid),
                clamp_u8(base_b as i16 + variation + grid),
            )
        }
        TerrainType::CarpetTile => {
            let base_r = 115u8;
            let base_g = 122u8;
            let base_b = 140u8;
            // Fine carpet fiber noise
            let variation = ((noise as i16) - 128) / 10;
            let fiber = ((fine as i16) - 128) / 14;
            (
                clamp_u8(base_r as i16 + variation + fiber),
                clamp_u8(base_g as i16 + variation + fiber),
                clamp_u8(base_b as i16 + variation / 2 + fiber),
            )
        }
        TerrainType::MetalGrate => {
            let base_r = 97u8;
            let base_g = 102u8;
            let base_b = 107u8;
            // Diamond-plate cross-hatch pattern
            let diamond = if ((px + py) % 8 < 2) || ((px.wrapping_add(8).wrapping_sub(py)) % 8 < 2) {
                12i16
            } else {
                0
            };
            let variation = ((noise as i16) - 128) / 14;
            (
                clamp_u8(base_r as i16 + variation + diamond),
                clamp_u8(base_g as i16 + variation + diamond),
                clamp_u8(base_b as i16 + variation + diamond),
            )
        }
        TerrainType::DryWall => {
            let base_r = 217u8;
            let base_g = 212u8;
            let base_b = 204u8;
            // Very subtle stucco texture
            let variation = ((fine as i16) - 128) / 20;
            (
                clamp_u8(base_r as i16 + variation),
                clamp_u8(base_g as i16 + variation),
                clamp_u8(base_b as i16 + variation),
            )
        }
    }
}

/// Pseudo-random noise based on pixel position (deterministic hash).
fn pseudo_noise(px: usize, py: usize) -> u8 {
    let h = px.wrapping_mul(374761393) ^ py.wrapping_mul(668265263);
    let h = h.wrapping_mul(h).wrapping_shr(16);
    (h & 0xFF) as u8
}

/// Fine-grained noise with different seed.
fn fine_noise(px: usize, py: usize) -> u8 {
    let h = px.wrapping_mul(1103515245).wrapping_add(12345)
        ^ py.wrapping_mul(214013).wrapping_add(2531011);
    let h = h.wrapping_shr(8);
    (h & 0xFF) as u8
}

fn clamp_u8(v: i16) -> u8 {
    v.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_mask_center_inside() {
        // Center of 64x32 tile should be inside
        let dx = (32.0f32 - 32.0).abs() / 32.0;
        let dy = (16.0f32 - 16.0).abs() / 16.0;
        assert!(dx + dy <= 1.0);
    }

    #[test]
    fn diamond_mask_corners_outside() {
        // Corners of the tile should be outside
        for (px, py) in [(0, 0), (63, 0), (0, 31), (63, 31)] {
            let dx = (px as f32 - 32.0).abs() / 32.0;
            let dy = (py as f32 - 16.0).abs() / 16.0;
            assert!(dx + dy > 1.0, "({px},{py}) should be outside diamond");
        }
    }

    #[test]
    fn diamond_mask_edges_inside() {
        // Points on the diamond edge should be inside (or exactly on boundary)
        // Left point: (0, 16)
        let dx = (0.0f32 - 32.0).abs() / 32.0;
        let dy = (16.0f32 - 16.0).abs() / 16.0;
        assert!(dx + dy <= 1.0, "Left edge should be inside");
        // Top point: (32, 0)
        let dx = (32.0f32 - 32.0).abs() / 32.0;
        let dy = (0.0f32 - 16.0).abs() / 16.0;
        assert!(dx + dy <= 1.0, "Top edge should be inside");
    }

    #[test]
    fn all_terrain_types_generate() {
        for terrain in TerrainType::ALL {
            let img = generate_tile_image(terrain, 0);
            assert_eq!(img.width(), TILE_W as u32);
            assert_eq!(img.height(), TILE_H as u32);
        }
    }
}
