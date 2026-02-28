use bevy::prelude::*;

use crate::input::PlacementPreview;
use crate::setup::{BuildingMesh, building_color};
use cc_core::components::{Building, Owner, Position};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Spawn visual components for buildings that lack a BuildingMesh marker.
/// (production_system in cc_sim spawns buildings engine-agnostically without visuals)
pub fn spawn_building_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    map_res: Res<MapResource>,
    new_buildings: Query<(Entity, &Building, &Owner, &Position), Without<BuildingMesh>>,
) {
    for (entity, _building, owner, pos) in new_buildings.iter() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elev = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;

        let mesh = meshes.add(Rectangle::new(28.0, 28.0));
        let mat = materials.add(ColorMaterial::from_color(building_color(owner.player_id)));

        commands.entity(entity).insert((
            BuildingMesh,
            Mesh2d(mesh),
            MeshMaterial2d(mat),
            Transform::from_xyz(screen.x, -screen.y + elev, depth_z(pos.world) - 0.05),
        ));
    }
}

/// Sync building transforms from their simulation Position.
pub fn sync_building_sprites(
    map_res: Res<MapResource>,
    mut query: Query<(&Position, &mut Transform), With<BuildingMesh>>,
) {
    for (pos, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elev = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y + elev;
        transform.translation.z = depth_z(pos.world) - 0.05;
    }
}

/// Render a semi-transparent ghost at the cursor grid position during build placement.
pub fn render_placement_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    preview: Res<PlacementPreview>,
    map_res: Res<MapResource>,
    existing_preview: Query<Entity, With<PlacementGhost>>,
) {
    // Despawn old ghost
    for entity in existing_preview.iter() {
        commands.entity(entity).despawn();
    }

    let Some(grid_pos) = preview.grid_pos else {
        return;
    };

    let world = cc_core::coords::WorldPos::from_grid(grid_pos);
    let screen = world_to_screen(world);
    let elev = map_res.map.elevation_at(grid_pos) as f32 * ELEVATION_PIXEL_OFFSET;

    let color = if preview.valid {
        Color::srgba(0.2, 0.8, 0.2, 0.4) // Green ghost
    } else {
        Color::srgba(0.8, 0.2, 0.2, 0.4) // Red ghost
    };

    let mesh = meshes.add(Rectangle::new(28.0, 28.0));
    let mat = materials.add(ColorMaterial::from_color(color));

    commands.spawn((
        PlacementGhost,
        Mesh2d(mesh),
        MeshMaterial2d(mat),
        Transform::from_xyz(screen.x, -screen.y + elev, depth_z(world) + 0.5),
    ));
}

/// Marker for the placement preview ghost entity.
#[derive(Component)]
pub struct PlacementGhost;
