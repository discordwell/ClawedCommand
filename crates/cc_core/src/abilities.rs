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
    PowerNap,
    // FerretSapper (demo)
    ShapedCharge,
    BoobyTrap,
    TunnelNetwork,
    // MechCommander (hero)
    TacticalUplink,
    Override,
    GeppityUplink,
    // --- LLAMA (Raccoons) ---
    DumpsterDiveAbility, PocketStash, PlayDead,
    StickyFingers, JuryRig, Getaway,
    ScrapArmorAbility, WreckBall, MagneticPulse,
    CableGnaw, SignalScramble, TunnelRat,
    DuctTapeFix, SalvageResurrection, FeignDeath,
    JunkLauncher, SalvageTurret, Overcharge,
    Eavesdrop, TrashHeapAmbush, LeakInjection,
    Disassemble, PryBar, ChainBreak,
    TreasureTrash, RefuseShield, StenchCloudAbility,
    OpenSourceUplinkAbility, FrankensteinProtocol, OverclockCascade,
    // --- Croak (Axolotls) ---
    // Ponderer (worker)
    AmbientGathering, MucusTrail, ExistentialDread,
    // Regeneron (skirmisher)
    LimbToss, RegrowthBurst, PhantomLimb,
    // Broodmother (support)
    SpawnPool, Transfusion, PrimordialSoup,
    // Gulper (heavy)
    Devour, Regurgitate, Bottomless,
    // Eftsaber (assassin)
    ToxicSkin, Waterway, Venomstrike,
    // Croaker (artillery)
    BogMortar, ResonanceChain, Inflate,
    // Leapfrog (harasser)
    Hop, TongueLash, Slipstream,
    // Shellwarden (tank)
    HunkerAbility, AncientMossAbility, TidalMemory,
    // Bogwhisper (caster)
    MireCurse, Prophecy, BogSongAbility,
    // MurkCommander (hero)
    UndyingPresenceAbility, GrokProtocol, MurkUplinkAbility,
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
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::SpiteCarry => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 5,
            duration_ticks: 50, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::Revulsion => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 5,
            duration_ticks: 0, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },

        // --- Nuisance ---
        AbilityId::AnnoyanceStacks => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::Hairball => AbilityDef {
            id, activation: Activated, cooldown_ticks: 60, gpu_cost: 5,
            duration_ticks: 0, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },
        AbilityId::Zoomies => AbilityDef {
            id, activation: Activated, cooldown_ticks: 120, gpu_cost: 10,
            duration_ticks: 30, range: Fixed::ZERO, max_charges: 0,
        },

        // --- Chonk ---
        AbilityId::GravitationalChonk => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        AbilityId::LoafMode => AbilityDef {
            id, activation: Toggle, cooldown_ticks: 10, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::NineLives => AbilityDef {
            id, activation: Passive, cooldown_ticks: 600, gpu_cost: 25,
            duration_ticks: 30, range: Fixed::ZERO, max_charges: 0,
        },

        // --- FlyingFox ---
        AbilityId::EcholocationPulse => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::from_bits(6 << 16), max_charges: 0,
        },
        AbilityId::FruitDrop => AbilityDef {
            id, activation: Activated, cooldown_ticks: 60, gpu_cost: 5,
            duration_ticks: 0, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        AbilityId::Disoriented => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 15,
            duration_ticks: 30, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },

        // --- Hisser ---
        AbilityId::CorrosiveSpit => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::DisgustMortar => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::from_bits(6 << 16), max_charges: 0,
        },
        AbilityId::Misinformation => AbilityDef {
            id, activation: Activated, cooldown_ticks: 150, gpu_cost: 20,
            duration_ticks: 50, range: Fixed::from_bits(5 << 16), max_charges: 0,
        },

        // --- Yowler ---
        AbilityId::HarmonicResonance => AbilityDef {
            id, activation: Toggle, cooldown_ticks: 10, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },
        AbilityId::DissonantScreech => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 10,
            duration_ticks: 30, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },
        AbilityId::Lullaby => AbilityDef {
            id, activation: Toggle, cooldown_ticks: 10, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },

        // --- Mouser ---
        AbilityId::Tagged => AbilityDef {
            id, activation: Activated, cooldown_ticks: 60, gpu_cost: 5,
            duration_ticks: 100, range: Fixed::from_bits(5 << 16), max_charges: 0,
        },
        AbilityId::DeadDrop => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 3,
        },
        AbilityId::ShadowNetwork => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 25,
            duration_ticks: 80, range: Fixed::ZERO, max_charges: 0,
        },

        // --- Catnapper ---
        AbilityId::DreamSiege => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::ContagiousYawning => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 15,
            duration_ticks: 20, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        AbilityId::PowerNap => AbilityDef {
            id, activation: Activated, cooldown_ticks: 150, gpu_cost: 10,
            duration_ticks: 40, range: Fixed::ZERO, max_charges: 0,
        },

        // --- FerretSapper ---
        AbilityId::ShapedCharge => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::from_bits(1 << 16), max_charges: 0,
        },
        AbilityId::BoobyTrap => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 3,
        },
        AbilityId::TunnelNetwork => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 30,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },

        // --- MechCommander ---
        AbilityId::TacticalUplink => AbilityDef {
            id, activation: Toggle, cooldown_ticks: 10, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(5 << 16), max_charges: 0,
        },
        AbilityId::Override => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 30,
            duration_ticks: 80, range: Fixed::from_bits(6 << 16), max_charges: 0,
        },
        AbilityId::GeppityUplink => AbilityDef {
            id, activation: Activated, cooldown_ticks: 300, gpu_cost: 50,
            duration_ticks: 100, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Scrounger ---
        AbilityId::DumpsterDiveAbility => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::PocketStash => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::PlayDead => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 0,
            duration_ticks: 80, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Bandit ---
        AbilityId::StickyFingers => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::JuryRig => AbilityDef {
            id, activation: Activated, cooldown_ticks: 50, gpu_cost: 0,
            duration_ticks: 20, range: Fixed::from_bits(1 << 16), max_charges: 0,
        },
        AbilityId::Getaway => AbilityDef {
            id, activation: Activated, cooldown_ticks: 150, gpu_cost: 0,
            duration_ticks: 15, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Heap Titan ---
        AbilityId::ScrapArmorAbility => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },
        AbilityId::WreckBall => AbilityDef {
            id, activation: Activated, cooldown_ticks: 120, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(5 << 16), max_charges: 0,
        },
        AbilityId::MagneticPulse => AbilityDef {
            id, activation: Activated, cooldown_ticks: 250, gpu_cost: 0,
            duration_ticks: 40, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        // --- LLAMA: Glitch Rat ---
        AbilityId::CableGnaw => AbilityDef {
            id, activation: Activated, cooldown_ticks: 300, gpu_cost: 0,
            duration_ticks: 30, range: Fixed::from_bits(1 << 16), max_charges: 0,
        },
        AbilityId::SignalScramble => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 4,
            duration_ticks: 40, range: Fixed::from_bits(6 << 16), max_charges: 0,
        },
        AbilityId::TunnelRat => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Patch Possum ---
        AbilityId::DuctTapeFix => AbilityDef {
            id, activation: Activated, cooldown_ticks: 100, gpu_cost: 0,
            duration_ticks: 50, range: Fixed::from_bits(4 << 16), max_charges: 0,
        },
        AbilityId::SalvageResurrection => AbilityDef {
            id, activation: Activated, cooldown_ticks: 250, gpu_cost: 0,
            duration_ticks: 40, range: Fixed::from_bits(1 << 16), max_charges: 0,
        },
        AbilityId::FeignDeath => AbilityDef {
            id, activation: Passive, cooldown_ticks: 450, gpu_cost: 0,
            duration_ticks: 30, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Grease Monkey ---
        AbilityId::JunkLauncher => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::SalvageTurret => AbilityDef {
            id, activation: Activated, cooldown_ticks: 150, gpu_cost: 0,
            duration_ticks: 200, range: Fixed::from_bits(2 << 16), max_charges: 1,
        },
        AbilityId::Overcharge => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 0,
            duration_ticks: 60, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Dead Drop ---
        AbilityId::Eavesdrop => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(8 << 16), max_charges: 0,
        },
        AbilityId::TrashHeapAmbush => AbilityDef {
            id, activation: Activated, cooldown_ticks: 80, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::LeakInjection => AbilityDef {
            id, activation: Activated, cooldown_ticks: 300, gpu_cost: 5,
            duration_ticks: 40, range: Fixed::ZERO, max_charges: 0,
        },
        // --- LLAMA: Wrecker ---
        AbilityId::Disassemble => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::PryBar => AbilityDef {
            id, activation: Activated, cooldown_ticks: 180, gpu_cost: 0,
            duration_ticks: 40, range: Fixed::from_bits(1 << 16), max_charges: 0,
        },
        AbilityId::ChainBreak => AbilityDef {
            id, activation: Activated, cooldown_ticks: 140, gpu_cost: 0,
            duration_ticks: 60, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        // --- LLAMA: Dumpster Diver ---
        AbilityId::TreasureTrash => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
        AbilityId::RefuseShield => AbilityDef {
            id, activation: Activated, cooldown_ticks: 200, gpu_cost: 0,
            duration_ticks: 150, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        AbilityId::StenchCloudAbility => AbilityDef {
            id, activation: Activated, cooldown_ticks: 180, gpu_cost: 0,
            duration_ticks: 60, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        // --- LLAMA: Junkyard King ---
        AbilityId::OpenSourceUplinkAbility => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::from_bits(8 << 16), max_charges: 0,
        },
        AbilityId::FrankensteinProtocol => AbilityDef {
            id, activation: Activated, cooldown_ticks: 450, gpu_cost: 10,
            duration_ticks: 0, range: Fixed::from_bits(3 << 16), max_charges: 0,
        },
        AbilityId::OverclockCascade => AbilityDef {
            id, activation: Activated, cooldown_ticks: 350, gpu_cost: 0,
            duration_ticks: 80, range: Fixed::from_bits(6 << 16), max_charges: 0,
        },
        // Non-cat/LLAMA faction abilities — placeholder stats
        _ => AbilityDef {
            id, activation: Passive, cooldown_ticks: 0, gpu_cost: 0,
            duration_ticks: 0, range: Fixed::ZERO, max_charges: 0,
        },
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
            AbilityId::PowerNap,
        ],
        UnitKind::FerretSapper => [
            AbilityId::ShapedCharge,
            AbilityId::BoobyTrap,
            AbilityId::TunnelNetwork,
        ],
        UnitKind::MechCommander => [
            AbilityId::TacticalUplink,
            AbilityId::Override,
            AbilityId::GeppityUplink,
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
            AbilityId::Overcharge,
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
        other => unimplemented!("unit_abilities not yet defined for {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All 60 AbilityId variants (30 cat + 30 LLAMA) have a valid AbilityDef.
    #[test]
    fn all_ability_defs_valid() {
        let all_ids = [
            // catGPT (30)
            AbilityId::OpportunisticHoarder, AbilityId::SpiteCarry, AbilityId::Revulsion,
            AbilityId::AnnoyanceStacks, AbilityId::Hairball, AbilityId::Zoomies,
            AbilityId::GravitationalChonk, AbilityId::LoafMode, AbilityId::NineLives,
            AbilityId::EcholocationPulse, AbilityId::FruitDrop, AbilityId::Disoriented,
            AbilityId::CorrosiveSpit, AbilityId::DisgustMortar, AbilityId::Misinformation,
            AbilityId::HarmonicResonance, AbilityId::DissonantScreech, AbilityId::Lullaby,
            AbilityId::Tagged, AbilityId::DeadDrop, AbilityId::ShadowNetwork,
            AbilityId::DreamSiege, AbilityId::ContagiousYawning, AbilityId::PowerNap,
            AbilityId::ShapedCharge, AbilityId::BoobyTrap, AbilityId::TunnelNetwork,
            AbilityId::TacticalUplink, AbilityId::Override, AbilityId::GeppityUplink,
            // LLAMA (30)
            AbilityId::DumpsterDiveAbility, AbilityId::PocketStash, AbilityId::PlayDead,
            AbilityId::StickyFingers, AbilityId::JuryRig, AbilityId::Getaway,
            AbilityId::ScrapArmorAbility, AbilityId::WreckBall, AbilityId::MagneticPulse,
            AbilityId::CableGnaw, AbilityId::SignalScramble, AbilityId::TunnelRat,
            AbilityId::DuctTapeFix, AbilityId::SalvageResurrection, AbilityId::FeignDeath,
            AbilityId::JunkLauncher, AbilityId::SalvageTurret, AbilityId::Overcharge,
            AbilityId::Eavesdrop, AbilityId::TrashHeapAmbush, AbilityId::LeakInjection,
            AbilityId::Disassemble, AbilityId::PryBar, AbilityId::ChainBreak,
            AbilityId::TreasureTrash, AbilityId::RefuseShield, AbilityId::StenchCloudAbility,
            AbilityId::OpenSourceUplinkAbility, AbilityId::FrankensteinProtocol, AbilityId::OverclockCascade,
        ];
        assert_eq!(all_ids.len(), 60);
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
            UnitKind::Pawdler, UnitKind::Nuisance, UnitKind::Chonk,
            UnitKind::FlyingFox, UnitKind::Hisser, UnitKind::Yowler,
            UnitKind::Mouser, UnitKind::Catnapper, UnitKind::FerretSapper,
            UnitKind::MechCommander,
            // LLAMA
            UnitKind::Scrounger, UnitKind::Bandit, UnitKind::HeapTitan,
            UnitKind::GlitchRat, UnitKind::PatchPossum, UnitKind::GreaseMonkey,
            UnitKind::DeadDropUnit, UnitKind::Wrecker, UnitKind::DumpsterDiver,
            UnitKind::JunkyardKing,
        ];
        for kind in kinds {
            let abilities = unit_abilities(kind);
            assert_eq!(abilities.len(), 3, "{kind:?} should have 3 abilities");
            // All three should be distinct
            assert_ne!(abilities[0], abilities[1], "{kind:?} abilities should be distinct");
            assert_ne!(abilities[1], abilities[2], "{kind:?} abilities should be distinct");
            assert_ne!(abilities[0], abilities[2], "{kind:?} abilities should be distinct");
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
        ];
        for id in passives {
            let def = ability_def(id);
            assert_eq!(def.activation, AbilityActivation::Passive, "{id:?} should be passive");
            assert_eq!(def.cooldown_ticks, 0, "{id:?} passive should have 0 cooldown");
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
        ];
        for id in toggles {
            let def = ability_def(id);
            assert_eq!(def.activation, AbilityActivation::Toggle, "{id:?} should be toggle");
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
            AbilityId::Overcharge,
            AbilityId::TrashHeapAmbush,
            AbilityId::LeakInjection,
            AbilityId::PryBar,
            AbilityId::ChainBreak,
            AbilityId::RefuseShield,
            AbilityId::StenchCloudAbility,
            AbilityId::FrankensteinProtocol,
            AbilityId::OverclockCascade,
        ];
        for id in activated {
            let def = ability_def(id);
            assert_eq!(def.activation, AbilityActivation::Activated, "{id:?} should be activated");
            assert!(def.cooldown_ticks > 0, "{id:?} activated should have cooldown");
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
}
