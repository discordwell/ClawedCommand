use bevy::prelude::*;

use cc_core::components::ProjectileKind;
use cc_core::coords::world_to_screen;
use cc_sim::systems::projectile_system::ProjectileHit;

use crate::renderer::projectiles::ProjectileSprite;

/// Global particle cap to prevent performance degradation.
const MAX_PARTICLES: usize = 200;

/// A single particle with linear interpolation over its lifetime.
#[derive(Component)]
pub struct Particle {
    pub elapsed: f32,
    pub lifetime: f32,
    pub velocity: Vec2,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_alpha: f32,
    pub end_alpha: f32,
}

/// Spawns particles from a template, self-destructs when done.
#[derive(Component)]
pub struct ParticleEmitter {
    pub remaining: u32,
    pub spawn_timer: Timer,
    pub particle_lifetime: f32,
    pub particle_velocity_range: (Vec2, Vec2),
    pub particle_color: Color,
    pub particle_size: f32,
    pub start_scale: f32,
    pub end_scale: f32,
}

/// Tick particle lifetimes, lerp scale/alpha, apply velocity, despawn expired.
pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut sprite) in particles.iter_mut() {
        particle.elapsed += dt;
        if particle.elapsed >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        let t = particle.elapsed / particle.lifetime;

        // Lerp scale
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * t;
        transform.scale = Vec3::splat(scale);

        // Lerp alpha
        let alpha = particle.start_alpha + (particle.end_alpha - particle.start_alpha) * t;
        sprite.color = sprite.color.with_alpha(alpha);

        // Apply velocity
        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;
    }
}

/// Tick emitters, spawn particles, self-destruct when done.
pub fn update_emitters(
    mut commands: Commands,
    time: Res<Time>,
    particle_count: Query<(), With<Particle>>,
    mut emitters: Query<(Entity, &mut ParticleEmitter, &Transform)>,
) {
    let mut current_particles = particle_count.iter().count();

    for (entity, mut emitter, transform) in emitters.iter_mut() {
        if emitter.remaining == 0 {
            commands.entity(entity).despawn();
            continue;
        }

        emitter.spawn_timer.tick(time.delta());
        if emitter.spawn_timer.just_finished() && current_particles < MAX_PARTICLES {
            // Spawn a particle at the emitter's position
            let vel_min = emitter.particle_velocity_range.0;
            let vel_max = emitter.particle_velocity_range.1;
            // Simple deterministic spread based on remaining count
            let t = emitter.remaining as f32 / (emitter.remaining as f32 + 1.0);
            let vel = Vec2::new(
                vel_min.x + (vel_max.x - vel_min.x) * t,
                vel_min.y + (vel_max.y - vel_min.y) * t,
            );

            commands.spawn((
                Particle {
                    elapsed: 0.0,
                    lifetime: emitter.particle_lifetime,
                    velocity: vel,
                    start_scale: emitter.start_scale,
                    end_scale: emitter.end_scale,
                    start_alpha: 1.0,
                    end_alpha: 0.0,
                },
                Sprite {
                    color: emitter.particle_color,
                    custom_size: Some(Vec2::splat(emitter.particle_size)),
                    ..default()
                },
                Transform::from_translation(transform.translation),
            ));

            emitter.remaining -= 1;
            current_particles += 1;
        }
    }
}

/// Spawn trail particles behind projectiles (every 3rd frame).
pub fn spawn_trail_particles(
    mut commands: Commands,
    mut frame_counter: Local<u32>,
    particle_count: Query<(), With<Particle>>,
    projectiles: Query<(&Transform, Option<&ProjectileKind>), With<ProjectileSprite>>,
) {
    *frame_counter += 1;
    if *frame_counter % 3 != 0 {
        return;
    }

    let mut current_particles = particle_count.iter().count();
    if current_particles >= MAX_PARTICLES {
        return;
    }

    for (transform, proj_kind) in projectiles.iter() {
        if current_particles >= MAX_PARTICLES {
            break;
        }

        let kind = proj_kind.copied().unwrap_or_default();
        let trail_color = trail_color_for_kind(kind);

        commands.spawn((
            Particle {
                elapsed: 0.0,
                lifetime: 0.2,
                velocity: Vec2::ZERO,
                start_scale: 0.8,
                end_scale: 0.1,
                start_alpha: 0.6,
                end_alpha: 0.0,
            },
            Sprite {
                color: trail_color,
                custom_size: Some(Vec2::splat(2.0)),
                ..default()
            },
            Transform::from_translation(transform.translation + Vec3::new(0.0, 0.0, -0.01)),
        ));
        current_particles += 1;
    }
}

/// Spawn impact VFX when projectiles hit.
pub fn spawn_impact_vfx(
    mut commands: Commands,
    mut hit_events: MessageReader<ProjectileHit>,
    particle_count: Query<(), With<Particle>>,
) {
    let mut current_particles = particle_count.iter().count();

    for hit in hit_events.read() {
        if current_particles >= MAX_PARTICLES {
            break;
        }

        let screen = world_to_screen(hit.position);
        let pos = Vec3::new(screen.x, -screen.y, 100.0); // High Z for visibility

        let profile = impact_profile(hit.kind);

        // Spawn burst particles
        let count = profile
            .particle_count
            .min((MAX_PARTICLES - current_particles) as u32);
        for i in 0..count {
            let angle = std::f32::consts::TAU * (i as f32 / count as f32);
            let speed = profile.speed;
            let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            commands.spawn((
                Particle {
                    elapsed: 0.0,
                    lifetime: profile.lifetime,
                    velocity: vel,
                    start_scale: 1.0,
                    end_scale: 0.2,
                    start_alpha: 1.0,
                    end_alpha: 0.0,
                },
                Sprite {
                    color: profile.color,
                    custom_size: Some(Vec2::splat(profile.particle_size)),
                    ..default()
                },
                Transform::from_translation(pos),
            ));
            current_particles += 1;
        }

        // Spawn a brief flash sprite at the impact point
        if current_particles < MAX_PARTICLES {
            commands.spawn((
                Particle {
                    elapsed: 0.0,
                    lifetime: profile.flash_duration,
                    velocity: Vec2::ZERO,
                    start_scale: 1.5,
                    end_scale: 0.0,
                    start_alpha: 0.9,
                    end_alpha: 0.0,
                },
                Sprite {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.9),
                    custom_size: Some(Vec2::splat(profile.flash_size)),
                    ..default()
                },
                Transform::from_translation(pos + Vec3::new(0.0, 0.0, 0.01)),
            ));
            current_particles += 1;
        }
    }
}

/// Trail particle color per projectile kind (slightly dimmer than the projectile).
fn trail_color_for_kind(kind: ProjectileKind) -> Color {
    match kind {
        ProjectileKind::Spit => Color::srgba(0.2, 0.7, 0.15, 0.5),
        ProjectileKind::LaserBeam => Color::srgba(1.0, 0.3, 0.1, 0.5),
        ProjectileKind::SonicWave => Color::srgba(0.6, 0.2, 0.9, 0.4),
        ProjectileKind::MechShot => Color::srgba(0.2, 0.7, 0.9, 0.5),
        ProjectileKind::Explosive => Color::srgba(0.9, 0.5, 0.1, 0.5),
        ProjectileKind::Generic => Color::srgba(0.9, 0.8, 0.2, 0.4),
    }
}

struct ImpactProfile {
    particle_count: u32,
    lifetime: f32,
    speed: f32,
    color: Color,
    particle_size: f32,
    flash_duration: f32,
    flash_size: f32,
}

fn impact_profile(kind: ProjectileKind) -> ImpactProfile {
    match kind {
        ProjectileKind::Spit => ImpactProfile {
            particle_count: 6,
            lifetime: 0.3,
            speed: 30.0,
            color: Color::srgb(0.3, 0.9, 0.2),
            particle_size: 2.5,
            flash_duration: 0.15,
            flash_size: 6.0,
        },
        ProjectileKind::LaserBeam => ImpactProfile {
            particle_count: 8,
            lifetime: 0.15,
            speed: 50.0,
            color: Color::srgb(1.0, 0.4, 0.2),
            particle_size: 2.0,
            flash_duration: 0.1,
            flash_size: 8.0,
        },
        ProjectileKind::SonicWave => ImpactProfile {
            particle_count: 4,
            lifetime: 0.4,
            speed: 20.0,
            color: Color::srgb(0.7, 0.3, 1.0),
            particle_size: 3.0,
            flash_duration: 0.2,
            flash_size: 10.0,
        },
        ProjectileKind::MechShot => ImpactProfile {
            particle_count: 8,
            lifetime: 0.3,
            speed: 45.0,
            color: Color::srgb(0.5, 0.9, 1.0),
            particle_size: 2.0,
            flash_duration: 0.15,
            flash_size: 7.0,
        },
        ProjectileKind::Explosive => ImpactProfile {
            particle_count: 10,
            lifetime: 0.4,
            speed: 40.0,
            color: Color::srgb(1.0, 0.6, 0.1),
            particle_size: 3.0,
            flash_duration: 0.2,
            flash_size: 12.0,
        },
        ProjectileKind::Generic => ImpactProfile {
            particle_count: 4,
            lifetime: 0.2,
            speed: 25.0,
            color: Color::srgb(1.0, 0.9, 0.3),
            particle_size: 2.0,
            flash_duration: 0.1,
            flash_size: 5.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_particles_cap_is_reasonable() {
        assert!(MAX_PARTICLES > 0);
        assert!(MAX_PARTICLES <= 500);
    }

    #[test]
    fn trail_colors_have_alpha() {
        let kinds = [
            ProjectileKind::Spit,
            ProjectileKind::LaserBeam,
            ProjectileKind::SonicWave,
            ProjectileKind::MechShot,
            ProjectileKind::Explosive,
            ProjectileKind::Generic,
        ];
        for kind in kinds {
            let color = trail_color_for_kind(kind);
            let alpha = color.alpha();
            assert!(
                alpha > 0.0 && alpha < 1.0,
                "Trail color for {kind:?} should be semi-transparent"
            );
        }
    }

    #[test]
    fn impact_profiles_have_positive_values() {
        let kinds = [
            ProjectileKind::Spit,
            ProjectileKind::LaserBeam,
            ProjectileKind::SonicWave,
            ProjectileKind::MechShot,
            ProjectileKind::Explosive,
            ProjectileKind::Generic,
        ];
        for kind in kinds {
            let profile = impact_profile(kind);
            assert!(profile.particle_count > 0, "{kind:?} should have particles");
            assert!(
                profile.lifetime > 0.0,
                "{kind:?} should have positive lifetime"
            );
            assert!(profile.speed > 0.0, "{kind:?} should have positive speed");
            assert!(
                profile.flash_duration > 0.0,
                "{kind:?} should have positive flash duration"
            );
            assert!(
                profile.flash_size > 0.0,
                "{kind:?} should have positive flash size"
            );
        }
    }

    #[test]
    fn spit_has_6_droplets() {
        let profile = impact_profile(ProjectileKind::Spit);
        assert_eq!(profile.particle_count, 6);
    }

    #[test]
    fn laser_flash_is_fast() {
        let profile = impact_profile(ProjectileKind::LaserBeam);
        assert!(profile.lifetime <= 0.2);
    }

    #[test]
    fn explosive_has_most_particles() {
        let explosive = impact_profile(ProjectileKind::Explosive);
        let generic = impact_profile(ProjectileKind::Generic);
        assert!(explosive.particle_count > generic.particle_count);
    }
}
