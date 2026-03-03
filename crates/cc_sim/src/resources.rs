use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use cc_core::commands::{CommandSource, EntityId, GameCommand};
use cc_core::components::UpgradeType;
use cc_core::coords::GridPos;
use cc_core::map::GameMap;

/// A queued command with its metadata.
#[derive(Debug, Clone)]
pub struct QueuedCommand {
    pub player_id: Option<u8>,
    pub source: CommandSource,
    pub command: GameCommand,
}

/// Queue of commands to process each simulation tick.
///
/// Commands are tagged with an optional player_id so the command processor
/// can interleave per-player commands for fairness (avoid first/last-mover bias).
#[derive(Resource, Default)]
pub struct CommandQueue {
    pub commands: Vec<QueuedCommand>,
}

impl CommandQueue {
    pub fn push(&mut self, cmd: GameCommand) {
        self.commands.push(QueuedCommand {
            player_id: None,
            source: CommandSource::default(),
            command: cmd,
        });
    }

    /// Push a command tagged with the issuing player's ID.
    pub fn push_for_player(&mut self, player_id: u8, cmd: GameCommand) {
        self.commands.push(QueuedCommand {
            player_id: Some(player_id),
            source: CommandSource::default(),
            command: cmd,
        });
    }

    /// Push a command with explicit source and player ID.
    pub fn push_sourced(&mut self, player_id: Option<u8>, source: CommandSource, cmd: GameCommand) {
        self.commands.push(QueuedCommand {
            player_id,
            source,
            command: cmd,
        });
    }

    /// Drain all commands, interleaving per-player commands for fairness.
    /// On even ticks player 0's commands go first in each pair; on odd ticks player 1 goes first.
    /// Returns (source, command) pairs for filtering by ControlRestrictions.
    pub fn drain_interleaved(&mut self, tick: u64) -> Vec<(CommandSource, GameCommand)> {
        let all = std::mem::take(&mut self.commands);
        let mut p0: Vec<(CommandSource, GameCommand)> = Vec::new();
        let mut p1: Vec<(CommandSource, GameCommand)> = Vec::new();
        let mut other: Vec<(CommandSource, GameCommand)> = Vec::new();

        for qc in all {
            let pair = (qc.source, qc.command);
            match qc.player_id {
                Some(0) => p0.push(pair),
                Some(1) => p1.push(pair),
                _ => other.push(pair),
            }
        }

        let mut result = Vec::with_capacity(p0.len() + p1.len() + other.len());

        let (first, second) = if tick % 2 == 0 { (p0, p1) } else { (p1, p0) };

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
        std::mem::take(&mut self.commands)
            .into_iter()
            .map(|qc| qc.command)
            .collect()
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

/// Current game state — Playing, Paused, or Victory with a winner.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
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
            supply_cap: 0,
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
            players: vec![
                PlayerResourceState::default(),
                PlayerResourceState::default(),
            ],
        }
    }
}

/// Tracks units whose commands were issued by voice and should not be
/// overridden by Script or AiAgent sources until the override expires.
///
/// When a voice command targets a set of units, their entity IDs are stored
/// here with a remaining-tick counter.  The command system skips movement-
/// related Script/AiAgent commands for these units until the counter reaches 0.
#[derive(Resource, Default, Debug, Clone)]
pub struct VoiceOverride {
    /// Map from entity id → remaining ticks of override.
    pub overrides: HashMap<EntityId, u32>,
}

impl VoiceOverride {
    /// Duration (in sim ticks) that a voice command suppresses script commands.
    /// At 10 Hz sim rate this is ~10 seconds — long enough that the player's
    /// intent sticks until combat resolves or they issue a new command.
    pub const DURATION_TICKS: u32 = 100;

    /// Register a set of entities as voice-overridden.
    pub fn set(&mut self, entities: &[EntityId]) {
        for &eid in entities {
            self.overrides.insert(eid, Self::DURATION_TICKS);
        }
    }

    /// Tick down all overrides; remove any that have expired.
    pub fn tick(&mut self) {
        self.overrides.retain(|_, remaining| {
            *remaining = remaining.saturating_sub(1);
            *remaining > 0
        });
    }

    /// Returns true if this entity is currently voice-overridden.
    pub fn is_overridden(&self, eid: &EntityId) -> bool {
        self.overrides.contains_key(eid)
    }

    /// Filter overridden entity IDs out of a movement command.
    /// Returns `None` if all IDs were stripped (command should be skipped).
    /// Non-movement commands pass through unchanged.
    pub fn filter_command(&self, cmd: GameCommand) -> Option<GameCommand> {
        if self.overrides.is_empty() {
            return Some(cmd);
        }
        match cmd {
            GameCommand::Move { unit_ids, target } => {
                let filtered: Vec<EntityId> = unit_ids
                    .into_iter()
                    .filter(|id| !self.is_overridden(id))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(GameCommand::Move {
                        unit_ids: filtered,
                        target,
                    })
                }
            }
            GameCommand::AttackMove { unit_ids, target } => {
                let filtered: Vec<EntityId> = unit_ids
                    .into_iter()
                    .filter(|id| !self.is_overridden(id))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(GameCommand::AttackMove {
                        unit_ids: filtered,
                        target,
                    })
                }
            }
            GameCommand::Attack { unit_ids, target } => {
                let filtered: Vec<EntityId> = unit_ids
                    .into_iter()
                    .filter(|id| !self.is_overridden(id))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(GameCommand::Attack {
                        unit_ids: filtered,
                        target,
                    })
                }
            }
            GameCommand::Stop { unit_ids } => {
                let filtered: Vec<EntityId> = unit_ids
                    .into_iter()
                    .filter(|id| !self.is_overridden(id))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(GameCommand::Stop { unit_ids: filtered })
                }
            }
            GameCommand::HoldPosition { unit_ids } => {
                let filtered: Vec<EntityId> = unit_ids
                    .into_iter()
                    .filter(|id| !self.is_overridden(id))
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some(GameCommand::HoldPosition { unit_ids: filtered })
                }
            }
            other => Some(other),
        }
    }
}

/// Cumulative combat event counters for observability.
#[derive(Resource, Default, Debug, Clone)]
pub struct CombatStats {
    /// Total melee attacks that dealt damage.
    pub melee_attack_count: u64,
    /// Total ranged attacks (projectiles spawned).
    pub ranged_attack_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::coords::GridPos;

    #[test]
    fn voice_override_filters_script_commands() {
        let mut vo = VoiceOverride::default();
        let e1 = EntityId(100);
        let e2 = EntityId(200);
        vo.set(&[e1, e2]);

        // All overridden → command fully suppressed (None)
        let cmd = GameCommand::AttackMove {
            unit_ids: vec![e1, e2],
            target: GridPos::new(10, 10),
        };
        assert!(vo.filter_command(cmd).is_none());

        // Mixed: overridden e1 stripped, non-overridden 999 remains
        let cmd_mixed = GameCommand::Move {
            unit_ids: vec![e1, EntityId(999)],
            target: GridPos::new(5, 5),
        };
        let filtered = vo.filter_command(cmd_mixed).unwrap();
        match filtered {
            GameCommand::Move { unit_ids, .. } => {
                assert_eq!(unit_ids, vec![EntityId(999)]);
            }
            _ => panic!("Expected Move command"),
        }

        // Non-movement command → passes through unchanged
        let cmd_train = GameCommand::TrainUnit {
            building: EntityId(50),
            unit_kind: cc_core::components::UnitKind::Chonk,
        };
        assert!(vo.filter_command(cmd_train).is_some());
    }

    #[test]
    fn voice_override_expires_after_ticks() {
        let mut vo = VoiceOverride::default();
        let e1 = EntityId(100);
        vo.set(&[e1]);

        assert!(vo.is_overridden(&e1));

        // Tick down to 1 remaining
        for _ in 0..VoiceOverride::DURATION_TICKS - 1 {
            vo.tick();
        }
        assert!(vo.is_overridden(&e1));

        // One more tick → expired
        vo.tick();
        assert!(!vo.is_overridden(&e1));
    }

    #[test]
    fn voice_override_reset_extends_duration() {
        let mut vo = VoiceOverride::default();
        let e1 = EntityId(100);
        vo.set(&[e1]);

        // Tick halfway
        for _ in 0..50 {
            vo.tick();
        }
        assert!(vo.is_overridden(&e1));
        assert_eq!(vo.overrides[&e1], VoiceOverride::DURATION_TICKS - 50);

        // Re-set resets the timer
        vo.set(&[e1]);
        assert_eq!(vo.overrides[&e1], VoiceOverride::DURATION_TICKS);
    }
}
