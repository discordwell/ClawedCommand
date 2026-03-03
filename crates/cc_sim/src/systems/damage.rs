use bevy::prelude::*;

use cc_core::commands::EntityId;
use cc_core::coords::WorldPos;
use cc_core::math::{FIXED_ZERO, Fixed};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance, is_cc};

/// Shared command to apply damage, avoiding borrow conflicts between attacker/target queries.
/// Used by both combat_system (melee) and projectile_system (ranged hits).
pub struct ApplyDamageCommand {
    pub target: Entity,
    pub damage: Fixed,
}

impl Command for ApplyDamageCommand {
    fn apply(self, world: &mut World) {
        if let Some(mut health) = world.get_mut::<cc_core::components::Health>(self.target) {
            health.current -= self.damage;
            if health.current < FIXED_ZERO {
                health.current = FIXED_ZERO;
            }
        }
    }
}

/// Deferred command to apply a status effect with stacking, duration refresh, and CC immunity.
pub struct ApplyStatusCommand {
    pub target: Entity,
    pub effect: StatusEffectId,
    pub duration: u32,
    pub stacks: u32,
    pub max_stacks: u32,
    pub source: EntityId,
}

impl Command for ApplyStatusCommand {
    fn apply(self, world: &mut World) {
        let Some(mut effects) = world.get_mut::<StatusEffects>(self.target) else {
            return;
        };

        // Check CC immunity
        if is_cc(self.effect) && effects.is_cc_immune() {
            return;
        }

        // Look for existing effect of same type
        if let Some(existing) = effects
            .effects
            .iter_mut()
            .find(|e| e.effect == self.effect && e.remaining_ticks > 0)
        {
            // Refresh duration
            existing.remaining_ticks = existing.remaining_ticks.max(self.duration);
            // Add stacks up to max
            if self.max_stacks > 0 {
                existing.stacks = (existing.stacks + self.stacks).min(self.max_stacks);
            }
        } else {
            // Apply new effect
            effects.effects.push(StatusInstance {
                effect: self.effect,
                remaining_ticks: self.duration,
                stacks: self.stacks.max(1),
                source: self.source,
            });
        }
    }
}

/// Deferred command to apply AoE CC to enemies within radius.
pub struct AoeCcCommand {
    pub source_entity: Entity,
    pub source_pos: WorldPos,
    pub radius: Fixed,
    pub effect: StatusEffectId,
    pub duration: u32,
    pub source_owner: u8,
}

impl Command for AoeCcCommand {
    fn apply(self, world: &mut World) {
        let radius_sq = self.radius * self.radius;

        // Fetch source position from entity (may differ from passed source_pos if default)
        let source_pos = world
            .get::<cc_core::components::Position>(self.source_entity)
            .map(|p| p.world)
            .unwrap_or(self.source_pos);

        // Collect targets first to avoid borrow issues
        let targets: Vec<Entity> = world
            .query::<(
                Entity,
                &cc_core::components::Position,
                &cc_core::components::Owner,
            )>()
            .iter(world)
            .filter(|(e, pos, owner)| {
                *e != self.source_entity
                    && owner.player_id != self.source_owner
                    && pos.world.distance_squared(source_pos) <= radius_sq
            })
            .map(|(e, _, _)| e)
            .collect();

        for target in targets {
            if let Some(mut effects) = world.get_mut::<StatusEffects>(target) {
                if is_cc(self.effect) && effects.is_cc_immune() {
                    continue;
                }
                effects.refresh_or_insert(
                    self.effect,
                    self.duration,
                    EntityId::from_entity(self.source_entity),
                );
            }
        }
    }
}

/// Deferred command to push enemies away from a source position (Revulsion).
pub struct RevulsionAoeCommand {
    pub source_entity: Entity,
    pub source_pos: WorldPos,
    pub radius: Fixed,
    pub push_distance: Fixed,
    pub source_owner: u8,
}

impl Command for RevulsionAoeCommand {
    fn apply(self, world: &mut World) {
        let radius_sq = self.radius * self.radius;

        // Fetch source position from entity
        let source_pos = world
            .get::<cc_core::components::Position>(self.source_entity)
            .map(|p| p.world)
            .unwrap_or(self.source_pos);

        // Get map bounds
        let (map_w, map_h) = {
            let map_res = world.resource::<crate::resources::MapResource>();
            (map_res.map.width as i32, map_res.map.height as i32)
        };

        // Collect targets
        let targets: Vec<(Entity, WorldPos)> = world
            .query::<(
                Entity,
                &cc_core::components::Position,
                &cc_core::components::Owner,
            )>()
            .iter(world)
            .filter(|(e, pos, owner)| {
                *e != self.source_entity
                    && owner.player_id != self.source_owner
                    && pos.world.distance_squared(source_pos) <= radius_sq
            })
            .map(|(e, pos, _)| (e, pos.world))
            .collect();

        let map_max_x = Fixed::from_num(map_w - 1);
        let map_max_y = Fixed::from_num(map_h - 1);

        for (target, target_pos) in targets {
            let dx = target_pos.x - source_pos.x;
            let dy = target_pos.y - source_pos.y;

            // Normalize direction
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= FIXED_ZERO {
                continue;
            }

            // Approximate normalization using the larger component
            let abs_dx = if dx < FIXED_ZERO { -dx } else { dx };
            let abs_dy = if dy < FIXED_ZERO { -dy } else { dy };
            let max_component = if abs_dx > abs_dy { abs_dx } else { abs_dy };

            if max_component <= FIXED_ZERO {
                continue;
            }

            let norm_dx = dx / max_component;
            let norm_dy = dy / max_component;

            let new_x = (target_pos.x + norm_dx * self.push_distance)
                .max(FIXED_ZERO)
                .min(map_max_x);
            let new_y = (target_pos.y + norm_dy * self.push_distance)
                .max(FIXED_ZERO)
                .min(map_max_y);

            if let Some(mut pos) = world.get_mut::<cc_core::components::Position>(target) {
                pos.world.x = new_x;
                pos.world.y = new_y;
            }

            // Clear movement to interrupt
            world
                .entity_mut(target)
                .remove::<cc_core::components::Path>();
            world
                .entity_mut(target)
                .remove::<cc_core::components::MoveTarget>();
        }
    }
}

/// Deferred command for Chonk's GravitationalChonk aura — pulls enemy toward source position.
pub struct GravitationalPullCommand {
    pub source_pos: WorldPos,
    pub target: Entity,
    pub pull_per_tick: Fixed,
}

impl Command for GravitationalPullCommand {
    fn apply(self, world: &mut World) {
        let Some(mut pos) = world.get_mut::<cc_core::components::Position>(self.target) else {
            return;
        };

        let dx = self.source_pos.x - pos.world.x;
        let dy = self.source_pos.y - pos.world.y;

        let abs_dx = if dx < FIXED_ZERO { -dx } else { dx };
        let abs_dy = if dy < FIXED_ZERO { -dy } else { dy };
        let max_component = if abs_dx > abs_dy { abs_dx } else { abs_dy };

        if max_component <= FIXED_ZERO {
            return;
        }

        let norm_dx = dx / max_component;
        let norm_dy = dy / max_component;

        pos.world.x += norm_dx * self.pull_per_tick;
        pos.world.y += norm_dy * self.pull_per_tick;
    }
}

/// Deferred command for AoE damage (e.g., FerretSapper's DemoCharge).
pub struct AoeDamageCommand {
    pub source_entity: Entity,
    pub source_pos: WorldPos,
    pub radius: Fixed,
    pub damage: Fixed,
    pub building_multiplier: Fixed,
    pub source_owner: u8,
}

impl Command for AoeDamageCommand {
    fn apply(self, world: &mut World) {
        let radius_sq = self.radius * self.radius;

        let source_pos = world
            .get::<cc_core::components::Position>(self.source_entity)
            .map(|p| p.world)
            .unwrap_or(self.source_pos);

        let targets: Vec<(Entity, bool)> = world
            .query::<(
                Entity,
                &cc_core::components::Position,
                &cc_core::components::Owner,
                Option<&cc_core::components::Building>,
            )>()
            .iter(world)
            .filter(|(e, pos, owner, _)| {
                *e != self.source_entity
                    && owner.player_id != self.source_owner
                    && pos.world.distance_squared(source_pos) <= radius_sq
            })
            .map(|(e, _, _, bld)| (e, bld.is_some()))
            .collect();

        for (target, is_building) in targets {
            if let Some(mut health) = world.get_mut::<cc_core::components::Health>(target) {
                let dmg = if is_building {
                    self.damage * self.building_multiplier
                } else {
                    self.damage
                };
                health.current -= dmg;
                if health.current < FIXED_ZERO {
                    health.current = FIXED_ZERO;
                }
            }
        }
    }
}

/// Deferred command to spawn a HairballObstacle entity at a position.
pub struct SpawnHairballCommand {
    pub position: WorldPos,
    pub owner_player_id: u8,
    pub duration_ticks: u32,
}

impl Command for SpawnHairballCommand {
    fn apply(self, world: &mut World) {
        use cc_core::components::{GridCell, HairballObstacle, Position};

        let grid = self.position.to_grid();

        world.spawn((
            Position {
                world: self.position,
            },
            GridCell { pos: grid },
            HairballObstacle {
                remaining_ticks: self.duration_ticks,
                owner_player_id: self.owner_player_id,
            },
        ));
    }
}

/// Deferred command to reveal enemies in a radius (Echolocation Pulse).
pub struct EcholocationRevealCommand {
    pub source_pos: WorldPos,
    pub radius: Fixed,
    pub reveal_ticks: u32,
    pub source_owner: u8,
}

impl Command for EcholocationRevealCommand {
    fn apply(self, world: &mut World) {
        use cc_core::components::{Owner, VisibleThroughFog};

        let radius_sq = self.radius * self.radius;

        let targets: Vec<Entity> = world
            .query::<(Entity, &cc_core::components::Position, &Owner)>()
            .iter(world)
            .filter(|(_, pos, owner)| {
                owner.player_id != self.source_owner
                    && pos.world.distance_squared(self.source_pos) <= radius_sq
            })
            .map(|(e, _, _)| e)
            .collect();

        for target in targets {
            if let Some(mut vtf) = world.get_mut::<VisibleThroughFog>(target) {
                vtf.remaining_ticks = vtf.remaining_ticks.max(self.reveal_ticks);
            } else {
                world.entity_mut(target).insert(VisibleThroughFog {
                    remaining_ticks: self.reveal_ticks,
                });
            }
        }
    }
}
