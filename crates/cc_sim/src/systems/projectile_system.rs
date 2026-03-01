use bevy::prelude::*;

use crate::systems::damage::ApplyDamageCommand;
use cc_core::components::{Dead, Position, Projectile, ProjectileKind, ProjectileTarget, Velocity};
use cc_core::coords::WorldPos;
use cc_core::math::{FIXED_ZERO, approx_distance};

/// Message emitted when a projectile hits its target, for VFX.
#[derive(Message, Debug, Clone)]
pub struct ProjectileHit {
    pub position: WorldPos,
    pub kind: ProjectileKind,
}

/// Move projectiles toward their targets, apply damage on arrival.
pub fn projectile_system(
    mut commands: Commands,
    mut hit_events: MessageWriter<ProjectileHit>,
    mut projectiles: Query<(Entity, &mut Position, &mut Velocity, &Projectile, &ProjectileTarget, Option<&ProjectileKind>)>,
    targets: Query<&Position, (Without<Projectile>, Without<Dead>)>,
) {
    for (entity, mut pos, mut vel, proj, proj_target, proj_kind) in projectiles.iter_mut() {
        let target_entity = Entity::from_bits(proj_target.target.0);
        let Ok(target_pos) = targets.get(target_entity) else {
            // Target despawned or dead — remove projectile
            commands.entity(entity).despawn();
            continue;
        };

        let dx = target_pos.world.x - pos.world.x;
        let dy = target_pos.world.y - pos.world.y;
        let dist_sq = dx * dx + dy * dy;
        let speed_sq = proj.speed * proj.speed;

        if dist_sq <= speed_sq {
            // Emit hit event for VFX before despawning
            let kind = proj_kind.copied().unwrap_or_default();
            hit_events.write(ProjectileHit {
                position: target_pos.world,
                kind,
            });

            // Arrived — apply damage and despawn
            commands.queue(ApplyDamageCommand {
                target: target_entity,
                damage: proj.damage,
            });
            commands.entity(entity).despawn();
        } else {
            // Homing movement using shared approx distance
            let ad = approx_distance(dx, dy);

            if ad > FIXED_ZERO {
                vel.dx = dx * proj.speed / ad;
                vel.dy = dy * proj.speed / ad;
            }

            pos.world.x += vel.dx;
            pos.world.y += vel.dy;
        }
    }
}
