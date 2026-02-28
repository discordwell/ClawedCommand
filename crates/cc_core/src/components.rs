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

/// The kind of unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    Worker,
    Infantry,
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
