use bevy::prelude::*;
use cc_core::commands::GameCommand;
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
