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
// Multi-faction Abilities
// ---------------------------------------------------------------------------

/// QuillBurst (Quillback/Clawed): AoE damage.
pub const QUILL_BURST_DAMAGE: Fixed = Fixed::from_bits(20 << 16);
/// QuillBurst: AoE radius (tiles).
pub const QUILL_BURST_RADIUS: Fixed = Fixed::from_bits(2 << 16);

/// ScorchedEarth (Embermaw/Seekers): AoE damage.
pub const SCORCHED_EARTH_DAMAGE: Fixed = Fixed::from_bits(25 << 16);
/// ScorchedEarth: AoE radius (tiles).
pub const SCORCHED_EARTH_RADIUS: Fixed = Fixed::from_bits(4 << 16);

/// SeismicSlam (Cragback/Seekers): AoE stun duration (ticks).
pub const SEISMIC_SLAM_STUN_TICKS: u32 = 20;

/// TalonDive (Rookclaw/Murder): AoE damage on impact.
pub const TALON_DIVE_DAMAGE: Fixed = Fixed::from_bits(30 << 16);
/// TalonDive: impact AoE radius (tiles).
pub const TALON_DIVE_RADIUS: Fixed = Fixed::from_bits(1 << 16);

/// WreckBall (HeapTitan/LLAMA): AoE damage.
pub const WRECK_BALL_DAMAGE: Fixed = Fixed::from_bits(30 << 16);
/// WreckBall: AoE radius (tiles).
pub const WRECK_BALL_RADIUS: Fixed = Fixed::from_bits(3 << 16);

/// ChainBreak (Wrecker/LLAMA): AoE damage with building bonus.
pub const CHAIN_BREAK_DAMAGE: Fixed = Fixed::from_bits(15 << 16);
/// ChainBreak: AoE radius (tiles).
pub const CHAIN_BREAK_RADIUS: Fixed = Fixed::from_bits(3 << 16);
/// ChainBreak: damage multiplier vs buildings.
pub const CHAIN_BREAK_BUILDING_MULT: Fixed = Fixed::from_bits(2 << 16);

/// LimbToss (Regeneron/Croak): single-target ranged damage.
pub const LIMB_TOSS_DAMAGE: Fixed = Fixed::from_bits(10 << 16);

/// Venomstrike (Eftsaber/Croak): single-target damage + Waterlogged.
pub const VENOMSTRIKE_DAMAGE: Fixed = Fixed::from_bits(15 << 16);
/// Venomstrike: Waterlogged debuff duration (ticks).
pub const VENOMSTRIKE_WATERLOGGED_TICKS: u32 = 60;

/// ShortCircuit (Sparks/The Clawed): AoE silence duration (ticks).
pub const SHORT_CIRCUIT_SILENCE_TICKS: u32 = 30;

/// SonicSpit (Shrieker/The Clawed): AoE stun duration (ticks).
pub const SONIC_SPIT_STUN_TICKS: u32 = 15;

/// DeepBore (SeekerTunneler/Seekers): long-range bore AoE damage.
pub const DEEP_BORE_DAMAGE: Fixed = Fixed::from_bits(20 << 16);

/// SilentStrike (Dusktalon/Murder): assassin burst AoE damage.
pub const SILENT_STRIKE_DAMAGE: Fixed = Fixed::from_bits(25 << 16);

/// SalvageTurret (GreaseMonkey/LLAMA): turret burst AoE damage.
pub const SALVAGE_TURRET_DAMAGE: Fixed = Fixed::from_bits(15 << 16);

/// FrankensteinProtocol (JunkyardKing/LLAMA): construct burst AoE damage.
pub const FRANKENSTEIN_DAMAGE: Fixed = Fixed::from_bits(20 << 16);

/// Regurgitate (Gulper/Croak): spit AoE damage.
pub const REGURGITATE_DAMAGE: Fixed = Fixed::from_bits(20 << 16);

// ---------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------

/// Ticks between re-issuing attack orders during the AI Attack phase (5s at 10hz).
pub const ATTACK_REISSUE_INTERVAL: u64 = 50;

/// Chebyshev distance (tiles) for detecting enemy threats near the AI's base.
pub const BASE_THREAT_RADIUS: i32 = 8;

/// Minimum Chebyshev distance between AI building placements (tiles).
pub const AI_BUILD_SPACING: i32 = 3;

/// Maximum workers the AI trains during MidGame phase.
pub const AI_MAX_MIDGAME_WORKERS: u32 = 6;

/// Chebyshev distance (tiles) for AI focus-fire target search.
pub const AI_FOCUS_FIRE_RADIUS: i32 = 15;

/// Flanking perpendicular offset (tiles) for AI attack maneuvers.
pub const AI_FLANK_OFFSET_TILES: i32 = 5;

/// Forward offset (tiles) for melee units during Rally positioning.
pub const AI_MELEE_FORWARD_OFFSET: i32 = 2;

// ---------------------------------------------------------------------------
// Tower Defense Building Combat Stats
// ---------------------------------------------------------------------------

/// LaserPointer (catGPT) tower damage.
pub const TOWER_DAMAGE_LASER_POINTER: Fixed = Fixed::from_bits(10 << 16);
/// LaserPointer (catGPT) tower range.
pub const TOWER_RANGE_LASER_POINTER: Fixed = Fixed::from_bits(6 << 16);
/// LaserPointer (catGPT) tower attack speed (ticks between attacks).
pub const TOWER_ATTACK_SPEED_LASER_POINTER: u32 = 15;

/// SporeTower (Croak) tower damage.
pub const TOWER_DAMAGE_SPORE_TOWER: Fixed = Fixed::from_bits(8 << 16);
/// SporeTower (Croak) tower range.
pub const TOWER_RANGE_SPORE_TOWER: Fixed = Fixed::from_bits(5 << 16);
/// SporeTower (Croak) tower attack speed.
pub const TOWER_ATTACK_SPEED_SPORE_TOWER: u32 = 15;

/// TetanusTower (LLAMA) tower damage.
pub const TOWER_DAMAGE_TETANUS_TOWER: Fixed = Fixed::from_bits(8 << 16);
/// TetanusTower (LLAMA) tower range.
pub const TOWER_RANGE_TETANUS_TOWER: Fixed = Fixed::from_bits(5 << 16);
/// TetanusTower (LLAMA) tower attack speed.
pub const TOWER_ATTACK_SPEED_TETANUS_TOWER: u32 = 12;

/// Watchtower (Murder) tower damage.
pub const TOWER_DAMAGE_WATCHTOWER: Fixed = Fixed::from_bits(12 << 16);
/// Watchtower (Murder) tower range.
pub const TOWER_RANGE_WATCHTOWER: Fixed = Fixed::from_bits(7 << 16);
/// Watchtower (Murder) tower attack speed.
pub const TOWER_ATTACK_SPEED_WATCHTOWER: u32 = 18;

/// SqueakTower (Clawed) tower damage.
pub const TOWER_DAMAGE_SQUEAK_TOWER: Fixed = Fixed::from_bits(8 << 16);
/// SqueakTower (Clawed) tower range.
pub const TOWER_RANGE_SQUEAK_TOWER: Fixed = Fixed::from_bits(5 << 16);
/// SqueakTower (Clawed) tower attack speed.
pub const TOWER_ATTACK_SPEED_SQUEAK_TOWER: u32 = 15;

/// SlagThrower (Seekers) tower damage.
pub const TOWER_DAMAGE_SLAG_THROWER: Fixed = Fixed::from_bits(15 << 16);
/// SlagThrower (Seekers) tower range.
pub const TOWER_RANGE_SLAG_THROWER: Fixed = Fixed::from_bits(7 << 16);
/// SlagThrower (Seekers) tower attack speed (slow AoE).
pub const TOWER_ATTACK_SPEED_SLAG_THROWER: u32 = 30;
