//! Composable behavior primitives for AI agents.
//! Each function takes a ScriptContext and produces GameCommands.

pub mod economy;
pub mod strategic;
pub mod tactical;

pub use economy::*;
pub use strategic::*;
pub use tactical::*;

use cc_core::commands::EntityId;
use cc_core::components::UnitKind;
use cc_core::coords::GridPos;
use cc_core::math::Fixed;

use crate::script_context::ScriptContext;

/// Result returned by all behavior primitives.
#[derive(Debug)]
pub struct BehaviorResult {
    pub commands_issued: usize,
    pub description: String,
}

/// All attackers issue Attack on the same target.
pub fn focus_fire(
    ctx: &mut ScriptContext,
    attacker_ids: &[EntityId],
    target_id: EntityId,
) -> BehaviorResult {
    let count = attacker_ids.len();
    if count > 0 {
        ctx.cmd_attack(attacker_ids.to_vec(), target_id);
    }
    BehaviorResult {
        commands_issued: if count > 0 { 1 } else { 0 },
        description: format!("{count} units focus-firing target {}", target_id.0),
    }
}

/// For each ranged unit: find nearest enemy, move to maintain attack range, then attack.
pub fn kite_squad(ctx: &mut ScriptContext, unit_ids: &[EntityId]) -> BehaviorResult {
    let mut commands_issued = 0;

    for &uid in unit_ids {
        let unit = match ctx.state.unit_by_id(uid) {
            Some(u) => u.clone(),
            None => continue,
        };

        // Find nearest enemy
        let enemy = match ctx.nearest_enemy(unit.pos) {
            Some(e) => e.clone(),
            None => continue,
        };

        let desired_range: i32 = unit.attack_range.to_num::<i32>().max(1);

        // Find kite position
        if let Some(kite_pos) = ctx.position_at_range(unit.pos, enemy.pos, desired_range) {
            ctx.cmd_move(vec![uid], kite_pos);
            commands_issued += 1;
        }

        ctx.cmd_attack(vec![uid], enemy.id);
        commands_issued += 1;
    }

    BehaviorResult {
        commands_issued,
        description: format!("Kiting {} units", unit_ids.len()),
    }
}

/// Find own units below HP%, move them to safe positions.
pub fn retreat_wounded(ctx: &mut ScriptContext, threshold_pct: f64) -> BehaviorResult {
    let wounded: Vec<_> = ctx
        .wounded_units(threshold_pct)
        .into_iter()
        .map(|u| (u.id, u.clone()))
        .collect();

    let mut commands_issued = 0;

    for (uid, unit) in &wounded {
        let safe = ctx.safe_positions(unit, 8);
        if let Some(pos) = safe.first() {
            ctx.cmd_move(vec![*uid], *pos);
            commands_issued += 1;
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!("Retreating {} wounded units", wounded.len()),
    }
}

/// Attack enemies inside radius, hold position if clear, move back if out of area.
pub fn defend_area(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    center: GridPos,
    radius: Fixed,
) -> BehaviorResult {
    let mut commands_issued = 0;

    // Find enemies inside the defense radius
    let enemies_inside: Vec<_> = ctx
        .enemies_in_range(center, radius)
        .iter()
        .map(|e| e.id)
        .collect();

    for &uid in unit_ids {
        let unit = match ctx.state.unit_by_id(uid) {
            Some(u) => u.clone(),
            None => continue,
        };

        if let Some(&target) = enemies_inside.first() {
            // Attack intruders
            ctx.cmd_attack(vec![uid], target);
            commands_issued += 1;
        } else {
            // Check if unit is within defense area
            let dx = (unit.pos.x - center.x).abs();
            let dy = (unit.pos.y - center.y).abs();
            let radius_i32: i32 = radius.to_num::<i32>().max(1);

            if dx > radius_i32 || dy > radius_i32 {
                // Move back to center
                ctx.cmd_move(vec![uid], center);
                commands_issued += 1;
            } else {
                // Hold position
                ctx.cmd_hold(vec![uid]);
                commands_issued += 1;
            }
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Defending area at ({},{}) with {} units",
            center.x,
            center.y,
            unit_ids.len()
        ),
    }
}

/// Find enemy Pawdlers and attack them; if no workers visible, attack-move toward enemy buildings.
pub fn harass_economy(ctx: &mut ScriptContext, raider_ids: &[EntityId]) -> BehaviorResult {
    let mut commands_issued = 0;

    // Find enemy workers (Pawdlers)
    let enemy_workers: Vec<EntityId> = ctx
        .enemy_units()
        .iter()
        .filter(|u| u.kind == UnitKind::Pawdler)
        .map(|u| u.id)
        .collect();

    if let Some(&worker_id) = enemy_workers.first() {
        // Attack workers
        if !raider_ids.is_empty() {
            ctx.cmd_attack(raider_ids.to_vec(), worker_id);
            commands_issued += 1;
        }
    } else {
        // No workers visible — attack-move toward enemy buildings
        let enemy_buildings: Vec<GridPos> = ctx.enemy_buildings().iter().map(|b| b.pos).collect();

        if let Some(&building_pos) = enemy_buildings.first() {
            if !raider_ids.is_empty() {
                ctx.cmd_attack_move(raider_ids.to_vec(), building_pos);
                commands_issued += 1;
            }
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!("Harassing with {} raiders", raider_ids.len()),
    }
}

/// Move scout to nearest unvisited waypoint.
pub fn scout_pattern(
    ctx: &mut ScriptContext,
    scout_id: EntityId,
    waypoints: &[GridPos],
) -> BehaviorResult {
    let scout = match ctx.state.unit_by_id(scout_id) {
        Some(u) => u.clone(),
        None => {
            return BehaviorResult {
                commands_issued: 0,
                description: "Scout not found".into(),
            };
        }
    };

    // Find the closest waypoint that isn't the scout's current position
    let mut best: Option<(GridPos, i64)> = None;
    for &wp in waypoints {
        if wp == scout.pos {
            continue;
        }
        let dx = (wp.x - scout.pos.x) as i64;
        let dy = (wp.y - scout.pos.y) as i64;
        let dist_sq = dx * dx + dy * dy;
        match best {
            Some((_, bd)) if dist_sq >= bd => {}
            _ => best = Some((wp, dist_sq)),
        }
    }

    match best {
        Some((target, _)) => {
            ctx.cmd_move(vec![scout_id], target);
            BehaviorResult {
                commands_issued: 1,
                description: format!("Scouting to ({},{})", target.x, target.y),
            }
        }
        None => BehaviorResult {
            commands_issued: 0,
            description: "No waypoints to scout".into(),
        },
    }
}

/// Check resources vs unit costs, train if affordable.
pub fn auto_produce(
    ctx: &mut ScriptContext,
    building_id: EntityId,
    unit_kind: UnitKind,
) -> BehaviorResult {
    let stats = cc_core::unit_stats::base_stats(unit_kind);
    let res = ctx.resources().clone();

    if res.food >= stats.food_cost
        && res.gpu_cores >= stats.gpu_cost
        && res.supply + stats.supply_cost <= res.supply_cap
    {
        ctx.cmd_train(building_id, unit_kind);
        BehaviorResult {
            commands_issued: 1,
            description: format!("Auto-producing {unit_kind:?} at building {}", building_id.0),
        }
    } else {
        BehaviorResult {
            commands_issued: 0,
            description: format!("Cannot afford {unit_kind:?}"),
        }
    }
}

/// Find weakest enemy near any unit in group, then focus_fire all on it.
pub fn focus_weakest(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    range: Fixed,
) -> BehaviorResult {
    // Find the weakest enemy within range of any unit in the group
    let mut weakest: Option<(EntityId, Fixed)> = None;

    for &uid in unit_ids {
        if let Some(unit) = ctx.state.unit_by_id(uid) {
            let pos = unit.pos;
            if let Some(enemy) = ctx.weakest_enemy_in_range(pos, range) {
                match weakest {
                    Some((_, hp)) if enemy.health_current >= hp => {}
                    _ => weakest = Some((enemy.id, enemy.health_current)),
                }
            }
        }
    }

    match weakest {
        Some((target_id, _)) => focus_fire(ctx, unit_ids, target_id),
        None => BehaviorResult {
            commands_issued: 0,
            description: "No weak enemies in range".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::map::GameMap;
    use cc_core::math::fixed_from_i32;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    use crate::snapshot::GameStateSnapshot;
    use crate::test_fixtures::{make_snapshot, make_unit};

    #[test]
    fn focus_fire_generates_attack_for_all() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Hisser, 6, 5, 0),
            ],
            vec![make_unit(10, UnitKind::Chonk, 8, 5, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = focus_fire(&mut ctx, &[EntityId(1), EntityId(2)], EntityId(10));
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            cc_core::commands::GameCommand::Attack { unit_ids, target } => {
                assert_eq!(unit_ids.len(), 2);
                assert_eq!(*target, EntityId(10));
            }
            _ => panic!("Expected Attack command"),
        }
    }

    #[test]
    fn kite_squad_moves_to_range() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![make_unit(10, UnitKind::Chonk, 7, 5, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = kite_squad(&mut ctx, &[EntityId(1)]);
        assert!(result.commands_issued >= 1);

        let cmds = ctx.take_commands();
        assert!(cmds.len() >= 1);
    }

    #[test]
    fn retreat_wounded_moves_low_hp() {
        let mut wounded = make_unit(1, UnitKind::Hisser, 5, 5, 0);
        wounded.health_current = fixed_from_i32(20);

        let mut enemy = make_unit(10, UnitKind::Chonk, 30, 30, 1);
        enemy.attack_range = fixed_from_i32(3);

        let snap = make_snapshot(vec![wounded], vec![enemy]);
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = retreat_wounded(&mut ctx, 0.5);
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            cmds[0],
            cc_core::commands::GameCommand::Move { .. }
        ));
    }

    #[test]
    fn defend_area_attacks_intruders() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 10, 10, 0)],
            vec![make_unit(10, UnitKind::Chonk, 11, 10, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = defend_area(
            &mut ctx,
            &[EntityId(1)],
            GridPos::new(10, 10),
            fixed_from_i32(5),
        );
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert!(matches!(
            cmds[0],
            cc_core::commands::GameCommand::Attack { .. }
        ));
    }

    #[test]
    fn defend_area_holds_when_clear() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 10, 10, 0)],
            vec![make_unit(10, UnitKind::Chonk, 50, 50, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = defend_area(
            &mut ctx,
            &[EntityId(1)],
            GridPos::new(10, 10),
            fixed_from_i32(5),
        );
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert!(matches!(
            cmds[0],
            cc_core::commands::GameCommand::HoldPosition { .. }
        ));
    }

    #[test]
    fn harass_economy_targets_workers() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Nuisance, 5, 5, 0)],
            vec![
                make_unit(10, UnitKind::Pawdler, 8, 8, 1),
                make_unit(11, UnitKind::Chonk, 20, 20, 1),
            ],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = harass_economy(&mut ctx, &[EntityId(1)]);
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        match &cmds[0] {
            cc_core::commands::GameCommand::Attack { target, .. } => {
                assert_eq!(*target, EntityId(10));
            }
            _ => panic!("Expected Attack command targeting worker"),
        }
    }

    #[test]
    fn auto_produce_checks_resources() {
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

        let result = auto_produce(&mut ctx, EntityId(50), UnitKind::Pawdler);
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert!(matches!(
            cmds[0],
            cc_core::commands::GameCommand::TrainUnit { .. }
        ));
    }

    #[test]
    fn focus_weakest_targets_lowest_hp() {
        let mut weak = make_unit(10, UnitKind::Hisser, 7, 5, 1);
        weak.health_current = fixed_from_i32(10);
        let strong = make_unit(11, UnitKind::Chonk, 8, 5, 1);

        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Hisser, 5, 5, 0)],
            vec![weak, strong],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = focus_weakest(&mut ctx, &[EntityId(1)], fixed_from_i32(10));
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        match &cmds[0] {
            cc_core::commands::GameCommand::Attack { target, .. } => {
                assert_eq!(*target, EntityId(10));
            }
            _ => panic!("Expected Attack targeting weakest"),
        }
    }
}
