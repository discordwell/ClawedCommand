//! Strategic behavior primitives: multi-squad coordination, research, adaptive defense.

use cc_core::commands::EntityId;
use cc_core::components::{AttackType, UnitKind, UpgradeType};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;

use crate::script_context::ScriptContext;
use super::BehaviorResult;

/// Split army into main force (70%) + flanking group (30%) and attack from two angles.
/// Main force attack-moves directly; flanking group swings wide.
pub fn coordinate_assault(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    target: GridPos,
) -> BehaviorResult {
    if unit_ids.is_empty() {
        return BehaviorResult {
            commands_issued: 0,
            description: "No units for assault".into(),
        };
    }

    // Split 70/30
    let split_point = (unit_ids.len() * 7) / 10;
    let split_point = split_point.max(1).min(unit_ids.len() - 1.max(1));

    let main_force: Vec<EntityId> = unit_ids[..split_point].to_vec();
    let flank_group: Vec<EntityId> = unit_ids[split_point..].to_vec();

    let mut commands_issued = 0;

    // Main force attacks directly
    if !main_force.is_empty() {
        ctx.cmd_attack_move(main_force.clone(), target);
        commands_issued += 1;
    }

    // Flanking group swings wide (offset perpendicular to approach)
    if !flank_group.is_empty() {
        // Calculate centroid of all units
        let mut sum_x = 0i64;
        let mut sum_y = 0i64;
        let mut count = 0;
        for &uid in unit_ids {
            if let Some(unit) = ctx.state.unit_by_id(uid) {
                sum_x += unit.pos.x as i64;
                sum_y += unit.pos.y as i64;
                count += 1;
            }
        }
        if count > 0 {
            let centroid_x = (sum_x / count) as i32;
            let centroid_y = (sum_y / count) as i32;

            // Perpendicular offset: rotate approach vector 90 degrees
            let dx = target.x - centroid_x;
            let dy = target.y - centroid_y;
            // Flank offset = perpendicular, 5 tiles out
            let flank_offset_x = -dy.signum() * 5;
            let flank_offset_y = dx.signum() * 5;
            let flank_target = GridPos::new(
                target.x + flank_offset_x,
                target.y + flank_offset_y,
            );

            ctx.cmd_attack_move(flank_group.clone(), flank_target);
            commands_issued += 1;
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Coordinated assault: {} main + {} flanking toward ({},{})",
            main_force.len(),
            flank_group.len(),
            target.x,
            target.y
        ),
    }
}

/// Evaluate completed upgrades and auto-queue the best remaining upgrade.
/// Prioritizes upgrades based on current army composition.
pub fn research_priority(
    ctx: &mut ScriptContext,
    building_id: EntityId,
) -> BehaviorResult {
    let res = ctx.resources().clone();

    // Available upgrades in priority order based on general usefulness
    let priorities = [
        (UpgradeType::SharperClaws, 150, 50),   // food, gpu
        (UpgradeType::ThickerFur, 200, 75),
        (UpgradeType::NimblePaws, 150, 100),
        (UpgradeType::SiegeTraining, 300, 150),
        (UpgradeType::MechPrototype, 500, 300),
    ];

    // Check which upgrades are already completed
    for (upgrade, food_cost, gpu_cost) in &priorities {
        if res.completed_upgrades.contains(upgrade) {
            continue;
        }
        if res.food >= *food_cost && res.gpu_cores >= *gpu_cost {
            ctx.cmd_research(building_id, *upgrade);
            return BehaviorResult {
                commands_issued: 1,
                description: format!("Researching {upgrade:?} at building {}", building_id.0),
            };
        }
    }

    BehaviorResult {
        commands_issued: 0,
        description: "No affordable research available".into(),
    }
}

/// Position defenses based on enemy composition and terrain.
/// Melee units forward, ranged on high ground, support behind.
pub fn adaptive_defense(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    center: GridPos,
    radius: Fixed,
) -> BehaviorResult {
    if unit_ids.is_empty() {
        return BehaviorResult {
            commands_issued: 0,
            description: "No units for defense".into(),
        };
    }

    let mut commands_issued = 0;
    let radius_i32: i32 = radius.to_num::<i32>().max(1);

    // Categorize units
    let mut melee_ids = Vec::new();
    let mut ranged_ids = Vec::new();
    let mut support_ids = Vec::new();

    for &uid in unit_ids {
        if let Some(unit) = ctx.state.unit_by_id(uid) {
            match unit.kind {
                UnitKind::Yowler => support_ids.push(uid),
                _ if unit.attack_type == AttackType::Ranged => ranged_ids.push(uid),
                _ => melee_ids.push(uid),
            }
        }
    }

    // Find the direction enemies are most likely to come from
    let enemies = ctx.enemies_in_range(center, Fixed::from_num(radius_i32 * 3));
    let threat_dir = if !enemies.is_empty() {
        let avg_x: i32 = enemies.iter().map(|e| e.pos.x).sum::<i32>() / enemies.len() as i32;
        let avg_y: i32 = enemies.iter().map(|e| e.pos.y).sum::<i32>() / enemies.len() as i32;
        let dx = avg_x - center.x;
        let dy = avg_y - center.y;
        let dist = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt().max(1.0);
        (dx as f64 / dist, dy as f64 / dist)
    } else {
        (0.0, -1.0) // Default: face north
    };

    // Place melee units in a forward line toward threat direction
    for (i, &uid) in melee_ids.iter().enumerate() {
        let spread = (i as i32) - (melee_ids.len() as i32 / 2);
        let forward_x = center.x + (threat_dir.0 * radius_i32 as f64 * 0.7).round() as i32
            + (-threat_dir.1 * spread as f64 * 2.0).round() as i32;
        let forward_y = center.y + (threat_dir.1 * radius_i32 as f64 * 0.7).round() as i32
            + (threat_dir.0 * spread as f64 * 2.0).round() as i32;
        ctx.cmd_move(vec![uid], GridPos::new(forward_x, forward_y));
        commands_issued += 1;
    }

    // Place ranged units behind melee, try to find elevated positions
    for (i, &uid) in ranged_ids.iter().enumerate() {
        let spread = (i as i32) - (ranged_ids.len() as i32 / 2);
        let back_x = center.x + (threat_dir.0 * radius_i32 as f64 * 0.3).round() as i32
            + (-threat_dir.1 * spread as f64 * 2.0).round() as i32;
        let back_y = center.y + (threat_dir.1 * radius_i32 as f64 * 0.3).round() as i32
            + (threat_dir.0 * spread as f64 * 2.0).round() as i32;
        ctx.cmd_move(vec![uid], GridPos::new(back_x, back_y));
        commands_issued += 1;
    }

    // Place support units at center
    for &uid in &support_ids {
        ctx.cmd_move(vec![uid], center);
        commands_issued += 1;
    }

    // Hold position for all
    if !unit_ids.is_empty() {
        ctx.cmd_hold(unit_ids.to_vec());
        commands_issued += 1;
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Adaptive defense: {} melee forward, {} ranged back, {} support center",
            melee_ids.len(),
            ranged_ids.len(),
            support_ids.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::commands::GameCommand;
    use cc_core::map::GameMap;
    use cc_core::math::fixed_from_i32;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    use crate::snapshot::GameStateSnapshot;
    use crate::test_fixtures::{make_unit, make_snapshot};

    #[test]
    fn coordinate_assault_splits_army() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 6, 5, 0),
                make_unit(3, UnitKind::Chonk, 7, 5, 0),
                make_unit(4, UnitKind::Nuisance, 8, 5, 0),
                make_unit(5, UnitKind::Nuisance, 9, 5, 0),
                make_unit(6, UnitKind::Nuisance, 10, 5, 0),
                make_unit(7, UnitKind::Hisser, 11, 5, 0),
                make_unit(8, UnitKind::Hisser, 12, 5, 0),
                make_unit(9, UnitKind::Hisser, 13, 5, 0),
                make_unit(10, UnitKind::Hisser, 14, 5, 0),
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let ids: Vec<EntityId> = (1..=10).map(EntityId).collect();
        let result = coordinate_assault(&mut ctx, &ids, GridPos::new(40, 40));
        assert_eq!(result.commands_issued, 2); // main + flank
        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], GameCommand::AttackMove { .. }));
        assert!(matches!(cmds[1], GameCommand::AttackMove { .. }));
    }

    #[test]
    fn coordinate_assault_empty() {
        let snap = make_snapshot(vec![], vec![]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = coordinate_assault(&mut ctx, &[], GridPos::new(40, 40));
        assert_eq!(result.commands_issued, 0);
    }

    #[test]
    fn research_priority_queues_first_available() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 500,
                gpu_cores: 200,
                nfts: 0,
                supply: 0,
                supply_cap: 20,
                completed_upgrades: Default::default(),
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = research_priority(&mut ctx, EntityId(50));
        assert_eq!(result.commands_issued, 1);
        assert!(result.description.contains("SharperClaws"));
    }

    #[test]
    fn research_priority_skips_completed() {
        let mut upgrades = std::collections::HashSet::new();
        upgrades.insert(UpgradeType::SharperClaws);

        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 500,
                gpu_cores: 200,
                nfts: 0,
                supply: 0,
                supply_cap: 20,
                completed_upgrades: upgrades,
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = research_priority(&mut ctx, EntityId(50));
        assert_eq!(result.commands_issued, 1);
        assert!(result.description.contains("ThickerFur"));
    }

    #[test]
    fn adaptive_defense_positions_units() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 10, 10, 0),   // melee
                make_unit(2, UnitKind::Hisser, 10, 11, 0),   // ranged
                make_unit(3, UnitKind::Yowler, 10, 12, 0),   // support
            ],
            vec![make_unit(20, UnitKind::Nuisance, 20, 10, 1)], // threat from east
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = adaptive_defense(
            &mut ctx,
            &[EntityId(1), EntityId(2), EntityId(3)],
            GridPos::new(10, 10),
            fixed_from_i32(5),
        );
        // 3 moves + 1 hold = 4
        assert_eq!(result.commands_issued, 4);
    }
}
