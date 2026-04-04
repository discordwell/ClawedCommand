//! Data types for the Strait dream sequence (DEFCON-style drone warfare).
//!
//! Pure data — no Bevy dependency. Used by cc_core, cc_client, and cc_agent.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Entity kinds
// ---------------------------------------------------------------------------

/// Classification of entities in the strait mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StraitEntityKind {
    PatrolDrone,
    InterceptorDrone,
    EnemyAaDrone,
    EnemyLauncher,
    EnemyDecoy,
    Tanker,
    Missile,
    SatelliteScan,
}

// ---------------------------------------------------------------------------
// Compute budget
// ---------------------------------------------------------------------------

/// How the player's compute budget is allocated across capabilities.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ComputeAllocation {
    /// Fraction for drone patrol vision (0.0–1.0).
    pub drone_vision: f32,
    /// Fraction for satellite scan requests.
    pub satellite: f32,
    /// Fraction for building zero-day exploits.
    pub zero_day: f32,
}

impl Default for ComputeAllocation {
    fn default() -> Self {
        Self {
            drone_vision: 0.6,
            satellite: 0.2,
            zero_day: 0.2,
        }
    }
}

// ---------------------------------------------------------------------------
// Zero-day exploits
// ---------------------------------------------------------------------------

/// Types of zero-day exploits the player can build and deploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZeroDayType {
    /// Make enemy missiles target decoys instead of tankers.
    Spoof,
    /// Disable enemy radar/sensors in an area.
    Blind,
    /// Temporarily turn an enemy drone to your side.
    Hijack,
    /// Permanently destroy a visible enemy launcher.
    Brick,
}

/// Build state for a zero-day exploit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ZeroDayState {
    /// Currently accumulating build progress.
    Building {
        exploit_type: ZeroDayType,
        progress: f32,
        required: f32,
    },
    /// Fully built, ready to deploy.
    Ready(ZeroDayType),
    /// No active build (idle slot).
    Idle,
}

impl Default for ZeroDayState {
    fn default() -> Self {
        Self::Idle
    }
}

// ---------------------------------------------------------------------------
// Enemy launcher phases
// ---------------------------------------------------------------------------

/// State machine phase for an enemy mobile launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LauncherPhase {
    /// Concealed at a hidden position, invisible without satellite.
    Hidden,
    /// Moving to a firing position (vulnerable, detectable by drone vision).
    Setting,
    /// Actively firing missiles at the shipping lane.
    Firing,
    /// Relocating to a new hidden position after a salvo.
    Retreating,
}

// ---------------------------------------------------------------------------
// Missile state
// ---------------------------------------------------------------------------

/// In-flight state for an anti-ship missile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MissileState {
    InFlight {
        origin_x: f32,
        origin_y: f32,
        target_x: f32,
        target_y: f32,
        /// 0.0 = just launched, 1.0 = arrived at target.
        progress: f32,
    },
    Intercepted,
    /// Hit its target tanker.
    Impact,
}

// ---------------------------------------------------------------------------
// Escalation wave config
// ---------------------------------------------------------------------------

/// Configuration for one escalation wave of the enemy AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyWaveConfig {
    /// How many real launchers are active.
    pub launcher_count: u32,
    /// How many decoy launchers.
    pub decoy_count: u32,
    /// How many AA drones hunting player patrol drones.
    pub aa_drone_count: u32,
    /// Whether the enemy coordinates diversionary attacks.
    pub coordinated_diversions: bool,
}

impl EnemyWaveConfig {
    /// Wave 1: tutorial pace.
    pub fn wave_1() -> Self {
        Self {
            launcher_count: 1,
            decoy_count: 0,
            aa_drone_count: 0,
            coordinated_diversions: false,
        }
    }

    /// Wave 2: AA appears.
    pub fn wave_2() -> Self {
        Self {
            launcher_count: 2,
            decoy_count: 1,
            aa_drone_count: 1,
            coordinated_diversions: false,
        }
    }

    /// Wave 3: active hunting.
    pub fn wave_3() -> Self {
        Self {
            launcher_count: 3,
            decoy_count: 2,
            aa_drone_count: 2,
            coordinated_diversions: false,
        }
    }

    /// Wave 4: full coordination.
    pub fn wave_4() -> Self {
        Self {
            launcher_count: 4,
            decoy_count: 2,
            aa_drone_count: 2,
            coordinated_diversions: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Total tankers in the convoy.
pub const TOTAL_TANKERS: u32 = 12;
/// Minimum tankers that must arrive for victory.
pub const MIN_TANKERS_WIN: u32 = 8;
/// Maximum tankers that can be lost before defeat (12 - 5 = 7 < 8 required).
pub const MAX_TANKERS_LOST: u32 = 5;

/// Starting compute budget.
pub const INITIAL_COMPUTE: f32 = 100.0;
/// Compute regeneration per tick.
pub const COMPUTE_REGEN_PER_TICK: f32 = 0.5;

/// Starting interceptor drone count.
pub const INITIAL_INTERCEPTORS: u32 = 15;
/// Max interceptor pool size.
pub const MAX_INTERCEPTORS: u32 = 20;
/// Ticks between interceptor replenishment (+1).
pub const INTERCEPTOR_REGEN_TICKS: u32 = 60;

/// Initial patrol drone count.
pub const INITIAL_PATROL_DRONES: u32 = 16;
/// Vision radius (in tiles) for a patrol drone.
pub const DRONE_VISION_RADIUS: i32 = 4;
/// Vision radius (in tiles) for a satellite scan.
pub const SATELLITE_VISION_RADIUS: i32 = 8;
/// Ticks that a satellite scan persists.
pub const SATELLITE_SCAN_DURATION: u32 = 30;

/// Ticks between tanker spawns.
pub const TANKER_SPAWN_INTERVAL: u32 = 300;
/// Tanker HP.
pub const TANKER_HP: u32 = 3;
/// Tanker movement speed (tiles per tick).
pub const TANKER_SPEED: f32 = 0.06;

/// Missile flight time in ticks.
pub const MISSILE_FLIGHT_TICKS: u32 = 80;

/// Compute cost to rebuild a destroyed drone from Dubai base.
pub const DRONE_REBUILD_COST: f32 = 25.0;
/// Ticks to rebuild a drone once started.
pub const DRONE_REBUILD_TICKS: u32 = 120;

/// Compute cost to call an airstrike on a visible target.
pub const AIRSTRIKE_COST: f32 = 30.0;
/// Ticks for airstrike to arrive after calling it.
pub const AIRSTRIKE_DELAY_TICKS: u32 = 40;
/// Airstrike damage radius in tiles.
pub const AIRSTRIKE_RADIUS: i32 = 3;

/// Maximum ticks after convoy launch before mission auto-fails (time pressure).
pub const CONVOY_TIME_LIMIT: u64 = 8000;

/// Build cost (compute ticks) for each zero-day type.
pub fn zero_day_build_cost(zd: ZeroDayType) -> f32 {
    match zd {
        ZeroDayType::Spoof => 80.0,
        ZeroDayType::Blind => 60.0,
        ZeroDayType::Hijack => 100.0,
        ZeroDayType::Brick => 120.0,
    }
}
