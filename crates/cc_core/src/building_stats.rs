use crate::components::{BuildingKind, UnitKind};
use crate::math::Fixed;

/// Compile-time base stats for each building type.
pub struct BuildingBaseStats {
    pub health: Fixed,
    pub build_time: u32,    // ticks (0 = pre-built)
    pub food_cost: u32,
    pub gpu_cost: u32,
    pub supply_provided: u32,
    pub can_produce: &'static [UnitKind],
}

/// Return the base stats for a given building kind.
pub fn building_stats(kind: BuildingKind) -> BuildingBaseStats {
    match kind {
        BuildingKind::TheBox => BuildingBaseStats {
            health: Fixed::from_bits(500 << 16),
            build_time: 0, // pre-built
            food_cost: 0,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[UnitKind::Pawdler],
        },
        BuildingKind::CatTree => BuildingBaseStats {
            health: Fixed::from_bits(300 << 16),
            build_time: 150, // 15 seconds at 10hz
            food_cost: 150,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Nuisance,
                UnitKind::Hisser,
                UnitKind::Chonk,
                UnitKind::Yowler,
            ],
        },
        BuildingKind::FishMarket => BuildingBaseStats {
            health: Fixed::from_bits(200 << 16),
            build_time: 100, // 10 seconds
            food_cost: 100,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::LitterBox => BuildingBaseStats {
            health: Fixed::from_bits(100 << 16),
            build_time: 75, // 7.5 seconds
            food_cost: 75,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[],
        },
        BuildingKind::ServerRack => BuildingBaseStats {
            health: Fixed::from_bits(250 << 16),
            build_time: 120, // 12 seconds
            food_cost: 100,
            gpu_cost: 75,
            supply_provided: 0,
            can_produce: &[
                UnitKind::FlyingFox,
                UnitKind::Mouser,
                UnitKind::Catnapper,
                UnitKind::FerretSapper,
                UnitKind::MechCommander,
            ],
        },
        BuildingKind::ScratchingPost => BuildingBaseStats {
            health: Fixed::from_bits(200 << 16),
            build_time: 100, // 10 seconds
            food_cost: 100,
            gpu_cost: 50,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::CatFlap => BuildingBaseStats {
            health: Fixed::from_bits(400 << 16),
            build_time: 100, // 10 seconds
            food_cost: 150,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::LaserPointer => BuildingBaseStats {
            health: Fixed::from_bits(150 << 16),
            build_time: 80, // 8 seconds
            food_cost: 75,
            gpu_cost: 25,
            supply_provided: 0,
            can_produce: &[],
        },
        // --- The Clawed (Mice) ---
        BuildingKind::TheBurrow => BuildingBaseStats {
            health: Fixed::from_bits(400 << 16),
            build_time: 0, // pre-built
            food_cost: 0,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[UnitKind::Nibblet],
        },
        BuildingKind::NestingBox => BuildingBaseStats {
            health: Fixed::from_bits(250 << 16),
            build_time: 120, // 12 seconds
            food_cost: 100,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Swarmer,
                UnitKind::Gnawer,
                UnitKind::Plaguetail,
                UnitKind::Sparks,
            ],
        },
        BuildingKind::SeedVault => BuildingBaseStats {
            health: Fixed::from_bits(180 << 16),
            build_time: 80, // 8 seconds
            food_cost: 75,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::JunkTransmitter => BuildingBaseStats {
            health: Fixed::from_bits(190 << 16),
            build_time: 100, // 10 seconds
            food_cost: 75,
            gpu_cost: 50,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Shrieker,
                UnitKind::Tunneler,
                UnitKind::Quillback,
                UnitKind::Whiskerwitch,
                UnitKind::WarrenMarshal,
            ],
        },
        BuildingKind::GnawLab => BuildingBaseStats {
            health: Fixed::from_bits(160 << 16),
            build_time: 100, // 10 seconds
            food_cost: 80,
            gpu_cost: 40,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::WarrenExpansion => BuildingBaseStats {
            health: Fixed::from_bits(80 << 16),
            build_time: 60, // 6 seconds
            food_cost: 50,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[],
        },
        BuildingKind::Mousehole => BuildingBaseStats {
            health: Fixed::from_bits(300 << 16),
            build_time: 80, // 8 seconds
            food_cost: 100,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::SqueakTower => BuildingBaseStats {
            health: Fixed::from_bits(120 << 16),
            build_time: 70, // 7 seconds
            food_cost: 60,
            gpu_cost: 20,
            supply_provided: 0,
            can_produce: &[],
        },
        // --- Croak (Axolotls) ---
        BuildingKind::TheGrotto => BuildingBaseStats {
            health: Fixed::from_bits(500 << 16),
            build_time: 0, // pre-built
            food_cost: 0,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[UnitKind::Ponderer],
        },
        BuildingKind::SpawningPools => BuildingBaseStats {
            health: Fixed::from_bits(300 << 16),
            build_time: 150, // 15 seconds
            food_cost: 150,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Regeneron,
                UnitKind::Croaker,
                UnitKind::Leapfrog,
                UnitKind::Gulper,
            ],
        },
        BuildingKind::LilyMarket => BuildingBaseStats {
            health: Fixed::from_bits(200 << 16),
            build_time: 100, // 10 seconds
            food_cost: 100,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::SunkenServer => BuildingBaseStats {
            health: Fixed::from_bits(250 << 16),
            build_time: 120, // 12 seconds
            food_cost: 100,
            gpu_cost: 75,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Eftsaber,
                UnitKind::Broodmother,
                UnitKind::Shellwarden,
                UnitKind::Bogwhisper,
                UnitKind::MurkCommander,
            ],
        },
        BuildingKind::FossilStones => BuildingBaseStats {
            health: Fixed::from_bits(200 << 16),
            build_time: 100, // 10 seconds
            food_cost: 100,
            gpu_cost: 50,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::ReedBed => BuildingBaseStats {
            health: Fixed::from_bits(100 << 16),
            build_time: 75, // 7.5 seconds
            food_cost: 75,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[],
        },
        BuildingKind::TidalGate => BuildingBaseStats {
            health: Fixed::from_bits(400 << 16),
            build_time: 100, // 10 seconds
            food_cost: 150,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::SporeTower => BuildingBaseStats {
            health: Fixed::from_bits(150 << 16),
            build_time: 80, // 8 seconds
            food_cost: 75,
            gpu_cost: 25,
            supply_provided: 0,
            can_produce: &[],
        },
        // --- The Murder (Corvids) ---
        BuildingKind::TheParliament => BuildingBaseStats {
            health: Fixed::from_bits(450 << 16),
            build_time: 0, // pre-built
            food_cost: 0,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[UnitKind::MurderScrounger],
        },
        BuildingKind::Rookery => BuildingBaseStats {
            health: Fixed::from_bits(275 << 16),
            build_time: 140, // 14 seconds
            food_cost: 150,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Sentinel,
                UnitKind::Rookclaw,
                UnitKind::Magpike,
                UnitKind::Jaycaller,
            ],
        },
        BuildingKind::CarrionCache => BuildingBaseStats {
            health: Fixed::from_bits(180 << 16),
            build_time: 100, // 10 seconds
            food_cost: 100,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::AntennaArray => BuildingBaseStats {
            health: Fixed::from_bits(225 << 16),
            build_time: 120, // 12 seconds
            food_cost: 100,
            gpu_cost: 75,
            supply_provided: 0,
            can_produce: &[
                UnitKind::Magpyre,
                UnitKind::Jayflicker,
                UnitKind::Dusktalon,
                UnitKind::Hootseer,
                UnitKind::CorvusRex,
            ],
        },
        BuildingKind::Panopticon => BuildingBaseStats {
            health: Fixed::from_bits(200 << 16),
            build_time: 120, // 12 seconds
            food_cost: 125,
            gpu_cost: 75,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::NestBox => BuildingBaseStats {
            health: Fixed::from_bits(90 << 16),
            build_time: 75, // 7.5 seconds
            food_cost: 75,
            gpu_cost: 0,
            supply_provided: 10,
            can_produce: &[],
        },
        BuildingKind::ThornHedge => BuildingBaseStats {
            health: Fixed::from_bits(120 << 16),
            build_time: 40, // 4 seconds
            food_cost: 30,
            gpu_cost: 0,
            supply_provided: 0,
            can_produce: &[],
        },
        BuildingKind::Watchtower => BuildingBaseStats {
            health: Fixed::from_bits(140 << 16),
            build_time: 80, // 8 seconds
            food_cost: 75,
            gpu_cost: 25,
            supply_provided: 0,
            can_produce: &[],
        },
        // --- Seekers of the Deep (Badgers) ---
        BuildingKind::TheSett => BuildingBaseStats { health: Fixed::from_bits(600 << 16), build_time: 0, food_cost: 0, gpu_cost: 0, supply_provided: 10, can_produce: &[UnitKind::Delver] },
        BuildingKind::WarHollow => BuildingBaseStats { health: Fixed::from_bits(400 << 16), build_time: 180, food_cost: 175, gpu_cost: 0, supply_provided: 0, can_produce: &[UnitKind::Ironhide, UnitKind::Sapjaw, UnitKind::Warden, UnitKind::Gutripper] },
        BuildingKind::BurrowDepot => BuildingBaseStats { health: Fixed::from_bits(250 << 16), build_time: 120, food_cost: 100, gpu_cost: 0, supply_provided: 0, can_produce: &[] },
        BuildingKind::CoreTap => BuildingBaseStats { health: Fixed::from_bits(300 << 16), build_time: 150, food_cost: 125, gpu_cost: 100, supply_provided: 0, can_produce: &[UnitKind::SeekerTunneler, UnitKind::Embermaw, UnitKind::Dustclaw, UnitKind::Cragback, UnitKind::Wardenmother] },
        BuildingKind::ClawMarks => BuildingBaseStats { health: Fixed::from_bits(250 << 16), build_time: 120, food_cost: 125, gpu_cost: 75, supply_provided: 0, can_produce: &[] },
        BuildingKind::DeepWarren => BuildingBaseStats { health: Fixed::from_bits(125 << 16), build_time: 95, food_cost: 80, gpu_cost: 0, supply_provided: 12, can_produce: &[] },
        BuildingKind::BulwarkGate => BuildingBaseStats { health: Fixed::from_bits(500 << 16), build_time: 120, food_cost: 175, gpu_cost: 0, supply_provided: 0, can_produce: &[] },
        BuildingKind::SlagThrower => BuildingBaseStats { health: Fixed::from_bits(200 << 16), build_time: 100, food_cost: 100, gpu_cost: 50, supply_provided: 0, can_produce: &[] },
        // --- LLAMA (Raccoons) ---
        BuildingKind::TheDumpster => BuildingBaseStats { health: Fixed::from_bits(500 << 16), build_time: 0, food_cost: 0, gpu_cost: 0, supply_provided: 10, can_produce: &[UnitKind::Scrounger] },
        BuildingKind::ScrapHeap => BuildingBaseStats { health: Fixed::from_bits(180 << 16), build_time: 90, food_cost: 90, gpu_cost: 0, supply_provided: 0, can_produce: &[] },
        BuildingKind::ChopShop => BuildingBaseStats { health: Fixed::from_bits(280 << 16), build_time: 140, food_cost: 140, gpu_cost: 0, supply_provided: 0, can_produce: &[UnitKind::Bandit, UnitKind::Wrecker, UnitKind::HeapTitan, UnitKind::GreaseMonkey] },
        BuildingKind::JunkServer => BuildingBaseStats { health: Fixed::from_bits(230 << 16), build_time: 110, food_cost: 90, gpu_cost: 65, supply_provided: 0, can_produce: &[UnitKind::GlitchRat, UnitKind::PatchPossum] },
        BuildingKind::TinkerBench => BuildingBaseStats { health: Fixed::from_bits(190 << 16), build_time: 95, food_cost: 85, gpu_cost: 55, supply_provided: 0, can_produce: &[UnitKind::DeadDropUnit, UnitKind::DumpsterDiver, UnitKind::JunkyardKing] },
        BuildingKind::TrashPile => BuildingBaseStats { health: Fixed::from_bits(90 << 16), build_time: 70, food_cost: 70, gpu_cost: 0, supply_provided: 10, can_produce: &[] },
        BuildingKind::DumpsterRelay => BuildingBaseStats { health: Fixed::from_bits(150 << 16), build_time: 80, food_cost: 80, gpu_cost: 30, supply_provided: 0, can_produce: &[] },
        BuildingKind::TetanusTower => BuildingBaseStats { health: Fixed::from_bits(140 << 16), build_time: 75, food_cost: 70, gpu_cost: 20, supply_provided: 0, can_produce: &[] },
        other => unimplemented!("building_stats not yet defined for {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheBox,
            BuildingKind::CatTree,
            BuildingKind::FishMarket,
            BuildingKind::LitterBox,
            BuildingKind::ServerRack,
            BuildingKind::ScratchingPost,
            BuildingKind::CatFlap,
            BuildingKind::LaserPointer,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn the_box_is_pre_built() {
        let stats = building_stats(BuildingKind::TheBox);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn the_box_produces_pawdler() {
        let stats = building_stats(BuildingKind::TheBox);
        assert!(stats.can_produce.contains(&UnitKind::Pawdler));
    }

    #[test]
    fn cat_tree_produces_basic_combat_units() {
        let stats = building_stats(BuildingKind::CatTree);
        assert!(stats.can_produce.contains(&UnitKind::Nuisance));
        assert!(stats.can_produce.contains(&UnitKind::Hisser));
        assert!(stats.can_produce.contains(&UnitKind::Chonk));
        assert!(stats.can_produce.contains(&UnitKind::Yowler));
        // CatTree should NOT produce advanced units (those moved to ServerRack)
        assert!(!stats.can_produce.contains(&UnitKind::FlyingFox));
        assert!(!stats.can_produce.contains(&UnitKind::MechCommander));
    }

    #[test]
    fn server_rack_produces_advanced_units() {
        let stats = building_stats(BuildingKind::ServerRack);
        assert!(stats.can_produce.contains(&UnitKind::FlyingFox));
        assert!(stats.can_produce.contains(&UnitKind::Mouser));
        assert!(stats.can_produce.contains(&UnitKind::Catnapper));
        assert!(stats.can_produce.contains(&UnitKind::FerretSapper));
        assert!(stats.can_produce.contains(&UnitKind::MechCommander));
    }

    #[test]
    fn litter_box_provides_supply() {
        let stats = building_stats(BuildingKind::LitterBox);
        assert_eq!(stats.supply_provided, 10);
    }

    #[test]
    fn laser_pointer_has_zero_supply() {
        let stats = building_stats(BuildingKind::LaserPointer);
        assert_eq!(stats.supply_provided, 0);
        assert!(stats.can_produce.is_empty());
    }

    #[test]
    fn scratching_post_no_production() {
        let stats = building_stats(BuildingKind::ScratchingPost);
        assert!(stats.can_produce.is_empty());
    }

    // --- Croak building tests ---

    #[test]
    fn all_croak_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheGrotto,
            BuildingKind::SpawningPools,
            BuildingKind::LilyMarket,
            BuildingKind::SunkenServer,
            BuildingKind::FossilStones,
            BuildingKind::ReedBed,
            BuildingKind::TidalGate,
            BuildingKind::SporeTower,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn grotto_is_pre_built() {
        let stats = building_stats(BuildingKind::TheGrotto);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn grotto_produces_ponderer() {
        let stats = building_stats(BuildingKind::TheGrotto);
        assert!(stats.can_produce.contains(&UnitKind::Ponderer));
    }

    #[test]
    fn spawning_pools_produces_basic_croak_units() {
        let stats = building_stats(BuildingKind::SpawningPools);
        assert!(stats.can_produce.contains(&UnitKind::Regeneron));
        assert!(stats.can_produce.contains(&UnitKind::Croaker));
        assert!(stats.can_produce.contains(&UnitKind::Leapfrog));
        assert!(stats.can_produce.contains(&UnitKind::Gulper));
    }

    #[test]
    fn sunken_server_produces_advanced_croak_units() {
        let stats = building_stats(BuildingKind::SunkenServer);
        assert!(stats.can_produce.contains(&UnitKind::Eftsaber));
        assert!(stats.can_produce.contains(&UnitKind::Broodmother));
        assert!(stats.can_produce.contains(&UnitKind::Shellwarden));
        assert!(stats.can_produce.contains(&UnitKind::Bogwhisper));
        assert!(stats.can_produce.contains(&UnitKind::MurkCommander));
    }

    #[test]
    fn reed_bed_provides_supply() {
        let stats = building_stats(BuildingKind::ReedBed);
        assert_eq!(stats.supply_provided, 10);
    }

    // --- Murder building tests ---

    #[test]
    fn all_murder_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheParliament,
            BuildingKind::Rookery,
            BuildingKind::CarrionCache,
            BuildingKind::AntennaArray,
            BuildingKind::Panopticon,
            BuildingKind::NestBox,
            BuildingKind::ThornHedge,
            BuildingKind::Watchtower,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn the_parliament_is_pre_built() {
        let stats = building_stats(BuildingKind::TheParliament);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn the_parliament_produces_murder_scrounger() {
        let stats = building_stats(BuildingKind::TheParliament);
        assert!(stats.can_produce.contains(&UnitKind::MurderScrounger));
    }

    #[test]
    fn rookery_produces_basic_murder_units() {
        let stats = building_stats(BuildingKind::Rookery);
        assert!(stats.can_produce.contains(&UnitKind::Sentinel));
        assert!(stats.can_produce.contains(&UnitKind::Rookclaw));
        assert!(stats.can_produce.contains(&UnitKind::Magpike));
        assert!(stats.can_produce.contains(&UnitKind::Jaycaller));
    }

    #[test]
    fn antenna_array_produces_advanced_murder_units() {
        let stats = building_stats(BuildingKind::AntennaArray);
        assert!(stats.can_produce.contains(&UnitKind::Magpyre));
        assert!(stats.can_produce.contains(&UnitKind::Jayflicker));
        assert!(stats.can_produce.contains(&UnitKind::Dusktalon));
        assert!(stats.can_produce.contains(&UnitKind::Hootseer));
        assert!(stats.can_produce.contains(&UnitKind::CorvusRex));
    }

    #[test]
    fn nest_box_provides_supply() {
        let stats = building_stats(BuildingKind::NestBox);
        assert_eq!(stats.supply_provided, 10);
    }

    #[test]
    fn panopticon_no_production() {
        let stats = building_stats(BuildingKind::Panopticon);
        assert!(stats.can_produce.is_empty());
    }

    // --- Clawed building tests ---

    #[test]
    fn all_clawed_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheBurrow,
            BuildingKind::NestingBox,
            BuildingKind::SeedVault,
            BuildingKind::JunkTransmitter,
            BuildingKind::GnawLab,
            BuildingKind::WarrenExpansion,
            BuildingKind::Mousehole,
            BuildingKind::SqueakTower,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn the_burrow_is_pre_built() {
        let stats = building_stats(BuildingKind::TheBurrow);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn nesting_box_produces_basic_clawed() {
        let stats = building_stats(BuildingKind::NestingBox);
        assert!(stats.can_produce.contains(&UnitKind::Swarmer));
        assert!(stats.can_produce.contains(&UnitKind::Gnawer));
        assert!(stats.can_produce.contains(&UnitKind::Plaguetail));
        assert!(stats.can_produce.contains(&UnitKind::Sparks));
    }

    #[test]
    fn junk_transmitter_produces_advanced_clawed() {
        let stats = building_stats(BuildingKind::JunkTransmitter);
        assert!(stats.can_produce.contains(&UnitKind::Shrieker));
        assert!(stats.can_produce.contains(&UnitKind::Tunneler));
        assert!(stats.can_produce.contains(&UnitKind::Quillback));
        assert!(stats.can_produce.contains(&UnitKind::Whiskerwitch));
        assert!(stats.can_produce.contains(&UnitKind::WarrenMarshal));
    }

    #[test]
    fn warren_expansion_provides_supply() {
        let stats = building_stats(BuildingKind::WarrenExpansion);
        assert_eq!(stats.supply_provided, 10);
    }

    // --- Seekers building tests ---

    #[test]
    fn all_seekers_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheSett, BuildingKind::WarHollow, BuildingKind::BurrowDepot,
            BuildingKind::CoreTap, BuildingKind::ClawMarks, BuildingKind::DeepWarren,
            BuildingKind::BulwarkGate, BuildingKind::SlagThrower,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn the_sett_is_pre_built() {
        let stats = building_stats(BuildingKind::TheSett);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn the_sett_produces_delver() {
        let stats = building_stats(BuildingKind::TheSett);
        assert!(stats.can_produce.contains(&UnitKind::Delver));
    }

    #[test]
    fn war_hollow_produces_basic_seekers_units() {
        let stats = building_stats(BuildingKind::WarHollow);
        assert!(stats.can_produce.contains(&UnitKind::Ironhide));
        assert!(stats.can_produce.contains(&UnitKind::Sapjaw));
        assert!(stats.can_produce.contains(&UnitKind::Warden));
        assert!(stats.can_produce.contains(&UnitKind::Gutripper));
    }

    #[test]
    fn core_tap_produces_advanced_seekers_units() {
        let stats = building_stats(BuildingKind::CoreTap);
        assert!(stats.can_produce.contains(&UnitKind::SeekerTunneler));
        assert!(stats.can_produce.contains(&UnitKind::Embermaw));
        assert!(stats.can_produce.contains(&UnitKind::Dustclaw));
        assert!(stats.can_produce.contains(&UnitKind::Cragback));
        assert!(stats.can_produce.contains(&UnitKind::Wardenmother));
    }

    #[test]
    fn deep_warren_provides_supply() {
        let stats = building_stats(BuildingKind::DeepWarren);
        assert_eq!(stats.supply_provided, 12);
    }

    #[test]
    fn slag_thrower_no_production() {
        let stats = building_stats(BuildingKind::SlagThrower);
        assert!(stats.can_produce.is_empty());
    }

    // --- LLAMA building tests ---

    #[test]
    fn all_llama_buildings_have_stats() {
        let kinds = [
            BuildingKind::TheDumpster,
            BuildingKind::ScrapHeap,
            BuildingKind::ChopShop,
            BuildingKind::JunkServer,
            BuildingKind::TinkerBench,
            BuildingKind::TrashPile,
            BuildingKind::DumpsterRelay,
            BuildingKind::TetanusTower,
        ];
        for kind in kinds {
            let stats = building_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
        }
    }

    #[test]
    fn the_dumpster_is_pre_built() {
        let stats = building_stats(BuildingKind::TheDumpster);
        assert_eq!(stats.build_time, 0);
        assert_eq!(stats.food_cost, 0);
        assert_eq!(stats.gpu_cost, 0);
    }

    #[test]
    fn the_dumpster_produces_scrounger() {
        let stats = building_stats(BuildingKind::TheDumpster);
        assert!(stats.can_produce.contains(&UnitKind::Scrounger));
    }

    #[test]
    fn chop_shop_produces_basic_combat_units() {
        let stats = building_stats(BuildingKind::ChopShop);
        assert!(stats.can_produce.contains(&UnitKind::Bandit));
        assert!(stats.can_produce.contains(&UnitKind::Wrecker));
        assert!(stats.can_produce.contains(&UnitKind::HeapTitan));
        assert!(stats.can_produce.contains(&UnitKind::GreaseMonkey));
    }

    #[test]
    fn tinker_bench_produces_advanced_units() {
        let stats = building_stats(BuildingKind::TinkerBench);
        assert!(stats.can_produce.contains(&UnitKind::DeadDropUnit));
        assert!(stats.can_produce.contains(&UnitKind::DumpsterDiver));
        assert!(stats.can_produce.contains(&UnitKind::JunkyardKing));
    }

    #[test]
    fn trash_pile_provides_supply() {
        let stats = building_stats(BuildingKind::TrashPile);
        assert_eq!(stats.supply_provided, 10);
    }
}
