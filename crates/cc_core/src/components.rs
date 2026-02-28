use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::abilities::AbilityId;
use crate::commands::EntityId;
use crate::coords::{GridPos, WorldPos};
use crate::hero::HeroId;
use crate::math::Fixed;

// ---------------------------------------------------------------------------
// Economy / Building enums
// ---------------------------------------------------------------------------

/// Resource types in the game economy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Food,
    GpuCores,
    Nft,
}

/// Building types for the cat faction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingKind {
    /// HQ — pre-built, produces Pawdler, resource drop-off.
    TheBox,
    /// Barracks — produces basic combat units (Nuisance, Hisser, Chonk, Yowler).
    CatTree,
    /// Resource drop-off + slight gather bonus.
    FishMarket,
    /// Supply depot — increases supply cap.
    LitterBox,
    /// Advanced tech building — produces FlyingFox, Mouser, Catnapper, FerretSapper, MechCommander.
    ServerRack,
    /// Research building — upgrades (SharperClaws, ThickerFur, etc.).
    ScratchingPost,
    /// Garrison building — units enter for protection (garrison mechanic deferred).
    CatFlap,
    /// Defensive tower — auto-attacks enemies in range.
    LaserPointer,
}

/// State machine for the Pawdler gather loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherState {
    MovingToDeposit,
    Harvesting { ticks_remaining: u32 },
    ReturningToBase,
}

/// World-space position of an entity.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Position {
    pub world: WorldPos,
}

/// Velocity in world-units per tick.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Velocity {
    pub dx: Fixed,
    pub dy: Fixed,
}

impl Velocity {
    pub fn zero() -> Self {
        Self {
            dx: Fixed::ZERO,
            dy: Fixed::ZERO,
        }
    }
}

/// Cached grid cell, recomputed from Position each tick.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct GridCell {
    pub pos: GridPos,
}

/// Which player owns this entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Owner {
    pub player_id: u8,
}

/// The kind of unit (cat faction roster — see GAME_DESIGN.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitKind {
    Pawdler,       // Worker (Cat) — gathers food, builds, scrounges GPU cores
    Nuisance,      // Light Harasser (Cat) — fast, cheap, debuffs enemies
    Chonk,         // Heavy Tank (Fat Cat) — immovable, absorbs damage, blocks pathing
    FlyingFox,     // Air Scout/Striker (Fruit Bat) — flies over terrain, night vision
    Hisser,        // Ranged (Cat) — medium-range spitter
    Yowler,        // Support (Cat) — buffs allies, debuffs enemies in range
    Mouser,        // Stealth Scout (Cat) — fast, stealthy, reveals fog
    Catnapper,     // Siege (Cat) — sleeps on buildings until they collapse
    FerretSapper,  // Demolitions (Ferret) — plants explosives, fast building destruction
    MechCommander, // Hero/Heavy (Cat in Mech) — late-game, commands nearby units
}

/// Identifies what type of unit this entity is.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct UnitType {
    pub kind: UnitKind,
}

/// Hit points.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Health {
    pub current: Fixed,
    pub max: Fixed,
}

/// How fast this unit moves (world-units per tick).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MovementSpeed {
    pub speed: Fixed,
}

/// Marker: this unit is currently selected by the player.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Selected;

/// The unit is moving toward this world position (simple direct move).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MoveTarget {
    pub target: WorldPos,
}

/// A* path the unit is following.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Path {
    pub waypoints: VecDeque<GridPos>,
}

// ---------------------------------------------------------------------------
// Combat components
// ---------------------------------------------------------------------------

/// Whether a unit attacks in melee or at range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Melee,
    Ranged,
}

/// Combat statistics for a unit.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackStats {
    pub damage: Fixed,
    pub range: Fixed,
    pub attack_speed: u32,        // ticks between attacks
    pub cooldown_remaining: u32,  // ticks until next attack
}

/// Marker: which attack type this unit uses.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackTypeMarker {
    pub attack_type: AttackType,
}

/// The entity this unit is targeting for attack.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackTarget {
    pub target: EntityId,
}

/// Marker: unit is chasing a target to get into attack range.
/// Distinguished from player-issued MoveTarget so Stop clears it.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ChasingTarget {
    pub target: EntityId,
}

/// A projectile in flight.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Projectile {
    pub damage: Fixed,
    pub speed: Fixed,
}

/// Which entity this projectile is homing toward.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ProjectileTarget {
    pub target: EntityId,
}

/// Marker: this entity is dead (awaiting despawn next tick).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Dead;

/// Marker: unit should hold position (attack in range only, no chasing).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct HoldPosition;

/// Unit is attack-moving toward a grid position (engages enemies along the way).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AttackMoveTarget {
    pub target: GridPos,
}

// ---------------------------------------------------------------------------
// Economy / Building components
// ---------------------------------------------------------------------------

/// A resource deposit on the map (fish pond, berry bush, GPU deposit, monkey mine).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ResourceDeposit {
    pub resource_type: ResourceType,
    pub remaining: u32,
}

/// Marker: this entity is a building.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Building {
    pub kind: BuildingKind,
}

/// Building is under construction. Ticks down each sim tick.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct UnderConstruction {
    pub remaining_ticks: u32,
    pub total_ticks: u32,
}

/// Production queue for a building that can train units.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ProductionQueue {
    pub queue: VecDeque<(UnitKind, u32)>, // (kind, ticks_remaining)
}

impl Default for ProductionQueue {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

/// Rally point for a production building — new units move here after spawning.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct RallyPoint {
    pub target: GridPos,
}

/// Pawdler is gathering resources — tracks the gather loop state machine.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Gathering {
    pub deposit_entity: EntityId,
    pub carried_type: ResourceType,
    pub carried_amount: u32,
    pub state: GatherState,
}

/// Marker: this building can produce units (has been fully constructed).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Producer;

// ---------------------------------------------------------------------------
// Ability / Status Effect components
// ---------------------------------------------------------------------------

/// Runtime state for a single ability slot.
#[derive(Debug, Clone, Copy)]
pub struct AbilityState {
    pub id: AbilityId,
    /// Ticks remaining before ability can be used again.
    pub cooldown_remaining: u32,
    /// Whether this ability is currently active (toggle on, or activated duration running).
    pub active: bool,
    /// Ticks remaining on the active duration (for Activated abilities).
    pub duration_remaining: u32,
    /// Current charges (for charge-based abilities).
    pub charges: u32,
}

/// The 3 ability slots for a unit.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct AbilitySlots {
    pub slots: [AbilityState; 3],
}

impl AbilitySlots {
    /// Create ability slots from a unit's ability IDs, initialized to ready state.
    pub fn from_abilities(ids: [AbilityId; 3]) -> Self {
        Self {
            slots: ids.map(|id| {
                let def = crate::abilities::ability_def(id);
                AbilityState {
                    id,
                    cooldown_remaining: 0,
                    active: false,
                    duration_remaining: 0,
                    charges: def.max_charges,
                }
            }),
        }
    }
}

/// Aggregate stat modifiers computed from status effects each tick.
/// Multiplicative modifiers default to 1.0 (FIXED_ONE), boolean flags to false.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct StatModifiers {
    pub speed_multiplier: Fixed,
    pub damage_multiplier: Fixed,
    pub attack_speed_multiplier: Fixed,
    pub damage_reduction: Fixed,
    pub gather_speed_multiplier: Fixed,
    pub cooldown_multiplier: Fixed,
    pub invulnerable: bool,
    pub immobilized: bool,
    pub silenced: bool,
}

impl Default for StatModifiers {
    fn default() -> Self {
        Self {
            speed_multiplier: Fixed::ONE,
            damage_multiplier: Fixed::ONE,
            attack_speed_multiplier: Fixed::ONE,
            damage_reduction: Fixed::ONE,
            gather_speed_multiplier: Fixed::ONE,
            cooldown_multiplier: Fixed::ONE,
            invulnerable: false,
            immobilized: false,
            silenced: false,
        }
    }
}

/// The type of aura a unit emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraType {
    GravitationalChonk,
    HarmonicResonance,
    Lullaby,
    ContagiousYawning,
    TacticalUplink,
    MinstralUplink,
}

/// Component for units that emit an area-of-effect aura.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Aura {
    pub aura_type: AuraType,
    pub radius: Fixed,
    pub active: bool,
}

/// Component for stealth-capable units.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Stealth {
    pub stealthed: bool,
    pub detection_radius: Fixed,
}

/// Component for entities visible through fog of war.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct VisibleThroughFog {
    pub remaining_ticks: u32,
}

// ---------------------------------------------------------------------------
// Research / Upgrade components
// ---------------------------------------------------------------------------

/// Available upgrade types that can be researched at the ScratchingPost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeType {
    /// +2 damage for all combat units.
    SharperClaws,
    /// +25 HP for all combat units.
    ThickerFur,
    /// +10% speed for all units.
    NimblePaws,
    /// Unlocks Catnapper training at ServerRack.
    SiegeTraining,
    /// Unlocks MechCommander training at ServerRack.
    MechPrototype,
}

/// Research queue for a ScratchingPost (parallels ProductionQueue).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct ResearchQueue {
    pub queue: VecDeque<(UpgradeType, u32)>, // (upgrade, ticks_remaining)
}

impl Default for ResearchQueue {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

/// Marker: this building can perform research (fully constructed ScratchingPost).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Researcher;

// ---------------------------------------------------------------------------
// Hero components
// ---------------------------------------------------------------------------

/// Marks an entity as a named hero character.
/// Heroes are regular units with boosted stats and story significance.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct HeroIdentity {
    pub hero_id: HeroId,
    /// If true, mission fails when this hero dies.
    pub mission_critical: bool,
}
