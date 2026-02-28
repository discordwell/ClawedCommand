use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, world_to_screen, TILE_HALF_HEIGHT, TILE_HALF_WIDTH};
use cc_core::terrain::TerrainType;
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Marker for tile mesh entities.
#[derive(Component)]
pub struct TileSprite;

/// Marker for water tiles (used by water animation system).
#[derive(Component)]
pub struct WaterTile;

/// Marker for cliff shadow overlays.
#[derive(Component)]
pub struct CliffShadow;

/// Shared material handles for water animation (mutated each frame).
#[derive(Resource)]
pub struct WaterMaterials {
    pub water_a: Handle<ColorMaterial>,
    pub water_b: Handle<ColorMaterial>,
    pub shallows_a: Handle<ColorMaterial>,
    pub shallows_b: Handle<ColorMaterial>,
}

/// Spawn a Mesh2d rhombus for each tile in the map.
pub fn spawn_tilemap(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;

    // One shared rhombus mesh for all tiles
    let tile_mesh: Handle<Mesh> =
        meshes.add(Rhombus::new(TILE_HALF_WIDTH * 2.0, TILE_HALF_HEIGHT * 2.0));

    // Pre-create shared materials: 10 terrain types x 3 elevations x 2 checkerboard = 60 variants
    // But many combos are sparse, so we use a HashMap-like approach with on-demand creation.
    // For simplicity and perf, pre-create the 20 base materials (10 terrains x 2 checker)
    // and apply elevation brightness per-material. With 3 elevations that's 60 materials max.
    use std::collections::HashMap;
    let mut material_cache: HashMap<(u8, u8, bool), Handle<ColorMaterial>> = HashMap::new();

    // Pre-create water materials for animation
    let water_a = materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
        TerrainType::Water,
        1,
        0,
        0,
    )));
    let water_b = materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
        TerrainType::Water,
        1,
        1,
        0,
    )));
    let shallows_a = materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
        TerrainType::Shallows,
        1,
        0,
        0,
    )));
    let shallows_b = materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
        TerrainType::Shallows,
        1,
        1,
        0,
    )));

    commands.insert_resource(WaterMaterials {
        water_a: water_a.clone(),
        water_b: water_b.clone(),
        shallows_a: shallows_a.clone(),
        shallows_b: shallows_b.clone(),
    });

    // Shared shadow material for cliff edges
    let shadow_material = materials.add(ColorMaterial::from_color(Color::srgba(0.0, 0.0, 0.0, 0.25)));

    // First pass: spawn tiles
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elevation_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;

            let is_water = tile.terrain == TerrainType::Water;
            let is_shallows = tile.terrain == TerrainType::Shallows;
            let checker = (x + y) % 2 == 0;

            // Use shared water materials or cached terrain materials
            let mat_handle = if is_water {
                if checker { water_a.clone() } else { water_b.clone() }
            } else if is_shallows {
                if checker { shallows_a.clone() } else { shallows_b.clone() }
            } else {
                let key = (tile.terrain as u8, tile.elevation, checker);
                material_cache
                    .entry(key)
                    .or_insert_with(|| {
                        materials.add(ColorMaterial::from_color(terrain_color_with_elevation(
                            tile.terrain,
                            tile.elevation,
                            x,
                            y,
                        )))
                    })
                    .clone()
            };

            let mut entity_commands = commands.spawn((
                TileSprite,
                Mesh2d(tile_mesh.clone()),
                MeshMaterial2d(mat_handle),
                Transform::from_xyz(screen.x, -screen.y + elevation_offset, -10.0),
            ));

            if is_water || is_shallows {
                entity_commands.insert(WaterTile);
            }
        }
    }

    // Second pass: cliff edge shadows
    // For each tile, check south (0,+1) and east (+1,0) neighbors.
    // If neighbor has lower elevation, spawn shadow overlay on the neighbor.
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
}

/// Map terrain type to a color with elevation-based brightness.
fn terrain_color_with_elevation(terrain: TerrainType, elevation: u8, x: i32, y: i32) -> Color {
    let base = terrain_base_color(terrain, x, y);

    // Brightness multiplier: elev 0 = 0.82, elev 1 = 1.0, elev 2 = 1.15
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
        TerrainType::Sand => {
            if checker {
                Color::srgb(0.83, 0.76, 0.53)
            } else {
                Color::srgb(0.85, 0.78, 0.55)
            }
        }
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
        TerrainType::Shallows => {
            if checker {
                Color::srgb(0.40, 0.68, 0.88)
            } else {
                Color::srgb(0.42, 0.70, 0.90)
            }
        }
        TerrainType::Rock => {
            if checker {
                Color::srgb(0.33, 0.30, 0.28)
            } else {
                Color::srgb(0.35, 0.32, 0.30)
            }
        }
        TerrainType::Ramp => {
            if checker {
                Color::srgb(0.48, 0.43, 0.36)
            } else {
                Color::srgb(0.50, 0.45, 0.38)
            }
        }
        TerrainType::Road => {
            if checker {
                Color::srgb(0.60, 0.52, 0.40)
            } else {
                Color::srgb(0.62, 0.54, 0.42)
            }
        }
        TerrainType::TechRuins => {
            if checker {
                Color::srgb(0.41, 0.41, 0.46)
            } else {
                Color::srgb(0.43, 0.43, 0.48)
            }
        }
    }
}
