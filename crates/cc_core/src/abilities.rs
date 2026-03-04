use crate::components::UnitKind;
use crate::math::Fixed;

/// Unique identifier for every ability in the game.
/// One variant per ability — flat enum matching the UnitKind/BuildingKind pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbilityId {
    // Pawdler (worker)
    OpportunisticHoarder,
    SpiteCarry,
    Revulsion,
    // Nuisance (harasser)
    AnnoyanceStacks,
    Hairball,
    Zoomies,
    // Chonk (tank)
    GravitationalChonk,
    LoafMode,
    NineLives,
    // FlyingFox (air)
    EcholocationPulse,
    FruitDrop,
    Disoriented,
    // Hisser (ranged)
    CorrosiveSpit,
    DisgustMortar,
    Misinformation,
    // Yowler (support)
    HarmonicResonance,
    DissonantScreech,
    Lullaby,
    // Mouser (stealth)
    Tagged,
    DeadDrop,
    ShadowNetwork,
    // Catnapper (siege)
    DreamSiege,
    ContagiousYawning,
    /// Siege Nap — toggle deploy mode with extended range (replaces PowerNap).
    SiegeNap,
    // FerretSapper (demo)
    ShapedCharge,
    BoobyTrap,
    TunnelNetwork,
    // MechCommander (hero)
    TacticalUplink,
    Override,
    LeChatUplink,
    // --- Seekers of the Deep (Badgers) ---
    SubterraneanHaul,
    Earthsense,
    EmergencyBurrow,
    Unbowed,
    ShieldWall,
    GrudgeCharge,
    BoulderBarrage,
    Entrench,
    SeismicSlam,
    VigilanceAura,
    Intercept,
    RallyCry,
    ArmorRend,
    PatientStrike,
    Lockjaw,
    DeepseekUplink,
    FortressProtocol,
    CalculatedCounterstrike,
    DeepBore,
    Undermine,
    TremorNetwork,
    MoltenShot,
    FuelReserve,
    ScorchedEarth,
    DustCloud,
    AmbushInstinct,
    SentryBurrow,
    Frenzy,
    Bloodgreed,
    RecklessLunge,
    // --- LLAMA (Raccoons) ---
    DumpsterDiveAbility,
    PocketStash,
    PlayDead,
    StickyFingers,
    JuryRig,
    Getaway,
    ScrapArmorAbility,
    WreckBall,
    MagneticPulse,
    CableGnaw,
    SignalScramble,
    TunnelRat,
    DuctTapeFix,
    SalvageResurrection,
    FeignDeath,
    JunkLauncher,
    SalvageTurret,
    /// Junk Mortar Mode — toggle deploy with AoE siege range (replaces Overcharge).
    JunkMortarMode,
    Eavesdrop,
    TrashHeapAmbush,
    LeakInjection,
    Disassemble,
    PryBar,
    ChainBreak,
    TreasureTrash,
    RefuseShield,
    StenchCloudAbility,
    OpenSourceUplinkAbility,
    FrankensteinProtocol,
    OverclockCascade,
    // --- The Murder (Corvids) ---
    // MurderScrounger (worker)
    TrinketStash,
    Scavenge,
    MimicCall,
    // Sentinel (ranged scout)
    Glintwatch,
    Overwatch,
    EvasiveAscent,
    // Rookclaw (melee dive striker)
    TalonDive,
    MurdersMark,
    CarrionInstinct,
    // Magpike (disruptor/thief)
    Pilfer,
    GlitterBomb,
    TrinketWard,
    // Magpyre (saboteur)
    SignalJam,
    DecoyNest,
    Rewire,
    // Jaycaller (support/buffer)
    MurderRallyCry,
    AlarmCall,
    Cacophony,
    // Jayflicker (illusion specialist)
    PhantomFlock,
    MirrorPosition,
    Refraction,
    // Dusktalon (stealth assassin)
    Nightcloak,
    SilentStrike,
    PreySense,
    // Hootseer (area denial/debuffer)
    PanopticGaze,
    DreadAuraAbility,
    /// Death Omen — long-range snipe, double damage vs stationary (replaces Omen).
    DeathOmen,
    // CorvusRex (hero)
    CorvidNetworkAbility,
    AllSeeingLie,
    OculusUplinkAbility,
    // --- Croak (Axolotls) ---
    // Ponderer (worker)
    AmbientGathering,
    MucusTrail,
    ExistentialDread,
    // Regeneron (skirmisher)
    LimbToss,
    RegrowthBurst,
    PhantomLimb,
    // Broodmother (support)
    SpawnPool,
    Transfusion,
    PrimordialSoup,
    // Gulper (heavy)
    Devour,
    Regurgitate,
    Bottomless,
    // Eftsaber (assassin)
    ToxicSkin,
    Waterway,
    Venomstrike,
    // Croaker (artillery)
    BogMortar,
    ResonanceChain,
    Inflate,
    // Leapfrog (harasser)
    Hop,
    TongueLash,
    Slipstream,
    // Shellwarden (tank)
    HunkerAbility,
    AncientMossAbility,
    TidalMemory,
    // Bogwhisper (caster)
    MireCurse,
    Prophecy,
    BogSongAbility,
    // MurkCommander (hero)
    UndyingPresenceAbility,
    GrokProtocol,
    MurkUplinkAbility,
    // --- The Clawed (Mice) ---
    // Nibblet (worker)
    CrumbTrail,
    StashNetwork,
    PanicProductivity,
    // Swarmer (light infantry)
    SafetyInNumbers,
    PileOn,
    Scatter,
    // Gnawer (anti-structure)
    StructuralWeakness,
    ChewThrough,
    IncisorsNeverStop,
    // Shrieker (ranged harasser)
    SonicSpit,
    EcholocationPing,
    /// Sonic Barrage — line AoE burst at range 8 (replaces FuryOfTheSmall).
    SonicBarrage,
    // Tunneler (transport/utility)
    BurrowExpress,
    BurrowUndermine,
    SwarmTremorSense,
    // Sparks (saboteur)
    StaticCharge,
    ShortCircuit,
    DaisyChain,
    // Quillback (heavy defender)
    SpineWall,
    QuillBurst,
    StubbornAdvance,
    // Whiskerwitch (caster/support)
    HexOfMultiplication,
    WhiskerWeave,
    DatacromanticRitual,
    // Plaguetail (area denial)
    ContagionCloud,
    MiasmaTrail,
    SympathySickness,
    // WarrenMarshal (hero/commander)
    RallyTheSwarm,
    ExpendableHeroism,
    WhiskernetRelay,
}

/// How an ability is activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityActivation {
    /// Always on, no player input.
    Passive,
    /// Click to use, goes on cooldown.
    Activated,
    /// Click to toggle on/off.
    Toggle,
}

/// Static definition for an ability.
#[derive(Debug, Clone, Copy)]
pub struct AbilityDef {
    pub id: AbilityId,
    pub activation: AbilityActivation,
    /// Ticks between uses (0 = no cooldown / passive).
    pub cooldown_ticks: u32,
    /// GPU cost per activation (0 = free).
    pub gpu_cost: u32,
    /// Duration in ticks (0 = instant or passive).
    pub duration_ticks: u32,
    /// Range in tiles (0 = self-only).
    pub range: Fixed,
    /// Max charges (0 = unlimited / not charge-based).
    pub max_charges: u32,
}

/// Return the static definition for any ability.
pub fn ability_def(id: AbilityId) -> AbilityDef {
    use AbilityActivation::*;
    match id {
        // --- Pawdler ---
        AbilityId::OpportunisticHoarder => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SpiteCarry => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 5,
            duration_ticks: 50,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Revulsion => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },

        // --- Nuisance ---
        AbilityId::AnnoyanceStacks => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Hairball => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 60,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::Zoomies => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120,
            gpu_cost: 10,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- Chonk ---
        AbilityId::GravitationalChonk => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::LoafMode => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::NineLives => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 600,
            gpu_cost: 25,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- FlyingFox ---
        AbilityId::EcholocationPulse => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::FruitDrop => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 60,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::Disoriented => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 15,
            duration_ticks: 30,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },

        // --- Hisser ---
        AbilityId::CorrosiveSpit => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::DisgustMortar => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::Misinformation => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 20,
            duration_ticks: 50,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },

        // --- Yowler ---
        AbilityId::HarmonicResonance => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::DissonantScreech => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 10,
            duration_ticks: 30,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::Lullaby => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },

        // --- Mouser ---
        AbilityId::Tagged => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 60,
            gpu_cost: 5,
            duration_ticks: 100,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::DeadDrop => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 3,
        },
        AbilityId::ShadowNetwork => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 25,
            duration_ticks: 80,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- Catnapper ---
        AbilityId::DreamSiege => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::ContagiousYawning => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 15,
            duration_ticks: 20,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::SiegeNap => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 20,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- FerretSapper ---
        AbilityId::ShapedCharge => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::BoobyTrap => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 3,
        },
        AbilityId::TunnelNetwork => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 30,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- MechCommander ---
        AbilityId::TacticalUplink => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::Override => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 30,
            duration_ticks: 80,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::LeChatUplink => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 50,
            duration_ticks: 100,
            range: Fixed::ZERO,
            max_charges: 0,
        },

        // --- The Clawed (Mice) ---
        // Nibblet (worker)
        AbilityId::CrumbTrail => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::StashNetwork => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PanicProductivity => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // Swarmer (light infantry)
        AbilityId::SafetyInNumbers => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::PileOn => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 60,
            gpu_cost: 5,
            duration_ticks: 30,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::Scatter => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 5,
            duration_ticks: 20,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // Gnawer (anti-structure)
        AbilityId::StructuralWeakness => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::ChewThrough => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::IncisorsNeverStop => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // Shrieker (ranged harasser)
        AbilityId::SonicSpit => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::EcholocationPing => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 10,
            duration_ticks: 50,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::SonicBarrage => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150, // 15s
            gpu_cost: 0,
            duration_ticks: 10, // 1s channel
            range: Fixed::from_bits(8 << 16), // 8 tiles
            max_charges: 0,
        },
        // Tunneler (transport/utility)
        AbilityId::BurrowExpress => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 15,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::BurrowUndermine => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 20,
            duration_ticks: 50,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::SwarmTremorSense => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        // Sparks (saboteur)
        AbilityId::StaticCharge => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::ShortCircuit => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::from_bits(2 << 16),
            max_charges: 0,
        },
        AbilityId::DaisyChain => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120,
            gpu_cost: 15,
            duration_ticks: 20,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // Quillback (heavy defender)
        AbilityId::SpineWall => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::QuillBurst => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120,
            gpu_cost: 15,
            duration_ticks: 0,
            range: Fixed::from_bits(2 << 16),
            max_charges: 0,
        },
        AbilityId::StubbornAdvance => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 10,
            duration_ticks: 50,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // Whiskerwitch (caster/support)
        AbilityId::HexOfMultiplication => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 25,
            duration_ticks: 100,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::WhiskerWeave => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::DatacromanticRitual => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 30,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        // Plaguetail (area denial)
        AbilityId::ContagionCloud => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::MiasmaTrail => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SympathySickness => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(2 << 16),
            max_charges: 0,
        },
        // WarrenMarshal (hero/commander)
        AbilityId::RallyTheSwarm => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::ExpendableHeroism => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 20,
            duration_ticks: 50,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::WhiskernetRelay => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 40,
            duration_ticks: 100,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Scrounger ---
        AbilityId::DumpsterDiveAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PocketStash => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PlayDead => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Bandit ---
        AbilityId::StickyFingers => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::JuryRig => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 50,
            gpu_cost: 0,
            duration_ticks: 20,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::Getaway => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 15,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Heap Titan ---
        AbilityId::ScrapArmorAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::WreckBall => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::MagneticPulse => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 40,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- LLAMA: Glitch Rat ---
        AbilityId::CableGnaw => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::SignalScramble => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 4,
            duration_ticks: 40,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::TunnelRat => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Patch Possum ---
        AbilityId::DuctTapeFix => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 0,
            duration_ticks: 50,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::SalvageResurrection => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 40,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::FeignDeath => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 450,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Grease Monkey ---
        AbilityId::JunkLauncher => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SalvageTurret => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 200,
            range: Fixed::from_bits(2 << 16),
            max_charges: 1,
        },
        AbilityId::JunkMortarMode => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 20,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Dead Drop ---
        AbilityId::Eavesdrop => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::TrashHeapAmbush => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 0,
            duration_ticks: 20,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::LeakInjection => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 5,
            duration_ticks: 40,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- LLAMA: Wrecker ---
        AbilityId::Disassemble => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PryBar => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 40,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::ChainBreak => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 140,
            gpu_cost: 0,
            duration_ticks: 60,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- LLAMA: Dumpster Diver ---
        AbilityId::TreasureTrash => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::RefuseShield => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 150,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::StenchCloudAbility => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 60,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- LLAMA: Junkyard King ---
        AbilityId::OpenSourceUplinkAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::FrankensteinProtocol => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 450,
            gpu_cost: 10,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::OverclockCascade => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 350,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        // --- Murder: MurderScrounger ---
        AbilityId::TrinketStash => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 3,
        },
        AbilityId::Scavenge => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 50,
            gpu_cost: 0,
            duration_ticks: 20,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::MimicCall => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 2,
            duration_ticks: 50,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        // --- Murder: Sentinel ---
        AbilityId::Glintwatch => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(12 << 16),
            max_charges: 0,
        },
        AbilityId::Overwatch => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 15,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::EvasiveAscent => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 20,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Murder: Rookclaw ---
        AbilityId::TalonDive => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 0,
            duration_ticks: 5,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::MurdersMark => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 150,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::CarrionInstinct => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        // --- Murder: Magpike ---
        AbilityId::Pilfer => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 10,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::GlitterBomb => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::TrinketWard => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Murder: Magpyre ---
        AbilityId::SignalJam => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 4,
            duration_ticks: 100,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::DecoyNest => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 600,
            range: Fixed::ZERO,
            max_charges: 2,
        },
        AbilityId::Rewire => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- Murder: Jaycaller ---
        AbilityId::MurderRallyCry => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::AlarmCall => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 80,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Cacophony => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        // --- Murder: Jayflicker ---
        AbilityId::PhantomFlock => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 4,
            duration_ticks: 120,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::MirrorPosition => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 5,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::Refraction => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        // --- Murder: Dusktalon ---
        AbilityId::Nightcloak => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SilentStrike => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::PreySense => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(10 << 16),
            max_charges: 0,
        },
        // --- Murder: Hootseer ---
        AbilityId::PanopticGaze => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::DreadAuraAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::DeathOmen => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120, // 12s
            gpu_cost: 4,
            duration_ticks: 0, // instant
            range: Fixed::from_bits(10 << 16), // 10 tiles
            max_charges: 0,
        },
        // --- Murder: CorvusRex ---
        AbilityId::CorvidNetworkAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(10 << 16),
            max_charges: 0,
        },
        AbilityId::AllSeeingLie => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 900,
            gpu_cost: 8,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::OculusUplinkAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(10 << 16),
            max_charges: 0,
        },
        // --- Croak: Ponderer ---
        AbilityId::AmbientGathering => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::MucusTrail => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::ExistentialDread => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- Croak: Regeneron ---
        AbilityId::LimbToss => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 30,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::RegrowthBurst => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PhantomLimb => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Broodmother ---
        AbilityId::SpawnPool => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 300,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Transfusion => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 0,
            duration_ticks: 50,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::PrimordialSoup => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 350,
            gpu_cost: 0,
            duration_ticks: 120,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Gulper ---
        AbilityId::Devour => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::Regurgitate => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::Bottomless => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Eftsaber ---
        AbilityId::ToxicSkin => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Waterway => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 50,
            gpu_cost: 0,
            duration_ticks: 15,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Venomstrike => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 120,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        // --- Croak: Croaker ---
        AbilityId::BogMortar => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::ResonanceChain => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Inflate => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Leapfrog ---
        AbilityId::Hop => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 60,
            gpu_cost: 0,
            duration_ticks: 15,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::TongueLash => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 100,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::Slipstream => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Shellwarden ---
        AbilityId::HunkerAbility => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 10,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::AncientMossAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::TidalMemory => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 600,
            gpu_cost: 6,
            duration_ticks: 200,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Croak: Bogwhisper ---
        AbilityId::MireCurse => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::Prophecy => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 4,
            duration_ticks: 60,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::BogSongAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        // --- Croak: MurkCommander ---
        AbilityId::UndyingPresenceAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::GrokProtocol => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 450,
            gpu_cost: 8,
            duration_ticks: 120,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::MurkUplinkAbility => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        // --- Seekers of the Deep (Badgers) ---
        AbilityId::SubterraneanHaul => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 80,
            range: Fixed::ZERO,
            max_charges: 4,
        },
        AbilityId::Earthsense => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::EmergencyBurrow => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Unbowed => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::ShieldWall => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 180,
            gpu_cost: 0,
            duration_ticks: 60,
            range: Fixed::from_bits(2 << 16),
            max_charges: 0,
        },
        AbilityId::GrudgeCharge => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 20,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::BoulderBarrage => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::Entrench => AbilityDef {
            id,
            activation: Toggle,
            cooldown_ticks: 30,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SeismicSlam => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::VigilanceAura => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(5 << 16),
            max_charges: 0,
        },
        AbilityId::Intercept => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 160,
            gpu_cost: 4,
            duration_ticks: 30,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::RallyCry => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 220,
            gpu_cost: 0,
            duration_ticks: 50,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::ArmorRend => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::PatientStrike => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Lockjaw => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 200,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::from_bits(1 << 16),
            max_charges: 0,
        },
        AbilityId::DeepseekUplink => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::FortressProtocol => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 450,
            gpu_cost: 10,
            duration_ticks: 200,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::CalculatedCounterstrike => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 6,
            duration_ticks: 80,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::DeepBore => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 5,
            duration_ticks: 0,
            range: Fixed::from_bits(15 << 16),
            max_charges: 3,
        },
        AbilityId::Undermine => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 300,
            gpu_cost: 0,
            duration_ticks: 50,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::TremorNetwork => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(8 << 16),
            max_charges: 0,
        },
        AbilityId::MoltenShot => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(6 << 16),
            max_charges: 0,
        },
        AbilityId::FuelReserve => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 3,
        },
        AbilityId::ScorchedEarth => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 250,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
        AbilityId::DustCloud => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 140,
            gpu_cost: 0,
            duration_ticks: 50,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::AmbushInstinct => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::SentryBurrow => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 80,
            gpu_cost: 0,
            duration_ticks: 40,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::Frenzy => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::from_bits(3 << 16),
            max_charges: 0,
        },
        AbilityId::Bloodgreed => AbilityDef {
            id,
            activation: Passive,
            cooldown_ticks: 0,
            gpu_cost: 0,
            duration_ticks: 0,
            range: Fixed::ZERO,
            max_charges: 0,
        },
        AbilityId::RecklessLunge => AbilityDef {
            id,
            activation: Activated,
            cooldown_ticks: 150,
            gpu_cost: 0,
            duration_ticks: 30,
            range: Fixed::from_bits(4 << 16),
            max_charges: 0,
        },
    }
}

/// Data-driven lookup: returns the list of status effects an ability applies to self
/// when active. The bool indicates whether to use `duration_remaining` (true) or
/// a constant duration of 2 (false, for toggles).
pub fn self_buff_effects(
    id: AbilityId,
) -> &'static [(crate::status_effects::StatusEffectId, bool)] {
    use crate::status_effects::StatusEffectId;
    match id {
        // --- Cat faction ---
        AbilityId::Zoomies => &[(StatusEffectId::Zoomies, true)],
        AbilityId::LoafMode => &[(StatusEffectId::LoafModeActive, false)],
        AbilityId::SiegeNap => &[(StatusEffectId::SiegeNapDeployed, false)],
        AbilityId::SpiteCarry => &[(StatusEffectId::SpiteCarryBuff, true)],
        // --- The Clawed (Mice) ---
        AbilityId::ChewThrough => &[(StatusEffectId::DamageBuff, false)],
        AbilityId::SpineWall => &[(StatusEffectId::ArmorBuff, false)],
        AbilityId::MiasmaTrail => &[(StatusEffectId::DamageBuff, false)],
        AbilityId::PileOn => &[(StatusEffectId::DamageBuff, true)],
        AbilityId::Scatter => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::StubbornAdvance => &[
            (StatusEffectId::DamageBuff, true),
            (StatusEffectId::ArmorBuff, true),
        ],
        AbilityId::BurrowExpress => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::WhiskernetRelay => &[(StatusEffectId::DamageBuff, true)],
        // --- Seekers of the Deep (Badgers) ---
        AbilityId::Entrench => &[(StatusEffectId::Entrenched, false)],
        AbilityId::ShieldWall => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::GrudgeCharge | AbilityId::RecklessLunge => &[
            (StatusEffectId::SpeedBuff, true),
            (StatusEffectId::DamageBuff, true),
        ],
        AbilityId::SubterraneanHaul => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::EmergencyBurrow => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::Intercept => &[
            (StatusEffectId::SpeedBuff, true),
            (StatusEffectId::ArmorBuff, true),
        ],
        AbilityId::FortressProtocol => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::CalculatedCounterstrike => &[(StatusEffectId::DamageBuff, true)],
        AbilityId::SentryBurrow => &[(StatusEffectId::ArmorBuff, true)],
        // --- The Murder (Corvids) ---
        AbilityId::Overwatch => &[(StatusEffectId::ArmorBuff, false)],
        AbilityId::Pilfer => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::MirrorPosition => &[(StatusEffectId::SpeedBuff, true)],
        // --- LLAMA (Raccoons) ---
        AbilityId::PlayDead => &[(StatusEffectId::PlayingDead, true)],
        AbilityId::Scavenge => &[(StatusEffectId::SpiteCarryBuff, true)],
        AbilityId::Getaway => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::JuryRig => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::DuctTapeFix => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::JunkMortarMode => &[(StatusEffectId::JunkMortarDeployed, false)],
        AbilityId::TrashHeapAmbush => &[
            (StatusEffectId::DamageBuff, true),
            (StatusEffectId::SpeedBuff, true),
        ],
        AbilityId::LeakInjection => &[(StatusEffectId::DamageBuff, true)],
        AbilityId::RefuseShield => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::OverclockCascade => &[
            (StatusEffectId::DamageBuff, true),
            (StatusEffectId::SpeedBuff, true),
        ],
        // --- Croak (Axolotls) ---
        AbilityId::HunkerAbility => &[(StatusEffectId::LoafModeActive, false)],
        AbilityId::Inflate => &[(StatusEffectId::InflatedBombardment, true)],
        AbilityId::Hop => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::RegrowthBurst => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::PrimordialSoup => &[
            (StatusEffectId::ArmorBuff, true),
            (StatusEffectId::DamageBuff, true),
        ],
        AbilityId::Waterway => &[(StatusEffectId::SpeedBuff, true)],
        AbilityId::TidalMemory => &[(StatusEffectId::ArmorBuff, true)],
        AbilityId::GrokProtocol => &[
            (StatusEffectId::DamageBuff, true),
            (StatusEffectId::SpeedBuff, true),
        ],
        _ => &[],
    }
}

/// Catnapper DreamSiege damage multiplier — ramps the longer it attacks the same target.
pub fn dream_siege_multiplier(ticks_on_target: u32) -> Fixed {
    match ticks_on_target {
        0..=49 => Fixed::ONE,            // 1x for first 5s
        50..=149 => Fixed::from_num(2),  // 2x at 5-15s
        150..=299 => Fixed::from_num(4), // 4x at 15-30s
        _ => Fixed::from_num(8),         // 8x at 30s+
    }
}

/// Gnawer Structural Weakness damage multiplier — scales with consecutive hit stacks on a building.
/// Returns 1.0 + 0.02 * min(stacks, 10), so max multiplier is 1.20 (20% bonus).
pub fn gnawer_structural_weakness_multiplier(stacks: u32) -> Fixed {
    let capped = stacks.min(10);
    Fixed::from_bits(65536 + 1310 * capped as i32)
}

/// Gnawer Incisors Never Stop — damage bonus from continuous attacking.
/// Returns 0.01 * min(seconds, 40) as fixed-point, so max bonus is 0.40 (40%).
pub fn incisors_damage_bonus(continuous_ticks: u32) -> Fixed {
    let seconds = continuous_ticks / 10;
    let capped = seconds.min(40);
    Fixed::from_bits(655 * capped as i32)
}

/// Return the 3 ability IDs for a given unit kind.
pub fn unit_abilities(kind: UnitKind) -> [AbilityId; 3] {
    match kind {
        UnitKind::Pawdler => [
            AbilityId::OpportunisticHoarder,
            AbilityId::SpiteCarry,
            AbilityId::Revulsion,
        ],
        UnitKind::Nuisance => [
            AbilityId::AnnoyanceStacks,
            AbilityId::Hairball,
            AbilityId::Zoomies,
        ],
        UnitKind::Chonk => [
            AbilityId::GravitationalChonk,
            AbilityId::LoafMode,
            AbilityId::NineLives,
        ],
        UnitKind::FlyingFox => [
            AbilityId::EcholocationPulse,
            AbilityId::FruitDrop,
            AbilityId::Disoriented,
        ],
        UnitKind::Hisser => [
            AbilityId::CorrosiveSpit,
            AbilityId::DisgustMortar,
            AbilityId::Misinformation,
        ],
        UnitKind::Yowler => [
            AbilityId::HarmonicResonance,
            AbilityId::DissonantScreech,
            AbilityId::Lullaby,
        ],
        UnitKind::Mouser => [
            AbilityId::Tagged,
            AbilityId::DeadDrop,
            AbilityId::ShadowNetwork,
        ],
        UnitKind::Catnapper => [
            AbilityId::DreamSiege,
            AbilityId::ContagiousYawning,
            AbilityId::SiegeNap,
        ],
        UnitKind::FerretSapper => [
            AbilityId::ShapedCharge,
            AbilityId::BoobyTrap,
            AbilityId::TunnelNetwork,
        ],
        UnitKind::MechCommander => [
            AbilityId::TacticalUplink,
            AbilityId::Override,
            AbilityId::LeChatUplink,
        ],
        // --- LLAMA (Raccoons) ---
        UnitKind::Scrounger => [
            AbilityId::DumpsterDiveAbility,
            AbilityId::PocketStash,
            AbilityId::PlayDead,
        ],
        UnitKind::Bandit => [
            AbilityId::StickyFingers,
            AbilityId::JuryRig,
            AbilityId::Getaway,
        ],
        UnitKind::HeapTitan => [
            AbilityId::ScrapArmorAbility,
            AbilityId::WreckBall,
            AbilityId::MagneticPulse,
        ],
        UnitKind::GlitchRat => [
            AbilityId::CableGnaw,
            AbilityId::SignalScramble,
            AbilityId::TunnelRat,
        ],
        UnitKind::PatchPossum => [
            AbilityId::DuctTapeFix,
            AbilityId::SalvageResurrection,
            AbilityId::FeignDeath,
        ],
        UnitKind::GreaseMonkey => [
            AbilityId::JunkLauncher,
            AbilityId::SalvageTurret,
            AbilityId::JunkMortarMode,
        ],
        UnitKind::DeadDropUnit => [
            AbilityId::Eavesdrop,
            AbilityId::TrashHeapAmbush,
            AbilityId::LeakInjection,
        ],
        UnitKind::Wrecker => [
            AbilityId::Disassemble,
            AbilityId::PryBar,
            AbilityId::ChainBreak,
        ],
        UnitKind::DumpsterDiver => [
            AbilityId::TreasureTrash,
            AbilityId::RefuseShield,
            AbilityId::StenchCloudAbility,
        ],
        UnitKind::JunkyardKing => [
            AbilityId::OpenSourceUplinkAbility,
            AbilityId::FrankensteinProtocol,
            AbilityId::OverclockCascade,
        ],
        // --- The Murder (Corvids) ---
        UnitKind::MurderScrounger => [
            AbilityId::TrinketStash,
            AbilityId::Scavenge,
            AbilityId::MimicCall,
        ],
        UnitKind::Sentinel => [
            AbilityId::Glintwatch,
            AbilityId::Overwatch,
            AbilityId::EvasiveAscent,
        ],
        UnitKind::Rookclaw => [
            AbilityId::TalonDive,
            AbilityId::MurdersMark,
            AbilityId::CarrionInstinct,
        ],
        UnitKind::Magpike => [
            AbilityId::Pilfer,
            AbilityId::GlitterBomb,
            AbilityId::TrinketWard,
        ],
        UnitKind::Magpyre => [
            AbilityId::SignalJam,
            AbilityId::DecoyNest,
            AbilityId::Rewire,
        ],
        UnitKind::Jaycaller => [
            AbilityId::MurderRallyCry,
            AbilityId::AlarmCall,
            AbilityId::Cacophony,
        ],
        UnitKind::Jayflicker => [
            AbilityId::PhantomFlock,
            AbilityId::MirrorPosition,
            AbilityId::Refraction,
        ],
        UnitKind::Dusktalon => [
            AbilityId::Nightcloak,
            AbilityId::SilentStrike,
            AbilityId::PreySense,
        ],
        UnitKind::Hootseer => [
            AbilityId::PanopticGaze,
            AbilityId::DreadAuraAbility,
            AbilityId::DeathOmen,
        ],
        UnitKind::CorvusRex => [
            AbilityId::CorvidNetworkAbility,
            AbilityId::AllSeeingLie,
            AbilityId::OculusUplinkAbility,
        ],
        // --- Croak (Axolotls) ---
        UnitKind::Ponderer => [
            AbilityId::AmbientGathering,
            AbilityId::MucusTrail,
            AbilityId::ExistentialDread,
        ],
        UnitKind::Regeneron => [
            AbilityId::LimbToss,
            AbilityId::RegrowthBurst,
            AbilityId::PhantomLimb,
        ],
        UnitKind::Broodmother => [
            AbilityId::SpawnPool,
            AbilityId::Transfusion,
            AbilityId::PrimordialSoup,
        ],
        UnitKind::Gulper => [
            AbilityId::Devour,
            AbilityId::Regurgitate,
            AbilityId::Bottomless,
        ],
        UnitKind::Eftsaber => [
            AbilityId::ToxicSkin,
            AbilityId::Waterway,
            AbilityId::Venomstrike,
        ],
        UnitKind::Croaker => [
            AbilityId::BogMortar,
            AbilityId::ResonanceChain,
            AbilityId::Inflate,
        ],
        UnitKind::Leapfrog => [AbilityId::Hop, AbilityId::TongueLash, AbilityId::Slipstream],
        UnitKind::Shellwarden => [
            AbilityId::HunkerAbility,
            AbilityId::AncientMossAbility,
            AbilityId::TidalMemory,
        ],
        UnitKind::Bogwhisper => [
            AbilityId::MireCurse,
            AbilityId::Prophecy,
            AbilityId::BogSongAbility,
        ],
        UnitKind::MurkCommander => [
            AbilityId::UndyingPresenceAbility,
            AbilityId::GrokProtocol,
            AbilityId::MurkUplinkAbility,
        ],
        // --- Seekers of the Deep (Badgers) ---
        UnitKind::Delver => [
            AbilityId::SubterraneanHaul,
            AbilityId::Earthsense,
            AbilityId::EmergencyBurrow,
        ],
        UnitKind::Ironhide => [
            AbilityId::Unbowed,
            AbilityId::ShieldWall,
            AbilityId::GrudgeCharge,
        ],
        UnitKind::Cragback => [
            AbilityId::BoulderBarrage,
            AbilityId::Entrench,
            AbilityId::SeismicSlam,
        ],
        UnitKind::Warden => [
            AbilityId::VigilanceAura,
            AbilityId::Intercept,
            AbilityId::RallyCry,
        ],
        UnitKind::Sapjaw => [
            AbilityId::ArmorRend,
            AbilityId::PatientStrike,
            AbilityId::Lockjaw,
        ],
        UnitKind::Wardenmother => [
            AbilityId::DeepseekUplink,
            AbilityId::FortressProtocol,
            AbilityId::CalculatedCounterstrike,
        ],
        UnitKind::SeekerTunneler => [
            AbilityId::DeepBore,
            AbilityId::Undermine,
            AbilityId::TremorNetwork,
        ],
        UnitKind::Embermaw => [
            AbilityId::MoltenShot,
            AbilityId::FuelReserve,
            AbilityId::ScorchedEarth,
        ],
        UnitKind::Dustclaw => [
            AbilityId::DustCloud,
            AbilityId::AmbushInstinct,
            AbilityId::SentryBurrow,
        ],
        UnitKind::Gutripper => [
            AbilityId::Frenzy,
            AbilityId::Bloodgreed,
            AbilityId::RecklessLunge,
        ],
        // --- The Clawed (Mice) ---
        UnitKind::Nibblet => [
            AbilityId::CrumbTrail,
            AbilityId::StashNetwork,
            AbilityId::PanicProductivity,
        ],
        UnitKind::Swarmer => [
            AbilityId::SafetyInNumbers,
            AbilityId::PileOn,
            AbilityId::Scatter,
        ],
        UnitKind::Gnawer => [
            AbilityId::StructuralWeakness,
            AbilityId::ChewThrough,
            AbilityId::IncisorsNeverStop,
        ],
        UnitKind::Shrieker => [
            AbilityId::SonicSpit,
            AbilityId::EcholocationPing,
            AbilityId::SonicBarrage,
        ],
        UnitKind::Tunneler => [
            AbilityId::BurrowExpress,
            AbilityId::BurrowUndermine,
            AbilityId::SwarmTremorSense,
        ],
        UnitKind::Sparks => [
            AbilityId::StaticCharge,
            AbilityId::ShortCircuit,
            AbilityId::DaisyChain,
        ],
        UnitKind::Quillback => [
            AbilityId::SpineWall,
            AbilityId::QuillBurst,
            AbilityId::StubbornAdvance,
        ],
        UnitKind::Whiskerwitch => [
            AbilityId::HexOfMultiplication,
            AbilityId::WhiskerWeave,
            AbilityId::DatacromanticRitual,
        ],
        UnitKind::Plaguetail => [
            AbilityId::ContagionCloud,
            AbilityId::MiasmaTrail,
            AbilityId::SympathySickness,
        ],
        UnitKind::WarrenMarshal => [
            AbilityId::RallyTheSwarm,
            AbilityId::ExpendableHeroism,
            AbilityId::WhiskernetRelay,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All AbilityId variants have a valid AbilityDef.
    #[test]
    fn all_ability_defs_valid() {
        let all_ids = [
            // catGPT (30)
            AbilityId::OpportunisticHoarder,
            AbilityId::SpiteCarry,
            AbilityId::Revulsion,
            AbilityId::AnnoyanceStacks,
            AbilityId::Hairball,
            AbilityId::Zoomies,
            AbilityId::GravitationalChonk,
            AbilityId::LoafMode,
            AbilityId::NineLives,
            AbilityId::EcholocationPulse,
            AbilityId::FruitDrop,
            AbilityId::Disoriented,
            AbilityId::CorrosiveSpit,
            AbilityId::DisgustMortar,
            AbilityId::Misinformation,
            AbilityId::HarmonicResonance,
            AbilityId::DissonantScreech,
            AbilityId::Lullaby,
            AbilityId::Tagged,
            AbilityId::DeadDrop,
            AbilityId::ShadowNetwork,
            AbilityId::DreamSiege,
            AbilityId::ContagiousYawning,
            AbilityId::SiegeNap,
            AbilityId::ShapedCharge,
            AbilityId::BoobyTrap,
            AbilityId::TunnelNetwork,
            AbilityId::TacticalUplink,
            AbilityId::Override,
            AbilityId::LeChatUplink,
            // LLAMA (30)
            AbilityId::DumpsterDiveAbility,
            AbilityId::PocketStash,
            AbilityId::PlayDead,
            AbilityId::StickyFingers,
            AbilityId::JuryRig,
            AbilityId::Getaway,
            AbilityId::ScrapArmorAbility,
            AbilityId::WreckBall,
            AbilityId::MagneticPulse,
            AbilityId::CableGnaw,
            AbilityId::SignalScramble,
            AbilityId::TunnelRat,
            AbilityId::DuctTapeFix,
            AbilityId::SalvageResurrection,
            AbilityId::FeignDeath,
            AbilityId::JunkLauncher,
            AbilityId::SalvageTurret,
            AbilityId::JunkMortarMode,
            AbilityId::Eavesdrop,
            AbilityId::TrashHeapAmbush,
            AbilityId::LeakInjection,
            AbilityId::Disassemble,
            AbilityId::PryBar,
            AbilityId::ChainBreak,
            AbilityId::TreasureTrash,
            AbilityId::RefuseShield,
            AbilityId::StenchCloudAbility,
            AbilityId::OpenSourceUplinkAbility,
            AbilityId::FrankensteinProtocol,
            AbilityId::OverclockCascade,
            // Murder (30)
            AbilityId::TrinketStash,
            AbilityId::Scavenge,
            AbilityId::MimicCall,
            AbilityId::Glintwatch,
            AbilityId::Overwatch,
            AbilityId::EvasiveAscent,
            AbilityId::TalonDive,
            AbilityId::MurdersMark,
            AbilityId::CarrionInstinct,
            AbilityId::Pilfer,
            AbilityId::GlitterBomb,
            AbilityId::TrinketWard,
            AbilityId::SignalJam,
            AbilityId::DecoyNest,
            AbilityId::Rewire,
            AbilityId::MurderRallyCry,
            AbilityId::AlarmCall,
            AbilityId::Cacophony,
            AbilityId::PhantomFlock,
            AbilityId::MirrorPosition,
            AbilityId::Refraction,
            AbilityId::Nightcloak,
            AbilityId::SilentStrike,
            AbilityId::PreySense,
            AbilityId::PanopticGaze,
            AbilityId::DreadAuraAbility,
            AbilityId::DeathOmen,
            AbilityId::CorvidNetworkAbility,
            AbilityId::AllSeeingLie,
            AbilityId::OculusUplinkAbility,
            // Croak (30)
            AbilityId::AmbientGathering,
            AbilityId::MucusTrail,
            AbilityId::ExistentialDread,
            AbilityId::LimbToss,
            AbilityId::RegrowthBurst,
            AbilityId::PhantomLimb,
            AbilityId::SpawnPool,
            AbilityId::Transfusion,
            AbilityId::PrimordialSoup,
            AbilityId::Devour,
            AbilityId::Regurgitate,
            AbilityId::Bottomless,
            AbilityId::ToxicSkin,
            AbilityId::Waterway,
            AbilityId::Venomstrike,
            AbilityId::BogMortar,
            AbilityId::ResonanceChain,
            AbilityId::Inflate,
            AbilityId::Hop,
            AbilityId::TongueLash,
            AbilityId::Slipstream,
            AbilityId::HunkerAbility,
            AbilityId::AncientMossAbility,
            AbilityId::TidalMemory,
            AbilityId::MireCurse,
            AbilityId::Prophecy,
            AbilityId::BogSongAbility,
            AbilityId::UndyingPresenceAbility,
            AbilityId::GrokProtocol,
            AbilityId::MurkUplinkAbility,
            // Seekers (30)
            AbilityId::SubterraneanHaul,
            AbilityId::Earthsense,
            AbilityId::EmergencyBurrow,
            AbilityId::Unbowed,
            AbilityId::ShieldWall,
            AbilityId::GrudgeCharge,
            AbilityId::BoulderBarrage,
            AbilityId::Entrench,
            AbilityId::SeismicSlam,
            AbilityId::VigilanceAura,
            AbilityId::Intercept,
            AbilityId::RallyCry,
            AbilityId::ArmorRend,
            AbilityId::PatientStrike,
            AbilityId::Lockjaw,
            AbilityId::DeepseekUplink,
            AbilityId::FortressProtocol,
            AbilityId::CalculatedCounterstrike,
            AbilityId::DeepBore,
            AbilityId::Undermine,
            AbilityId::TremorNetwork,
            AbilityId::MoltenShot,
            AbilityId::FuelReserve,
            AbilityId::ScorchedEarth,
            AbilityId::DustCloud,
            AbilityId::AmbushInstinct,
            AbilityId::SentryBurrow,
            AbilityId::Frenzy,
            AbilityId::Bloodgreed,
            AbilityId::RecklessLunge,
            // The Clawed (30)
            AbilityId::CrumbTrail,
            AbilityId::StashNetwork,
            AbilityId::PanicProductivity,
            AbilityId::SafetyInNumbers,
            AbilityId::PileOn,
            AbilityId::Scatter,
            AbilityId::StructuralWeakness,
            AbilityId::ChewThrough,
            AbilityId::IncisorsNeverStop,
            AbilityId::SonicSpit,
            AbilityId::EcholocationPing,
            AbilityId::SonicBarrage,
            AbilityId::BurrowExpress,
            AbilityId::BurrowUndermine,
            AbilityId::SwarmTremorSense,
            AbilityId::StaticCharge,
            AbilityId::ShortCircuit,
            AbilityId::DaisyChain,
            AbilityId::SpineWall,
            AbilityId::QuillBurst,
            AbilityId::StubbornAdvance,
            AbilityId::HexOfMultiplication,
            AbilityId::WhiskerWeave,
            AbilityId::DatacromanticRitual,
            AbilityId::ContagionCloud,
            AbilityId::MiasmaTrail,
            AbilityId::SympathySickness,
            AbilityId::RallyTheSwarm,
            AbilityId::ExpendableHeroism,
            AbilityId::WhiskernetRelay,
        ];
        assert_eq!(all_ids.len(), 180);
        for id in all_ids {
            let def = ability_def(id);
            assert_eq!(def.id, id, "{id:?} def should match its id");
        }
    }

    /// Every unit kind returns exactly 3 distinct abilities.
    #[test]
    fn unit_abilities_returns_three_per_kind() {
        let kinds = [
            // catGPT
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
            // LLAMA
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
            // Murder
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
            // Croak
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
            // Seekers
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
            // The Clawed
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
        ];
        for kind in kinds {
            let abilities = unit_abilities(kind);
            assert_eq!(abilities.len(), 3, "{kind:?} should have 3 abilities");
            // All three should be distinct
            assert_ne!(
                abilities[0], abilities[1],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[1], abilities[2],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[0], abilities[2],
                "{kind:?} abilities should be distinct"
            );
        }
    }

    /// Passive abilities have zero cooldown.
    #[test]
    fn passive_abilities_no_cooldown() {
        let passives = [
            // catGPT
            AbilityId::OpportunisticHoarder,
            AbilityId::AnnoyanceStacks,
            AbilityId::GravitationalChonk,
            AbilityId::CorrosiveSpit,
            AbilityId::DreamSiege,
            // Murder
            AbilityId::TrinketStash,
            AbilityId::Glintwatch,
            AbilityId::MurdersMark,
            AbilityId::CarrionInstinct,
            AbilityId::TrinketWard,
            AbilityId::Nightcloak,
            AbilityId::PreySense,
            AbilityId::DreadAuraAbility,
            AbilityId::CorvidNetworkAbility,
            AbilityId::OculusUplinkAbility,
            // LLAMA
            AbilityId::DumpsterDiveAbility,
            AbilityId::PocketStash,
            AbilityId::StickyFingers,
            AbilityId::ScrapArmorAbility,
            AbilityId::TunnelRat,
            AbilityId::JunkLauncher,
            AbilityId::Eavesdrop,
            AbilityId::Disassemble,
            AbilityId::TreasureTrash,
            AbilityId::OpenSourceUplinkAbility,
            // Croak
            AbilityId::AmbientGathering,
            AbilityId::MucusTrail,
            AbilityId::PhantomLimb,
            AbilityId::Bottomless,
            AbilityId::ToxicSkin,
            AbilityId::BogMortar,
            AbilityId::ResonanceChain,
            AbilityId::Slipstream,
            AbilityId::AncientMossAbility,
            AbilityId::BogSongAbility,
            AbilityId::UndyingPresenceAbility,
            AbilityId::MurkUplinkAbility,
        ];
        for id in passives {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Passive,
                "{id:?} should be passive"
            );
            assert_eq!(
                def.cooldown_ticks, 0,
                "{id:?} passive should have 0 cooldown"
            );
        }
    }

    /// Toggle abilities have short cooldowns to prevent spam.
    #[test]
    fn toggle_abilities_have_cooldown() {
        let toggles = [
            AbilityId::LoafMode,
            AbilityId::HarmonicResonance,
            AbilityId::Lullaby,
            AbilityId::TacticalUplink,
            AbilityId::SiegeNap,
            // Murder
            AbilityId::Overwatch,
            AbilityId::PanopticGaze,
            // LLAMA
            AbilityId::JunkMortarMode,
            // Croak
            AbilityId::HunkerAbility,
        ];
        for id in toggles {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Toggle,
                "{id:?} should be toggle"
            );
            assert!(def.cooldown_ticks > 0, "{id:?} toggle should have cooldown");
        }
    }

    /// Activated abilities have nonzero cooldown.
    #[test]
    fn activated_abilities_have_cooldown() {
        let activated = [
            // catGPT
            AbilityId::SpiteCarry,
            AbilityId::Hairball,
            AbilityId::Zoomies,
            AbilityId::EcholocationPulse,
            AbilityId::DisgustMortar,
            AbilityId::DissonantScreech,
            AbilityId::Override,
            // LLAMA
            AbilityId::PlayDead,
            AbilityId::JuryRig,
            AbilityId::Getaway,
            AbilityId::WreckBall,
            AbilityId::MagneticPulse,
            AbilityId::CableGnaw,
            AbilityId::SignalScramble,
            AbilityId::DuctTapeFix,
            AbilityId::SalvageResurrection,
            AbilityId::SalvageTurret,
            AbilityId::TrashHeapAmbush,
            AbilityId::LeakInjection,
            AbilityId::PryBar,
            AbilityId::ChainBreak,
            AbilityId::RefuseShield,
            AbilityId::StenchCloudAbility,
            AbilityId::FrankensteinProtocol,
            AbilityId::OverclockCascade,
            // Clawed
            AbilityId::SonicBarrage,
            // Murder
            AbilityId::Scavenge,
            AbilityId::MimicCall,
            AbilityId::TalonDive,
            AbilityId::Pilfer,
            AbilityId::GlitterBomb,
            AbilityId::SignalJam,
            AbilityId::DecoyNest,
            AbilityId::Rewire,
            AbilityId::MurderRallyCry,
            AbilityId::Cacophony,
            AbilityId::PhantomFlock,
            AbilityId::MirrorPosition,
            AbilityId::SilentStrike,
            AbilityId::DeathOmen,
            AbilityId::AllSeeingLie,
            // Croak
            AbilityId::LimbToss,
            AbilityId::RegrowthBurst,
            AbilityId::PrimordialSoup,
            AbilityId::Devour,
            AbilityId::Regurgitate,
            AbilityId::Waterway,
            AbilityId::Venomstrike,
            AbilityId::Inflate,
            AbilityId::Hop,
            AbilityId::TongueLash,
            AbilityId::TidalMemory,
            AbilityId::MireCurse,
            AbilityId::Prophecy,
            AbilityId::GrokProtocol,
        ];
        for id in activated {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Activated,
                "{id:?} should be activated"
            );
            assert!(
                def.cooldown_ticks > 0,
                "{id:?} activated should have cooldown"
            );
        }
    }

    #[test]
    fn dream_siege_multiplier_tiers() {
        assert_eq!(dream_siege_multiplier(0), Fixed::ONE);
        assert_eq!(dream_siege_multiplier(49), Fixed::ONE);
        assert_eq!(dream_siege_multiplier(50), Fixed::from_num(2));
        assert_eq!(dream_siege_multiplier(149), Fixed::from_num(2));
        assert_eq!(dream_siege_multiplier(150), Fixed::from_num(4));
        assert_eq!(dream_siege_multiplier(299), Fixed::from_num(4));
        assert_eq!(dream_siege_multiplier(300), Fixed::from_num(8));
        assert_eq!(dream_siege_multiplier(1000), Fixed::from_num(8));
    }

    // --- Murder ability tests ---

    #[test]
    fn all_murder_ability_defs_valid() {
        let murder_ids = [
            AbilityId::TrinketStash,
            AbilityId::Scavenge,
            AbilityId::MimicCall,
            AbilityId::Glintwatch,
            AbilityId::Overwatch,
            AbilityId::EvasiveAscent,
            AbilityId::TalonDive,
            AbilityId::MurdersMark,
            AbilityId::CarrionInstinct,
            AbilityId::Pilfer,
            AbilityId::GlitterBomb,
            AbilityId::TrinketWard,
            AbilityId::SignalJam,
            AbilityId::DecoyNest,
            AbilityId::Rewire,
            AbilityId::MurderRallyCry,
            AbilityId::AlarmCall,
            AbilityId::Cacophony,
            AbilityId::PhantomFlock,
            AbilityId::MirrorPosition,
            AbilityId::Refraction,
            AbilityId::Nightcloak,
            AbilityId::SilentStrike,
            AbilityId::PreySense,
            AbilityId::PanopticGaze,
            AbilityId::DreadAuraAbility,
            AbilityId::DeathOmen,
            AbilityId::CorvidNetworkAbility,
            AbilityId::AllSeeingLie,
            AbilityId::OculusUplinkAbility,
        ];
        assert_eq!(murder_ids.len(), 30);
        for id in murder_ids {
            let def = ability_def(id);
            assert_eq!(def.id, id, "{id:?} def should match its id");
        }
    }

    #[test]
    fn murder_unit_abilities_returns_three() {
        let kinds = [
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
        ];
        for kind in kinds {
            let abilities = unit_abilities(kind);
            assert_eq!(abilities.len(), 3, "{kind:?} should have 3 abilities");
            assert_ne!(
                abilities[0], abilities[1],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[1], abilities[2],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[0], abilities[2],
                "{kind:?} abilities should be distinct"
            );
        }
    }

    #[test]
    fn murder_toggle_abilities_have_cooldown() {
        let toggles = [AbilityId::Overwatch, AbilityId::PanopticGaze];
        for id in toggles {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Toggle,
                "{id:?} should be toggle"
            );
            assert!(def.cooldown_ticks > 0, "{id:?} toggle should have cooldown");
        }
    }

    #[test]
    fn murder_passive_abilities_no_cooldown() {
        let passives = [
            AbilityId::TrinketStash,
            AbilityId::Glintwatch,
            AbilityId::MurdersMark,
            AbilityId::CarrionInstinct,
            AbilityId::TrinketWard,
            AbilityId::Nightcloak,
            AbilityId::PreySense,
            AbilityId::DreadAuraAbility,
            AbilityId::CorvidNetworkAbility,
            AbilityId::OculusUplinkAbility,
        ];
        for id in passives {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Passive,
                "{id:?} should be passive"
            );
            assert_eq!(
                def.cooldown_ticks, 0,
                "{id:?} passive should have 0 cooldown"
            );
        }
    }

    // --- Seekers of the Deep ability tests ---

    #[test]
    fn all_seekers_ability_defs_valid() {
        let seekers_ids = [
            AbilityId::SubterraneanHaul,
            AbilityId::Earthsense,
            AbilityId::EmergencyBurrow,
            AbilityId::Unbowed,
            AbilityId::ShieldWall,
            AbilityId::GrudgeCharge,
            AbilityId::BoulderBarrage,
            AbilityId::Entrench,
            AbilityId::SeismicSlam,
            AbilityId::VigilanceAura,
            AbilityId::Intercept,
            AbilityId::RallyCry,
            AbilityId::ArmorRend,
            AbilityId::PatientStrike,
            AbilityId::Lockjaw,
            AbilityId::DeepseekUplink,
            AbilityId::FortressProtocol,
            AbilityId::CalculatedCounterstrike,
            AbilityId::DeepBore,
            AbilityId::Undermine,
            AbilityId::TremorNetwork,
            AbilityId::MoltenShot,
            AbilityId::FuelReserve,
            AbilityId::ScorchedEarth,
            AbilityId::DustCloud,
            AbilityId::AmbushInstinct,
            AbilityId::SentryBurrow,
            AbilityId::Frenzy,
            AbilityId::Bloodgreed,
            AbilityId::RecklessLunge,
        ];
        assert_eq!(seekers_ids.len(), 30);
        for id in seekers_ids {
            let def = ability_def(id);
            assert_eq!(def.id, id, "{id:?} def should match its id");
        }
    }

    #[test]
    fn seekers_unit_abilities_returns_three() {
        let kinds = [
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
        ];
        for kind in kinds {
            let abilities = unit_abilities(kind);
            assert_eq!(abilities.len(), 3, "{kind:?} should have 3 abilities");
            assert_ne!(
                abilities[0], abilities[1],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[1], abilities[2],
                "{kind:?} abilities should be distinct"
            );
            assert_ne!(
                abilities[0], abilities[2],
                "{kind:?} abilities should be distinct"
            );
        }
    }

    #[test]
    fn seekers_passive_abilities_no_cooldown() {
        let passives = [
            AbilityId::Earthsense,
            AbilityId::Unbowed,
            AbilityId::BoulderBarrage,
            AbilityId::VigilanceAura,
            AbilityId::ArmorRend,
            AbilityId::PatientStrike,
            AbilityId::DeepseekUplink,
            AbilityId::TremorNetwork,
            AbilityId::MoltenShot,
            AbilityId::FuelReserve,
            AbilityId::AmbushInstinct,
            AbilityId::Frenzy,
            AbilityId::Bloodgreed,
        ];
        for id in passives {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Passive,
                "{id:?} should be passive"
            );
            assert_eq!(
                def.cooldown_ticks, 0,
                "{id:?} passive should have 0 cooldown"
            );
        }
    }

    #[test]
    fn seekers_toggle_abilities_have_cooldown() {
        let toggles = [AbilityId::Entrench];
        for id in toggles {
            let def = ability_def(id);
            assert_eq!(
                def.activation,
                AbilityActivation::Toggle,
                "{id:?} should be toggle"
            );
            assert!(def.cooldown_ticks > 0, "{id:?} toggle should have cooldown");
        }
    }

    #[test]
    fn fortress_protocol_is_expensive() {
        let def = ability_def(AbilityId::FortressProtocol);
        assert_eq!(def.activation, AbilityActivation::Activated);
        assert_eq!(def.gpu_cost, 10);
        assert_eq!(def.cooldown_ticks, 450);
        assert_eq!(def.duration_ticks, 200);
    }
}
