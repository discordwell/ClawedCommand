use bevy::prelude::*;

use crate::input::{InputMode, PlacementPreview};
use crate::renderer::building_anim_assets::BuildingAnimSheets;
use crate::renderer::building_gen::{
    BuildingRole, BuildingSprites, building_kind_index, building_role, building_scale,
};
use crate::setup::{BuildingMesh, building_color, team_color};
use cc_core::components::{Building, BuildingKind, Health, Owner, Position, UnderConstruction};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

// ---------------------------------------------------------------------------
// Building animation components
// ---------------------------------------------------------------------------

/// Animation phase for a building.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BuildingAnimState {
    /// Progress-driven construction frames (tied to UnderConstruction::progress_f32()).
    Constructing,
    /// Timer-driven looping ambient animation after construction completes.
    AmbientIdle,
    /// No animation sheet available — static idle sprite.
    Static,
}

/// Timer for ambient idle loop (0.6s per frame, repeating).
/// Not used during construction — that's progress-driven.
#[derive(Component, Deref, DerefMut)]
pub struct BuildingAnimTimer(pub Timer);

impl Default for BuildingAnimTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            BUILDING_AMBIENT_FRAME_SECS,
            TimerMode::Repeating,
        ))
    }
}

/// Marker: building animation has been initialized.
#[derive(Component)]
pub struct BuildingAnimInit;

/// Seconds per frame for ambient building animations.
const BUILDING_AMBIENT_FRAME_SECS: f32 = 0.6;

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

/// Marker for buildings that use Sprite-based rendering (vs MeshMaterial2d fallback).
#[derive(Component)]
pub struct SpriteBuilding;

const CONSTRUCTION_BAR_HEIGHT: f32 = 4.0;
const CONSTRUCTION_BAR_Y_OFFSET: f32 = 22.0;

/// Bar width for buildings by role (works for all factions).
fn construction_bar_width(kind: BuildingKind) -> f32 {
    match building_role(kind) {
        BuildingRole::Hq => 30.0,
        BuildingRole::Barracks | BuildingRole::TechBuilding => super::BUILDING_SPRITE_SIZE,
        BuildingRole::ResourceDepot | BuildingRole::Garrison | BuildingRole::Research => 24.0,
        BuildingRole::SupplyDepot | BuildingRole::DefenseTower => 20.0,
    }
}

// ---------------------------------------------------------------------------
// Spawn / sync building visuals
// ---------------------------------------------------------------------------

/// Spawn visual components for buildings that lack a BuildingMesh marker.
/// Uses sprite-based rendering when BuildingSprites is available, Mesh2d fallback otherwise.
pub fn spawn_building_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    map_res: Res<MapResource>,
    building_sprites: Option<Res<BuildingSprites>>,
    new_buildings: Query<(Entity, &Building, &Owner, &Position), Without<BuildingMesh>>,
) {
    for (entity, building, owner, pos) in new_buildings.iter() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elev = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        let z = depth_z(pos.world) - 0.05;

        if let Some(ref sprites) = building_sprites {
            let idx = building_kind_index(building.kind);
            let image = sprites.sprites[idx].clone();
            let has_art = sprites.has_art.get(idx).copied().unwrap_or(false);
            let scale = building_scale(building.kind, has_art);
            let tint = team_color(owner.player_id);

            commands.entity(entity).insert((
                BuildingMesh,
                SpriteBuilding,
                Sprite {
                    image,
                    color: tint,
                    ..default()
                },
                Transform::from_xyz(screen.x, -screen.y + elev, z).with_scale(Vec3::splat(scale)),
            ));
        } else {
            // Fallback: colored rectangle mesh
            let mesh = meshes.add(Rectangle::new(
                super::BUILDING_SPRITE_SIZE,
                super::BUILDING_SPRITE_SIZE,
            ));
            let mat = materials.add(ColorMaterial::from_color(building_color(owner.player_id)));

            commands.entity(entity).insert((
                BuildingMesh,
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(screen.x, -screen.y + elev, z),
            ));
        }
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
/// Uses the building sprite as a ghost when BuildingSprites is available.
pub fn render_placement_preview(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    preview: Res<PlacementPreview>,
    input_mode: Res<InputMode>,
    map_res: Res<MapResource>,
    building_sprites: Option<Res<BuildingSprites>>,
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

    // Determine ghost color based on placement validity
    let ghost_alpha = 0.4;
    let (gr, gg, gb) = if preview.valid {
        (0.2, 0.8, 0.2) // green
    } else {
        (0.8, 0.2, 0.2) // red
    };

    // Try to use the building sprite for the ghost when in BuildPlacement mode
    let placement_kind = match *input_mode {
        InputMode::BuildPlacement { kind } => Some(kind),
        _ => None,
    };

    if let (Some(kind), Some(sprites)) = (placement_kind, &building_sprites) {
        let idx = building_kind_index(kind);
        let image = sprites.sprites[idx].clone();
        let has_art = sprites.has_art.get(idx).copied().unwrap_or(false);
        let scale = building_scale(kind, has_art);

        commands.spawn((
            PlacementGhost,
            Sprite {
                image,
                color: Color::srgba(gr, gg, gb, ghost_alpha),
                ..default()
            },
            Transform::from_xyz(screen.x, -screen.y + elev, depth_z(world) + 0.5)
                .with_scale(Vec3::splat(scale)),
        ));
    } else {
        // Fallback: colored rectangle mesh
        let color = Color::srgba(gr, gg, gb, ghost_alpha);
        let mesh = meshes.add(Rectangle::new(
            super::BUILDING_SPRITE_SIZE,
            super::BUILDING_SPRITE_SIZE,
        ));
        let mat = materials.add(ColorMaterial::from_color(color));

        commands.spawn((
            PlacementGhost,
            Mesh2d(mesh),
            MeshMaterial2d(mat),
            Transform::from_xyz(screen.x, -screen.y + elev, depth_z(world) + 0.5),
        ));
    }
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
        (
            With<UnderConstruction>,
            With<BuildingMesh>,
            Without<HasConstructionBar>,
        ),
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
    finished: Query<(Entity, &Children), (With<HasConstructionBar>, Without<UnderConstruction>)>,
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

/// Construction alpha for Sprite-based buildings — modify sprite.color alpha.
pub fn update_construction_alpha_sprite(
    mut buildings: Query<
        (Option<&UnderConstruction>, &mut Sprite, &Owner),
        (With<BuildingMesh>, With<SpriteBuilding>),
    >,
) {
    for (uc, mut sprite, owner) in buildings.iter_mut() {
        let base_tint = team_color(owner.player_id);
        if let Some(uc) = uc {
            let alpha = 0.4 + 0.6 * uc.progress_f32();
            sprite.color = base_tint.with_alpha(alpha);
        } else if sprite.color.alpha() < 1.0 {
            sprite.color = base_tint.with_alpha(1.0);
        }
    }
}

/// Construction alpha for MeshMaterial2d-based buildings (legacy fallback).
pub fn update_construction_alpha_mesh(
    buildings: Query<
        (Option<&UnderConstruction>, &MeshMaterial2d<ColorMaterial>),
        (With<BuildingMesh>, Without<SpriteBuilding>),
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (uc, mat_handle) in buildings.iter() {
        let Some(mat) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        if let Some(uc) = uc {
            let alpha = 0.4 + 0.6 * uc.progress_f32();
            mat.color = mat.color.with_alpha(alpha);
        } else {
            let current_alpha = mat.color.alpha();
            if current_alpha < 1.0 {
                mat.color = mat.color.with_alpha(1.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Damage tint (darkens + red cast when HP < 50%)
// ---------------------------------------------------------------------------

/// Darken and add red cast to buildings below 50% HP. Only applies to completed buildings.
/// Smoothly scales from normal at 50% HP to maximum damage tint at 0% HP.
pub fn update_building_damage_tint(
    mut sprite_buildings: Query<
        (&Health, &Owner, &mut Sprite),
        (
            With<BuildingMesh>,
            With<SpriteBuilding>,
            Without<UnderConstruction>,
        ),
    >,
    mesh_buildings: Query<
        (&Health, &Owner, &MeshMaterial2d<ColorMaterial>),
        (
            With<BuildingMesh>,
            Without<SpriteBuilding>,
            Without<UnderConstruction>,
        ),
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Sprite-based buildings: modify sprite.color
    for (health, owner, mut sprite) in sprite_buildings.iter_mut() {
        let max_f = health.max.to_num::<f32>();
        if max_f <= 0.0 {
            continue;
        }
        let ratio = health.current.to_num::<f32>() / max_f;
        let base = team_color(owner.player_id);

        if ratio < 0.5 {
            // Damage factor: 0.0 at 50% HP, 1.0 at 0% HP
            let damage = 1.0 - (ratio / 0.5);
            // Darken by up to 40% and add red cast
            let darken = 1.0 - 0.4 * damage;
            let red_boost = 0.3 * damage;
            let base_srgba = base.to_srgba();
            let r = (base_srgba.red * darken + red_boost).min(1.0);
            let g = (base_srgba.green * darken * (1.0 - 0.3 * damage)).min(1.0);
            let b = (base_srgba.blue * darken * (1.0 - 0.3 * damage)).min(1.0);
            sprite.color = Color::srgba(r, g, b, sprite.color.alpha());
        } else {
            // Reset to base team color, preserving alpha
            sprite.color = base.with_alpha(sprite.color.alpha());
        }
    }

    // MeshMaterial2d-based buildings: modify material color
    // Use building_color(owner) as base to avoid compounding tint each frame.
    for (health, owner, mat_handle) in mesh_buildings.iter() {
        let max_f = health.max.to_num::<f32>();
        if max_f <= 0.0 {
            continue;
        }
        let ratio = health.current.to_num::<f32>() / max_f;

        let Some(mat) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        let base = building_color(owner.player_id);
        if ratio < 0.5 {
            let damage = 1.0 - (ratio / 0.5);
            let darken = 1.0 - 0.4 * damage;
            let red_boost = 0.3 * damage;
            let base_srgba = base.to_srgba();
            let r = (base_srgba.red * darken + red_boost).min(1.0);
            let g = (base_srgba.green * darken * (1.0 - 0.3 * damage)).min(1.0);
            let b = (base_srgba.blue * darken * (1.0 - 0.3 * damage)).min(1.0);
            mat.color = Color::srgba(r, g, b, mat.color.alpha());
        } else {
            // Reset to base building color when above 50% HP
            mat.color = base.with_alpha(mat.color.alpha());
        }
    }
}

// ---------------------------------------------------------------------------
// Building animation systems
// ---------------------------------------------------------------------------

/// Initialize building animation state on newly spawned buildings.
/// Attaches BuildingAnimState, BuildingAnimTimer, and optionally a TextureAtlas.
pub fn init_building_anim(
    mut commands: Commands,
    anim_sheets: Option<Res<BuildingAnimSheets>>,
    mut new_buildings: Query<
        (Entity, &Building, Option<&UnderConstruction>, &mut Sprite),
        (
            With<BuildingMesh>,
            With<SpriteBuilding>,
            Without<BuildingAnimInit>,
        ),
    >,
) {
    let Some(sheets) = anim_sheets else { return };

    for (entity, building, uc, mut sprite) in new_buildings.iter_mut() {
        let idx = building_kind_index(building.kind);

        let (state, sheet_opt) = if uc.is_some() {
            // Building is under construction — use construction sheet if available
            if let Some(ref entry) = sheets.construct[idx] {
                (BuildingAnimState::Constructing, Some(entry))
            } else {
                (BuildingAnimState::Static, None)
            }
        } else {
            // Building already complete — use ambient sheet if available
            if let Some(ref entry) = sheets.ambient[idx] {
                (BuildingAnimState::AmbientIdle, Some(entry))
            } else {
                (BuildingAnimState::Static, None)
            }
        };

        commands
            .entity(entity)
            .insert((state, BuildingAnimTimer::default(), BuildingAnimInit));

        if let Some((img, layout)) = sheet_opt {
            sprite.image = img.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: layout.clone(),
                index: 0,
            });
        }
    }
}

/// Advance construction animation frames based on build progress.
/// Frame index = floor(progress * 3.99) → maps 0.0..1.0 to frames 0..3.
/// Also swaps the sprite image to the construction sheet.
pub fn advance_building_construction_anim(
    anim_sheets: Option<Res<BuildingAnimSheets>>,
    mut buildings: Query<
        (&Building, &UnderConstruction, &mut Sprite),
        (With<BuildingAnimInit>, With<SpriteBuilding>),
    >,
) {
    let Some(sheets) = anim_sheets else { return };

    for (building, uc, mut sprite) in buildings.iter_mut() {
        let idx = building_kind_index(building.kind);
        let Some(ref entry) = sheets.construct[idx] else {
            continue;
        };

        let progress = uc.progress_f32();
        let frame = (progress * 3.99).floor() as usize;

        // Swap to construction sheet image if not already set
        if sprite.image != entry.0 {
            sprite.image = entry.0.clone();
        }

        // Set atlas frame
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = frame;
        } else {
            sprite.texture_atlas = Some(TextureAtlas {
                layout: entry.1.clone(),
                index: frame,
            });
        }
    }
}

/// Transition buildings from Constructing to AmbientIdle (or Static) when
/// UnderConstruction component is removed.
pub fn transition_building_to_ambient(
    anim_sheets: Option<Res<BuildingAnimSheets>>,
    building_sprites: Option<Res<BuildingSprites>>,
    mut finished: Query<
        (
            &Building,
            &mut Sprite,
            &mut BuildingAnimState,
            &mut BuildingAnimTimer,
        ),
        (
            With<BuildingAnimInit>,
            With<SpriteBuilding>,
            Without<UnderConstruction>,
        ),
    >,
) {
    for (building, mut sprite, mut anim_state, mut timer) in finished.iter_mut() {
        // Only transition from Constructing or Static (no construct sheet but might have ambient)
        if *anim_state == BuildingAnimState::AmbientIdle {
            continue;
        }

        let idx = building_kind_index(building.kind);

        // Check for ambient sheet
        let has_ambient = anim_sheets
            .as_ref()
            .and_then(|s| s.ambient[idx].as_ref())
            .is_some();

        if has_ambient {
            let entry = anim_sheets.as_ref().unwrap().ambient[idx].as_ref().unwrap();
            *anim_state = BuildingAnimState::AmbientIdle;
            timer.set_duration(std::time::Duration::from_secs_f32(
                BUILDING_AMBIENT_FRAME_SECS,
            ));
            timer.reset();

            // Swap to ambient sheet
            sprite.image = entry.0.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: entry.1.clone(),
                index: 0,
            });
        } else {
            // No ambient sheet — restore idle sprite and go static
            *anim_state = BuildingAnimState::Static;

            if let Some(ref sprites) = building_sprites {
                sprite.image = sprites.sprites[idx].clone();
            }
            sprite.texture_atlas = None;
        }
    }
}

/// Advance ambient idle animation frames on a timer (0.6s per frame, looping 0→3).
/// Only runs on buildings in AmbientIdle state (not Constructing or Static).
pub fn advance_building_ambient_anim(
    time: Res<Time>,
    mut buildings: Query<
        (&BuildingAnimState, &mut Sprite, &mut BuildingAnimTimer),
        (
            With<BuildingAnimInit>,
            With<SpriteBuilding>,
            Without<UnderConstruction>,
        ),
    >,
) {
    for (anim_state, mut sprite, mut timer) in buildings.iter_mut() {
        if *anim_state != BuildingAnimState::AmbientIdle {
            continue;
        }
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = (atlas.index + 1) % 4;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn building_anim_state_constructing_ne_ambient() {
        assert_ne!(
            BuildingAnimState::Constructing,
            BuildingAnimState::AmbientIdle
        );
        assert_ne!(BuildingAnimState::Constructing, BuildingAnimState::Static);
        assert_ne!(BuildingAnimState::AmbientIdle, BuildingAnimState::Static);
    }

    #[test]
    fn building_anim_timer_default_is_600ms() {
        let timer = BuildingAnimTimer::default();
        let secs = timer.duration().as_secs_f32();
        assert!((secs - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn construction_frame_index_calculation() {
        // Progress 0.0 → frame 0
        assert_eq!((0.0_f32 * 3.99).floor() as usize, 0);
        // Progress 0.25 → frame 0
        assert_eq!((0.25_f32 * 3.99).floor() as usize, 0);
        // Progress 0.26 → frame 1
        assert_eq!((0.26_f32 * 3.99).floor() as usize, 1);
        // Progress 0.5 → frame 1
        assert_eq!((0.5_f32 * 3.99).floor() as usize, 1);
        // Progress 0.51 → frame 2
        assert_eq!((0.51_f32 * 3.99).floor() as usize, 2);
        // Progress 0.75 → frame 2
        assert_eq!((0.75_f32 * 3.99).floor() as usize, 2);
        // Progress 0.76 → frame 3
        assert_eq!((0.76_f32 * 3.99).floor() as usize, 3);
        // Progress 1.0 → frame 3
        assert_eq!((1.0_f32 * 3.99).floor() as usize, 3);
    }

    #[test]
    fn ambient_frame_loops_0_to_3() {
        for i in 0..8 {
            let frame = (i + 1) % 4;
            assert!(frame <= 3);
        }
    }

    #[test]
    fn ambient_frame_rate_constant() {
        assert!((BUILDING_AMBIENT_FRAME_SECS - 0.6).abs() < f32::EPSILON);
    }
}
