use bevy::prelude::*;

use cc_core::components::{Position, Projectile, ProjectileKind, Velocity};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::math::FIXED_ZERO;
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

use crate::renderer::projectile_assets::{ProjectileSprites, kind_color, kind_size, kind_to_index};

/// Marker for projectile sprites (distinguishes from unit sprites).
#[derive(Component)]
pub struct ProjectileSprite;

/// Attach a visual sprite to projectile entities that don't have one yet.
/// Uses per-kind color, size, and optionally a sprite image.
pub fn spawn_projectile_sprites(
    mut commands: Commands,
    proj_sprites: Option<Res<ProjectileSprites>>,
    projectiles: Query<
        (Entity, Option<&ProjectileKind>),
        (With<Projectile>, Without<ProjectileSprite>),
    >,
) {
    for (entity, proj_kind) in projectiles.iter() {
        let kind = proj_kind.copied().unwrap_or_default();
        let color = kind_color(kind);
        let size = kind_size(kind);

        let mut sprite = Sprite {
            color,
            custom_size: Some(size),
            ..default()
        };

        // Use art sprite if available
        if let Some(ref sprites) = proj_sprites {
            sprite.image = sprites.sprites[kind_to_index(kind)].clone();
            // When using a sprite image, don't override custom_size for art-loaded
            // sprites — keep custom_size so the tint color works as overlay
        }

        commands
            .entity(entity)
            .insert((ProjectileSprite, sprite, Transform::default()));
    }
}

/// Sync projectile Transform positions from their simulation Position.
/// Also applies velocity-based rotation so projectiles point in their travel direction.
pub fn sync_projectile_sprites(
    map_res: Res<MapResource>,
    mut query: Query<(&Position, &Velocity, &mut Transform), With<ProjectileSprite>>,
) {
    for (pos, vel, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y + elevation_offset;
        // Projectiles render above units
        transform.translation.z = depth_z(pos.world) + 0.5;

        // Rotate to face travel direction (project velocity to screen space)
        if vel.dx != FIXED_ZERO || vel.dy != FIXED_ZERO {
            let vx: f32 = vel.dx.to_num();
            let vy: f32 = vel.dy.to_num();
            // Isometric projection: screen_x = (dx + dy), screen_y = (dy - dx) / 2
            // Y is negated for screen coords
            let screen_vx = vx + vy;
            let screen_vy = -(vy - vx) * 0.5;
            let angle = screen_vy.atan2(screen_vx);
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}
