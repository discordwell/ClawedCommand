use bevy::prelude::*;

use crate::setup::UnitMesh;
use cc_core::components::Dead;

/// Timer tracking how long a unit has been dead (for client-side fade).
/// Stores (elapsed_time, original_scale).
#[derive(Component)]
pub struct DeathTimer(pub f32, pub f32);

const FADE_DURATION: f32 = 0.8;

/// When a unit is first marked Dead, prepare for death animation.
/// For Mesh2d units: clone shared material so fade doesn't affect living units.
/// For all units: insert a DeathTimer with original scale.
pub fn isolate_dead_material(
    mut commands: Commands,
    mesh_query: Query<
        (Entity, Option<&MeshMaterial2d<ColorMaterial>>, &Transform),
        (Added<Dead>, With<UnitMesh>, Without<DeathTimer>),
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, mesh_mat, transform) in mesh_query.iter() {
        let original_scale = transform.scale.x;
        let mut ecmds = commands.entity(entity);
        ecmds.insert(DeathTimer(0.0, original_scale));

        // Clone material for Mesh2d units so fade doesn't affect living units
        if let Some(mat) = mesh_mat {
            if let Some(existing) = materials.get(&mat.0).cloned() {
                let cloned = materials.add(existing);
                ecmds.insert(MeshMaterial2d(cloned));
            }
        }
    }
}

/// Enhanced death effect: fade, shrink, red tint lerp. Despawns when done.
pub fn death_fade_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &mut DeathTimer,
            &mut Transform,
            Option<&mut Sprite>,
            Option<&MeshMaterial2d<ColorMaterial>>,
        ),
        (With<Dead>, With<UnitMesh>),
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let dt = time.delta_secs();
    for (entity, mut timer, mut transform, sprite, mesh_mat) in query.iter_mut() {
        timer.0 += dt;
        let progress = (timer.0 / FADE_DURATION).clamp(0.0, 1.0);
        let alpha = 1.0 - progress;

        // Shrink toward 0.5× original size using stored original scale
        let shrink = 1.0 - progress * 0.5;
        transform.scale = Vec3::splat(timer.1 * shrink);

        if let Some(mut sprite) = sprite {
            // Sprite-based: tint toward red and fade alpha
            let linear = sprite.color.to_linear();
            let red_shift = progress * 0.4;
            sprite.color = Color::LinearRgba(LinearRgba::new(
                (linear.red + red_shift).min(1.0),
                (linear.green * (1.0 - progress * 0.5)).max(0.0),
                (linear.blue * (1.0 - progress * 0.5)).max(0.0),
                alpha,
            ));
        } else if let Some(mat_handle) = mesh_mat {
            // Mesh2d fallback: fade material alpha
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.color.set_alpha(alpha);
            }
        }

        if timer.0 >= FADE_DURATION {
            commands.entity(entity).despawn();
        }
    }
}
