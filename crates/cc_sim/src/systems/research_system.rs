use bevy::prelude::*;

use cc_core::components::{
    AttackStats, Dead, Health, MovementSpeed, Owner, ResearchQueue, Researcher, UnitKind, UnitType,
    UpgradeType,
};
use cc_core::math::Fixed;

use crate::resources::PlayerResources;

/// Categories of stat-boosting upgrades. Gate upgrades unlock units, not stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeCategory {
    /// +20% damage to all combat units.
    Damage,
    /// +25% max HP to all combat units (and heal the bonus).
    Health,
    /// +10% speed to all units.
    Speed,
    /// Unlock gate — no stat bonus.
    Gate,
}

/// Classify an upgrade into its stat-boost category.
pub fn upgrade_category(upgrade: UpgradeType) -> UpgradeCategory {
    use UpgradeType::*;
    match upgrade {
        // Damage upgrades — one per faction
        SharperClaws | SharperTeeth | SharperFangs | SharperTalons | RustyFangs | SlickerMucus => {
            UpgradeCategory::Damage
        }

        // HP upgrades — one per faction
        ThickerFur | ThickerHide | ReinforcedHide | HardenedPlumage | ScrapPlating
        | TougherHide => UpgradeCategory::Health,

        // Speed upgrades — one per faction
        NimblePaws | QuickPaws | SteadyStance | SwiftWings | TrashRunning | AmphibianAgility => {
            UpgradeCategory::Speed
        }

        // Everything else is a gate (unlocks units, no stat bonus)
        _ => UpgradeCategory::Gate,
    }
}

/// Tick research queues on ScratchingPosts. On completion, apply upgrades.
pub fn research_system(
    mut buildings: Query<(&Owner, &mut ResearchQueue), (With<Researcher>, Without<Dead>)>,
    mut player_resources: ResMut<PlayerResources>,
    mut units: Query<
        (
            &Owner,
            &UnitType,
            &mut Health,
            &mut AttackStats,
            &mut MovementSpeed,
        ),
        Without<Dead>,
    >,
) {
    for (owner, mut queue) in buildings.iter_mut() {
        if let Some((upgrade, ticks_remaining)) = queue.queue.front_mut() {
            if *ticks_remaining > 0 {
                *ticks_remaining -= 1;
            }
            if *ticks_remaining == 0 {
                let completed_upgrade = *upgrade;
                queue.queue.pop_front();

                let player_id = owner.player_id as usize;
                if let Some(pres) = player_resources.players.get_mut(player_id) {
                    pres.completed_upgrades.insert(completed_upgrade);
                }

                // Apply upgrade bonuses to all existing units of this player
                apply_upgrade_to_existing_units(completed_upgrade, owner.player_id, &mut units);
            }
        }
    }
}

/// Apply an upgrade's stat bonuses to all existing units of a player.
pub fn apply_upgrade_to_existing_units(
    upgrade: UpgradeType,
    player_id: u8,
    units: &mut Query<
        (
            &Owner,
            &UnitType,
            &mut Health,
            &mut AttackStats,
            &mut MovementSpeed,
        ),
        Without<Dead>,
    >,
) {
    let category = upgrade_category(upgrade);

    for (owner, unit_type, mut health, mut attack_stats, mut speed) in units.iter_mut() {
        if owner.player_id != player_id {
            continue;
        }

        match category {
            UpgradeCategory::Damage => {
                // +20% damage for combat units (not workers)
                if !unit_type.kind.is_worker() {
                    let bonus = attack_stats.damage * Fixed::from_bits((1 << 16) * 20 / 100);
                    attack_stats.damage += bonus;
                }
            }
            UpgradeCategory::Health => {
                // +25% max HP for combat units (not workers), heal the bonus
                if !unit_type.kind.is_worker() {
                    let bonus = health.max * Fixed::from_bits((1 << 16) * 25 / 100);
                    health.max += bonus;
                    health.current += bonus;
                }
            }
            UpgradeCategory::Speed => {
                // +10% speed for all units (including workers)
                let bonus = speed.speed * Fixed::from_bits((1 << 16) * 10 / 100);
                speed.speed += bonus;
            }
            UpgradeCategory::Gate => {
                // No stat bonus — just unlocks training
            }
        }
    }
}

/// Apply all completed upgrades to a newly spawned unit.
pub fn apply_upgrades_to_new_unit(
    kind: UnitKind,
    completed: &std::collections::HashSet<UpgradeType>,
    health: &mut Health,
    attack_stats: &mut AttackStats,
    speed: &mut MovementSpeed,
) {
    let is_worker = kind.is_worker();

    for &upgrade in completed {
        match upgrade_category(upgrade) {
            UpgradeCategory::Damage => {
                if !is_worker {
                    let bonus = attack_stats.damage * Fixed::from_bits((1 << 16) * 20 / 100);
                    attack_stats.damage += bonus;
                }
            }
            UpgradeCategory::Health => {
                if !is_worker {
                    let bonus = health.max * Fixed::from_bits((1 << 16) * 25 / 100);
                    health.max += bonus;
                    health.current += bonus;
                }
            }
            UpgradeCategory::Speed => {
                let bonus = speed.speed * Fixed::from_bits((1 << 16) * 10 / 100);
                speed.speed += bonus;
            }
            UpgradeCategory::Gate => {}
        }
    }
}
