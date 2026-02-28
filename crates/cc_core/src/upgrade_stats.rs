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
        UpgradeType::SharperClaws => UpgradeBaseStats {
            research_time: 200, // 20s
            food_cost: 150,
            gpu_cost: 50,
        },
        UpgradeType::ThickerFur => UpgradeBaseStats {
            research_time: 200, // 20s
            food_cost: 150,
            gpu_cost: 50,
        },
        UpgradeType::NimblePaws => UpgradeBaseStats {
            research_time: 150, // 15s
            food_cost: 100,
            gpu_cost: 25,
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
            assert!(stats.research_time > 0, "{upgrade:?} should have research time");
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
}
