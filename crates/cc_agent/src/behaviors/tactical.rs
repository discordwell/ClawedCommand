//! Tactical behavior primitives: abilities, squad splitting, positioning.

use cc_core::commands::{AbilityTarget, EntityId};
use cc_core::components::{AttackType, UnitKind};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;

use crate::script_context::ScriptContext;
use super::BehaviorResult;

/// Smart ability activation: checks that the unit exists, then issues the ability command.
/// The actual cooldown/GPU cost validation happens in the sim's ability system.
pub fn use_ability(
    ctx: &mut ScriptContext,
    unit_id: EntityId,
    slot: u8,
    target: AbilityTarget,
) -> BehaviorResult {
    let unit = match ctx.state.unit_by_id(unit_id) {
        Some(u) => u,
        None => {
            return BehaviorResult {
                commands_issued: 0,
                description: "Unit not found for ability".into(),
            }
        }
    };

    if slot > 2 {
        return BehaviorResult {
            commands_issued: 0,
            description: format!("Invalid ability slot {slot} (must be 0-2)"),
        };
    }

    ctx.cmd_ability(unit_id, slot, target);
    BehaviorResult {
        commands_issued: 1,
        description: format!(
            "Activated ability slot {} on {:?} #{}",
            slot, unit.kind, unit_id.0
        ),
    }
}

/// Categorize units into melee, ranged, and support groups.
/// Returns a BehaviorResult with description listing group sizes.
/// Does not issue commands — purely informational for planning.
pub fn split_squads(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
) -> (Vec<EntityId>, Vec<EntityId>, Vec<EntityId>, BehaviorResult) {
    let mut melee = Vec::new();
    let mut ranged = Vec::new();
    let mut support = Vec::new();

    for &uid in unit_ids {
        if let Some(unit) = ctx.state.unit_by_id(uid) {
            match unit.kind {
                UnitKind::Yowler => support.push(uid),
                _ if unit.attack_type == AttackType::Ranged => ranged.push(uid),
                _ => melee.push(uid),
            }
        }
    }

    let result = BehaviorResult {
        commands_issued: 0,
        description: format!(
            "Split {} units: {} melee, {} ranged, {} support",
            unit_ids.len(),
            melee.len(),
            ranged.len(),
            support.len()
        ),
    };

    (melee, ranged, support, result)
}

/// Escort units stay near a VIP and engage threats within guard radius.
/// Escorts without threats move to within 2 tiles of the VIP.
pub fn protect_unit(
    ctx: &mut ScriptContext,
    escort_ids: &[EntityId],
    vip_id: EntityId,
    guard_radius: Fixed,
) -> BehaviorResult {
    let vip = match ctx.state.unit_by_id(vip_id) {
        Some(u) => u.clone(),
        None => {
            return BehaviorResult {
                commands_issued: 0,
                description: "VIP not found".into(),
            }
        }
    };

    let mut commands_issued = 0;

    // Find threats near the VIP
    let threats = ctx.enemies_in_range(vip.pos, guard_radius);
    let threat_id = threats.first().map(|t| t.id);

    for &eid in escort_ids {
        let escort = match ctx.state.unit_by_id(eid) {
            Some(u) => u.clone(),
            None => continue,
        };

        if let Some(tid) = threat_id {
            // Engage threat
            ctx.cmd_attack(vec![eid], tid);
            commands_issued += 1;
        } else {
            // No threats — stay near VIP
            let dx = (escort.pos.x - vip.pos.x).abs();
            let dy = (escort.pos.y - vip.pos.y).abs();
            if dx > 2 || dy > 2 {
                ctx.cmd_move(vec![eid], vip.pos);
                commands_issued += 1;
            }
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Protecting unit {} with {} escorts",
            vip_id.0,
            escort_ids.len()
        ),
    }
}

/// Position units in a ring around an enemy target, then attack.
/// Places each unit at evenly spaced positions around the target.
pub fn surround_target(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    target_id: EntityId,
    ring_radius: Fixed,
) -> BehaviorResult {
    let target = match ctx.state.unit_by_id(target_id) {
        Some(u) => u.clone(),
        None => {
            return BehaviorResult {
                commands_issued: 0,
                description: "Target not found for surround".into(),
            }
        }
    };

    let count = unit_ids.len();
    if count == 0 {
        return BehaviorResult {
            commands_issued: 0,
            description: "No units to surround with".into(),
        };
    }

    let radius_i32: i32 = ring_radius.to_num::<i32>().max(1);
    let mut commands_issued = 0;

    for (i, &uid) in unit_ids.iter().enumerate() {
        // Distribute positions evenly in a ring using integer approximation
        let angle_frac = (i as f64) / (count as f64);
        let angle = angle_frac * std::f64::consts::TAU;
        let dx = (angle.cos() * radius_i32 as f64).round() as i32;
        let dy = (angle.sin() * radius_i32 as f64).round() as i32;

        let surround_pos = GridPos::new(target.pos.x + dx, target.pos.y + dy);

        // Move to surround position
        ctx.cmd_move(vec![uid], surround_pos);
        commands_issued += 1;

        // Then attack the target
        ctx.cmd_attack(vec![uid], target_id);
        commands_issued += 1;
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Surrounding target {} with {} units at radius {}",
            target_id.0, count, radius_i32
        ),
    }
}

/// Group attack-move with ranged units positioned behind melee.
/// Melee units attack-move directly; ranged units move to a position offset from the target.
pub fn attack_move_group(
    ctx: &mut ScriptContext,
    unit_ids: &[EntityId],
    target: GridPos,
) -> BehaviorResult {
    let mut melee_ids = Vec::new();
    let mut ranged_ids = Vec::new();

    for &uid in unit_ids {
        if let Some(unit) = ctx.state.unit_by_id(uid) {
            if unit.attack_type == AttackType::Ranged {
                ranged_ids.push((uid, unit.pos));
            } else {
                melee_ids.push(uid);
            }
        }
    }

    let mut commands_issued = 0;

    // Melee units attack-move directly
    if !melee_ids.is_empty() {
        ctx.cmd_attack_move(melee_ids.clone(), target);
        commands_issued += 1;
    }

    // Ranged units move to a position slightly behind the melee
    if !ranged_ids.is_empty() {
        // Calculate centroid of ranged units
        let (sum_x, sum_y) = ranged_ids
            .iter()
            .fold((0i64, 0i64), |(sx, sy), (_, pos)| {
                (sx + pos.x as i64, sy + pos.y as i64)
            });
        let centroid_x = (sum_x / ranged_ids.len() as i64) as i32;
        let centroid_y = (sum_y / ranged_ids.len() as i64) as i32;

        // Offset ranged toward target but stop 3 tiles short
        let dx = target.x - centroid_x;
        let dy = target.y - centroid_y;
        let dist = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt().max(1.0);
        let offset = 3.0 / dist;
        let ranged_target = GridPos::new(
            target.x - (dx as f64 * offset).round() as i32,
            target.y - (dy as f64 * offset).round() as i32,
        );

        let ranged_unit_ids: Vec<EntityId> = ranged_ids.iter().map(|(id, _)| *id).collect();
        ctx.cmd_attack_move(ranged_unit_ids, ranged_target);
        commands_issued += 1;
    }

    BehaviorResult {
        commands_issued,
        description: format!(
            "Attack-moving {} melee + {} ranged to ({},{})",
            melee_ids.len(),
            ranged_ids.len(),
            target.x,
            target.y
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

    use crate::test_fixtures::{make_unit, make_snapshot};

    #[test]
    fn use_ability_issues_command() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Yowler, 5, 5, 0)],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = use_ability(&mut ctx, EntityId(1), 0, AbilityTarget::SelfCast);
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert!(matches!(cmds[0], GameCommand::ActivateAbility { .. }));
    }

    #[test]
    fn use_ability_rejects_invalid_slot() {
        let snap = make_snapshot(
            vec![make_unit(1, UnitKind::Yowler, 5, 5, 0)],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = use_ability(&mut ctx, EntityId(1), 5, AbilityTarget::SelfCast);
        assert_eq!(result.commands_issued, 0);
    }

    #[test]
    fn split_squads_categorizes_correctly() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 5, 5, 0),   // melee
                make_unit(2, UnitKind::Hisser, 6, 5, 0),   // ranged
                make_unit(3, UnitKind::Yowler, 7, 5, 0),   // support
                make_unit(4, UnitKind::Nuisance, 8, 5, 0),  // melee
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let ids = [EntityId(1), EntityId(2), EntityId(3), EntityId(4)];
        let (melee, ranged, support, result) = split_squads(&mut ctx, &ids);
        assert_eq!(melee.len(), 2); // Chonk + Nuisance
        assert_eq!(ranged.len(), 1); // Hisser
        assert_eq!(support.len(), 1); // Yowler
        assert_eq!(result.commands_issued, 0); // informational only
    }

    #[test]
    fn protect_unit_engages_threats() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 10, 10, 0), // escort
                make_unit(2, UnitKind::Yowler, 10, 11, 0), // VIP
            ],
            vec![make_unit(20, UnitKind::Nuisance, 12, 11, 1)], // threat
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = protect_unit(
            &mut ctx,
            &[EntityId(1)],
            EntityId(2),
            fixed_from_i32(5),
        );
        assert_eq!(result.commands_issued, 1);
        let cmds = ctx.take_commands();
        assert!(matches!(cmds[0], GameCommand::Attack { .. }));
    }

    #[test]
    fn protect_unit_moves_to_vip_when_no_threats() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 20, 20, 0), // escort far away
                make_unit(2, UnitKind::Yowler, 10, 10, 0), // VIP
            ],
            vec![], // no enemies
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = protect_unit(
            &mut ctx,
            &[EntityId(1)],
            EntityId(2),
            fixed_from_i32(5),
        );
        assert_eq!(result.commands_issued, 1);
        let cmds = ctx.take_commands();
        assert!(matches!(cmds[0], GameCommand::Move { .. }));
    }

    #[test]
    fn surround_target_positions_ring() {
        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Nuisance, 5, 5, 0),
                make_unit(2, UnitKind::Nuisance, 6, 5, 0),
                make_unit(3, UnitKind::Nuisance, 7, 5, 0),
                make_unit(4, UnitKind::Nuisance, 8, 5, 0),
            ],
            vec![make_unit(20, UnitKind::Chonk, 15, 15, 1)],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = surround_target(
            &mut ctx,
            &[EntityId(1), EntityId(2), EntityId(3), EntityId(4)],
            EntityId(20),
            fixed_from_i32(3),
        );
        // 4 units × 2 commands each (move + attack)
        assert_eq!(result.commands_issued, 8);
    }

    #[test]
    fn attack_move_group_splits_melee_ranged() {
        let mut hisser = make_unit(2, UnitKind::Hisser, 6, 5, 0);
        hisser.attack_type = AttackType::Ranged;

        let snap = make_snapshot(
            vec![
                make_unit(1, UnitKind::Chonk, 5, 5, 0), // melee
                hisser,                                    // ranged
            ],
            vec![],
        );
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = attack_move_group(
            &mut ctx,
            &[EntityId(1), EntityId(2)],
            GridPos::new(20, 20),
        );
        assert_eq!(result.commands_issued, 2); // one for melee, one for ranged
        let cmds = ctx.take_commands();
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], GameCommand::AttackMove { .. }));
        assert!(matches!(cmds[1], GameCommand::AttackMove { .. }));
    }
}
