use bevy::prelude::*;

use crate::resources::MapResource;
use cc_core::commands::EntityId;
use cc_core::components::{
    AttackStats, AttackTypeMarker, Building, Dead, Owner, Position, Projectile, ProjectileKind,
    ProjectileTarget, StatModifiers, UnderConstruction, UnitType, Velocity,
};
use cc_core::math::{Fixed, FIXED_ONE};
use cc_core::tuning::TOWER_PROJECTILE_SPEED;

/// Tower combat system: buildings with AttackStats auto-attack nearest enemy in range.
/// No chasing — towers are stationary. Applies cover/elevation multipliers.
pub fn tower_combat_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut towers: Query<
        (
            Entity,
            &Position,
            &mut AttackStats,
            &AttackTypeMarker,
            &Owner,
        ),
        (With<Building>, Without<Dead>, Without<UnderConstruction>),
    >,
    potential_targets: Query<
        (Entity, &Position, &Owner, Option<&StatModifiers>),
        (With<UnitType>, Without<Dead>),
    >,
) {
    for (_tower_entity, tower_pos, mut stats, _atk_type, tower_owner) in towers.iter_mut() {
        // Tick cooldown
        if stats.cooldown_remaining > 0 {
            stats.cooldown_remaining -= 1;
            continue;
        }

        // Find nearest enemy in range
        let mut best_target: Option<(Entity, Fixed)> = None;
        let range_sq = stats.range * stats.range;

        for (target_entity, target_pos, target_owner, target_mods) in potential_targets.iter() {
            // Skip allies
            if target_owner.player_id == tower_owner.player_id {
                continue;
            }

            // Skip invulnerable targets
            if target_mods.map(|m| m.invulnerable).unwrap_or(false) {
                continue;
            }

            let dist_sq = tower_pos.world.distance_squared(target_pos.world);
            if dist_sq <= range_sq {
                if best_target.is_none() || dist_sq < best_target.unwrap().1 {
                    best_target = Some((target_entity, dist_sq));
                }
            }
        }

        let Some((target_entity, _)) = best_target else {
            continue;
        };

        let Ok((_, target_pos, _, target_mods)) = potential_targets.get(target_entity) else {
            continue;
        };

        // Fire!
        stats.cooldown_remaining = stats.attack_speed;

        // Calculate damage with cover + elevation modifiers
        let target_grid = target_pos.world.to_grid();
        let attacker_grid = tower_pos.world.to_grid();

        let cover_mult = map_res
            .map
            .terrain_at(target_grid)
            .map(|t| t.cover().damage_multiplier())
            .unwrap_or(FIXED_ONE);

        let elev_advantage = map_res.map.elevation_advantage(attacker_grid, target_grid);
        let elev_mult = cc_core::terrain::elevation_damage_multiplier(elev_advantage);

        // Apply target's damage_reduction from StatModifiers
        let dr_mult = target_mods.map(|m| m.damage_reduction).unwrap_or(FIXED_ONE);
        let final_damage = stats.damage * cover_mult * elev_mult * dr_mult;

        // Towers always fire ranged projectiles
        commands.spawn((
            Position {
                world: tower_pos.world,
            },
            Velocity::zero(),
            Projectile {
                damage: final_damage,
                speed: TOWER_PROJECTILE_SPEED,
            },
            ProjectileTarget {
                target: EntityId(target_entity.to_bits()),
            },
            ProjectileKind::LaserBeam,
        ));
    }
}
