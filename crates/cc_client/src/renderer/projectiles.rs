use bevy::prelude::*;

use cc_core::components::{Position, Projectile};
use cc_core::coords::{depth_z, world_to_screen};
use cc_core::terrain::ELEVATION_PIXEL_OFFSET;
use cc_sim::resources::MapResource;

/// Marker for projectile sprites (distinguishes from unit sprites).
#[derive(Component)]
pub struct ProjectileSprite;

/// Attach a visual sprite to projectile entities that don't have one yet.
pub fn spawn_projectile_sprites(
    mut commands: Commands,
    projectiles: Query<Entity, (With<Projectile>, Without<ProjectileSprite>)>,
) {
    for entity in projectiles.iter() {
        commands.entity(entity).insert((
            ProjectileSprite,
            Sprite {
                color: Color::srgb(1.0, 0.8, 0.2),
                custom_size: Some(Vec2::new(4.0, 4.0)),
                ..default()
            },
            Transform::default(),
        ));
    }
}

/// Sync projectile Transform positions from their simulation Position.
pub fn sync_projectile_sprites(
    map_res: Res<MapResource>,
    mut query: Query<(&Position, &mut Transform), With<ProjectileSprite>>,
) {
    for (pos, mut transform) in query.iter_mut() {
        let screen = world_to_screen(pos.world);
        let grid = pos.world.to_grid();
        let elevation_offset = map_res.map.elevation_at(grid) as f32 * ELEVATION_PIXEL_OFFSET;
        transform.translation.x = screen.x;
        transform.translation.y = -screen.y + elevation_offset;
        // Projectiles render above units
        transform.translation.z = depth_z(pos.world) + 0.5;
    }
}
