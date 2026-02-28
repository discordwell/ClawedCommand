use bevy::prelude::*;

use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map_gen::{self, MapGenParams};
use cc_core::math::Fixed;
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Set up the initial game state: procedurally generated map, camera, starter units.
pub fn setup_game(mut commands: Commands, mut map_res: ResMut<MapResource>) {
    // Generate a 64x64 map with the procedural generator
    let params = MapGenParams {
        width: 64,
        height: 64,
        num_players: 2,
        seed: 42,
        ..Default::default()
    };
    let map_def = map_gen::generate_map(&params);
    let map = map_def.to_game_map();
    map_res.map = map;

    // Spawn camera
    commands.spawn(Camera2d);

    // Spawn player units at spawn points
    for sp in &map_def.spawn_points {
        let base_pos = GridPos::new(sp.pos.0, sp.pos.1);

        // Spawn a cluster of units around each spawn point
        let offsets = [
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (-1, 0),
            (0, -1),
        ];

        for &(dx, dy) in &offsets {
            let grid = GridPos::new(base_pos.x + dx, base_pos.y + dy);

            // Skip impassable positions
            if !map_res.map.is_passable(grid) {
                continue;
            }

            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;

            // Color by player
            let color = if sp.player == 0 {
                Color::srgb(0.2, 0.4, 0.9) // Blue
            } else {
                Color::srgb(0.9, 0.2, 0.2) // Red
            };

            commands.spawn((
                // Core simulation components
                Position { world },
                Velocity::zero(),
                GridCell { pos: grid },
                Owner {
                    player_id: sp.player,
                },
                UnitType {
                    kind: UnitKind::Nuisance,
                },
                Health {
                    current: Fixed::from_num(100),
                    max: Fixed::from_num(100),
                },
                MovementSpeed {
                    speed: Fixed::from_num(0.15f32),
                },
                // Rendering: colored rectangle as placeholder sprite
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(20.0, 20.0)),
                    ..default()
                },
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, depth_z(world)),
            ));
        }
    }
}
