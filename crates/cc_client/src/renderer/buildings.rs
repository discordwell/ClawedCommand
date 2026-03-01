use bevy::prelude::*;

use crate::input::PlacementPreview;
use crate::setup::{BuildingMesh, building_color};
use cc_core::components::{Building, BuildingKind, Owner, Position, UnderConstruction};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_core::tuning::BUILDING_SPRITE_SIZE;
use cc_sim::resources::MapResource;

// ---------------------------------------------------------------------------
// Construction bar components
// ---------------------------------------------------------------------------

/// Marker added to a building once its construction bar has been spawned.
#[derive(Component)]
pub struct HasConstructionBar;

/// Marker for construction bar foreground sprite.
#[derive(Component)]
pub struct ConstructionBarFg;

/// Marker for construction bar background sprite.
#[derive(Component)]
pub struct ConstructionBarBg;

const CONSTRUCTION_BAR_HEIGHT: f32 = 4.0;
const CONSTRUCTION_BAR_Y_OFFSET: f32 = 22.0;

/// Bar width for buildings by kind.
fn construction_bar_width(kind: BuildingKind) -> f32 {
    match kind {
        BuildingKind::TheBox => 30.0,
        BuildingKind::CatTree | BuildingKind::ServerRack => BUILDING_SPRITE_SIZE,
        BuildingKind::FishMarket | BuildingKind::ScratchingPost | BuildingKind::CatFlap => 24.0,
        BuildingKind::LitterBox | BuildingKind::LaserPointer => 20.0,
        _ => 24.0,
    }
}

// ---------------------------------------------------------------------------
// Existing systems
// ---------------------------------------------------------------------------

/// Spawn visual components for buildings that lack a BuildingMesh marker.
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

        let mesh = meshes.add(Rectangle::new(BUILDING_SPRITE_SIZE, BUILDING_SPRITE_SIZE));
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

    let mesh = meshes.add(Rectangle::new(BUILDING_SPRITE_SIZE, BUILDING_SPRITE_SIZE));
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

// ---------------------------------------------------------------------------
// Construction progress bar
// ---------------------------------------------------------------------------

/// Spawn construction bar child entities for buildings under construction.
pub fn spawn_construction_bars(
    mut commands: Commands,
    buildings: Query<
        (Entity, &Building),
        (With<UnderConstruction>, With<BuildingMesh>, Without<HasConstructionBar>),
    >,
) {
    for (entity, building) in buildings.iter() {
        let bar_width = construction_bar_width(building.kind);

        // Background (dark)
        let bg = commands
            .spawn((
                ConstructionBarBg,
                Sprite {
                    color: Color::srgba(0.1, 0.1, 0.1, 0.8),
                    custom_size: Some(Vec2::new(bar_width, CONSTRUCTION_BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_xyz(0.0, CONSTRUCTION_BAR_Y_OFFSET, 0.3),
            ))
            .id();

        // Foreground (yellow/orange fill)
        let fg = commands
            .spawn((
                ConstructionBarFg,
                Sprite {
                    color: Color::srgb(0.9, 0.7, 0.1),
                    custom_size: Some(Vec2::new(0.0, CONSTRUCTION_BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_xyz(0.0, CONSTRUCTION_BAR_Y_OFFSET, 0.4),
            ))
            .id();

        commands
            .entity(entity)
            .insert(HasConstructionBar)
            .add_children(&[bg, fg]);
    }
}

/// Update construction bar fill width based on progress.
pub fn update_construction_bars(
    buildings: Query<(&Building, &UnderConstruction, &Children), With<HasConstructionBar>>,
    mut fg_bars: Query<(&mut Sprite, &mut Transform), With<ConstructionBarFg>>,
) {
    for (building, uc, children) in buildings.iter() {
        let bar_width = construction_bar_width(building.kind);
        let progress = uc.progress_f32();
        let fill_width = bar_width * progress;

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = fg_bars.get_mut(child) {
                sprite.custom_size = Some(Vec2::new(fill_width, CONSTRUCTION_BAR_HEIGHT));
                // Left-align the fill bar
                transform.translation.x = (fill_width - bar_width) / 2.0;
            }
        }
    }
}

/// Remove construction bars when building finishes construction.
pub fn remove_construction_bars(
    mut commands: Commands,
    finished: Query<
        (Entity, &Children),
        (With<HasConstructionBar>, Without<UnderConstruction>),
    >,
    bg_bars: Query<Entity, With<ConstructionBarBg>>,
    fg_bars: Query<Entity, With<ConstructionBarFg>>,
) {
    for (entity, children) in finished.iter() {
        for child in children.iter() {
            if bg_bars.get(child).is_ok() || fg_bars.get(child).is_ok() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(entity).remove::<HasConstructionBar>();
    }
}

// ---------------------------------------------------------------------------
// Construction alpha (semi-transparent during build)
// ---------------------------------------------------------------------------

/// Make buildings semi-transparent while under construction, solid when complete.
pub fn update_construction_alpha(
    buildings: Query<
        (Option<&UnderConstruction>, &MeshMaterial2d<ColorMaterial>),
        With<BuildingMesh>,
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (uc, mat_handle) in buildings.iter() {
        let Some(mat) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        if let Some(uc) = uc {
            let progress = uc.progress_f32();
            let alpha = 0.4 + 0.6 * progress;
            mat.color = mat.color.with_alpha(alpha);
        } else {
            // Ensure fully opaque when not under construction
            let current_alpha = mat.color.alpha();
            if current_alpha < 1.0 {
                mat.color = mat.color.with_alpha(1.0);
            }
        }
    }
}
