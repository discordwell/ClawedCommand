use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::commands::EntityId;
use crate::coords::{GridPos, WorldPos};
use crate::math::Fixed;

/// World-space position of an entity.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Position {
    pub world: WorldPos,
}

/// Velocity in world-units per tick.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Velocity {
    pub dx: Fixed,
    pub dy: Fixed,
}

impl Velocity {
    pub fn zero() -> Self {
        Self {
            dx: Fixed::ZERO,
            dy: Fixed::ZERO,
        }
    }
}

/// Cached grid cell, recomputed from Position each tick.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct GridCell {
    pub pos: GridPos,
}

/// Which player owns this entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Owner {
    pub player_id: u8,
}

/// The kind of unit (cat faction roster — see GAME_DESIGN.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    Pawdler,       // Worker (Cat) — gathers food, builds, scrounges GPU cores
    Nuisance,      // Light Harasser (Cat) — fast, cheap, debuffs enemies
    Chonk,         // Heavy Tank (Fat Cat) — immovable, absorbs damage, blocks pathing
    FlyingFox,     // Air Scout/Striker (Fruit Bat) — flies over terrain, night vision
    Hisser,        // Ranged (Cat) — medium-range spitter
    Yowler,        // Support (Cat) — buffs allies, debuffs enemies in range
    Mouser,        // Stealth Scout (Cat) — fast, stealthy, reveals fog
    Catnapper,     // Siege (Cat) — sleeps on buildings until they collapse
    FerretSapper,  // Demolitions (Ferret) — plants explosives, fast building destruction
    MechCommander, // Hero/Heavy (Cat in Mech) — late-game, commands nearby units
}

/// Identifies what type of unit this entity is.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct UnitType {
    pub kind: UnitKind,
}

/// Hit points.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Health {
    pub current: Fixed,
    pub max: Fixed,
}

/// How fast this unit moves (world-units per tick).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MovementSpeed {
    pub speed: Fixed,
}

/// Marker: this unit is currently selected by the player.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Selected;

/// The unit is moving toward this world position (simple direct move).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MoveTarget {
    pub target: WorldPos,
}

/// A* path the unit is following.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Path {
    pub waypoints: VecDeque<GridPos>,
}

// ---------------------------------------------------------------------------
// Combat components
// ---------------------------------------------------------------------------

/// Whether a unit attacks in melee or at range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Melee,
    Ranged,
}

/// Combat statistics for a unit.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackStats {
    pub damage: Fixed,
    pub range: Fixed,
    pub attack_speed: u32,        // ticks between attacks
    pub cooldown_remaining: u32,  // ticks until next attack
}

/// Marker: which attack type this unit uses.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackTypeMarker {
    pub attack_type: AttackType,
}

/// The entity this unit is targeting for attack.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackTarget {
    pub target: EntityId,
}

/// Marker: unit is chasing a target to get into attack range.
/// Distinguished from player-issued MoveTarget so Stop clears it.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ChasingTarget {
    pub target: EntityId,
}

/// A projectile in flight.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Projectile {
    pub damage: Fixed,
    pub speed: Fixed,
}

/// Which entity this projectile is homing toward.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ProjectileTarget {
    pub target: EntityId,
}

/// Marker: this entity is dead (awaiting despawn next tick).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Dead;

/// Marker: unit should hold position (attack in range only, no chasing).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct HoldPosition;

/// Unit is attack-moving toward a grid position (engages enemies along the way).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackMoveTarget {
    pub target: GridPos,
}
