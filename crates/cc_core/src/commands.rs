use serde::{Deserialize, Serialize};

use crate::components::{BuildingKind, UnitKind, UpgradeType};
use crate::coords::GridPos;

/// Unique identifier for entities in commands.
/// In Bevy this maps to bevy::ecs::entity::Entity, but cc_core
/// stays engine-agnostic so we use a simple u64 wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

#[cfg(feature = "bevy")]
impl EntityId {
    /// Create an EntityId from a Bevy Entity.
    pub fn from_entity(entity: bevy_ecs::entity::Entity) -> Self {
        Self(entity.to_bits())
    }
}

/// Where a command originated — used by control restriction mutators
/// to filter commands by input source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CommandSource {
    /// Mouse/keyboard input from the player.
    #[default]
    PlayerInput,
    /// Voice command (on-device keyword spotting).
    VoiceCommand,
    /// AI agent (Lua script or LLM).
    AiAgent,
    /// Scripted trigger or mission system.
    Script,
}

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
    /// Attack a specific enemy entity.
    Attack {
        unit_ids: Vec<EntityId>,
        target: EntityId,
    },
    /// Move to a position, engaging enemies along the way.
    AttackMove {
        unit_ids: Vec<EntityId>,
        target: GridPos,
    },
    /// Hold position: attack in range only, no chasing.
    HoldPosition { unit_ids: Vec<EntityId> },
    /// Send worker units to gather from a resource deposit.
    GatherResource {
        unit_ids: Vec<EntityId>,
        deposit: EntityId,
    },
    /// Place a building at a grid position.
    Build {
        builder: EntityId,
        building_kind: BuildingKind,
        position: GridPos,
    },
    /// Train a unit from a production building.
    TrainUnit {
        building: EntityId,
        unit_kind: UnitKind,
    },
    /// Set rally point for a production building.
    SetRallyPoint {
        building: EntityId,
        target: GridPos,
    },
    /// Cancel the front item in a building's production queue.
    CancelQueue { building: EntityId },
    /// Assign selected units to a control group (0-9).
    SetControlGroup {
        group: u8,
        unit_ids: Vec<EntityId>,
    },
    /// Recall (select) units in a control group.
    RecallControlGroup { group: u8 },
    /// Activate a unit's ability by slot index.
    ActivateAbility {
        unit_id: EntityId,
        slot: u8,
        target: AbilityTarget,
    },
    /// Queue research at a ScratchingPost.
    Research {
        building: EntityId,
        upgrade: UpgradeType,
    },
    /// Cancel the front item in a building's research queue.
    CancelResearch { building: EntityId },
}

impl GameCommand {
    /// Hint which player issued this command (for interleaving fairness).
    /// Returns None when the issuing player is unknown.
    pub fn player_hint(&self) -> Option<u8> {
        None
    }

    /// Check if this command is a Build action.
    pub fn is_build(&self) -> bool {
        matches!(self, GameCommand::Build { .. })
    }

    /// Return the unit_ids this command targets (if any).
    pub fn unit_ids(&self) -> Option<&Vec<EntityId>> {
        match self {
            GameCommand::Move { unit_ids, .. }
            | GameCommand::Stop { unit_ids }
            | GameCommand::Select { unit_ids }
            | GameCommand::Attack { unit_ids, .. }
            | GameCommand::AttackMove { unit_ids, .. }
            | GameCommand::HoldPosition { unit_ids }
            | GameCommand::GatherResource { unit_ids, .. }
            | GameCommand::SetControlGroup { unit_ids, .. } => Some(unit_ids),
            _ => None,
        }
    }
}

/// Target for an ability activation.
#[derive(Debug, Clone, Copy)]
pub enum AbilityTarget {
    /// Ability targets self (no external target).
    SelfCast,
    /// Ability targets a grid position.
    Position(GridPos),
    /// Ability targets a specific entity.
    Entity(EntityId),
}
