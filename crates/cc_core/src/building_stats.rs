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
}
