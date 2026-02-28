use bevy::prelude::*;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::map::GameMap;

/// Queue of commands to process each simulation tick.
#[derive(Resource, Default)]
pub struct CommandQueue {
    pub commands: Vec<GameCommand>,
}

impl CommandQueue {
    pub fn push(&mut self, cmd: GameCommand) {
        self.commands.push(cmd);
    }

    pub fn drain(&mut self) -> Vec<GameCommand> {
        std::mem::take(&mut self.commands)
    }
}

/// The current simulation tick count.
#[derive(Resource, Default)]
pub struct SimClock {
    pub tick: u64,
}

/// The game map, accessible as a Bevy resource.
#[derive(Resource)]
pub struct MapResource {
    pub map: GameMap,
}

/// Control groups: 10 groups (0-9), each holding a list of unit EntityIds.
#[derive(Resource)]
pub struct ControlGroups {
    pub groups: [Vec<EntityId>; 10],
}

impl Default for ControlGroups {
    fn default() -> Self {
        Self {
            groups: Default::default(),
        }
    }
}

/// Per-player resource state.
#[derive(Debug, Clone)]
pub struct PlayerResourceState {
    pub food: u32,
    pub gpu_cores: u32,
    pub nfts: u32,
    pub supply: u32,
    pub supply_cap: u32,
}

impl Default for PlayerResourceState {
    fn default() -> Self {
        Self {
            food: 200,
            gpu_cores: 50,
            nfts: 0,
            supply: 0,
            supply_cap: 10,
        }
    }
}

/// Global resource tracker for all players.
#[derive(Resource)]
pub struct PlayerResources {
    pub players: Vec<PlayerResourceState>,
}

impl Default for PlayerResources {
    fn default() -> Self {
        Self {
            players: vec![PlayerResourceState::default(), PlayerResourceState::default()],
        }
    }
}
