//! Invariant checking for wet test harness.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use cc_core::components::*;

use crate::resources::PlayerResources;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Error,
    Fatal,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    pub tick: u64,
    pub severity: Severity,
    pub kind: String,
    pub message: String,
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} tick {}] {}: {}",
            self.severity, self.tick, self.kind, self.message
        )
    }
}

// ---------------------------------------------------------------------------
// Checker
// ---------------------------------------------------------------------------

pub struct InvariantChecker {
    pub violations: Vec<InvariantViolation>,
    map_width: u32,
    map_height: u32,
    /// Previous tick's position hash for stuck detection.
    prev_position_hash: Option<u64>,
    /// How many consecutive ticks the position hash has been unchanged.
    stuck_counter: u64,
}

impl InvariantChecker {
    pub fn new(map_width: u32, map_height: u32) -> Self {
        Self {
            violations: Vec::new(),
            map_width,
            map_height,
            prev_position_hash: None,
            stuck_counter: 0,
        }
    }

    /// Run all invariant checks for the current tick.
    pub fn check_all(&mut self, world: &mut World, tick: u64) {
        self.check_resources(world, tick);
        self.check_health(world, tick);
        self.check_bounds(world, tick);
        self.check_friendly_fire(world, tick);
        self.check_stuck(world, tick);
    }

    /// Record a panic as a Fatal violation.
    pub fn record_panic(&mut self, tick: u64, message: &str) {
        self.violations.push(InvariantViolation {
            tick,
            severity: Severity::Fatal,
            kind: "Panic".into(),
            message: message.to_string(),
        });
    }

    /// Record a timeout as a Warning violation.
    pub fn record_timeout(&mut self, tick: u64) {
        self.violations.push(InvariantViolation {
            tick,
            severity: Severity::Warning,
            kind: "Timeout".into(),
            message: "Game exceeded max_ticks without a winner".into(),
        });
    }

    // --- Individual checks ---

    fn check_resources(&mut self, world: &mut World, tick: u64) {
        let player_res = world.resource::<PlayerResources>();
        for (i, pres) in player_res.players.iter().enumerate() {
            // Resources are u32, so can't be negative.
            // But we check supply overflow.
            if pres.supply > pres.supply_cap + 5 {
                // Allow small transient overflow during production
                self.violations.push(InvariantViolation {
                    tick,
                    severity: Severity::Error,
                    kind: "SupplyOverflow".into(),
                    message: format!(
                        "Player {i}: supply {} > cap {} + 5",
                        pres.supply, pres.supply_cap
                    ),
                });
            }
        }
    }

    fn check_health(&mut self, world: &mut World, tick: u64) {
        for (entity, health, dead) in world
            .query::<(Entity, &Health, Option<&Dead>)>()
            .iter(world)
        {
            if dead.is_some() {
                continue; // Dead units may have weird health
            }
            if health.current > health.max {
                self.violations.push(InvariantViolation {
                    tick,
                    severity: Severity::Error,
                    kind: "HealthExceedsMax".into(),
                    message: format!(
                        "Entity {:?}: health {} > max {}",
                        entity,
                        health.current.to_num::<f32>(),
                        health.max.to_num::<f32>()
                    ),
                });
            }
        }
    }

    fn check_bounds(&mut self, world: &mut World, tick: u64) {
        for (entity, grid) in world.query::<(Entity, &GridCell)>().iter(world) {
            let x = grid.pos.x;
            let y = grid.pos.y;
            if x < 0 || y < 0 || x >= self.map_width as i32 || y >= self.map_height as i32 {
                self.violations.push(InvariantViolation {
                    tick,
                    severity: Severity::Error,
                    kind: "UnitOutOfBounds".into(),
                    message: format!(
                        "Entity {:?}: grid ({}, {}) outside map {}x{}",
                        entity, x, y, self.map_width, self.map_height
                    ),
                });
            }
        }
    }

    fn check_friendly_fire(&mut self, world: &mut World, tick: u64) {
        // Collect attackers first to avoid nested borrow
        let attackers: Vec<(Entity, u8, u64)> = world
            .query::<(Entity, &Owner, &AttackTarget)>()
            .iter(world)
            .map(|(e, o, t)| (e, o.player_id, t.target.0))
            .collect();

        for (entity, attacker_player, target_bits) in &attackers {
            let target_entity = Entity::from_bits(*target_bits);
            if let Ok((target_owner,)) = world
                .query::<(&Owner,)>()
                .get(world, target_entity)
            {
                if *attacker_player == target_owner.player_id {
                    self.violations.push(InvariantViolation {
                        tick,
                        severity: Severity::Error,
                        kind: "FriendlyFire".into(),
                        message: format!(
                            "Entity {:?} (player {}) attacking {:?} (same player)",
                            entity, attacker_player, target_entity
                        ),
                    });
                }
            }
        }
    }

    fn check_stuck(&mut self, world: &mut World, tick: u64) {
        // Hash all unit positions
        let mut hasher = SimpleHasher::new();
        for (grid,) in world.query::<(&GridCell,)>().iter(world) {
            hasher.add(grid.pos.x as u64);
            hasher.add(grid.pos.y as u64);
        }
        let current_hash = hasher.finish();

        if let Some(prev) = self.prev_position_hash {
            if current_hash == prev {
                self.stuck_counter += 1;
            } else {
                self.stuck_counter = 0;
            }
        }
        self.prev_position_hash = Some(current_hash);

        // 200 consecutive invariant checks with no position changes = stuck
        // At invariant_interval=10, that's 2000 ticks = 200 seconds
        if self.stuck_counter >= 200 {
            self.violations.push(InvariantViolation {
                tick,
                severity: Severity::Warning,
                kind: "StuckState".into(),
                message: format!(
                    "No position changes for {} consecutive checks",
                    self.stuck_counter
                ),
            });
            // Reset counter so we don't spam warnings
            self.stuck_counter = 0;
        }
    }
}

/// Simple non-cryptographic hasher for position hashing.
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self { state: 0xcbf29ce484222325 }
    }

    fn add(&mut self, val: u64) {
        self.state ^= val;
        self.state = self.state.wrapping_mul(0x100000001b3);
    }

    fn finish(&self) -> u64 {
        self.state
    }
}
