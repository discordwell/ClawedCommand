//! Economy behavior primitives: worker management, production, expansion.

use cc_core::commands::EntityId;
use cc_core::components::{BuildingKind, UnitKind};
use cc_core::coords::GridPos;

use crate::script_context::ScriptContext;
use super::BehaviorResult;

/// Send idle Pawdlers to the nearest resource deposit.
/// Scans all own idle Pawdlers and assigns each to the closest deposit.
pub fn assign_idle_workers(ctx: &mut ScriptContext) -> BehaviorResult {
    let idle_pawdlers: Vec<(EntityId, GridPos)> = ctx
        .idle_units(Some(UnitKind::Pawdler))
        .into_iter()
        .map(|u| (u.id, u.pos))
        .collect();

    let mut commands_issued = 0;

    for (uid, pos) in &idle_pawdlers {
        if let Some(deposit_id) = ctx.nearest_deposit(*pos, None).map(|d| d.id) {
            ctx.cmd_gather(vec![*uid], deposit_id);
            commands_issued += 1;
        }
    }

    BehaviorResult {
        commands_issued,
        description: format!("Assigned {} idle workers to deposits", commands_issued),
    }
}

/// Analyze army composition and auto-queue a missing unit type.
/// Tries to maintain a balanced army by checking current unit counts
/// and training the type with the fewest representatives.
pub fn balanced_production(
    ctx: &mut ScriptContext,
    building_id: EntityId,
) -> BehaviorResult {
    let res = ctx.resources().clone();

    // Count current combat units (not Pawdlers)
    let combat_kinds = [
        UnitKind::Nuisance,
        UnitKind::Chonk,
        UnitKind::Hisser,
    ];

    let mut counts: Vec<(UnitKind, usize)> = combat_kinds
        .iter()
        .map(|&kind| {
            let count = ctx.my_units(Some(kind)).len();
            (kind, count)
        })
        .collect();

    // Sort by count ascending — train the least-represented type
    counts.sort_by_key(|(_, count)| *count);

    for (kind, _) in &counts {
        let stats = cc_core::unit_stats::base_stats(*kind);
        if res.food >= stats.food_cost
            && res.gpu_cores >= stats.gpu_cost
            && res.supply + stats.supply_cost <= res.supply_cap
        {
            ctx.cmd_train(building_id, *kind);
            return BehaviorResult {
                commands_issued: 1,
                description: format!("Balanced production: training {kind:?}"),
            };
        }
    }

    BehaviorResult {
        commands_issued: 0,
        description: "Cannot afford any balanced production".into(),
    }
}

/// Build economic infrastructure: FishMarkets near deposits, LitterBoxes for supply.
/// Checks if the builder is idle, finds the nearest unserved deposit, and builds there.
pub fn expand_economy(
    ctx: &mut ScriptContext,
    builder_id: EntityId,
) -> BehaviorResult {
    let builder = match ctx.state.unit_by_id(builder_id) {
        Some(u) => u.clone(),
        None => {
            return BehaviorResult {
                commands_issued: 0,
                description: "Builder not found".into(),
            }
        }
    };

    let res = ctx.resources().clone();

    // Check if we need more supply (approaching cap)
    let supply_headroom = res.supply_cap.saturating_sub(res.supply);
    if supply_headroom <= 3 {
        let litter_stats = cc_core::building_stats::building_stats(BuildingKind::LitterBox);
        if res.food >= litter_stats.food_cost && res.gpu_cores >= litter_stats.gpu_cost {
            // Build LitterBox near the builder
            let pos = GridPos::new(builder.pos.x + 2, builder.pos.y);
            ctx.cmd_build(builder_id, BuildingKind::LitterBox, pos);
            return BehaviorResult {
                commands_issued: 1,
                description: "Expanding economy: building LitterBox for supply".into(),
            };
        }
    }

    // Otherwise, try to build a FishMarket near an unserved food deposit
    if let Some(deposit_pos) = ctx.nearest_deposit(builder.pos, None).map(|d| d.pos) {
        // Check if we already have a FishMarket near this deposit
        let existing_markets: Vec<_> = ctx
            .my_buildings(Some(BuildingKind::FishMarket))
            .into_iter()
            .collect();

        let already_served = existing_markets.iter().any(|b| {
            let dx = (b.pos.x - deposit_pos.x).abs();
            let dy = (b.pos.y - deposit_pos.y).abs();
            dx <= 3 && dy <= 3
        });

        if !already_served {
            let market_stats = cc_core::building_stats::building_stats(BuildingKind::FishMarket);
            if res.food >= market_stats.food_cost && res.gpu_cores >= market_stats.gpu_cost {
                let pos = GridPos::new(deposit_pos.x + 1, deposit_pos.y);
                ctx.cmd_build(builder_id, BuildingKind::FishMarket, pos);
                return BehaviorResult {
                    commands_issued: 1,
                    description: "Expanding economy: building FishMarket near deposit".into(),
                };
            }
        }
    }

    BehaviorResult {
        commands_issued: 0,
        description: "No expansion needed or affordable".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::components::ResourceType;
    use cc_core::map::GameMap;
    use cc_core::math::fixed_from_i32;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    use crate::snapshot::{GameStateSnapshot, ResourceSnapshot};
    use crate::test_fixtures::{make_unit, make_snapshot};

    #[test]
    fn assign_idle_workers_sends_pawdlers_to_deposits() {
        let mut pawdler = make_unit(1, UnitKind::Pawdler, 5, 5, 0);
        pawdler.is_idle = true;

        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![pawdler],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(8, 8),
                remaining: 500,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = assign_idle_workers(&mut ctx);
        assert_eq!(result.commands_issued, 1);

        let cmds = ctx.take_commands();
        assert!(matches!(cmds[0], cc_core::commands::GameCommand::GatherResource { .. }));
    }

    #[test]
    fn assign_idle_workers_skips_non_idle() {
        let mut pawdler = make_unit(1, UnitKind::Pawdler, 5, 5, 0);
        pawdler.is_idle = false;
        pawdler.is_gathering = true;

        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![pawdler],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(8, 8),
                remaining: 500,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = assign_idle_workers(&mut ctx);
        assert_eq!(result.commands_issued, 0);
    }

    #[test]
    fn balanced_production_trains_least_represented() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![
                // 2 Nuisances, 1 Chonk, 0 Hissers → should train Hisser
                make_unit(1, UnitKind::Nuisance, 5, 5, 0),
                make_unit(2, UnitKind::Nuisance, 6, 5, 0),
                make_unit(3, UnitKind::Chonk, 7, 5, 0),
            ],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 500,
                gpu_cores: 200,
                nfts: 0,
                supply: 3,
                supply_cap: 20,
                completed_upgrades: Default::default(),
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = balanced_production(&mut ctx, EntityId(50));
        assert_eq!(result.commands_issued, 1);
        assert!(result.description.contains("Hisser"));
    }

    #[test]
    fn balanced_production_no_resources() {
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
                food: 0,
                gpu_cores: 0,
                nfts: 0,
                supply: 0,
                supply_cap: 0,
                completed_upgrades: Default::default(),
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = balanced_production(&mut ctx, EntityId(50));
        assert_eq!(result.commands_issued, 0);
    }

    #[test]
    fn expand_economy_builds_litter_box_near_cap() {
        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![make_unit(1, UnitKind::Pawdler, 10, 10, 0)],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState {
                food: 500,
                gpu_cores: 200,
                nfts: 0,
                supply: 18,
                supply_cap: 20,
                completed_upgrades: Default::default(),
            },
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let result = expand_economy(&mut ctx, EntityId(1));
        assert_eq!(result.commands_issued, 1);
        assert!(result.description.contains("LitterBox"));
    }
}
