use bevy::prelude::*;
use std::collections::HashSet;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::UpgradeType;
use cc_core::coords::GridPos;
use cc_core::map::GameMap;

/// Queue of commands to process each simulation tick.
///
/// Commands are tagged with an optional player_id so the command processor
/// can interleave per-player commands for fairness (avoid first/last-mover bias).
#[derive(Resource, Default)]
pub struct CommandQueue {
    pub commands: Vec<(Option<u8>, GameCommand)>,
}

impl CommandQueue {
    pub fn push(&mut self, cmd: GameCommand) {
        self.commands.push((None, cmd));
    }

    /// Push a command tagged with the issuing player's ID.
    pub fn push_for_player(&mut self, player_id: u8, cmd: GameCommand) {
        self.commands.push((Some(player_id), cmd));
    }

    /// Drain all commands, interleaving per-player commands for fairness.
    /// On even ticks player 0's commands go first in each pair; on odd ticks player 1 goes first.
    pub fn drain_interleaved(&mut self, tick: u64) -> Vec<GameCommand> {
        let all = std::mem::take(&mut self.commands);
        let mut p0: Vec<GameCommand> = Vec::new();
        let mut p1: Vec<GameCommand> = Vec::new();
        let mut other: Vec<GameCommand> = Vec::new();

        for (player, cmd) in all {
            match player {
                Some(0) => p0.push(cmd),
                Some(1) => p1.push(cmd),
                _ => other.push(cmd),
            }
        }

        let mut result = Vec::with_capacity(p0.len() + p1.len() + other.len());

        let (first, second) = if tick % 2 == 0 {
            (p0, p1)
        } else {
            (p1, p0)
        };

        // Interleave: one from first, one from second, repeat
        let mut i = 0;
        let mut j = 0;
        while i < first.len() || j < second.len() {
            if i < first.len() {
                result.push(first[i].clone());
                i += 1;
            }
            if j < second.len() {
                result.push(second[j].clone());
                j += 1;
            }
        }

        result.extend(other);
        result
    }

    pub fn drain(&mut self) -> Vec<GameCommand> {
        std::mem::take(&mut self.commands).into_iter().map(|(_, cmd)| cmd).collect()
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

/// Cumulative combat event counters for observability.
/// Tracks both melee and ranged attacks so combat can be detected
/// even when no projectiles happen to be in flight at snapshot time.
#[derive(Resource, Default, Debug, Clone)]
pub struct CombatStats {
    /// Total melee attacks that dealt damage.
    pub melee_attack_count: u64,
    /// Total ranged attacks (projectiles spawned).
    pub ranged_attack_count: u64,
}
