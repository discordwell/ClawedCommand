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
// Building
// ---------------------------------------------------------------------------

/// Chebyshev distance (tiles) a builder must be within to start construction.
pub const BUILDER_PROXIMITY: i32 = 1;

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
// Abilities (Phase 4C)
// ---------------------------------------------------------------------------

/// GravitationalChonk: pull speed toward Chonk per tick (~0.03 tiles/tick).
pub const GRAV_PULL_PER_TICK: Fixed = Fixed::from_bits(1966);

/// NineLives: revive HP fraction (30% of max).
pub const NINE_LIVES_HP_FRACTION: Fixed = Fixed::from_bits(19661);

/// NineLives: GPU cost to trigger.
pub const NINE_LIVES_GPU_COST: u32 = 25;

/// NineLives: invulnerability duration after revive (ticks).
pub const NINE_LIVES_REVIVE_TICKS: u32 = 30;

/// NineLives: minimum ticks between triggers.
pub const NINE_LIVES_COOLDOWN_TICKS: u64 = 600;

/// Hairball obstacle lifetime (ticks).
pub const HAIRBALL_DURATION_TICKS: u32 = 100;

/// PowerNap: generate 1 GPU every N ticks (0.5 GPU/tick average).
pub const POWER_NAP_GPU_INTERVAL: u32 = 2;

/// DisgustMortar: AoE damage.
pub const DISGUST_MORTAR_DAMAGE: Fixed = Fixed::from_bits(15 << 16);

/// DisgustMortar: AoE radius (tiles).
pub const DISGUST_MORTAR_RADIUS: Fixed = Fixed::from_bits(2 << 16);

/// ShapedCharge: base damage.
pub const SHAPED_CHARGE_DAMAGE: Fixed = Fixed::from_bits(40 << 16);

/// ShapedCharge: AoE radius (tiles).
pub const SHAPED_CHARGE_RADIUS: Fixed = Fixed::from_bits(2 << 16);

/// ShapedCharge: damage multiplier vs buildings.
pub const SHAPED_CHARGE_BUILDING_MULT: Fixed = Fixed::from_bits(3 << 16);

/// EcholocationPulse: reveal duration (ticks).
pub const ECHOLOCATION_REVEAL_TICKS: u32 = 20;

// ---------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------

/// Ticks between re-issuing attack orders during the AI Attack phase (5s at 10hz).
pub const ATTACK_REISSUE_INTERVAL: u64 = 50;

/// Chebyshev distance (tiles) for detecting enemy threats near the AI's base.
pub const BASE_THREAT_RADIUS: i32 = 8;

/// Minimum Chebyshev distance between AI building placements (tiles).
pub const AI_BUILD_SPACING: i32 = 3;
