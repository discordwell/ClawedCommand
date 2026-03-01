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

/// Faction affiliations in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    /// Neutral / unaligned (e.g. Kelpie before joining a faction).
    Neutral,
    /// catGPT — cats led by AI "Geppity".
    CatGpt,
    /// The Clawed — mice led by AI "Claudeus Maximus".
    TheClawed,
    /// Seekers of the Deep — badgers led by AI "Deepseek".
    SeekersOfTheDeep,
    /// The Murder — corvids led by AI "Gemineye".
    TheMurder,
    /// LLAMA — raccoons led by AI "Llhama".
    Llama,
    /// Croak — axolotls led by AI "Grok".
    Croak,
}

impl Faction {
    /// Returns the canonical string name for this faction.
    pub fn as_str(&self) -> &'static str {
        match self {
            Faction::Neutral => "neutral",
            Faction::CatGpt => "catGPT",
            Faction::TheClawed => "The Clawed",
            Faction::SeekersOfTheDeep => "Seekers of the Deep",
            Faction::TheMurder => "The Murder",
            Faction::Llama => "LLAMA",
            Faction::Croak => "Croak",
        }
    }

    /// Parse a faction from its string representation.
    pub fn from_faction_str(s: &str) -> Option<Self> {
        match s {
            "neutral" => Some(Faction::Neutral),
            "catGPT" => Some(Faction::CatGpt),
            "The Clawed" => Some(Faction::TheClawed),
            "Seekers of the Deep" => Some(Faction::SeekersOfTheDeep),
            "The Murder" => Some(Faction::TheMurder),
            "LLAMA" => Some(Faction::Llama),
            "Croak" => Some(Faction::Croak),
            _ => None,
        }
    }
}

impl std::fmt::Display for Faction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resource types in the game economy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Food,
    GpuCores,
    Nft,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for ResourceType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Food" => Ok(Self::Food),
            "GpuCores" => Ok(Self::GpuCores),
            "Nft" => Ok(Self::Nft),
            _ => Err(()),
        }
    }
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
    // --- The Murder (Corvids) ---
    /// HQ — pre-built, produces MurderScrounger, command center.
    TheParliament,
    /// Barracks — produces Sentinel, Rookclaw, Magpike, Jaycaller.
    Rookery,
    /// Resource depot — food storage.
    CarrionCache,
    /// Tech building — produces Magpyre, Jayflicker, Dusktalon, Hootseer, CorvusRex.
    AntennaArray,
    /// Research building — upgrades, unique (limit 1).
    Panopticon,
    /// Supply depot — increases supply cap.
    NestBox,
    /// Defensive wall — blocks ground, cheap and fast.
    ThornHedge,
    /// Defense tower — long-range ranged auto-attack.
    Watchtower,
    // --- Croak (Axolotls) ---
    /// HQ (Croak) — pre-built, produces Ponderer.
    TheGrotto,
    /// Barracks (Croak) — produces Regeneron, Croaker, Leapfrog, Gulper.
    SpawningPools,
    /// Resource Depot (Croak) — food drop-off.
    LilyMarket,
    /// Tech Building (Croak) — produces Eftsaber, Broodmother, Shellwarden, Bogwhisper, MurkCommander.
    SunkenServer,
    /// Research (Croak) — Croak-specific upgrades.
    FossilStones,
    /// Supply Depot (Croak) — increases supply cap.
    ReedBed,
    /// Garrison/Gate (Croak) — units enter for protection.
    TidalGate,
    /// Defense Tower (Croak) — applies Waterlogged, DoT.
    SporeTower,
}

impl std::fmt::Display for BuildingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for BuildingKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "TheBox" => Ok(Self::TheBox),
            "CatTree" => Ok(Self::CatTree),
            "FishMarket" => Ok(Self::FishMarket),
            "LitterBox" => Ok(Self::LitterBox),
            "ServerRack" => Ok(Self::ServerRack),
            "ScratchingPost" => Ok(Self::ScratchingPost),
            "CatFlap" => Ok(Self::CatFlap),
            "LaserPointer" => Ok(Self::LaserPointer),
            // The Murder (Corvids)
            "TheParliament" => Ok(Self::TheParliament),
            "Rookery" => Ok(Self::Rookery),
            "CarrionCache" => Ok(Self::CarrionCache),
            "AntennaArray" => Ok(Self::AntennaArray),
            "Panopticon" => Ok(Self::Panopticon),
            "NestBox" => Ok(Self::NestBox),
            "ThornHedge" => Ok(Self::ThornHedge),
            "Watchtower" => Ok(Self::Watchtower),
            // Croak (Axolotls)
            "TheGrotto" => Ok(Self::TheGrotto),
            "SpawningPools" => Ok(Self::SpawningPools),
            "LilyMarket" => Ok(Self::LilyMarket),
            "SunkenServer" => Ok(Self::SunkenServer),
            "FossilStones" => Ok(Self::FossilStones),
            "ReedBed" => Ok(Self::ReedBed),
            "TidalGate" => Ok(Self::TidalGate),
            "SporeTower" => Ok(Self::SporeTower),
            _ => Err(()),
        }
    }
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
    // --- The Murder (Corvids) ---
    MurderScrounger, // Worker (Crow) — gathers food, builds, scavenges
    Sentinel,      // Ranged Scout (Crow) — long-range glass cannon
    Rookclaw,      // Melee Dive Striker (Crow) — fast, bursty, fragile
    Magpike,       // Disruptor/Thief (Magpie) — steals resources, disrupts
    Magpyre,       // Saboteur (Magpie) — signal jamming, decoys, rewiring
    Jaycaller,     // Support/Buffer (Jay) — rally cry, alarm call
    Jayflicker,    // Illusion Specialist (Jay) — phantom flock, mirror position
    Dusktalon,     // Stealth Assassin (Owl) — ground-based stealth, high burst
    Hootseer,      // Area Denial/Debuffer (Owl) — panoptic gaze, dread aura
    CorvusRex,     // Hero/Heavy (Augmented Crow) — corvid network, all-seeing lie
    // --- The Clawed (Mice) ---
    Nibblet,       // Worker (Mouse) — gathers food, builds
    Swarmer,       // Light Infantry (Mouse) — cheap, fast attack speed, swarm in numbers
    Gnawer,        // Anti-Structure (Mouse) — Structural Weakness passive vs buildings
    Shrieker,      // Ranged Harasser (Shrew) — cone attack, fragile
    Tunneler,      // Transport/Utility (Vole) — burrow express, tremor sense
    Sparks,        // Saboteur (Mouse) — static charge burst after movement
    Quillback,     // Heavy Defender (Hedgehog) — spine wall DR, stubborn advance
    Whiskerwitch,  // Caster/Support (Shrew) — hex of multiplication, whisker weave
    Plaguetail,    // Area Denial (Mouse) — contagion cloud on death
    WarrenMarshal, // Hero/Commander (Mouse) — rally the swarm aura, whiskernet relay
    // --- Croak (Axolotls) ---
    Ponderer,       // Worker (Croak) — gathers via ambient gathering on water
    Regeneron,      // Light Skirmisher (Croak) — Limb Economy, self-regen
    Broodmother,    // Healer/Support (Croak) — spawns Spawnlings, burst heals
    Gulper,         // Heavy Bruiser (Croak) — Devour mechanic, massive regen
    Eftsaber,       // Assassin/Flanker (Croak) — poison, Waterway stealth
    Croaker,        // Ranged Artillery (Croak) — Bog Mortar, terrain creation
    Leapfrog,       // Mobile Harasser (Croak) — Hop chains on water
    Shellwarden,    // Tank/Defender (Croak) — Hunker, Ancient Moss aura
    Bogwhisper,     // Support/Caster (Croak) — Mire Curse, Prophecy
    MurkCommander,  // Hero/Heavy (Croak) — Grok Protocol, Murk Uplink
}

impl std::fmt::Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for UnitKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Pawdler" => Ok(Self::Pawdler),
            "Nuisance" => Ok(Self::Nuisance),
            "Chonk" => Ok(Self::Chonk),
            "FlyingFox" => Ok(Self::FlyingFox),
            "Hisser" => Ok(Self::Hisser),
            "Yowler" => Ok(Self::Yowler),
            "Mouser" => Ok(Self::Mouser),
            "Catnapper" => Ok(Self::Catnapper),
            "FerretSapper" => Ok(Self::FerretSapper),
            "MechCommander" => Ok(Self::MechCommander),
            // The Murder (Corvids)
            "MurderScrounger" => Ok(Self::MurderScrounger),
            "Sentinel" => Ok(Self::Sentinel),
            "Rookclaw" => Ok(Self::Rookclaw),
            "Magpike" => Ok(Self::Magpike),
            "Magpyre" => Ok(Self::Magpyre),
            "Jaycaller" => Ok(Self::Jaycaller),
            "Jayflicker" => Ok(Self::Jayflicker),
            "Dusktalon" => Ok(Self::Dusktalon),
            "Hootseer" => Ok(Self::Hootseer),
            "CorvusRex" => Ok(Self::CorvusRex),
            // Croak (Axolotls)
            "Ponderer" => Ok(Self::Ponderer),
            "Regeneron" => Ok(Self::Regeneron),
            "Broodmother" => Ok(Self::Broodmother),
            "Gulper" => Ok(Self::Gulper),
            "Eftsaber" => Ok(Self::Eftsaber),
            "Croaker" => Ok(Self::Croaker),
            "Leapfrog" => Ok(Self::Leapfrog),
            "Shellwarden" => Ok(Self::Shellwarden),
            "Bogwhisper" => Ok(Self::Bogwhisper),
            "MurkCommander" => Ok(Self::MurkCommander),
            _ => Err(()),
        }
    }
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

impl std::fmt::Display for AttackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::str::FromStr for AttackType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Melee" => Ok(Self::Melee),
            "Ranged" => Ok(Self::Ranged),
            _ => Err(()),
        }
    }
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
// Builder components
// ---------------------------------------------------------------------------

/// A pending build order attached to a builder unit.
/// The builder walks to the target position and spawns the building on arrival.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct BuildOrder {
    pub building_kind: BuildingKind,
    pub position: GridPos,
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
    /// Last known position for staleness detection (world-space x,y).
    pub last_pos: (Fixed, Fixed),
    /// Number of consecutive ticks with no positional progress while moving.
    /// When this exceeds `GATHERER_STALE_TICKS`, the Gathering component is
    /// removed so the worker can be reassigned.
    pub stale_ticks: u32,
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
    pub cannot_attack: bool,
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
            cannot_attack: false,
        }
    }
}

/// The type of aura a unit emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraType {
    GravitationalChonk,
    HarmonicResonance,
    // The Murder (Corvids)
    DreadAura,
    CorvidNetwork,
    OculusUplink,
    Lullaby,
    ContagiousYawning,
    TacticalUplink,
    GeppityUplink,
    // Croak (Axolotls)
    AncientMoss,
    BogSong,
    UndyingPresence,
    MurkUplinkAura,
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

/// Tracks Catnapper's DreamSiege passive — damage ramps the longer it attacks the same target.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct DreamSiegeTimer {
    pub ticks_on_target: u32,
    pub current_target: Option<EntityId>,
    pub last_hp: Fixed,
}

impl Default for DreamSiegeTimer {
    fn default() -> Self {
        Self {
            ticks_on_target: 0,
            current_target: None,
            last_hp: Fixed::ZERO,
        }
    }
}

/// Tracks Chonk's NineLives passive — revives once on lethal damage.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct NineLivesTracker {
    /// Tick when last triggered (0 = never triggered).
    pub last_triggered_tick: u64,
}

impl Default for NineLivesTracker {
    fn default() -> Self {
        Self {
            last_triggered_tick: 0,
        }
    }
}

/// A hairball obstacle spawned by Nuisance — blocks terrain for a limited time.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct HairballObstacle {
    pub remaining_ticks: u32,
    pub owner_player_id: u8,
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
    // --- The Murder (Corvids) ---
    /// +2 damage for all Murder combat units.
    SharperTalons,
    /// +20 HP for all Murder combat units.
    HardenedPlumage,
    /// +10% speed for all Murder units.
    SwiftWings,
    /// Unlocks Dusktalon training at AntennaArray.
    AssassinTraining,
    /// Unlocks CorvusRex training at AntennaArray.
    RexPrototype,
}

impl std::fmt::Display for UpgradeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SharperClaws => write!(f, "SharperClaws"),
            Self::ThickerFur => write!(f, "ThickerFur"),
            Self::NimblePaws => write!(f, "NimblePaws"),
            Self::SiegeTraining => write!(f, "SiegeTraining"),
            Self::MechPrototype => write!(f, "MechPrototype"),
            Self::SharperTalons => write!(f, "SharperTalons"),
            Self::HardenedPlumage => write!(f, "HardenedPlumage"),
            Self::SwiftWings => write!(f, "SwiftWings"),
            Self::AssassinTraining => write!(f, "AssassinTraining"),
            Self::RexPrototype => write!(f, "RexPrototype"),
        }
    }
}

impl std::str::FromStr for UpgradeType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "SharperClaws" => Ok(Self::SharperClaws),
            "ThickerFur" => Ok(Self::ThickerFur),
            "NimblePaws" => Ok(Self::NimblePaws),
            "SiegeTraining" => Ok(Self::SiegeTraining),
            "MechPrototype" => Ok(Self::MechPrototype),
            "SharperTalons" => Ok(Self::SharperTalons),
            "HardenedPlumage" => Ok(Self::HardenedPlumage),
            "SwiftWings" => Ok(Self::SwiftWings),
            "AssassinTraining" => Ok(Self::AssassinTraining),
            "RexPrototype" => Ok(Self::RexPrototype),
            _ => Err(()),
        }
    }
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

/// Marks an entity as belonging to a specific enemy wave (for WaveEliminated tracking).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct WaveMember {
    pub wave_id: String,
}


// ---------------------------------------------------------------------------
// Murder faction components
// ---------------------------------------------------------------------------

/// Marker: this unit is aerial -- ignores terrain pathing, immune to melee unless Grounded.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Aerial;

/// Marker: aerial unit is temporarily grounded -- can be hit by melee, cannot fly.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Grounded {
    pub remaining_ticks: u32,
}

/// Murder-specific debuff: target is visible through fog to all Murder units.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Exposed {
    pub remaining_ticks: u32,
    pub source_player: u8,
}

/// Tracks Murder's Mark debuff on an enemy unit.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MurdersMarkDebuff {
    pub remaining_ticks: u32,
}

/// Tracks Magpike's Trinket Ward passive stacking.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct TrinketWardTracker {
    pub trinkets_collected: u32,
}

/// Tracks Hootseer's Panoptic Gaze cone direction.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct PanopticGazeCone {
    pub direction: Fixed,
    pub half_angle: Fixed,
}

/// Unique building limit tracker (e.g., Panopticon is limit-1).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct UniqueBuildingLimit;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_display_from_str_round_trip() {
        for rt in [ResourceType::Food, ResourceType::GpuCores, ResourceType::Nft] {
            let s = rt.to_string();
            let parsed: ResourceType = s.parse().unwrap();
            assert_eq!(parsed, rt);
        }
        assert!("Bogus".parse::<ResourceType>().is_err());
    }

    #[test]
    fn building_kind_display_from_str_round_trip() {
        for kind in [
            BuildingKind::TheBox, BuildingKind::CatTree, BuildingKind::FishMarket,
            BuildingKind::LitterBox, BuildingKind::ServerRack, BuildingKind::ScratchingPost,
            BuildingKind::CatFlap, BuildingKind::LaserPointer,
        ] {
            let s = kind.to_string();
            let parsed: BuildingKind = s.parse().unwrap();
            assert_eq!(parsed, kind);
        }
        assert!("Bogus".parse::<BuildingKind>().is_err());
    }

    #[test]
    fn unit_kind_display_from_str_round_trip() {
        for kind in [
            UnitKind::Pawdler, UnitKind::Nuisance, UnitKind::Chonk, UnitKind::FlyingFox,
            UnitKind::Hisser, UnitKind::Yowler, UnitKind::Mouser, UnitKind::Catnapper,
            UnitKind::FerretSapper, UnitKind::MechCommander,
        ] {
            let s = kind.to_string();
            let parsed: UnitKind = s.parse().unwrap();
            assert_eq!(parsed, kind);
        }
        assert!("Bogus".parse::<UnitKind>().is_err());
    }

    #[test]
    fn attack_type_display_from_str_round_trip() {
        for at in [AttackType::Melee, AttackType::Ranged] {
            let s = at.to_string();
            let parsed: AttackType = s.parse().unwrap();
            assert_eq!(parsed, at);
        }
        assert!("Bogus".parse::<AttackType>().is_err());
    }

    #[test]
    fn dream_siege_timer_defaults() {
        let timer = DreamSiegeTimer::default();
        assert_eq!(timer.ticks_on_target, 0);
        assert!(timer.current_target.is_none());
    }

    #[test]
    fn stat_modifiers_cannot_attack_default_false() {
        let mods = StatModifiers::default();
        assert!(!mods.cannot_attack);
    }

    #[test]
    fn faction_as_str_round_trip() {
        let factions = [
            Faction::Neutral, Faction::CatGpt, Faction::TheClawed,
            Faction::SeekersOfTheDeep, Faction::TheMurder, Faction::Llama, Faction::Croak,
        ];
        for f in factions {
            let s = f.as_str();
            let parsed = Faction::from_faction_str(s).unwrap();
            assert_eq!(parsed, f);
        }
    }

    #[test]
    fn faction_display_matches_as_str() {
        for f in [Faction::CatGpt, Faction::Llama, Faction::Croak] {
            assert_eq!(format!("{f}"), f.as_str());
        }
    }

    #[test]
    fn faction_from_unknown_returns_none() {
        assert!(Faction::from_faction_str("bogus").is_none());
    }

    #[test]
    fn nine_lives_tracker_default() {
        let tracker = NineLivesTracker::default();
        assert_eq!(tracker.last_triggered_tick, 0);
    }

    #[test]
    fn wave_member_stores_wave_id() {
        let wm = WaveMember { wave_id: "wave_1".into() };
        assert_eq!(wm.wave_id, "wave_1");
    }
}
