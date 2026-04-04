use bevy::prelude::*;

use cc_core::coords::{GridPos, WorldPos, depth_z, world_to_screen};
use cc_core::map::GameMap;
use cc_core::terrain::{ELEVATION_PIXEL_OFFSET, TerrainType};
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
                    if hash.is_multiple_of(3) {
                        let mat = if hash.is_multiple_of(2) {
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
                    if hash.is_multiple_of(5) {
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

// ---------------------------------------------------------------------------
// Wall props — isometric wall faces on DryWall tiles
// ---------------------------------------------------------------------------

/// Which isometric wall faces are visible on a DryWall tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WallFaces {
    /// Both south-east and south-west faces visible (exposed corner).
    Both,
    /// Only south-east face visible (wall runs along NE-SW axis).
    SouthEast,
    /// Only south-west face visible (wall runs along NW-SE axis).
    SouthWest,
    /// No visible faces — interior wall, just show the top cap.
    Top,
}

/// Check if a neighboring tile is "open" (not DryWall/Rock — shows a wall face).
fn is_open(map: &GameMap, x: i32, y: i32) -> bool {
    map.terrain_at(GridPos::new(x, y))
        .map(|t| !matches!(t, TerrainType::DryWall | TerrainType::Rock))
        .unwrap_or(false) // out-of-bounds = wall (no face shown)
}

/// Determine which wall faces to show based on neighbors.
/// In isometric view, "south" on screen = +x,+y in grid.
/// The two visible faces in iso are:
///   - South-East face: visible when the tile at (x+1, y) is open
///   - South-West face: visible when the tile at (x, y+1) is open
fn wall_faces(map: &GameMap, x: i32, y: i32) -> WallFaces {
    let se_open = is_open(map, x + 1, y); // right neighbor in grid = SE face in iso
    let sw_open = is_open(map, x, y + 1); // bottom neighbor in grid = SW face in iso

    match (se_open, sw_open) {
        (true, true) => WallFaces::Both,
        (true, false) => WallFaces::SouthEast,
        (false, true) => WallFaces::SouthWest,
        (false, false) => WallFaces::Top,
    }
}

/// Spawn isometric wall prop sprites on DryWall tiles.
/// Uses generated art sprites if available, falls back to procedural meshes.
pub fn spawn_wall_props(
    mut commands: Commands,
    map_res: Res<MapResource>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let map = &map_res.map;

    // Try loading wall sprites
    let wall_both = try_load_wall_sprite(&asset_server, "sprites/dream/wall_both.png");
    let wall_se = try_load_wall_sprite(&asset_server, "sprites/dream/wall_se.png");
    let wall_sw = try_load_wall_sprite(&asset_server, "sprites/dream/wall_sw.png");
    let wall_top = try_load_wall_sprite(&asset_server, "sprites/dream/wall_top.png");

    // Fallback materials for procedural walls
    let wall_face_se_mat =
        materials.add(ColorMaterial::from_color(Color::srgb(0.78, 0.76, 0.72)));
    let wall_face_sw_mat =
        materials.add(ColorMaterial::from_color(Color::srgb(0.68, 0.66, 0.62)));
    let wall_top_mat =
        materials.add(ColorMaterial::from_color(Color::srgb(0.85, 0.83, 0.80)));
    let wall_face_mesh = meshes.add(Rectangle::new(14.0, 12.0));
    let wall_top_mesh = meshes.add(Rectangle::new(14.0, 4.0));

    // Wall height in screen pixels
    let wall_height: f32 = 12.0;

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let grid = GridPos::new(x, y);
            let tile = map.get(grid).unwrap();
            if tile.terrain != TerrainType::DryWall {
                continue;
            }

            let faces = wall_faces(map, x, y);
            let world = WorldPos::from_grid(grid);
            let screen = world_to_screen(world);
            let elev_offset = tile.elevation as f32 * ELEVATION_PIXEL_OFFSET;
            let base_z = depth_z(world) - 2.0; // between tiles and unit props

            let sx = screen.x;
            let sy = -screen.y + elev_offset;

            // Try sprite-based walls first
            let sprite_handle = match faces {
                WallFaces::Both => wall_both.clone(),
                WallFaces::SouthEast => wall_se.clone(),
                WallFaces::SouthWest => wall_sw.clone(),
                WallFaces::Top => wall_top.clone(),
            };

            if let Some(handle) = sprite_handle {
                commands.spawn((
                    Prop,
                    Sprite {
                        image: handle,
                        ..default()
                    },
                    Transform::from_xyz(sx, sy + wall_height / 2.0, base_z)
                        .with_scale(Vec3::splat(0.5)),
                ));
            } else {
                // Procedural fallback: colored rectangles for wall faces
                match faces {
                    WallFaces::Both => {
                        // SE face (lighter, left-facing)
                        commands.spawn((
                            Prop,
                            Mesh2d(wall_face_mesh.clone()),
                            MeshMaterial2d(wall_face_se_mat.clone()),
                            Transform::from_xyz(sx + 3.0, sy + wall_height / 2.0, base_z),
                        ));
                        // SW face (darker, right-facing)
                        commands.spawn((
                            Prop,
                            Mesh2d(wall_face_mesh.clone()),
                            MeshMaterial2d(wall_face_sw_mat.clone()),
                            Transform::from_xyz(sx - 3.0, sy + wall_height / 2.0, base_z + 0.001),
                        ));
                    }
                    WallFaces::SouthEast => {
                        commands.spawn((
                            Prop,
                            Mesh2d(wall_face_mesh.clone()),
                            MeshMaterial2d(wall_face_se_mat.clone()),
                            Transform::from_xyz(sx, sy + wall_height / 2.0, base_z),
                        ));
                    }
                    WallFaces::SouthWest => {
                        commands.spawn((
                            Prop,
                            Mesh2d(wall_face_mesh.clone()),
                            MeshMaterial2d(wall_face_sw_mat.clone()),
                            Transform::from_xyz(sx, sy + wall_height / 2.0, base_z),
                        ));
                    }
                    WallFaces::Top => {
                        // Just a thin cap on top
                        commands.spawn((
                            Prop,
                            Mesh2d(wall_top_mesh.clone()),
                            MeshMaterial2d(wall_top_mat.clone()),
                            Transform::from_xyz(sx, sy + wall_height, base_z),
                        ));
                    }
                }
            }
        }
    }
}

fn try_load_wall_sprite(asset_server: &AssetServer, path: &'static str) -> Option<Handle<Image>> {
    if super::asset_exists_on_disk(path) {
        Some(asset_server.load(path))
    } else {
        None
    }
}
