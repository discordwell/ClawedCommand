use bevy::prelude::*;

use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_sim::resources::MapResource;

/// Set up the initial game state: map, camera, starter units.
pub fn setup_game(mut commands: Commands, mut map_res: ResMut<MapResource>) {
    // Create a 32x32 map with some impassable terrain
    let mut map = GameMap::new(32, 32);

    // Add some obstacles (a wall and some rocks)
    for y in 8..18 {
        map.get_mut(GridPos::new(15, y)).unwrap().passable = false;
    }
    for x in 10..14 {
        map.get_mut(GridPos::new(x, 20)).unwrap().passable = false;
    }
    // Scatter a few rocks
    for pos in [
        GridPos::new(5, 5),
        GridPos::new(6, 5),
        GridPos::new(22, 10),
        GridPos::new(23, 11),
        GridPos::new(8, 25),
    ] {
        if let Some(tile) = map.get_mut(pos) {
            tile.passable = false;
        }
    }

    map_res.map = map;

    // Spawn camera
    commands.spawn(Camera2d);

    // Spawn player units (blue team, player_id = 0)
    let unit_positions = [
        GridPos::new(3, 3),
        GridPos::new(4, 3),
        GridPos::new(5, 3),
        GridPos::new(3, 4),
        GridPos::new(4, 4),
        GridPos::new(3, 5),
        GridPos::new(4, 5),
        GridPos::new(5, 5), // This one is on a rock — we'll skip it
    ];

    for grid in &unit_positions {
        // Skip positions that are impassable
        if !map_res.map.is_passable(*grid) {
            continue;
        }

        let world = WorldPos::from_grid(*grid);
        let screen = world_to_screen(world);

        commands.spawn((
            // Core simulation components
            Position { world },
            Velocity::zero(),
            GridCell { pos: *grid },
            Owner { player_id: 0 },
            UnitType {
                kind: UnitKind::Infantry,
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
                color: Color::srgb(0.2, 0.4, 0.9), // Blue
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..default()
            },
            Transform::from_xyz(screen.x, -screen.y, depth_z(world)),
        ));
    }
}
