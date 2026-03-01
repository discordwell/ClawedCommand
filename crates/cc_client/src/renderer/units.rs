use bevy::prelude::*;

use crate::renderer::animation::{AnimIndices, AnimState, AnimTimer, PrevAnimState};
use crate::renderer::hero_sprites::HeroSprites;
use crate::setup::{TeamMaterials, UnitMesh, team_color, unit_scale};
use crate::renderer::unit_gen::{UnitSprites, kind_index};
use crate::renderer::zoom_lod::{self, ZoomTier};
use cc_core::components::{HeroIdentity, Owner, Position, UnitType};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Sync unit mesh Transform positions from their simulation Position each frame.
pub fn sync_unit_sprites(
    map_res: Res<MapResource>,
    mut query: Query<(&Position, &mut Transform), With<UnitMesh>>,
) {
    for (pos, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y + elevation_offset;
        transform.translation.z = depth_z(pos.world);
    }
}

/// Spawn visual components for units produced by production_system (which lacks client visuals).
/// Detects entities with UnitType but without UnitMesh and gives them renderable components.
pub fn spawn_unit_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    team_mats: Option<Res<TeamMaterials>>,
    unit_sprites: Option<Res<UnitSprites>>,
    hero_sprites: Option<Res<HeroSprites>>,
    tier: Res<ZoomTier>,
    map_res: Res<MapResource>,
    new_units: Query<(Entity, &UnitType, &Owner, &Position, Option<&HeroIdentity>), Without<UnitMesh>>,
) {
    for (entity, unit_type, owner, pos, hero_identity) in new_units.iter() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elev = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        let art_loaded = unit_sprites.as_ref().map_or(false, |s| s.art_loaded);
        let scale = unit_scale(unit_type.kind, art_loaded);
        let tint = team_color(owner.player_id);

        if let Some(ref sprites) = unit_sprites {
            // Use hero sprite if available, otherwise fall back to unit kind sprite
            let image = hero_identity
                .and_then(|hi| hero_sprites.as_ref()?.sprites.get(&hi.hero_id).cloned())
                .unwrap_or_else(|| sprites.sprites[kind_index(unit_type.kind)].clone());
            commands.entity(entity).insert((
                UnitMesh,
                Sprite {
                    image,
                    color: tint,
                    ..default()
                },
                Transform::from_xyz(screen.x, -screen.y + elev, depth_z(pos.world))
                    .with_scale(Vec3::splat(scale)),
                AnimState::default(),
                PrevAnimState::default(),
                AnimIndices::default(),
                AnimTimer::default(),
            ));
        } else if let Some(ref team_mats) = team_mats {
            // Fallback: colored circle mesh
            let body_mesh = meshes.add(Circle::new(12.0));
            let body_mat = if owner.player_id == 0 {
                team_mats.player.clone()
            } else {
                team_mats.enemy.clone()
            };
            commands.entity(entity).insert((
                UnitMesh,
                Mesh2d(body_mesh),
                MeshMaterial2d(body_mat),
                Transform::from_xyz(screen.x, -screen.y + elev, depth_z(pos.world))
                    .with_scale(Vec3::splat(scale)),
            ));
        }

        // Spawn strategic zoom icon as child (hidden unless in Strategic tier)
        zoom_lod::spawn_strategic_icon(
            &mut commands, &mut meshes, &mut materials,
            entity, scale, tint, &tier,
        );
    }
}
