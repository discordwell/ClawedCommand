use bevy::prelude::*;
use std::collections::HashSet;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::UpgradeType;
use cc_core::coords::GridPos;
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

/// Current game state — Playing or Victory with a winner.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Victory { winner: u8 },
}

impl Default for GameState {
    fn default() -> Self {
        GameState::Playing
    }
}

/// Spawn positions for each player (player_id → grid position).
#[derive(Resource, Default, Debug, Clone)]
pub struct SpawnPositions {
    pub positions: Vec<(u8, GridPos)>,
}

/// Per-player resource state.
#[derive(Debug, Clone)]
pub struct PlayerResourceState {
    pub food: u32,
    pub gpu_cores: u32,
    pub nfts: u32,
    pub supply: u32,
    pub supply_cap: u32,
    /// Upgrades that have been fully researched.
    pub completed_upgrades: HashSet<UpgradeType>,
}

impl Default for PlayerResourceState {
    fn default() -> Self {
        Self {
            food: 300,
            gpu_cores: 50,
            nfts: 0,
            supply: 0,
            supply_cap: 0, // All supply comes from buildings (TheBox provides 10)
            completed_upgrades: HashSet::new(),
        }
    }
}

/// Deterministic RNG for simulation randomness (Hairball, Contagious Yawning, etc.).
/// Seeded at match start for deterministic replay.
#[derive(Resource)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Simple LCG for deterministic pseudo-random numbers.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state >> 33
    }

    /// Returns a value in [0, max) deterministically.
    pub fn next_bounded(&mut self, max: u32) -> u32 {
        (self.next_u64() % max as u64) as u32
    }
}

impl Default for SimRng {
    fn default() -> Self {
        Self::new(42)
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
