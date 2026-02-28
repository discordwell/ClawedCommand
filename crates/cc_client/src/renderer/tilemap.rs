use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, world_to_screen, TILE_HALF_HEIGHT, TILE_HALF_WIDTH};
use cc_core::terrain::TerrainType;
use cc_sim::resources::MapResource;

/// Marker for tile sprite entities.
#[derive(Component)]
pub struct TileSprite;

/// Spawn a sprite for each tile in the map.
pub fn spawn_tilemap(mut commands: Commands, map_res: Res<MapResource>) {
    let map = &map_res.map;

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);

            let color = terrain_color(tile.terrain, x, y);

            // Elevation visual offset: higher tiles shift up on screen
            let elevation_offset = tile.elevation as f32 * 8.0;

            commands.spawn((
                TileSprite,
                Sprite {
                    color,
                    // Diamond shape approximated by a rotated rectangle
                    custom_size: Some(Vec2::new(
                        TILE_HALF_WIDTH * 2.0 - 1.0,
                        TILE_HALF_HEIGHT * 2.0 - 1.0,
                    )),
                    ..default()
                },
                // Note: screen Y is inverted (Bevy Y is up, isometric Y goes down)
                Transform {
                    translation: Vec3::new(
                        screen.x,
                        -screen.y + elevation_offset,
                        -10.0,
                    ),
                    // Rotate 45 degrees to create diamond shape
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    scale: Vec3::new(0.71, 0.71, 1.0), // sqrt(2)/2 to fit diamond into tile bounds
                    ..default()
                },
            ));
        }
    }
}

/// Map terrain type to a placeholder color for rendering.
fn terrain_color(terrain: TerrainType, x: i32, y: i32) -> Color {
    let checker = (x + y) % 2 == 0;
    match terrain {
        TerrainType::Grass => {
            if checker {
                Color::srgb(0.28, 0.55, 0.25)
            } else {
                Color::srgb(0.32, 0.58, 0.28)
            }
        }
        TerrainType::Dirt => {
            if checker {
                Color::srgb(0.55, 0.42, 0.28)
            } else {
                Color::srgb(0.58, 0.45, 0.30)
            }
        }
        TerrainType::Sand => Color::srgb(0.85, 0.78, 0.55),
        TerrainType::Forest => {
            if checker {
                Color::srgb(0.18, 0.42, 0.15)
            } else {
                Color::srgb(0.22, 0.45, 0.18)
            }
        }
        TerrainType::Water => {
            if checker {
                Color::srgb(0.15, 0.35, 0.65)
            } else {
                Color::srgb(0.18, 0.38, 0.68)
            }
        }
        TerrainType::Shallows => Color::srgb(0.42, 0.70, 0.90),
        TerrainType::Rock => Color::srgb(0.35, 0.32, 0.30),
        TerrainType::Ramp => Color::srgb(0.50, 0.45, 0.38),
        TerrainType::Road => Color::srgb(0.62, 0.54, 0.42),
        TerrainType::TechRuins => Color::srgb(0.43, 0.43, 0.48),
    }
}
