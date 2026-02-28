use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, world_to_screen, TILE_HALF_HEIGHT, TILE_HALF_WIDTH};
use cc_core::terrain::TerrainType;
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

use super::tile_gen::ProceduralTiles;

/// Marker for tile entities.
#[derive(Component)]
pub struct TileSprite;

/// Marker for water tiles (used by water animation system).
#[derive(Component)]
pub struct WaterTile {
    /// Which terrain variant (Water vs Shallows).
    pub is_shallows: bool,
    /// Whether currently showing the alt image.
    pub showing_alt: bool,
}

/// Marker for cliff shadow overlays.
#[derive(Component)]
pub struct CliffShadow;

/// Spawn a sprite for each tile using procedurally generated terrain images.
pub fn spawn_tilemap(
    mut commands: Commands,
    map_res: Res<MapResource>,
    tiles: Option<Res<ProceduralTiles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;

    if let Some(tiles) = tiles {
        // Use procedural tile images
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let grid = GridPos::new(x, y);
                let tile = map.get(grid).unwrap();
                let world = WorldPos::from_grid(grid);
                let screen = world_to_screen(world);
                let elevation_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

                let image_handle = tiles.terrain[tile.terrain as usize].clone();

                // Elevation brightness tint
                let brightness = match tile.elevation {
                    0 => 0.82,
                    1 => 1.0,
                    _ => 1.15,
                };
                let tint = Color::srgb(brightness, brightness, brightness);

                let mut entity_commands = commands.spawn((
                    TileSprite,
                    Sprite {
                        image: image_handle,
                        color: tint,
                        ..default()
                    },
                    Transform::from_xyz(screen.x, -screen.y + elevation_offset, -10.0),
                ));

                let is_water = tile.terrain == TerrainType::Water;
                let is_shallows = tile.terrain == TerrainType::Shallows;
                if is_water || is_shallows {
                    entity_commands.insert(WaterTile {
                        is_shallows,
                        showing_alt: false,
                    });
                }
            }
        }

        // Cliff edge shadows using Mesh2d (overlays only)
        let shadow_mesh = meshes.add(Rhombus::new(TILE_HALF_WIDTH * 2.0, TILE_HALF_HEIGHT * 2.0));
        let shadow_material =
            materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.25)));

        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let grid = GridPos::new(x, y);
                let tile = map.get(grid).unwrap();

                for (dx, dy) in [(0, 1), (1, 0)] {
                    let neighbor_grid = GridPos::new(x + dx, y + dy);
                    if let Some(neighbor) = map.get(neighbor_grid) {
                        if neighbor.elevation < tile.elevation {
                            let nworld = WorldPos::from_grid(neighbor_grid);
                            let nscreen = world_to_screen(nworld);
                            let n_elev_offset =
                                neighbor.elevation as f32 * ELEVATION_PIXEL_OFFSET;

                            commands.spawn((
                                CliffShadow,
                                Mesh2d(shadow_mesh.clone()),
                                MeshMaterial2d(shadow_material.clone()),
                                Transform::from_xyz(
                                    nscreen.x,
                                    -nscreen.y + n_elev_offset,
                                    -9.9,
                                ),
                            ));
                        }
                    }
                }
            }
        }
    } else {
        // Fallback: Mesh2d rhombus if tile_gen hasn't run (shouldn't happen)
        spawn_tilemap_fallback(&mut commands, map_res, &mut meshes, &mut materials);
    }
}

/// Fallback tilemap using Mesh2d rhombus (kept for robustness).
fn spawn_tilemap_fallback(
    commands: &mut Commands,
    map_res: Res<MapResource>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;
    let tile_mesh: Handle<Mesh> =
        meshes.add(Rhombus::new(TILE_HALF_WIDTH * 2.0, TILE_HALF_HEIGHT * 2.0));

    let shadow_material =
        materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.25)));

    use std::collections::HashMap;
    let mut material_cache: HashMap<(u8, u8, bool), Handle<ColorMaterial>> = HashMap::new();

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;
            let checker = (x + y) % 2 == 0;

            let key = (tile.terrain as u8, tile.elevation, checker);
            let mat_handle = material_cache
                .entry(key)
                .or_insert_with(|| {
                    materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
                        tile.terrain,
                        tile.elevation,
                        x,
                        y,
                    )))
                })
                .clone();

            commands.spawn((
                TileSprite,
                Mesh2d(tile_mesh.clone()),
                MeshMaterial2d(mat_handle),
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, -10.0),
            ));
        }
    }

    // Cliff shadows
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();

            for (dx, dy) in [(0, 1), (1, 0)] {
                let neighbor_grid = GridPos::new(x + dx, y + dy);
                if let Some(neighbor) = map.get(neighbor_grid) {
                    if neighbor.elevation < tile.elevation {
                        let nworld = WorldPos::from_grid(neighbor_grid);
                        let nscreen = world_to_screen(nworld);
                        let n_elev_offset = neighbor.elevation as f32 * ELEVATION_PIXEL_OFFSET;

                        commands.spawn((
                            CliffShadow,
                            Mesh2d(tile_mesh.clone()),
                            MeshMaterial2d(shadow_material.clone()),
                            Transform::from_xyz(nscreen.x, -nscreen.y + n_elev_offset, -9.9),
                        ));
                    }
                }
            }
        }
    }
}

/// Map terrain type to a color with elevation-based brightness (for fallback).
fn terrain_color_with_elevation(terrain: TerrainType, elevation: u8, x: i32, y: i32) -> Color {
    let base = terrain_base_color(terrain, x, y);

    let brightness = match elevation {
        0 => 0.82,
        1 => 1.0,
        _ => 1.15,
    };

    let linear = base.to_linear();
    Color::LinearRgba(LinearRgba::new(
        (linear.red * brightness).min(1.0),
        (linear.green * brightness).min(1.0),
        (linear.blue * brightness).min(1.0),
        linear.alpha,
    ))
}

/// Base terrain color (checkerboard pattern).
fn terrain_base_color(terrain: TerrainType, x: i32, y: i32) -> Color {
    let checker = (x + y) % 2 == 0;
    match terrain {
        TerrainType::Grass => {
            if checker { Color::srgb(0.28, 0.55, 0.25) } else { Color::srgb(0.32, 0.58, 0.28) }
        }
        TerrainType::Dirt => {
            if checker { Color::srgb(0.55, 0.42, 0.28) } else { Color::srgb(0.58, 0.45, 0.30) }
        }
        TerrainType::Sand => {
            if checker { Color::srgb(0.83, 0.76, 0.53) } else { Color::srgb(0.85, 0.78, 0.55) }
        }
        TerrainType::Forest => {
            if checker { Color::srgb(0.18, 0.42, 0.15) } else { Color::srgb(0.22, 0.45, 0.18) }
        }
        TerrainType::Water => {
            if checker { Color::srgb(0.15, 0.35, 0.65) } else { Color::srgb(0.18, 0.38, 0.68) }
        }
        TerrainType::Shallows => {
            if checker { Color::srgb(0.40, 0.68, 0.88) } else { Color::srgb(0.42, 0.70, 0.90) }
        }
        TerrainType::Rock => {
            if checker { Color::srgb(0.33, 0.30, 0.28) } else { Color::srgb(0.35, 0.32, 0.30) }
        }
        TerrainType::Ramp => {
            if checker { Color::srgb(0.48, 0.43, 0.36) } else { Color::srgb(0.50, 0.45, 0.38) }
        }
        TerrainType::Road => {
            if checker { Color::srgb(0.60, 0.52, 0.40) } else { Color::srgb(0.62, 0.54, 0.42) }
        }
        TerrainType::TechRuins => {
            if checker { Color::srgb(0.41, 0.41, 0.46) } else { Color::srgb(0.43, 0.43, 0.48) }
        }
    }
}
