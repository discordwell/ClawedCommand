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
    EnemyAaDrone,
    EnemyLauncher,
    EnemyDecoy,
    EnemySoldier,
    EnemyShaheed,
    Tanker,
    Missile,
}

// ---------------------------------------------------------------------------
// Drone modes
// ---------------------------------------------------------------------------

/// Active mode for a player drone.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DroneMode {
    /// Following assigned patrol waypoints.
    Patrol,
    /// Direct move to a position.
    MoveTo { x: f32, y: f32 },
    /// Moving to bomb a ground target.
    BombTarget { x: f32, y: f32 },
    /// Stationed at base, auto-intercepts shaheeds.
    GuardBase,
}

impl Default for DroneMode {
    fn default() -> Self {
        Self::Patrol
    }
}

// ---------------------------------------------------------------------------
// Patriot targeting mode
// ---------------------------------------------------------------------------

/// How the base Patriot interceptors select targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatriotMode {
    /// Shoot down both missiles and shaheeds (default — wasteful).
    Auto,
    /// Only engage missiles. Shaheeds must be intercepted by drones.
    MissilesOnly,
}

impl Default for PatriotMode {
    fn default() -> Self {
        Self::Auto
    }
}

// ---------------------------------------------------------------------------
// Enemy soldier phases
// ---------------------------------------------------------------------------

/// State machine for a ground soldier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoldierPhase {
    /// Advancing toward the shipping lane.
    Advancing,
    /// Within danger range of a tanker — dealing damage.
    Engaged,
}

// ---------------------------------------------------------------------------
// Enemy shaheed (suicide drone)
// ---------------------------------------------------------------------------

/// What the shaheed is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShaheedTarget {
    /// Heading for the player base.
    Base,
    /// Heading for a specific tanker (by tanker_index).
    Ship(u32),
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
    /// Tick at which this wave spawns (relative to mission start).
    pub trigger_tick: u64,
    /// How many missile launchers to spawn.
    pub launcher_count: u32,
    /// How many AA drones.
    pub aa_drone_count: u32,
    /// How many ground soldiers.
    pub soldier_count: u32,
    /// How many suicide drones (shaheeds).
    pub shaheed_count: u32,
    /// Whether the enemy coordinates diversionary attacks.
    pub coordinated: bool,
}

// ---------------------------------------------------------------------------
// Master configuration
// ---------------------------------------------------------------------------

/// All tunable parameters for the strait mission. Systems read `Res<StraitConfig>`.
/// The headless harness can pass modified configs for balance iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StraitConfig {
    // -- Map --
    pub map_width: u32,
    pub map_height: u32,

    // -- Convoy --
    pub total_tankers: u32,
    pub min_tankers_win: u32,
    pub max_tankers_lost: u32,
    pub tanker_spawn_interval: u32,
    pub tanker_hp: u32,
    pub tanker_speed: f32,

    // -- Player drones --
    pub initial_patrol_drones: u32,
    pub drone_vision_radius: i32,
    pub drone_speed: f32,
    pub drone_flare_cooldown: u32,
    pub drone_bomb_reload: u32,
    pub drone_bomb_range: f32,
    pub drone_intercept_range: f32,

    // -- Satellite --
    pub satellite_vision_radius: i32,

    // -- Drone rebuild (logistics) --
    pub drone_rebuild_ticks: u32,
    pub drone_rebuild_max_charges: u32,
    pub drone_rebuild_charge_regen_ticks: u32,

    // -- Airstrikes (logistics) --
    pub airstrike_delay_ticks: u32,
    pub airstrike_radius: i32,
    pub airstrike_max_charges: u32,
    pub airstrike_charge_regen_ticks: u32,

    // -- Patriots (finite, no regen) --
    pub initial_patriots: u32,
    pub patriot_range: f32,

    // -- Base --
    pub base_hp: u32,
    /// Base grid position (x, y).
    pub base_x: i32,
    pub base_y: i32,

    // -- Enemy: AA drones --
    pub aa_suppress_range: f32,
    pub aa_engagement_ticks: u32,
    pub aa_swarm_threshold: u32,

    // -- Enemy: Soldiers --
    pub soldier_speed: f32,
    pub soldier_danger_range: f32,
    pub soldier_hp: u32,

    // -- Enemy: Shaheeds --
    pub shaheed_speed: f32,
    pub shaheed_damage: u32,

    // -- Enemy: Missiles --
    pub missile_flight_ticks: u32,

    // -- Pre-deployed Iranian force (spawned at mission start) --
    pub deployed_launchers: u32,
    pub deployed_aa: u32,
    pub deployed_soldiers: u32,
    pub deployed_shaheeds: u32,

    // -- Reinforcement trickle (slow spawns over time) --
    /// Ticks between each reinforcement spawn.
    pub reinforcement_interval: u32,
    /// What spawns each interval: (launchers, aa, soldiers, shaheeds).
    pub reinforcement_batch: (u32, u32, u32, u32),
    /// Max total reinforcements (across all types combined).
    pub reinforcement_cap: u32,

    // -- Zero-day build ticks at 100% slice --
    pub zeroday_ticks_spoof: f32,
    pub zeroday_ticks_blind: f32,
    pub zeroday_ticks_hijack: f32,
    pub zeroday_ticks_brick: f32,

    // -- Timing --
    pub convoy_time_limit: u64,

    // -- Waves --
    pub waves: Vec<EnemyWaveConfig>,
}

impl Default for StraitConfig {
    fn default() -> Self {
        Self {
            map_width: 300,
            map_height: 60,

            total_tankers: 12,
            min_tankers_win: 8,
            max_tankers_lost: 5,
            tanker_spawn_interval: 300,
            tanker_hp: 3,
            tanker_speed: 0.15,

            initial_patrol_drones: 20,
            drone_vision_radius: 4,
            drone_speed: 8.0,
            drone_flare_cooldown: 60,
            drone_bomb_reload: 120,
            drone_bomb_range: 3.0,
            drone_intercept_range: 5.0,

            satellite_vision_radius: 8,

            drone_rebuild_ticks: 120,
            drone_rebuild_max_charges: 2,
            drone_rebuild_charge_regen_ticks: 400,

            airstrike_delay_ticks: 40,
            airstrike_radius: 3,
            airstrike_max_charges: 3,
            airstrike_charge_regen_ticks: 600,

            initial_patriots: 25,
            patriot_range: 200.0,

            base_hp: 10,
            base_x: 50,
            base_y: 50,

            aa_suppress_range: 8.0,
            aa_engagement_ticks: 50,
            aa_swarm_threshold: 3,

            soldier_speed: 0.03,
            soldier_danger_range: 2.0,
            soldier_hp: 2,

            shaheed_speed: 0.15,
            shaheed_damage: 2,

            missile_flight_ticks: 80,

            // Pre-deployed Iranian force: this is what you're clearing
            deployed_launchers: 3,
            deployed_aa: 1,
            deployed_soldiers: 6,
            deployed_shaheeds: 8,

            // Slow trickle: 1 shaheed + 1 soldier every 600 ticks
            reinforcement_interval: 600,
            reinforcement_batch: (0, 0, 1, 1),
            reinforcement_cap: 10,

            zeroday_ticks_spoof: 260.0,
            zeroday_ticks_blind: 200.0,
            zeroday_ticks_hijack: 330.0,
            zeroday_ticks_brick: 400.0,

            // Waves kept for legacy compatibility but unused by V2 director
            convoy_time_limit: 0,
            waves: vec![],
        }
    }
}

impl EnemyWaveConfig {
    /// Legacy wave constructors used by dream_strait.rs until sim extraction.
    pub fn wave_1() -> Self {
        Self { trigger_tick: 0, launcher_count: 1, aa_drone_count: 0, soldier_count: 2, shaheed_count: 3, coordinated: false }
    }
    pub fn wave_2() -> Self {
        Self { trigger_tick: 1500, launcher_count: 2, aa_drone_count: 1, soldier_count: 5, shaheed_count: 5, coordinated: false }
    }
    pub fn wave_3() -> Self {
        Self { trigger_tick: 3000, launcher_count: 2, aa_drone_count: 2, soldier_count: 5, shaheed_count: 8, coordinated: true }
    }
    pub fn wave_4() -> Self {
        Self { trigger_tick: 4500, launcher_count: 1, aa_drone_count: 3, soldier_count: 8, shaheed_count: 14, coordinated: true }
    }
}

impl StraitConfig {
    /// Zero-day build ticks for a given type at 100% allocation slice.
    pub fn zero_day_build_ticks(&self, zd: ZeroDayType) -> f32 {
        match zd {
            ZeroDayType::Spoof => self.zeroday_ticks_spoof,
            ZeroDayType::Blind => self.zeroday_ticks_blind,
            ZeroDayType::Hijack => self.zeroday_ticks_hijack,
            ZeroDayType::Brick => self.zeroday_ticks_brick,
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy constants (used by cc_client until sim extraction is complete)
// ---------------------------------------------------------------------------

pub const TOTAL_TANKERS: u32 = 12;
pub const MIN_TANKERS_WIN: u32 = 8;
pub const MAX_TANKERS_LOST: u32 = 5;
pub const INITIAL_PATROL_DRONES: u32 = 16;
pub const DRONE_VISION_RADIUS: i32 = 4;
pub const SATELLITE_VISION_RADIUS: i32 = 8;
pub const TANKER_SPAWN_INTERVAL: u32 = 300;
pub const TANKER_HP: u32 = 3;
pub const TANKER_SPEED: f32 = 0.06;
pub const MISSILE_FLIGHT_TICKS: u32 = 80;
pub const DRONE_REBUILD_TICKS: u32 = 120;
pub const DRONE_REBUILD_MAX_CHARGES: u32 = 2;
pub const DRONE_REBUILD_CHARGE_REGEN_TICKS: u32 = 400;
pub const AIRSTRIKE_DELAY_TICKS: u32 = 40;
pub const AIRSTRIKE_RADIUS: i32 = 3;
pub const AIRSTRIKE_MAX_CHARGES: u32 = 3;
pub const AIRSTRIKE_CHARGE_REGEN_TICKS: u32 = 600;
pub const CONVOY_TIME_LIMIT: u64 = 0; // disabled — soft pressure via Patriot drain
pub const INITIAL_PATRIOTS: u32 = 20;
/// Legacy: Patriots replace interceptors but old code still references these.
pub const INITIAL_INTERCEPTORS: u32 = 20;
pub const MAX_INTERCEPTORS: u32 = 20;
pub const INTERCEPTOR_REGEN_TICKS: u32 = 9999; // effectively disabled — patriots don't regen

/// Legacy function — prefer `StraitConfig::zero_day_build_ticks()`.
pub fn zero_day_build_ticks(zd: ZeroDayType) -> f32 {
    StraitConfig::default().zero_day_build_ticks(zd)
}
