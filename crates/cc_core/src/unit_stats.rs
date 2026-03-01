use crate::components::{AttackType, UnitKind};
use crate::math::Fixed;

/// Compile-time base stats for each unit type.
pub struct UnitBaseStats {
    pub health: Fixed,
    pub speed: Fixed,
    pub damage: Fixed,
    pub range: Fixed,
    pub attack_speed: u32, // ticks between attacks
    pub attack_type: AttackType,
    // Economy
    pub food_cost: u32,
    pub gpu_cost: u32,
    pub supply_cost: u32,
    pub train_time: u32, // ticks
}

/// Return the base stats for a given unit kind.
/// All values are compile-time constants — no Resource needed.
pub fn base_stats(kind: UnitKind) -> UnitBaseStats {
    match kind {
        UnitKind::Pawdler => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),   // 60
            speed: Fixed::from_bits(7864),         // 0.12
            damage: Fixed::from_bits(4 << 16),     // 4
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 50,
        },
        UnitKind::Nuisance => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),    // 80
            speed: Fixed::from_bits(11796),         // 0.18
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 60,
        },
        UnitKind::Chonk => UnitBaseStats {
            health: Fixed::from_bits(300 << 16),   // 300
            speed: Fixed::from_bits(5242),          // 0.08
            damage: Fixed::from_bits(12 << 16),    // 12
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 20,
            attack_type: AttackType::Melee,
            food_cost: 150, gpu_cost: 25, supply_cost: 3, train_time: 120,
        },
        UnitKind::FlyingFox => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),    // 50
            speed: Fixed::from_bits(14745),         // 0.225
            damage: Fixed::from_bits(6 << 16),     // 6
            range: Fixed::from_bits(2 << 16),      // 2
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 25, supply_cost: 2, train_time: 80,
        },
        UnitKind::Hisser => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),    // 70
            speed: Fixed::from_bits(7864),          // 0.12
            damage: Fixed::from_bits(14 << 16),    // 14
            range: Fixed::from_bits(5 << 16),      // 5
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 0, supply_cost: 2, train_time: 80,
        },
        UnitKind::Yowler => UnitBaseStats {
            health: Fixed::from_bits(90 << 16),    // 90
            speed: Fixed::from_bits(9175),          // 0.14
            damage: Fixed::from_bits(5 << 16),     // 5
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 50, supply_cost: 2, train_time: 100,
        },
        UnitKind::Mouser => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(13107),         // 0.20
            damage: Fixed::from_bits(10 << 16),    // 10
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 25, supply_cost: 1, train_time: 60,
        },
        UnitKind::Catnapper => UnitBaseStats {
            health: Fixed::from_bits(120 << 16),   // 120
            speed: Fixed::from_bits(3932),          // 0.06
            damage: Fixed::from_bits(25 << 16),    // 25
            range: Fixed::from_bits(2 << 16),      // 2
            attack_speed: 30,
            attack_type: AttackType::Ranged,
            food_cost: 200, gpu_cost: 50, supply_cost: 3, train_time: 150,
        },
        UnitKind::FerretSapper => UnitBaseStats {
            health: Fixed::from_bits(65 << 16),    // 65
            speed: Fixed::from_bits(11141),         // 0.17
            damage: Fixed::from_bits(20 << 16),    // 20
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 25,
            attack_type: AttackType::Melee,
            food_cost: 125, gpu_cost: 50, supply_cost: 2, train_time: 100,
        },
        UnitKind::MechCommander => UnitBaseStats {
            health: Fixed::from_bits(500 << 16),   // 500
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(18 << 16),    // 18
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400, gpu_cost: 200, supply_cost: 6, train_time: 250,
        },
        other => unimplemented!("base_stats not yet defined for {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_stats() {
        let kinds = [
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
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
            assert!(stats.speed > Fixed::ZERO, "{kind:?} should have positive speed");
            assert!(stats.damage > Fixed::ZERO, "{kind:?} should have positive damage");
            assert!(stats.range > Fixed::ZERO, "{kind:?} should have positive range");
            assert!(stats.attack_speed > 0, "{kind:?} should have positive attack_speed");
        }
    }

    #[test]
    fn melee_units_have_range_one() {
        let melee_kinds = [
            UnitKind::Pawdler,
            UnitKind::Nuisance,
            UnitKind::Chonk,
            UnitKind::Mouser,
            UnitKind::FerretSapper,
        ];
        for kind in melee_kinds {
            let stats = base_stats(kind);
            assert_eq!(stats.attack_type, AttackType::Melee, "{kind:?} should be melee");
            assert_eq!(
                stats.range,
                Fixed::from_bits(1 << 16),
                "{kind:?} melee should have range 1"
            );
        }
    }

    #[test]
    fn ranged_units_have_range_greater_than_one() {
        let ranged_kinds = [
            UnitKind::FlyingFox,
            UnitKind::Hisser,
            UnitKind::Yowler,
            UnitKind::Catnapper,
            UnitKind::MechCommander,
        ];
        for kind in ranged_kinds {
            let stats = base_stats(kind);
            assert_eq!(stats.attack_type, AttackType::Ranged, "{kind:?} should be ranged");
            assert!(
                stats.range > Fixed::from_bits(1 << 16),
                "{kind:?} ranged should have range > 1"
            );
        }
    }

    #[test]
    fn chonk_is_tankiest() {
        // Among non-hero units, Chonk should have the most HP
        let chonk = base_stats(UnitKind::Chonk);
        let nuisance = base_stats(UnitKind::Nuisance);
        let hisser = base_stats(UnitKind::Hisser);
        assert!(chonk.health > nuisance.health);
        assert!(chonk.health > hisser.health);
    }

    #[test]
    fn mech_commander_is_strongest() {
        let mech = base_stats(UnitKind::MechCommander);
        let chonk = base_stats(UnitKind::Chonk);
        assert!(mech.health > chonk.health);
    }
}
