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
    // --- The Clawed (Mice) ---
    /// HQ (Clawed) — pre-built, produces Nibblet, resource drop-off.
    TheBurrow,
    /// Barracks (Clawed) — produces Swarmer, Gnawer, Plaguetail, Sparks.
    NestingBox,
    /// Resource drop-off + food storage.
    SeedVault,
    /// Tech building (Clawed) — produces Shrieker, Tunneler, Quillback, Whiskerwitch, WarrenMarshal.
    JunkTransmitter,
    /// Research building (Clawed) — upgrades.
    GnawLab,
    /// Supply depot (Clawed) — increases supply cap.
    WarrenExpansion,
    /// Garrison building (Clawed) — units enter for protection.
    Mousehole,
    /// Defensive tower (Clawed) — auto-attacks enemies in range.
    SqueakTower,
    // --- Seekers of the Deep (Badgers) ---
    TheSett,
    WarHollow,
    BurrowDepot,
    CoreTap,
    ClawMarks,
    DeepWarren,
    BulwarkGate,
    SlagThrower,
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
    // --- LLAMA (Raccoons) ---
    TheDumpster,   // HQ (LLAMA) — pre-built, produces Scrounger
    ScrapHeap,     // Resource Depot (LLAMA) — food/scrap storage
    ChopShop,      // Barracks (LLAMA) — trains Bandit, Wrecker, HeapTitan, GreaseMonkey
    JunkServer,    // Tech Building (LLAMA) — produces GlitchRat, PatchPossum
    TinkerBench,   // Research (LLAMA) — produces DeadDropUnit, DumpsterDiver, JunkyardKing
    TrashPile,     // Supply Depot (LLAMA) — increases supply cap
    DumpsterRelay, // Comms Tower (LLAMA) — reduces leak chance, +3 vision
    TetanusTower,  // Defense Tower (LLAMA) — shoots rusty nails, applies Corroded
}

impl BuildingKind {
    /// Returns a human-readable display name for this building kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            // catGPT
            BuildingKind::TheBox => "The Box",
            BuildingKind::CatTree => "Cat Tree",
            BuildingKind::FishMarket => "Fish Market",
            BuildingKind::LitterBox => "Litter Box",
            BuildingKind::ServerRack => "Server Rack",
            BuildingKind::ScratchingPost => "Scratching Post",
            BuildingKind::CatFlap => "Cat Flap",
            BuildingKind::LaserPointer => "Laser Pointer",
            // The Murder (Corvids)
            BuildingKind::TheParliament => "The Parliament",
            BuildingKind::Rookery => "Rookery",
            BuildingKind::CarrionCache => "Carrion Cache",
            BuildingKind::AntennaArray => "Antenna Array",
            BuildingKind::Panopticon => "Panopticon",
            BuildingKind::NestBox => "Nest Box",
            BuildingKind::ThornHedge => "Thorn Hedge",
            BuildingKind::Watchtower => "Watchtower",
            // The Clawed (Mice)
            BuildingKind::TheBurrow => "The Burrow",
            BuildingKind::NestingBox => "Nesting Box",
            BuildingKind::SeedVault => "Seed Vault",
            BuildingKind::JunkTransmitter => "Junk Transmitter",
            BuildingKind::GnawLab => "Gnaw Lab",
            BuildingKind::WarrenExpansion => "Warren Expansion",
            BuildingKind::Mousehole => "Mousehole",
            BuildingKind::SqueakTower => "Squeak Tower",
            // Seekers of the Deep (Badgers)
            BuildingKind::TheSett => "The Sett",
            BuildingKind::WarHollow => "War Hollow",
            BuildingKind::BurrowDepot => "Burrow Depot",
            BuildingKind::CoreTap => "Core Tap",
            BuildingKind::ClawMarks => "Claw Marks",
            BuildingKind::DeepWarren => "Deep Warren",
            BuildingKind::BulwarkGate => "Bulwark Gate",
            BuildingKind::SlagThrower => "Slag Thrower",
            // Croak (Axolotls)
            BuildingKind::TheGrotto => "The Grotto",
            BuildingKind::SpawningPools => "Spawning Pools",
            BuildingKind::LilyMarket => "Lily Market",
            BuildingKind::SunkenServer => "Sunken Server",
            BuildingKind::FossilStones => "Fossil Stones",
            BuildingKind::ReedBed => "Reed Bed",
            BuildingKind::TidalGate => "Tidal Gate",
            BuildingKind::SporeTower => "Spore Tower",
            // LLAMA (Raccoons)
            BuildingKind::TheDumpster => "The Dumpster",
            BuildingKind::ScrapHeap => "Scrap Heap",
            BuildingKind::ChopShop => "Chop Shop",
            BuildingKind::JunkServer => "Junk Server",
            BuildingKind::TinkerBench => "Tinker Bench",
            BuildingKind::TrashPile => "Trash Pile",
            BuildingKind::DumpsterRelay => "Dumpster Relay",
            BuildingKind::TetanusTower => "Tetanus Tower",
        }
    }

    /// Returns true if this building is a faction HQ (victory condition target).
    pub fn is_hq(&self) -> bool {
        matches!(
            self,
            BuildingKind::TheBox
                | BuildingKind::TheParliament
                | BuildingKind::TheBurrow
                | BuildingKind::TheSett
                | BuildingKind::TheGrotto
                | BuildingKind::TheDumpster
        )
    }
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
            // The Clawed (Mice)
            "TheBurrow" => Ok(Self::TheBurrow),
            "NestingBox" => Ok(Self::NestingBox),
            "SeedVault" => Ok(Self::SeedVault),
            "JunkTransmitter" => Ok(Self::JunkTransmitter),
            "GnawLab" => Ok(Self::GnawLab),
            "WarrenExpansion" => Ok(Self::WarrenExpansion),
            "Mousehole" => Ok(Self::Mousehole),
            "SqueakTower" => Ok(Self::SqueakTower),
            // Seekers of the Deep (Badgers)
            "TheSett" => Ok(Self::TheSett),
            "WarHollow" => Ok(Self::WarHollow),
            "BurrowDepot" => Ok(Self::BurrowDepot),
            "CoreTap" => Ok(Self::CoreTap),
            "ClawMarks" => Ok(Self::ClawMarks),
            "DeepWarren" => Ok(Self::DeepWarren),
            "BulwarkGate" => Ok(Self::BulwarkGate),
            "SlagThrower" => Ok(Self::SlagThrower),
            // Croak (Axolotls)
            "TheGrotto" => Ok(Self::TheGrotto),
            "SpawningPools" => Ok(Self::SpawningPools),
            "LilyMarket" => Ok(Self::LilyMarket),
            "SunkenServer" => Ok(Self::SunkenServer),
            "FossilStones" => Ok(Self::FossilStones),
            "ReedBed" => Ok(Self::ReedBed),
            "TidalGate" => Ok(Self::TidalGate),
            "SporeTower" => Ok(Self::SporeTower),
            // LLAMA (Raccoons)
            "TheDumpster" => Ok(Self::TheDumpster),
            "ScrapHeap" => Ok(Self::ScrapHeap),
            "ChopShop" => Ok(Self::ChopShop),
            "JunkServer" => Ok(Self::JunkServer),
            "TinkerBench" => Ok(Self::TinkerBench),
            "TrashPile" => Ok(Self::TrashPile),
            "DumpsterRelay" => Ok(Self::DumpsterRelay),
            "TetanusTower" => Ok(Self::TetanusTower),
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
    Sentinel,        // Ranged Scout (Crow) — long-range glass cannon
    Rookclaw,        // Melee Dive Striker (Crow) — fast, bursty, fragile
    Magpike,         // Disruptor/Thief (Magpie) — steals resources, disrupts
    Magpyre,         // Saboteur (Magpie) — signal jamming, decoys, rewiring
    Jaycaller,       // Support/Buffer (Jay) — rally cry, alarm call
    Jayflicker,      // Illusion Specialist (Jay) — phantom flock, mirror position
    Dusktalon,       // Stealth Assassin (Owl) — ground-based stealth, high burst
    Hootseer,        // Area Denial/Debuffer (Owl) — panoptic gaze, dread aura
    CorvusRex,       // Hero/Heavy (Augmented Crow) — corvid network, all-seeing lie
    // --- Seekers of the Deep (Badgers) ---
    Delver,
    Ironhide,
    Cragback,
    Warden,
    Sapjaw,
    Wardenmother,
    SeekerTunneler,
    Embermaw,
    Dustclaw,
    Gutripper,
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
    Ponderer,      // Worker (Croak) — gathers via ambient gathering on water
    Regeneron,     // Light Skirmisher (Croak) — Limb Economy, self-regen
    Broodmother,   // Healer/Support (Croak) — spawns Spawnlings, burst heals
    Gulper,        // Heavy Bruiser (Croak) — Devour mechanic, massive regen
    Eftsaber,      // Assassin/Flanker (Croak) — poison, Waterway stealth
    Croaker,       // Ranged Artillery (Croak) — Bog Mortar, terrain creation
    Leapfrog,      // Mobile Harasser (Croak) — Hop chains on water
    Shellwarden,   // Tank/Defender (Croak) — Hunker, Ancient Moss aura
    Bogwhisper,    // Support/Caster (Croak) — Mire Curse, Prophecy
    MurkCommander, // Hero/Heavy (Croak) — Grok Protocol, Murk Uplink
    // --- LLAMA (Raccoons) ---
    Scrounger,     // Worker (Raccoon) — gathers, scavenges, builds
    Bandit,        // Light Harasser (Raccoon) — sticky fingers, jury rig
    HeapTitan,     // Heavy Tank (Raccoon) — scrap armor, wreck ball
    GlitchRat,     // Saboteur (Raccoon) — cable gnaw, signal scramble
    PatchPossum,   // Support/Healer (Possum) — duct tape fix, feign death
    GreaseMonkey,  // Ranged (Raccoon) — junk launcher, salvage turret
    DeadDropUnit,  // Stealth Scout (Raccoon) — eavesdrop, trash heap ambush
    Wrecker,       // Anti-Structure (Raccoon) — disassemble, pry bar
    DumpsterDiver, // Area Denial (Raccoon) — treasure trash, refuse shield
    JunkyardKing,  // Hero/Heavy (Raccoon) — open source uplink, overclock cascade
}

impl UnitKind {
    /// Returns true if this unit is a worker (resource gatherer / builder) for any faction.
    pub fn is_worker(&self) -> bool {
        matches!(
            self,
            UnitKind::Pawdler
                | UnitKind::MurderScrounger
                | UnitKind::Delver
                | UnitKind::Nibblet
                | UnitKind::Ponderer
                | UnitKind::Scrounger
        )
    }
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
            // Seekers of the Deep (Badgers)
            "Delver" => Ok(Self::Delver),
            "Ironhide" => Ok(Self::Ironhide),
            "Cragback" => Ok(Self::Cragback),
            "Warden" => Ok(Self::Warden),
            "Sapjaw" => Ok(Self::Sapjaw),
            "Wardenmother" => Ok(Self::Wardenmother),
            "SeekerTunneler" => Ok(Self::SeekerTunneler),
            "Embermaw" => Ok(Self::Embermaw),
            "Dustclaw" => Ok(Self::Dustclaw),
            "Gutripper" => Ok(Self::Gutripper),
            // The Clawed (Mice)
            "Nibblet" => Ok(Self::Nibblet),
            "Swarmer" => Ok(Self::Swarmer),
            "Gnawer" => Ok(Self::Gnawer),
            "Shrieker" => Ok(Self::Shrieker),
            "Tunneler" => Ok(Self::Tunneler),
            "Sparks" => Ok(Self::Sparks),
            "Quillback" => Ok(Self::Quillback),
            "Whiskerwitch" => Ok(Self::Whiskerwitch),
            "Plaguetail" => Ok(Self::Plaguetail),
            "WarrenMarshal" => Ok(Self::WarrenMarshal),
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
            // LLAMA (Raccoons)
            "Scrounger" => Ok(Self::Scrounger),
            "Bandit" => Ok(Self::Bandit),
            "HeapTitan" => Ok(Self::HeapTitan),
            "GlitchRat" => Ok(Self::GlitchRat),
            "PatchPossum" => Ok(Self::PatchPossum),
            "GreaseMonkey" => Ok(Self::GreaseMonkey),
            "DeadDropUnit" => Ok(Self::DeadDropUnit),
            "Wrecker" => Ok(Self::Wrecker),
            "DumpsterDiver" => Ok(Self::DumpsterDiver),
            "JunkyardKing" => Ok(Self::JunkyardKing),
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

/// Marker: this unit received the golden voice-command speed buff.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct VoiceBuffed;

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
    pub attack_speed: u32,       // ticks between attacks
    pub cooldown_remaining: u32, // ticks until next attack
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

/// Visual style of a projectile, derived from the attacker's unit type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub enum ProjectileKind {
    /// Hisser's acid spit (green)
    Spit,
    /// Tower/laser-based attacks (red)
    LaserBeam,
    /// Yowler's sonic attack (purple)
    SonicWave,
    /// MechCommander's cannon (cyan)
    MechShot,
    /// Catnapper's siege projectile (orange)
    Explosive,
    /// Default fallback (yellow)
    #[default]
    Generic,
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

impl ProjectileKind {
    /// Derive the projectile kind from the attacker's unit type.
    pub fn from_unit_kind(kind: UnitKind) -> Self {
        match kind {
            UnitKind::Hisser => ProjectileKind::Spit,
            UnitKind::Yowler | UnitKind::Jaycaller => ProjectileKind::SonicWave,
            UnitKind::MechCommander
            | UnitKind::CorvusRex
            | UnitKind::MurkCommander
            | UnitKind::Wardenmother
            | UnitKind::JunkyardKing
            | UnitKind::WarrenMarshal => ProjectileKind::MechShot,
            UnitKind::Catnapper | UnitKind::Cragback => ProjectileKind::Explosive,
            _ => ProjectileKind::Generic,
        }
    }
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

impl UnderConstruction {
    /// Returns construction progress as 0.0 (just started) to 1.0 (complete).
    pub fn progress_f32(&self) -> f32 {
        if self.total_ticks > 0 {
            1.0 - (self.remaining_ticks as f32 / self.total_ticks as f32)
        } else {
            1.0
        }
    }
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
    // Seekers of the Deep (Badgers)
    VigilanceAura,
    DeepseekUplinkAura,
    FortressProtocolAura,
    FrenzyAura,
    // The Clawed (Mice)
    RallyTheSwarm,
    WhiskernetRelay,
    SqueakTowerPulse,
    // Croak (Axolotls)
    AncientMoss,
    BogSong,
    UndyingPresence,
    MurkUplinkAura,
    // LLAMA (Raccoons)
    OpenSourceUplinkAura,
    ScrapArmorAura,
    DumpsterRelayAura,
    StenchCloudAura,
    // Toggle-specific aura types
    WhiskerWeave,
    SwarmTremorSense,
    PanopticGaze,
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
// LLAMA-specific components
// ---------------------------------------------------------------------------

/// Scrounger's personal scrap inventory (PocketStash ability).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct PocketStashInventory {
    pub count: u32,
    pub max: u32,
}

impl Default for PocketStashInventory {
    fn default() -> Self {
        Self { count: 0, max: 3 }
    }
}

/// Tracks Patch Possum's FeignDeath cooldown (passive auto-trigger).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct FeignDeathTracker {
    pub last_triggered_tick: u64,
}

impl Default for FeignDeathTracker {
    fn default() -> Self {
        Self {
            last_triggered_tick: 0,
        }
    }
}

/// Tracks Grease Monkey's JunkLauncher attack count for crit calculation.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct JunkLauncherState {
    pub attack_count: u32,
}

impl Default for JunkLauncherState {
    fn default() -> Self {
        Self { attack_count: 0 }
    }
}

/// Tracks Junkyard King's active Frankenstein Protocol summons.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct FrankensteinTracker {
    pub active_count: u32,
    pub max: u32,
}

impl Default for FrankensteinTracker {
    fn default() -> Self {
        Self {
            active_count: 0,
            max: 3,
        }
    }
}

/// Applied to units that deal Corroded stacks (TetanusTower, Wrecker).
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct CorrodedApplicator {
    pub stacks_per_hit: u32,
    pub max_stacks: u32,
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
    // --- Seekers of the Deep (Badgers) ---
    SharperFangs,
    ReinforcedHide,
    SteadyStance,
    SiegeEngineering,
    ExosuitPrototype,
    // --- The Clawed (Mice) ---
    /// +2 damage for all Clawed combat units.
    SharperTeeth,
    /// +20 HP for all Clawed combat units.
    ThickerHide,
    /// +10% speed for all Clawed units.
    QuickPaws,
    /// Unlocks advanced Gnawer siege abilities.
    AdvancedGnawing,
    /// Unlocks WarrenMarshal training at JunkTransmitter.
    WarrenProtocol,
    /// Unlocks CorvusRex training at AntennaArray.
    RexPrototype,
    // --- Croak (Axolotls) ---
    /// +15% HP for all Croak combat units.
    TougherHide,
    /// +10% speed for all Croak units.
    SlickerMucus,
    /// +0.5% HP/s regen for all Croak units on water.
    AmphibianAgility,
    /// Unlocks Shellwarden training at SunkenServer.
    SiegeEvolution,
    /// Unlocks MurkCommander training at SunkenServer.
    MurkPrototype,
    // --- LLAMA (Raccoons) ---
    /// +2 damage for LLAMA combat units.
    RustyFangs,
    /// +25 HP for LLAMA combat units.
    ScrapPlating,
    /// +10% speed for LLAMA units.
    TrashRunning,
    /// Unlocks advanced Grease Monkey mode at Chop Shop.
    SiegeWelding,
    /// Unlocks Junkyard King training at TinkerBench.
    MechSalvage,
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
            Self::SharperFangs => write!(f, "SharperFangs"),
            Self::ReinforcedHide => write!(f, "ReinforcedHide"),
            Self::SteadyStance => write!(f, "SteadyStance"),
            Self::SiegeEngineering => write!(f, "SiegeEngineering"),
            Self::ExosuitPrototype => write!(f, "ExosuitPrototype"),
            Self::SharperTeeth => write!(f, "SharperTeeth"),
            Self::ThickerHide => write!(f, "ThickerHide"),
            Self::QuickPaws => write!(f, "QuickPaws"),
            Self::AdvancedGnawing => write!(f, "AdvancedGnawing"),
            Self::WarrenProtocol => write!(f, "WarrenProtocol"),
            Self::TougherHide => write!(f, "TougherHide"),
            Self::SlickerMucus => write!(f, "SlickerMucus"),
            Self::AmphibianAgility => write!(f, "AmphibianAgility"),
            Self::SiegeEvolution => write!(f, "SiegeEvolution"),
            Self::MurkPrototype => write!(f, "MurkPrototype"),
            Self::RustyFangs => write!(f, "RustyFangs"),
            Self::ScrapPlating => write!(f, "ScrapPlating"),
            Self::TrashRunning => write!(f, "TrashRunning"),
            Self::SiegeWelding => write!(f, "SiegeWelding"),
            Self::MechSalvage => write!(f, "MechSalvage"),
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
            "SharperFangs" => Ok(Self::SharperFangs),
            "ReinforcedHide" => Ok(Self::ReinforcedHide),
            "SteadyStance" => Ok(Self::SteadyStance),
            "SiegeEngineering" => Ok(Self::SiegeEngineering),
            "ExosuitPrototype" => Ok(Self::ExosuitPrototype),
            "SharperTeeth" => Ok(Self::SharperTeeth),
            "ThickerHide" => Ok(Self::ThickerHide),
            "QuickPaws" => Ok(Self::QuickPaws),
            "AdvancedGnawing" => Ok(Self::AdvancedGnawing),
            "WarrenProtocol" => Ok(Self::WarrenProtocol),
            "TougherHide" => Ok(Self::TougherHide),
            "SlickerMucus" => Ok(Self::SlickerMucus),
            "AmphibianAgility" => Ok(Self::AmphibianAgility),
            "SiegeEvolution" => Ok(Self::SiegeEvolution),
            "MurkPrototype" => Ok(Self::MurkPrototype),
            // LLAMA (Raccoons)
            "RustyFangs" => Ok(Self::RustyFangs),
            "ScrapPlating" => Ok(Self::ScrapPlating),
            "TrashRunning" => Ok(Self::TrashRunning),
            "SiegeWelding" => Ok(Self::SiegeWelding),
            "MechSalvage" => Ok(Self::MechSalvage),
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

// ---------------------------------------------------------------------------
// The Clawed (Mice) faction components
// ---------------------------------------------------------------------------

/// Tracks Gnawer structural weakness stacks against buildings.
/// Each consecutive attack on the same building adds a stack, increasing damage.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct StructuralWeaknessTimer {
    pub stacks: u32,
    pub target_entity: Option<EntityId>,
}

impl Default for StructuralWeaknessTimer {
    fn default() -> Self {
        Self {
            stacks: 0,
            target_entity: None,
        }
    }
}

/// Tracks Sparks static charge stacks, accumulated during movement.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct StaticChargeStacks {
    pub stacks: u32,
}

impl Default for StaticChargeStacks {
    fn default() -> Self {
        Self { stacks: 0 }
    }
}

/// Marker: Plaguetail spawns a contagion cloud on death.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[derive(Debug, Clone, Copy)]
pub struct ContagionCloudOnDeath;
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

// ---------------------------------------------------------------------------
// Seekers of the Deep faction components
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct StationaryTimer {
    pub ticks_stationary: u32,
    pub dug_in: bool,
}
impl Default for StationaryTimer {
    fn default() -> Self {
        Self {
            ticks_stationary: 0,
            dug_in: false,
        }
    }
}
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct HeavyUnit;
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Entrenched;
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct FrenzyStacks {
    pub current_stacks: u32,
    pub frozen_until_tick: u64,
}
impl Default for FrenzyStacks {
    fn default() -> Self {
        Self {
            current_stacks: 0,
            frozen_until_tick: 0,
        }
    }
}
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct BloodgreedTracker {
    pub lifesteal_fraction: Fixed,
}

// ---------------------------------------------------------------------------
// Croak faction components
// ---------------------------------------------------------------------------

/// Tracks the Limb Economy for axolotl units (Regeneron).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct LimbTracker {
    pub current_limbs: u8,
    pub max_limbs: u8,
    pub regen_ticks: u32,
}

/// Marker: Croak unit is standing on water and receiving Water Affinity bonuses.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct WaterAffinityBuff;

/// Tracks Broodmother's active Spawnlings.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct SpawnlingCounter {
    pub count: u8,
    pub spawn_cooldown: u32,
}

/// Links a Spawnling back to its parent Broodmother.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct SpawnlingParent {
    pub parent_entity: EntityId,
}

/// Tracks Croaker's active Bog Patches for Resonance Chain logic.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct BogPatchCounter {
    pub active_patches: Vec<(i32, i32)>,
}

/// Tracks Gulper's Devour ability state.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct DevourState {
    pub swallowed_entity: EntityId,
    pub swallowed_max_hp: Fixed,
    pub digest_ticks_remaining: u32,
    pub digest_damage_per_tick: Fixed,
    pub temp_shields: Fixed,
}

/// Marker: Shellwarden is in Hunker mode (75% DR, immobile, reflects damage).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Hunkered;

/// Marker: Eftsaber is submerged via Waterway (untargetable, invisible, water-only movement).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Submerged;

/// Stasis state from MurkCommander's Undying Presence aura.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Stasis {
    pub remaining_ticks: u32,
    pub revive_hp_fraction: Fixed,
    pub per_unit_cooldown_tick: u64,
}

// ---------------------------------------------------------------------------
// Cursor position (shared between input systems)
// ---------------------------------------------------------------------------

/// Current cursor position in grid coordinates, updated each frame by the client.
///
/// Engine-agnostic data — the client writes it, voice/AI systems read it.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::prelude::Resource))]
pub struct CursorGridPos {
    pub pos: Option<GridPos>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_display_from_str_round_trip() {
        for rt in [
            ResourceType::Food,
            ResourceType::GpuCores,
            ResourceType::Nft,
        ] {
            let s = rt.to_string();
            let parsed: ResourceType = s.parse().unwrap();
            assert_eq!(parsed, rt);
        }
        assert!("Bogus".parse::<ResourceType>().is_err());
    }

    #[test]
    fn building_kind_display_from_str_round_trip() {
        for kind in [
            BuildingKind::TheBox,
            BuildingKind::CatTree,
            BuildingKind::FishMarket,
            BuildingKind::LitterBox,
            BuildingKind::ServerRack,
            BuildingKind::ScratchingPost,
            BuildingKind::CatFlap,
            BuildingKind::LaserPointer,
            // The Clawed (Mice)
            BuildingKind::TheBurrow,
            BuildingKind::NestingBox,
            BuildingKind::SeedVault,
            BuildingKind::JunkTransmitter,
            BuildingKind::GnawLab,
            BuildingKind::WarrenExpansion,
            BuildingKind::Mousehole,
            BuildingKind::SqueakTower,
            // Seekers of the Deep (Badgers)
            BuildingKind::TheSett,
            BuildingKind::WarHollow,
            BuildingKind::BurrowDepot,
            BuildingKind::CoreTap,
            BuildingKind::ClawMarks,
            BuildingKind::DeepWarren,
            BuildingKind::BulwarkGate,
            BuildingKind::SlagThrower,
            // Croak (Axolotls)
            BuildingKind::TheGrotto,
            BuildingKind::SpawningPools,
            BuildingKind::LilyMarket,
            BuildingKind::SunkenServer,
            BuildingKind::FossilStones,
            BuildingKind::ReedBed,
            BuildingKind::TidalGate,
            BuildingKind::SporeTower,
            // LLAMA (Raccoons)
            BuildingKind::TheDumpster,
            BuildingKind::ScrapHeap,
            BuildingKind::ChopShop,
            BuildingKind::JunkServer,
            BuildingKind::TinkerBench,
            BuildingKind::TrashPile,
            BuildingKind::DumpsterRelay,
            BuildingKind::TetanusTower,
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
            UnitKind::Pawdler,
            UnitKind::Nuisance,
            UnitKind::Chonk,
            UnitKind::FlyingFox,
            UnitKind::Hisser,
            UnitKind::Yowler,
            UnitKind::Mouser,
            UnitKind::Catnapper,
            UnitKind::FerretSapper,
            UnitKind::MechCommander,
            // The Murder (Corvids)
            UnitKind::MurderScrounger,
            UnitKind::Sentinel,
            UnitKind::Rookclaw,
            UnitKind::Magpike,
            UnitKind::Magpyre,
            UnitKind::Jaycaller,
            UnitKind::Jayflicker,
            UnitKind::Dusktalon,
            UnitKind::Hootseer,
            UnitKind::CorvusRex,
            // Seekers of the Deep (Badgers)
            UnitKind::Delver,
            UnitKind::Ironhide,
            UnitKind::Cragback,
            UnitKind::Warden,
            UnitKind::Sapjaw,
            UnitKind::Wardenmother,
            UnitKind::SeekerTunneler,
            UnitKind::Embermaw,
            UnitKind::Dustclaw,
            UnitKind::Gutripper,
            // The Clawed (Mice)
            UnitKind::Nibblet,
            UnitKind::Swarmer,
            UnitKind::Gnawer,
            UnitKind::Shrieker,
            UnitKind::Tunneler,
            UnitKind::Sparks,
            UnitKind::Quillback,
            UnitKind::Whiskerwitch,
            UnitKind::Plaguetail,
            UnitKind::WarrenMarshal,
            // Croak (Axolotls)
            UnitKind::Ponderer,
            UnitKind::Regeneron,
            UnitKind::Broodmother,
            UnitKind::Gulper,
            UnitKind::Eftsaber,
            UnitKind::Croaker,
            UnitKind::Leapfrog,
            UnitKind::Shellwarden,
            UnitKind::Bogwhisper,
            UnitKind::MurkCommander,
            // LLAMA (Raccoons)
            UnitKind::Scrounger,
            UnitKind::Bandit,
            UnitKind::HeapTitan,
            UnitKind::GlitchRat,
            UnitKind::PatchPossum,
            UnitKind::GreaseMonkey,
            UnitKind::DeadDropUnit,
            UnitKind::Wrecker,
            UnitKind::DumpsterDiver,
            UnitKind::JunkyardKing,
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
            Faction::Neutral,
            Faction::CatGpt,
            Faction::TheClawed,
            Faction::SeekersOfTheDeep,
            Faction::TheMurder,
            Faction::Llama,
            Faction::Croak,
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
        let wm = WaveMember {
            wave_id: "wave_1".into(),
        };
        assert_eq!(wm.wave_id, "wave_1");
    }

    #[test]
    fn structural_weakness_timer_default() {
        let timer = StructuralWeaknessTimer::default();
        assert_eq!(timer.stacks, 0);
        assert!(timer.target_entity.is_none());
    }

    #[test]
    fn static_charge_stacks_default() {
        let stacks = StaticChargeStacks::default();
        assert_eq!(stacks.stacks, 0);
    }

    #[test]
    fn upgrade_type_clawed_round_trip() {
        for ut in [
            UpgradeType::SharperTeeth,
            UpgradeType::ThickerHide,
            UpgradeType::QuickPaws,
            UpgradeType::AdvancedGnawing,
            UpgradeType::WarrenProtocol,
        ] {
            let s = ut.to_string();
            let parsed: UpgradeType = s.parse().unwrap();
            assert_eq!(parsed, ut);
        }
    }

    #[test]
    fn upgrade_type_seekers_round_trip() {
        for ut in [
            UpgradeType::SharperFangs,
            UpgradeType::ReinforcedHide,
            UpgradeType::SteadyStance,
            UpgradeType::SiegeEngineering,
            UpgradeType::ExosuitPrototype,
        ] {
            let s = ut.to_string();
            let parsed: UpgradeType = s.parse().unwrap();
            assert_eq!(parsed, ut);
        }
    }

    #[test]
    fn seekers_faction_components_defaults() {
        let timer = StationaryTimer::default();
        assert_eq!(timer.ticks_stationary, 0);
        assert!(!timer.dug_in);

        let stacks = FrenzyStacks::default();
        assert_eq!(stacks.current_stacks, 0);
        assert_eq!(stacks.frozen_until_tick, 0);
    }

    #[test]
    fn is_worker_identifies_all_faction_workers() {
        let workers = [
            UnitKind::Pawdler,
            UnitKind::MurderScrounger,
            UnitKind::Delver,
            UnitKind::Nibblet,
            UnitKind::Ponderer,
            UnitKind::Scrounger,
        ];
        for w in &workers {
            assert!(w.is_worker(), "{:?} should be a worker", w);
        }
    }

    #[test]
    fn is_worker_rejects_combat_units() {
        let non_workers = [
            UnitKind::Nuisance,
            UnitKind::Chonk,
            UnitKind::FlyingFox,
            UnitKind::Hisser,
            UnitKind::Sentinel,
            UnitKind::Rookclaw,
            UnitKind::Swarmer,
            UnitKind::Regeneron,
            UnitKind::Bandit,
            UnitKind::Ironhide,
        ];
        for u in &non_workers {
            assert!(!u.is_worker(), "{:?} should not be a worker", u);
        }
    }

    #[test]
    fn is_hq_identifies_all_faction_hqs() {
        let hqs = [
            BuildingKind::TheBox,
            BuildingKind::TheParliament,
            BuildingKind::TheBurrow,
            BuildingKind::TheSett,
            BuildingKind::TheGrotto,
            BuildingKind::TheDumpster,
        ];
        for h in &hqs {
            assert!(h.is_hq(), "{:?} should be an HQ", h);
        }
        // Non-HQ buildings
        assert!(!BuildingKind::CatTree.is_hq());
        assert!(!BuildingKind::LitterBox.is_hq());
        assert!(!BuildingKind::Rookery.is_hq());
    }

    #[test]
    fn projectile_kind_default_is_generic() {
        assert_eq!(ProjectileKind::default(), ProjectileKind::Generic);
    }

    #[test]
    fn projectile_kind_from_unit_kind_hisser_is_spit() {
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::Hisser),
            ProjectileKind::Spit
        );
    }

    #[test]
    fn projectile_kind_from_unit_kind_yowler_is_sonic() {
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::Yowler),
            ProjectileKind::SonicWave
        );
    }

    #[test]
    fn projectile_kind_from_unit_kind_mech_is_mechshot() {
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::MechCommander),
            ProjectileKind::MechShot
        );
    }

    #[test]
    fn projectile_kind_from_unit_kind_catnapper_is_explosive() {
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::Catnapper),
            ProjectileKind::Explosive
        );
    }

    #[test]
    fn projectile_kind_from_unit_kind_generic_fallback() {
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::FlyingFox),
            ProjectileKind::Generic
        );
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::Pawdler),
            ProjectileKind::Generic
        );
    }

    #[test]
    fn projectile_kind_from_hero_units() {
        // All hero units should get MechShot
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::CorvusRex),
            ProjectileKind::MechShot
        );
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::MurkCommander),
            ProjectileKind::MechShot
        );
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::Wardenmother),
            ProjectileKind::MechShot
        );
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::JunkyardKing),
            ProjectileKind::MechShot
        );
        assert_eq!(
            ProjectileKind::from_unit_kind(UnitKind::WarrenMarshal),
            ProjectileKind::MechShot
        );
    }

    #[test]
    fn projectile_kind_equality() {
        assert_eq!(ProjectileKind::Spit, ProjectileKind::Spit);
        assert_ne!(ProjectileKind::Spit, ProjectileKind::LaserBeam);
    }
}
