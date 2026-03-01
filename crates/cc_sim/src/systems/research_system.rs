use bevy::prelude::*;

use cc_core::components::{
    AttackStats, Dead, Health, MovementSpeed, Owner, ResearchQueue, Researcher, UnitKind, UnitType,
    UpgradeType,
};
use cc_core::math::Fixed;

use crate::resources::PlayerResources;

/// Tick research queues on ScratchingPosts. On completion, apply upgrades.
pub fn research_system(
    mut buildings: Query<
        (&Owner, &mut ResearchQueue),
        (With<Researcher>, Without<Dead>),
    >,
    mut player_resources: ResMut<PlayerResources>,
    mut units: Query<
        (&Owner, &UnitType, &mut Health, &mut AttackStats, &mut MovementSpeed),
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
                apply_upgrade_to_existing_units(
                    completed_upgrade,
                    owner.player_id,
                    &mut units,
                );
            }
        }
    }
}

/// Apply an upgrade's stat bonuses to all existing units of a player.
pub fn apply_upgrade_to_existing_units(
    upgrade: UpgradeType,
    player_id: u8,
    units: &mut Query<
        (&Owner, &UnitType, &mut Health, &mut AttackStats, &mut MovementSpeed),
        Without<Dead>,
    >,
) {
    for (owner, unit_type, mut health, mut attack_stats, mut speed) in units.iter_mut() {
        if owner.player_id != player_id {
            continue;
        }

        match upgrade {
            UpgradeType::SharperClaws => {
                // +2 damage for combat units (not Pawdler)
                if unit_type.kind != UnitKind::Pawdler {
                    attack_stats.damage += Fixed::from_bits(2 << 16);
                }
            }
            UpgradeType::ThickerFur => {
                // +25 HP for combat units (not Pawdler)
                if unit_type.kind != UnitKind::Pawdler {
                    let bonus = Fixed::from_bits(25 << 16);
                    health.max += bonus;
                    health.current += bonus;
                }
            }
            UpgradeType::NimblePaws => {
                // +10% speed for all units
                speed.speed = speed.speed
                    + speed.speed * Fixed::from_bits((1 << 16) * 10 / 100);
            }
            UpgradeType::SiegeTraining | UpgradeType::MechPrototype => {
                // These are unlock gates, not stat bonuses
            }
            // Non-cat faction upgrades — no effect in cat game loop yet
            _ => {}
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
    let is_combat = kind != UnitKind::Pawdler;

    if is_combat && completed.contains(&UpgradeType::SharperClaws) {
        attack_stats.damage += Fixed::from_bits(2 << 16);
    }
    if is_combat && completed.contains(&UpgradeType::ThickerFur) {
        let bonus = Fixed::from_bits(25 << 16);
        health.max += bonus;
        health.current += bonus;
    }
    if completed.contains(&UpgradeType::NimblePaws) {
        speed.speed = speed.speed + speed.speed * Fixed::from_bits((1 << 16) * 10 / 100);
    }
}
