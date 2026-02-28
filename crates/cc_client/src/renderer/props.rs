use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::terrain::{TerrainType, ELEVATION_PIXEL_OFFSET};
use cc_sim::resources::MapResource;

/// Marker for prop entities.
#[derive(Component)]
pub struct Prop;

/// Spawn procedural trees on Forest tiles and rocks on Rock tiles.
pub fn spawn_props(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;

    // Shared meshes
    let trunk_mesh = meshes.add(Rectangle::new(3.0, 6.0));
    let canopy_mesh = meshes.add(Circle::new(5.0));
    let rock_mesh = meshes.add(RegularPolygon::new(6.0, 5));

    // Shared materials
    let trunk_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.35, 0.22, 0.12)));
    let canopy_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.15, 0.50, 0.18)));
    let rock_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.40, 0.38, 0.35)));

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();

            // Deterministic pseudo-random offset for slight positional variation
            let offset_x = ((x.wrapping_mul(7).wrapping_add(y.wrapping_mul(13))) % 5) as f32 - 2.0;
            let offset_y = ((x.wrapping_mul(11).wrapping_add(y.wrapping_mul(3))) % 5) as f32 - 2.0;

            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elev_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;
            let base_z = depth_z(world) - 5.0;

            let sx = screen.x + offset_x;
            let sy = -screen.y + elev_offset + offset_y;

            match tile.terrain {
                TerrainType::Forest => {
                    // Trunk
                    commands.spawn((
                        Prop,
                        Mesh2d(trunk_mesh.clone()),
                        MeshMaterial2d(trunk_mat.clone()),
                        Transform::from_xyz(sx, sy, base_z),
                    ));
                    // Canopy (above trunk)
                    commands.spawn((
                        Prop,
                        Mesh2d(canopy_mesh.clone()),
                        MeshMaterial2d(canopy_mat.clone()),
                        Transform::from_xyz(sx, sy + 6.0, base_z + 0.01),
                    ));
                }
                TerrainType::Rock => {
                    commands.spawn((
                        Prop,
                        Mesh2d(rock_mesh.clone()),
                        MeshMaterial2d(rock_mat.clone()),
                        Transform::from_xyz(sx, sy, base_z),
                    ));
                }
                _ => {}
            }
        }
    }
}
