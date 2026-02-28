//! Centralized tuning constants for gameplay balance.
//!
//! All magic numbers that affect gameplay feel (tick counts, ranges, amounts,
//! speeds, intervals) live here so designers can tweak the game from one place.

use crate::math::Fixed;

// ---------------------------------------------------------------------------
// Economy / Gathering
// ---------------------------------------------------------------------------

/// How many ticks a Pawdler takes to harvest a load of resources.
pub const HARVEST_TICKS: u32 = 15;

/// How many resource units a Pawdler carries per trip.
pub const CARRY_AMOUNT: u32 = 10;

/// Maximum ticks a gatherer can be stuck (Gathering + MoveTarget, no positional
/// progress) before the Gathering component is removed so it can be reassigned.
pub const GATHERER_STALE_TICKS: u32 = 30;

/// Maximum distance-squared from a drop-off building for a ReturningToBase
/// worker to deposit resources. Approximately 2 tiles (2^2 = 4).
pub const DROPOFF_PROXIMITY_SQ: i32 = 4;

// ---------------------------------------------------------------------------
// Combat
// ---------------------------------------------------------------------------

/// Projectile speed in 16.16 fixed-point. 0.5 = 1 << 15 = 32768 bits.
pub const PROJECTILE_SPEED: Fixed = Fixed::from_bits(1 << 15);

/// Projectile speed for tower (LaserPointer) projectiles.
pub const TOWER_PROJECTILE_SPEED: Fixed = Fixed::from_bits(1 << 15);

/// Sight range (tiles) for units on AttackMove — scan this far for enemies.
pub const ATTACK_MOVE_SIGHT_RANGE: i32 = 8;

// ---------------------------------------------------------------------------
// Status Effects
// ---------------------------------------------------------------------------

/// CC immunity duration granted after a crowd-control effect expires (ticks).
pub const CC_IMMUNITY_TICKS: u32 = 10;

// ---------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------

/// Ticks between re-issuing attack orders during the AI Attack phase (5s at 10hz).
pub const ATTACK_REISSUE_INTERVAL: u64 = 50;

/// Chebyshev distance (tiles) for detecting enemy threats near the AI's base.
pub const BASE_THREAT_RADIUS: i32 = 8;
