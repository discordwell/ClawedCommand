use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::terrain::{TerrainType, ELEVATION_PIXEL_OFFSET};
use cc_sim::resources::MapResource;

/// Marker for prop entities.
#[derive(Component)]
pub struct Prop;

/// Spawn procedural props on terrain tiles.
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
    let grass_tuft_mesh = meshes.add(Rectangle::new(2.0, 3.0));
    let sparkle_mesh = meshes.add(Circle::new(1.0));
    let splash_mesh = meshes.add(Circle::new(1.5));

    // Shared materials
    let trunk_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.35, 0.22, 0.12)));
    let canopy_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.15, 0.50, 0.18)));
    let rock_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.40, 0.38, 0.35)));
    let grass_dark_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.22, 0.48, 0.20)));
    let grass_light_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.30, 0.58, 0.25)));
    let sparkle_mat = materials.add(ColorMaterial::from_color(Color::srgba(0.3, 0.8, 0.9, 0.6)));
    let splash_mat = materials.add(ColorMaterial::from_color(Color::srgba(0.7, 0.85, 1.0, 0.4)));

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();

            // Deterministic pseudo-random offset for slight positional variation
            let offset_x = ((x.wrapping_mul(7).wrapping_add(y.wrapping_mul(13))) % 5) as f32 - 2.0;
            let offset_y = ((x.wrapping_mul(11).wrapping_add(y.wrapping_mul(3))) % 5) as f32 - 2.0;
            let hash = (x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17))) as u32;

            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elev_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;
            // Props sit between tiles (z=-10) and units (z~0 to -1.3)
            let base_z = depth_z(world) - 3.0;

            let sx = screen.x + offset_x;
            let sy = -screen.y + elev_offset + offset_y;

            match tile.terrain {
                TerrainType::Forest => {
                    // Tree: trunk + canopy
                    commands.spawn((
                        Prop,
                        Mesh2d(trunk_mesh.clone()),
                        MeshMaterial2d(trunk_mat.clone()),
                        Transform::from_xyz(sx, sy, base_z),
                    ));
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
                TerrainType::Grass => {
                    // Scattered grass tufts (only on ~30% of tiles)
                    if hash % 3 == 0 {
                        let mat = if hash % 2 == 0 {
                            grass_dark_mat.clone()
                        } else {
                            grass_light_mat.clone()
                        };
                        commands.spawn((
                            Prop,
                            Mesh2d(grass_tuft_mesh.clone()),
                            MeshMaterial2d(mat),
                            Transform::from_xyz(sx + 3.0, sy - 2.0, base_z),
                        ));
                    }
                }
                TerrainType::TechRuins => {
                    // Sparkle dots (~40% of tiles)
                    if hash % 5 < 2 {
                        commands.spawn((
                            Prop,
                            Mesh2d(sparkle_mesh.clone()),
                            MeshMaterial2d(sparkle_mat.clone()),
                            Transform::from_xyz(sx + 2.0, sy + 1.0, base_z + 0.02),
                        ));
                    }
                }
                TerrainType::Water => {
                    // Tiny splash near shore tiles (~20% of water tiles)
                    if hash % 5 == 0 {
                        commands.spawn((
                            Prop,
                            Mesh2d(splash_mesh.clone()),
                            MeshMaterial2d(splash_mat.clone()),
                            Transform::from_xyz(sx, sy + 2.0, base_z + 0.01),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}
