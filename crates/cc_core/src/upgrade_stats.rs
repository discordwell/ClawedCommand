use crate::components::UpgradeType;

/// Compile-time stats for each upgrade type.
pub struct UpgradeBaseStats {
    /// Ticks to research (at 10hz).
    pub research_time: u32,
    pub food_cost: u32,
    pub gpu_cost: u32,
}

/// Return the base stats for a given upgrade type.
pub fn upgrade_stats(upgrade: UpgradeType) -> UpgradeBaseStats {
    match upgrade {
        // --- catGPT ---
        UpgradeType::SharperClaws | UpgradeType::ThickerFur => UpgradeBaseStats {
            research_time: 100, // 10s (was 20s)
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::NimblePaws => UpgradeBaseStats {
            research_time: 75, // 7.5s (was 15s)
            food_cost: 75,
            gpu_cost: 15,
        },
        UpgradeType::SiegeTraining => UpgradeBaseStats {
            research_time: 250, // 25s
            food_cost: 200,
            gpu_cost: 100,
        },
        UpgradeType::MechPrototype => UpgradeBaseStats {
            research_time: 400, // 40s
            food_cost: 400,
            gpu_cost: 200,
        },
        // --- The Clawed ---
        UpgradeType::SharperTeeth | UpgradeType::ThickerHide => UpgradeBaseStats {
            research_time: 100,
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::QuickPaws => UpgradeBaseStats {
            research_time: 75,
            food_cost: 75,
            gpu_cost: 15,
        },
        // --- Seekers of the Deep ---
        UpgradeType::SharperFangs | UpgradeType::ReinforcedHide => UpgradeBaseStats {
            research_time: 100,
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::SteadyStance => UpgradeBaseStats {
            research_time: 75,
            food_cost: 75,
            gpu_cost: 15,
        },
        // --- The Murder ---
        UpgradeType::SharperTalons | UpgradeType::HardenedPlumage => UpgradeBaseStats {
            research_time: 100,
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::SwiftWings => UpgradeBaseStats {
            research_time: 75,
            food_cost: 75,
            gpu_cost: 15,
        },
        // --- LLAMA ---
        UpgradeType::RustyFangs | UpgradeType::ScrapPlating => UpgradeBaseStats {
            research_time: 100,
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::TrashRunning => UpgradeBaseStats {
            research_time: 75,
            food_cost: 75,
            gpu_cost: 15,
        },
        // --- Croak ---
        UpgradeType::SlickerMucus | UpgradeType::TougherHide => UpgradeBaseStats {
            research_time: 100,
            food_cost: 100,
            gpu_cost: 25,
        },
        UpgradeType::AmphibianAgility => UpgradeBaseStats {
            research_time: 75,
            food_cost: 75,
            gpu_cost: 15,
        },
        // Gate upgrades (unlock units) — keep original timings
        _ => UpgradeBaseStats {
            research_time: 250,
            food_cost: 200,
            gpu_cost: 100,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_upgrades_have_stats() {
        let upgrades = [
            UpgradeType::SharperClaws,
            UpgradeType::ThickerFur,
            UpgradeType::NimblePaws,
            UpgradeType::SiegeTraining,
            UpgradeType::MechPrototype,
        ];
        for upgrade in upgrades {
            let stats = upgrade_stats(upgrade);
            assert!(
                stats.research_time > 0,
                "{upgrade:?} should have research time"
            );
            assert!(stats.food_cost > 0, "{upgrade:?} should have food cost");
        }
    }

    #[test]
    fn mech_prototype_is_most_expensive() {
        let mech = upgrade_stats(UpgradeType::MechPrototype);
        let claws = upgrade_stats(UpgradeType::SharperClaws);
        assert!(mech.food_cost > claws.food_cost);
        assert!(mech.gpu_cost > claws.gpu_cost);
        assert!(mech.research_time > claws.research_time);
    }

    #[test]
    fn damage_hp_upgrades_halved() {
        // All faction damage/HP upgrades should be 100 ticks, 100f, 25gpu
        let damage_hp = [
            UpgradeType::SharperClaws,
            UpgradeType::ThickerFur,
            UpgradeType::SharperTeeth,
            UpgradeType::ThickerHide,
            UpgradeType::SharperFangs,
            UpgradeType::ReinforcedHide,
            UpgradeType::SharperTalons,
            UpgradeType::HardenedPlumage,
            UpgradeType::RustyFangs,
            UpgradeType::ScrapPlating,
            UpgradeType::SlickerMucus,
            UpgradeType::TougherHide,
        ];
        for upgrade in damage_hp {
            let stats = upgrade_stats(upgrade);
            assert_eq!(stats.research_time, 100, "{upgrade:?} research_time");
            assert_eq!(stats.food_cost, 100, "{upgrade:?} food_cost");
            assert_eq!(stats.gpu_cost, 25, "{upgrade:?} gpu_cost");
        }
    }

    #[test]
    fn speed_upgrades_halved() {
        // All faction speed upgrades should be 75 ticks, 75f, 15gpu
        let speed = [
            UpgradeType::NimblePaws,
            UpgradeType::QuickPaws,
            UpgradeType::SteadyStance,
            UpgradeType::SwiftWings,
            UpgradeType::TrashRunning,
            UpgradeType::AmphibianAgility,
        ];
        for upgrade in speed {
            let stats = upgrade_stats(upgrade);
            assert_eq!(stats.research_time, 75, "{upgrade:?} research_time");
            assert_eq!(stats.food_cost, 75, "{upgrade:?} food_cost");
            assert_eq!(stats.gpu_cost, 15, "{upgrade:?} gpu_cost");
        }
    }
}
