use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
