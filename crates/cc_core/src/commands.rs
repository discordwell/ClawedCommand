use crate::coords::GridPos;

/// Unique identifier for entities in commands.
/// In Bevy this maps to bevy::ecs::entity::Entity, but cc_core
/// stays engine-agnostic so we use a simple u64 wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// All commands that can be issued to the game simulation.
/// Both player input and AI agents produce these.
#[derive(Debug, Clone)]
pub enum GameCommand {
    /// Move units to a grid position (pathfinding will resolve the route).
    Move {
        unit_ids: Vec<EntityId>,
        target: GridPos,
    },
    /// Stop units immediately.
    Stop { unit_ids: Vec<EntityId> },
    /// Select units (UI concern routed through command queue for determinism).
    Select { unit_ids: Vec<EntityId> },
    /// Deselect all units.
    Deselect,
}
