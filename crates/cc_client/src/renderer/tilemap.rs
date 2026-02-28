use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, world_to_screen, TILE_HALF_HEIGHT, TILE_HALF_WIDTH};
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

            let color = if tile.passable {
                // Slight variation for visual interest
                if (x + y) % 2 == 0 {
                    Color::srgb(0.28, 0.55, 0.25) // Grass green
                } else {
                    Color::srgb(0.32, 0.58, 0.28) // Slightly lighter green
                }
            } else {
                Color::srgb(0.35, 0.32, 0.30) // Rock/wall gray
            };

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
                    translation: Vec3::new(screen.x, -screen.y, -10.0),
                    // Rotate 45 degrees to create diamond shape
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    scale: Vec3::new(0.71, 0.71, 1.0), // sqrt(2)/2 to fit diamond into tile bounds
                    ..default()
                },
            ));
        }
    }
}
