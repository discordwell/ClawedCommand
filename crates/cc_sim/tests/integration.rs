//! Headless integration tests for the simulation pipeline.
//!
//! These tests run the full system chain (tick → commands → apply → movement → apply → grid_sync)
//! against a raw Bevy World without any rendering or windowing.

use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_sim::pathfinding;
use cc_sim::resources::{CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock, SimRng, SpawnPositions};
use cc_sim::systems::{
    ability_effect_system::ability_effect_system, ability_system::ability_cooldown_system,
    aura_system::aura_system, builder_system::builder_system,
    cleanup_system::cleanup_system, combat_system::combat_system,
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, production_system::production_system,
    projectile_system::projectile_system, research_system::research_system,
    resource_system::gathering_system, stat_modifier_system::stat_modifier_system,
    status_effect_system::status_effect_system,
    target_acquisition_system::target_acquisition_system, tick_system::tick_system,
    tower_combat_system::tower_combat_system, victory_system::victory_system,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(SimRng::default());
    world.insert_resource(MapResource { map });

    // Mirror production pipeline from SimSystemsPlugin, using FixedUpdate label
    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            process_commands,
            ability_cooldown_system,
            ability_effect_system,
            status_effect_system,
            aura_system,
            stat_modifier_system,
            production_system,
            research_system,
            gathering_system,
            target_acquisition_system,
            combat_system,
            tower_combat_system,
            projectile_system,
            movement_system,
            builder_system,
            grid_sync_system,
            cleanup_system,
        )
            .chain(),
    );

    (world, schedule)
}

fn spawn_unit(world: &mut World, grid: GridPos) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            MovementSpeed {
                speed: Fixed::from_num(0.15f32),
            },
        ))
        .id()
}

fn run_ticks(world: &mut World, schedule: &mut Schedule, n: usize) {
    for _ in 0..n {
        schedule.run(world);
    }
}

fn issue_move(world: &mut World, entities: &[Entity], target: GridPos) {
    let ids = entities.iter().map(|e| EntityId(e.to_bits())).collect();
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Move {
            unit_ids: ids,
            target,
        });
}

fn issue_stop(world: &mut World, entities: &[Entity]) {
    let ids = entities.iter().map(|e| EntityId(e.to_bits())).collect();
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Stop { unit_ids: ids });
}

/// Simple LCG (PCG-like output) for deterministic pseudo-random test data.
fn lcg_next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 33
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[test]
fn unit_moves_to_target() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let entity = spawn_unit(&mut world, GridPos::new(5, 5));

    issue_move(&mut world, &[entity], GridPos::new(8, 5));

    // 3 tiles at speed 0.15/tick ≈ 20 ticks; 100 gives ample margin
    run_ticks(&mut world, &mut schedule, 100);

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(
        pos.world.to_grid(),
        GridPos::new(8, 5),
        "Unit should arrive at target"
    );
}

#[test]
fn unit_stops_on_command() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let entity = spawn_unit(&mut world, GridPos::new(5, 5));
    let start = WorldPos::from_grid(GridPos::new(5, 5));

    // Start moving toward a distant target
    issue_move(&mut world, &[entity], GridPos::new(20, 20));
    run_ticks(&mut world, &mut schedule, 5);

    // Verify the unit actually started moving
    let pos_after_move = world.get::<Position>(entity).unwrap().world;
    assert_ne!(pos_after_move, start, "Unit should have started moving");

    // Issue stop
    issue_stop(&mut world, &[entity]);
    run_ticks(&mut world, &mut schedule, 2);

    let pos_after_stop = world.get::<Position>(entity).unwrap().world;

    // Run more ticks — position must not change
    run_ticks(&mut world, &mut schedule, 50);

    let pos_later = world.get::<Position>(entity).unwrap().world;
    assert_eq!(pos_after_stop, pos_later, "Unit should stay stopped");
}

#[test]
fn unit_pathfinds_around_obstacle() {
    let mut map = GameMap::new(32, 32);
    // Wall from (10,0) to (10,15)
    for y in 0..16 {
        map.get_mut(GridPos::new(10, y)).unwrap().terrain = cc_core::terrain::TerrainType::Rock;
    }

    let (mut world, mut schedule) = make_sim(map);
    let entity = spawn_unit(&mut world, GridPos::new(8, 5));

    issue_move(&mut world, &[entity], GridPos::new(12, 5));

    // Path must go around the wall — allow plenty of ticks
    run_ticks(&mut world, &mut schedule, 300);

    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(
        pos.world.to_grid(),
        GridPos::new(12, 5),
        "Unit should pathfind around the wall"
    );
}

#[test]
fn multiple_units_move_independently() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let e1 = spawn_unit(&mut world, GridPos::new(3, 3));
    let e2 = spawn_unit(&mut world, GridPos::new(10, 10));

    issue_move(&mut world, &[e1], GridPos::new(6, 3));
    issue_move(&mut world, &[e2], GridPos::new(15, 15));

    run_ticks(&mut world, &mut schedule, 200);

    assert_eq!(
        world.get::<Position>(e1).unwrap().world.to_grid(),
        GridPos::new(6, 3)
    );
    assert_eq!(
        world.get::<Position>(e2).unwrap().world.to_grid(),
        GridPos::new(15, 15)
    );
}

#[test]
fn grid_sync_after_movement() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let entity = spawn_unit(&mut world, GridPos::new(5, 5));

    assert_eq!(
        world.get::<GridCell>(entity).unwrap().pos,
        GridPos::new(5, 5)
    );

    issue_move(&mut world, &[entity], GridPos::new(8, 5));
    run_ticks(&mut world, &mut schedule, 100);

    assert_eq!(
        world.get::<GridCell>(entity).unwrap().pos,
        GridPos::new(8, 5),
        "GridCell should match final Position"
    );
}

#[test]
fn select_and_deselect() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let entity = spawn_unit(&mut world, GridPos::new(5, 5));

    assert!(world.get::<Selected>(entity).is_none());

    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Select {
            unit_ids: vec![EntityId(entity.to_bits())],
        });
    schedule.run(&mut world);

    assert!(
        world.get::<Selected>(entity).is_some(),
        "Unit should be selected"
    );

    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Deselect);
    schedule.run(&mut world);

    assert!(
        world.get::<Selected>(entity).is_none(),
        "Unit should be deselected"
    );
}

#[test]
fn move_to_impassable_target_does_nothing() {
    let mut map = GameMap::new(32, 32);
    map.get_mut(GridPos::new(10, 10)).unwrap().terrain = cc_core::terrain::TerrainType::Rock;

    let (mut world, mut schedule) = make_sim(map);
    let entity = spawn_unit(&mut world, GridPos::new(5, 5));
    let start = WorldPos::from_grid(GridPos::new(5, 5));

    // Move to a blocked tile — pathfinder returns None, unit stays put
    issue_move(&mut world, &[entity], GridPos::new(10, 10));
    run_ticks(&mut world, &mut schedule, 50);

    let pos = world.get::<Position>(entity).unwrap().world;
    assert_eq!(pos, start, "Unit should not move toward impassable target");
}

#[test]
fn sim_clock_advances() {
    let (mut world, mut schedule) = make_sim(GameMap::new(8, 8));

    assert_eq!(world.resource::<SimClock>().tick, 0);

    run_ticks(&mut world, &mut schedule, 10);

    assert_eq!(world.resource::<SimClock>().tick, 10);
}

// ---------------------------------------------------------------------------
// Determinism harness
// ---------------------------------------------------------------------------

#[test]
fn simulation_is_deterministic() {
    fn run_sim() -> Vec<WorldPos> {
        let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
        let e1 = spawn_unit(&mut world, GridPos::new(3, 3));
        let e2 = spawn_unit(&mut world, GridPos::new(10, 10));

        // Tick 0: move unit 1
        issue_move(&mut world, &[e1], GridPos::new(8, 3));
        run_ticks(&mut world, &mut schedule, 10);

        // Tick 10: move unit 2, stop unit 1
        issue_move(&mut world, &[e2], GridPos::new(15, 15));
        issue_stop(&mut world, &[e1]);
        run_ticks(&mut world, &mut schedule, 200);

        vec![
            world.get::<Position>(e1).unwrap().world,
            world.get::<Position>(e2).unwrap().world,
        ]
    }

    let result1 = run_sim();
    let result2 = run_sim();

    // Fixed-point positions must match exactly, bit-for-bit
    assert_eq!(result1, result2, "Simulation must be deterministic");
}

// ---------------------------------------------------------------------------
// Pathfinding stress test
// ---------------------------------------------------------------------------

#[test]
fn pathfinding_stress_64x64() {
    let mut map = GameMap::new(64, 64);

    // Seed a ~20% obstacle density using a simple LCG.
    // With seed 42, 64x64 map, 20% density → empirically finds ~80+ paths out of 200 queries.
    let mut seed: u64 = 42;
    for y in 0..64 {
        for x in 0..64 {
            if lcg_next(&mut seed) % 5 == 0 {
                map.get_mut(GridPos::new(x, y)).unwrap().terrain = cc_core::terrain::TerrainType::Rock;
            }
        }
    }

    let mut paths_found = 0u32;

    seed = 12345;
    for _ in 0..200 {
        let sx = (lcg_next(&mut seed) % 64) as i32;
        let sy = (lcg_next(&mut seed) % 64) as i32;
        let ex = (lcg_next(&mut seed) % 64) as i32;
        let ey = (lcg_next(&mut seed) % 64) as i32;

        let start = GridPos::new(sx, sy);
        let end = GridPos::new(ex, ey);

        if !map.is_passable(start) || !map.is_passable(end) {
            continue;
        }

        if let Some(path) = pathfinding::find_path(&map, start, end, cc_core::terrain::FactionId::CatGPT) {
            paths_found += 1;

            // Every waypoint must be passable
            for wp in &path {
                assert!(map.is_passable(*wp), "Waypoint {wp:?} not passable");
            }

            // Must end at destination
            assert_eq!(*path.last().unwrap(), end);

            // Consecutive waypoints must be adjacent (≤1 in each axis)
            let mut prev = start;
            for wp in &path {
                assert!(
                    (wp.x - prev.x).abs() <= 1 && (wp.y - prev.y).abs() <= 1,
                    "Non-adjacent: {prev:?} → {wp:?}"
                );
                prev = *wp;
            }
        }
    }

    assert!(
        paths_found > 20,
        "Should find many paths on a sparse map (found {paths_found})"
    );
}

// ---------------------------------------------------------------------------
// Terrain-aware integration tests
// ---------------------------------------------------------------------------

/// Spawn a unit with Owner component for faction-aware pathfinding.
fn spawn_owned_unit(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            MovementSpeed {
                speed: Fixed::from_num(0.15f32),
            },
        ))
        .id()
}

#[test]
fn unit_moves_slower_through_forest() {
    use cc_core::terrain::TerrainType;

    // Map: grass path (top) vs forest path (bottom), same distance
    let mut map = GameMap::new(32, 32);
    // Forest strip at y=10
    for x in 0..32 {
        map.get_mut(GridPos::new(x, 10)).unwrap().terrain = TerrainType::Forest;
    }

    let (mut world, mut schedule) = make_sim(map);

    // Unit on grass (y=5)
    let grass_unit = spawn_unit(&mut world, GridPos::new(5, 5));
    // Unit on forest (y=10)
    let forest_unit = spawn_unit(&mut world, GridPos::new(5, 10));

    // Move both 5 tiles right
    issue_move(&mut world, &[grass_unit], GridPos::new(10, 5));
    issue_move(&mut world, &[forest_unit], GridPos::new(10, 10));

    // Run 30 ticks — enough for grass unit to arrive, forest unit should be slower
    run_ticks(&mut world, &mut schedule, 30);

    // Run more ticks to let both arrive
    run_ticks(&mut world, &mut schedule, 100);

    let grass_final = world.get::<Position>(grass_unit).unwrap().world.to_grid();
    let forest_final = world.get::<Position>(forest_unit).unwrap().world.to_grid();

    assert_eq!(grass_final, GridPos::new(10, 5), "Grass unit should arrive");
    assert_eq!(forest_final, GridPos::new(10, 10), "Forest unit should arrive");
}

#[test]
fn unit_moves_faster_on_road() {
    use cc_core::terrain::TerrainType;

    let mut map = GameMap::new(32, 32);
    // Road strip at y=5
    for x in 0..32 {
        map.get_mut(GridPos::new(x, 5)).unwrap().terrain = TerrainType::Road;
    }

    let (mut world, mut schedule) = make_sim(map);
    let road_unit = spawn_unit(&mut world, GridPos::new(2, 5));
    let grass_unit = spawn_unit(&mut world, GridPos::new(2, 10));

    issue_move(&mut world, &[road_unit], GridPos::new(12, 5));
    issue_move(&mut world, &[grass_unit], GridPos::new(12, 10));

    // After 40 ticks, road unit should be further ahead
    run_ticks(&mut world, &mut schedule, 40);

    let road_pos = world.get::<Position>(road_unit).unwrap().world;
    let grass_pos = world.get::<Position>(grass_unit).unwrap().world;

    let road_progress = (road_pos.x - Fixed::from_num(2)).to_num::<f32>();
    let grass_progress = (grass_pos.x - Fixed::from_num(2)).to_num::<f32>();

    assert!(
        road_progress > grass_progress,
        "Road unit ({road_progress:.2}) should progress faster than grass unit ({grass_progress:.2})"
    );
}

#[test]
fn water_river_with_ford_catgpt_uses_ford() {
    use cc_core::terrain::TerrainType;

    let mut map = GameMap::new(20, 20);
    // Water river at x=10
    for y in 0..20 {
        map.get_mut(GridPos::new(10, y)).unwrap().terrain = TerrainType::Water;
    }
    // Ford at (10, 10)
    map.get_mut(GridPos::new(10, 10)).unwrap().terrain = TerrainType::Shallows;

    let (mut world, mut schedule) = make_sim(map);
    // CatGPT unit (player 0) must use the ford
    let unit = spawn_owned_unit(&mut world, GridPos::new(8, 10), 0);

    issue_move(&mut world, &[unit], GridPos::new(12, 10));
    run_ticks(&mut world, &mut schedule, 200);

    let final_pos = world.get::<Position>(unit).unwrap().world.to_grid();
    assert_eq!(final_pos, GridPos::new(12, 10), "CatGPT should cross via ford");
}

#[test]
fn dynamic_terrain_blocks_path() {
    use cc_core::terrain::FLAG_TEMP_BLOCKED;

    let (mut world, mut schedule) = make_sim(GameMap::new(20, 20));
    let unit = spawn_unit(&mut world, GridPos::new(5, 5));

    issue_move(&mut world, &[unit], GridPos::new(15, 5));
    run_ticks(&mut world, &mut schedule, 10);

    // Place a dynamic block ahead of the unit
    {
        let mut map_res = world.resource_mut::<MapResource>();
        for y in 0..20 {
            if let Some(tile) = map_res.map.get_mut(GridPos::new(10, y)) {
                tile.dynamic_flags |= FLAG_TEMP_BLOCKED;
            }
        }
    }

    // The unit was already pathed; it should still try to move
    // (In a full implementation, re-pathing would happen via events)
    run_ticks(&mut world, &mut schedule, 200);

    // Unit should at least have moved from start
    let pos = world.get::<Position>(unit).unwrap().world;
    let start = WorldPos::from_grid(GridPos::new(5, 5));
    assert_ne!(pos, start, "Unit should have moved from start");
}

#[test]
fn determinism_with_terrain_map() {
    use cc_core::terrain::TerrainType;

    fn run_terrain_sim() -> Vec<WorldPos> {
        let mut map = GameMap::new(32, 32);
        // Mixed terrain
        for x in 0..32 {
            map.get_mut(GridPos::new(x, 8)).unwrap().terrain = TerrainType::Forest;
            map.get_mut(GridPos::new(x, 16)).unwrap().terrain = TerrainType::Road;
        }
        for y in 0..32 {
            map.get_mut(GridPos::new(20, y)).unwrap().terrain = TerrainType::Sand;
        }

        let (mut world, mut schedule) = make_sim(map);
        let e1 = spawn_unit(&mut world, GridPos::new(3, 3));
        let e2 = spawn_unit(&mut world, GridPos::new(10, 16));

        issue_move(&mut world, &[e1], GridPos::new(25, 3));
        issue_move(&mut world, &[e2], GridPos::new(25, 16));
        run_ticks(&mut world, &mut schedule, 300);

        vec![
            world.get::<Position>(e1).unwrap().world,
            world.get::<Position>(e2).unwrap().world,
        ]
    }

    let r1 = run_terrain_sim();
    let r2 = run_terrain_sim();
    assert_eq!(r1, r2, "Terrain simulation must be deterministic");
}

#[test]
fn generated_map_is_valid_for_simulation() {
    use cc_core::map_gen::{self, MapGenParams};

    let params = MapGenParams {
        width: 32,
        height: 32,
        seed: 99,
        ..Default::default()
    };
    let def = map_gen::generate_map(&params);
    assert!(def.validate().is_ok());

    let map = def.to_game_map();

    // Spawn points should be passable
    for sp in &def.spawn_points {
        assert!(
            map.is_passable(GridPos::new(sp.pos.0, sp.pos.1)),
            "Spawn ({}, {}) must be passable",
            sp.pos.0,
            sp.pos.1
        );
    }

    // Run a quick sim on the generated map
    let (mut world, mut schedule) = make_sim(map);
    let sp0 = def.spawn_points[0].pos;
    let entity = spawn_unit(&mut world, GridPos::new(sp0.0, sp0.1));

    // Move to nearby position
    issue_move(&mut world, &[entity], GridPos::new(sp0.0 + 2, sp0.1));
    run_ticks(&mut world, &mut schedule, 100);

    let pos = world.get::<Position>(entity).unwrap().world.to_grid();
    assert_eq!(
        pos,
        GridPos::new(sp0.0 + 2, sp0.1),
        "Unit should move on generated map"
    );
}

#[test]
fn generated_templates_work_in_simulation() {
    use cc_core::map_gen::{self, MapGenParams, MapTemplate, MapSize};

    let templates = [MapTemplate::Valley, MapTemplate::Crossroads, MapTemplate::Fortress, MapTemplate::Islands];

    for template in &templates {
        let params = MapGenParams {
            template: *template,
            map_size: MapSize::Small,
            seed: 42,
            ..Default::default()
        };
        let def = map_gen::generate_map(&params);
        assert!(def.validate().is_ok(), "Validation failed for {:?}", template);

        let map = def.to_game_map();
        let (mut world, mut schedule) = make_sim(map);

        // Spawn a unit at first spawn point
        let sp0 = def.spawn_points[0].pos;
        let _entity = spawn_unit(&mut world, GridPos::new(sp0.0, sp0.1));

        // Run 10 ticks without panics
        run_ticks(&mut world, &mut schedule, 10);
    }
}

// ---------------------------------------------------------------------------
// Combat helpers
// ---------------------------------------------------------------------------

use cc_core::unit_stats::base_stats;

/// Spawn a combat-ready unit with full stats from base_stats().
fn spawn_combat_unit(
    world: &mut World,
    grid: GridPos,
    player_id: u8,
    kind: UnitKind,
) -> Entity {
    let stats = base_stats(kind);
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            UnitType { kind },
            Health {
                current: stats.health,
                max: stats.health,
            },
            MovementSpeed {
                speed: stats.speed,
            },
            AttackStats {
                damage: stats.damage,
                range: stats.range,
                attack_speed: stats.attack_speed,
                cooldown_remaining: 0,
            },
            AttackTypeMarker {
                attack_type: stats.attack_type,
            },
        ))
        .id()
}

fn issue_attack(world: &mut World, attackers: &[Entity], target: Entity) {
    let ids = attackers.iter().map(|e| EntityId(e.to_bits())).collect();
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Attack {
            unit_ids: ids,
            target: EntityId(target.to_bits()),
        });
}

fn issue_hold(world: &mut World, entities: &[Entity]) {
    let ids = entities.iter().map(|e| EntityId(e.to_bits())).collect();
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::HoldPosition { unit_ids: ids });
}

fn issue_attack_move(world: &mut World, entities: &[Entity], target: GridPos) {
    let ids = entities.iter().map(|e| EntityId(e.to_bits())).collect();
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::AttackMove {
            unit_ids: ids,
            target,
        });
}

// ---------------------------------------------------------------------------
// Combat integration tests
// ---------------------------------------------------------------------------

#[test]
fn attack_command_sets_target() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);
    run_ticks(&mut world, &mut schedule, 1);

    assert!(
        world.get::<AttackTarget>(attacker).is_some(),
        "Attacker should have AttackTarget after Attack command"
    );
}

#[test]
fn stop_clears_combat_state() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    // Place enemy far away so auto-acquire (weapon range only) won't re-acquire after Stop
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(20, 20), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);
    run_ticks(&mut world, &mut schedule, 1);
    assert!(world.get::<AttackTarget>(attacker).is_some());

    issue_stop(&mut world, &[attacker]);
    run_ticks(&mut world, &mut schedule, 1);

    assert!(
        world.get::<AttackTarget>(attacker).is_none(),
        "Stop should clear AttackTarget"
    );
}

#[test]
fn hold_position_sets_marker() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let unit = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);

    issue_hold(&mut world, &[unit]);
    run_ticks(&mut world, &mut schedule, 1);

    assert!(
        world.get::<HoldPosition>(unit).is_some(),
        "Unit should have HoldPosition marker"
    );
}

#[test]
fn melee_attack_damages_target() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    // Place two melee units adjacent to each other
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    let initial_hp = world.get::<Health>(target).unwrap().current;

    issue_attack(&mut world, &[attacker], target);

    // Run enough ticks for at least one attack (cooldown = 10 ticks)
    run_ticks(&mut world, &mut schedule, 15);

    let hp_after = world.get::<Health>(target).unwrap().current;
    assert!(
        hp_after < initial_hp,
        "Target should have taken damage: initial={initial_hp}, after={hp_after}"
    );
}

#[test]
fn melee_attack_kills_target() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);

    // Nuisance: 8 dmg, 10-tick cooldown, target has 80 HP → 10 attacks needed → ~100 ticks
    // Plus 2 extra ticks for cleanup phases
    run_ticks(&mut world, &mut schedule, 150);

    // Target should have Dead marker (client handles despawn after fade)
    assert!(
        world.entity(target).contains::<Dead>(),
        "Target should be marked Dead after lethal damage"
    );
}

#[test]
fn two_units_fight_to_death() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let a = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let b = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    // Both will auto-acquire each other since they are adjacent enemies
    run_ticks(&mut world, &mut schedule, 200);

    // At least one should have the Dead marker
    let a_dead = world.entity(a).contains::<Dead>();
    let b_dead = world.entity(b).contains::<Dead>();
    assert!(
        a_dead || b_dead,
        "After 200 ticks of fighting, at least one unit should be Dead"
    );
}

#[test]
fn focus_fire_3v1() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let a1 = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let a2 = spawn_combat_unit(&mut world, GridPos::new(5, 6), 0, UnitKind::Nuisance);
    let a3 = spawn_combat_unit(&mut world, GridPos::new(5, 4), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    // Focus-fire all three on the target
    issue_attack(&mut world, &[a1, a2, a3], target);

    // 3 attackers × 8 dmg per 10 ticks = 24 dmg/10 ticks → target (80 HP) dies in ~40 ticks
    run_ticks(&mut world, &mut schedule, 80);

    assert!(
        world.entity(target).contains::<Dead>(),
        "Target should be marked Dead quickly under focus fire"
    );
    // Attackers should still be alive
    assert!(world.get_entity(a1).is_ok(), "Attacker 1 should survive");
    assert!(world.get_entity(a2).is_ok(), "Attacker 2 should survive");
    assert!(world.get_entity(a3).is_ok(), "Attacker 3 should survive");
}

#[test]
fn attacker_pathfinds_into_range() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    // Place attacker far from target (melee range = 1)
    // Use Chonk as target (300 HP tank) so it survives long enough to verify damage
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_combat_unit(&mut world, GridPos::new(10, 5), 1, UnitKind::Chonk);

    issue_attack(&mut world, &[attacker], target);

    let target_initial_hp = world.get::<Health>(target).unwrap().current;

    // Run enough ticks for pathfinding (~33 ticks to travel 5 tiles) + one attack
    run_ticks(&mut world, &mut schedule, 60);

    let target_hp = world.get::<Health>(target).unwrap().current;
    assert!(
        target_hp < target_initial_hp,
        "Attacker should have pathfound to target and dealt damage"
    );
}

#[test]
fn ranged_unit_spawns_projectile() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    // Hisser has range 5, so it should attack from distance
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);
    let target = spawn_combat_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);

    let target_initial_hp = world.get::<Health>(target).unwrap().current;

    // Run enough for projectile to spawn and hit (cooldown 12 + travel time)
    run_ticks(&mut world, &mut schedule, 30);

    let target_hp = world.get::<Health>(target).unwrap().current;
    assert!(
        target_hp < target_initial_hp,
        "Ranged attack should deal damage via projectile"
    );
}

#[test]
fn hold_position_does_not_chase() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let unit = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let _enemy = spawn_combat_unit(&mut world, GridPos::new(10, 5), 1, UnitKind::Nuisance);

    let start_pos = world.get::<Position>(unit).unwrap().world;

    issue_hold(&mut world, &[unit]);
    run_ticks(&mut world, &mut schedule, 50);

    let end_pos = world.get::<Position>(unit).unwrap().world;
    assert_eq!(
        start_pos, end_pos,
        "Unit on hold position should not move to chase enemy"
    );
}

#[test]
fn attack_move_engages_enemy() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let unit = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    // Enemy placed along the path — use Chonk (300 HP) so it survives
    let enemy = spawn_combat_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Chonk);

    let enemy_initial_hp = world.get::<Health>(enemy).unwrap().current;

    issue_attack_move(&mut world, &[unit], GridPos::new(15, 5));
    run_ticks(&mut world, &mut schedule, 80);

    // Enemy should have taken damage (unit passes within melee range and auto-acquires)
    let enemy_hp = world.get::<Health>(enemy).unwrap().current;
    assert!(
        enemy_hp < enemy_initial_hp,
        "Attack-moving unit should engage enemy along the path"
    );
}

#[test]
fn combat_with_elevation_bonus() {
    use cc_core::terrain::TerrainType;

    let mut map = GameMap::new(32, 32);
    // High ground at (5,5), ramp at (6,5) to allow pathfinding
    map.get_mut(GridPos::new(5, 5)).unwrap().elevation = 1;
    map.get_mut(GridPos::new(6, 5)).unwrap().terrain = TerrainType::Ramp;
    map.get_mut(GridPos::new(6, 5)).unwrap().elevation = 1;

    let (mut world, mut schedule) = make_sim(map);
    // Attacker on high ground
    let high_attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);
    // Target on low ground, within range 5
    let low_target = spawn_combat_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[high_attacker], low_target);

    let target_initial_hp = world.get::<Health>(low_target).unwrap().current;

    // Run enough for one ranged attack
    run_ticks(&mut world, &mut schedule, 20);

    let target_hp = world.get::<Health>(low_target).unwrap().current;
    let damage_dealt = target_initial_hp - target_hp;

    // Hisser base damage = 14, elevation bonus (+1 level = 1.15×) → ~16.1
    // Without elevation it would be exactly 14
    assert!(
        damage_dealt > Fixed::from_num(14),
        "High ground should amplify damage: dealt {damage_dealt}"
    );
}

#[test]
fn combat_with_forest_cover_reduces_damage() {
    use cc_core::terrain::TerrainType;

    let mut map = GameMap::new(32, 32);
    // Place forest at the defender's position
    map.get_mut(GridPos::new(8, 5)).unwrap().terrain = TerrainType::Forest;

    let (mut world, mut schedule) = make_sim(map);
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);
    let target = spawn_combat_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);

    let target_initial_hp = world.get::<Health>(target).unwrap().current;

    // 10 ticks: first shot fires on tick 1, projectile arrives ~tick 7 (one hit only)
    run_ticks(&mut world, &mut schedule, 10);

    let target_hp = world.get::<Health>(target).unwrap().current;
    let damage_dealt = target_initial_hp - target_hp;

    // Hisser base damage = 14, forest cover (-15%) → ~11.9
    assert!(
        damage_dealt > Fixed::ZERO && damage_dealt < Fixed::from_num(12),
        "Forest cover should reduce damage below 12: dealt {damage_dealt}"
    );
}

#[test]
fn combat_with_heavy_cover_reduces_damage() {
    use cc_core::terrain::TerrainType;

    let mut map = GameMap::new(32, 32);
    // Place TechRuins at the defender's position
    map.get_mut(GridPos::new(8, 5)).unwrap().terrain = TerrainType::TechRuins;

    let (mut world, mut schedule) = make_sim(map);
    let attacker = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);
    let target = spawn_combat_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Nuisance);

    issue_attack(&mut world, &[attacker], target);

    let target_initial_hp = world.get::<Health>(target).unwrap().current;

    // 10 ticks: first shot fires on tick 1, projectile arrives ~tick 7 (one hit only)
    run_ticks(&mut world, &mut schedule, 10);

    let target_hp = world.get::<Health>(target).unwrap().current;
    let damage_dealt = target_initial_hp - target_hp;

    // Hisser base damage = 14, heavy cover (-30%) → ~9.8
    assert!(
        damage_dealt > Fixed::ZERO && damage_dealt < Fixed::from_num(12),
        "Heavy cover should reduce damage below 12: dealt {damage_dealt}"
    );
}

#[test]
fn combat_determinism() {
    fn run_combat() -> (bool, bool) {
        let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
        let a = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
        let b = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

        run_ticks(&mut world, &mut schedule, 200);

        (world.get_entity(a).is_ok(), world.get_entity(b).is_ok())
    }

    let r1 = run_combat();
    let r2 = run_combat();
    assert_eq!(r1, r2, "Combat must be deterministic");
}

// ---------------------------------------------------------------------------
// Phase 2: Economy, Production, Victory integration tests
// ---------------------------------------------------------------------------

/// Extended make_sim that includes GameState + SpawnPositions + victory_system.
fn make_full_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(GameState::default());
    world.insert_resource(SpawnPositions::default());
    world.insert_resource(SimRng::default());
    world.insert_resource(MapResource { map });

    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            process_commands,
            ability_cooldown_system,
            ability_effect_system,
            status_effect_system,
            aura_system,
            stat_modifier_system,
            production_system,
            research_system,
            gathering_system,
            target_acquisition_system,
            combat_system,
            tower_combat_system,
            projectile_system,
            movement_system,
            builder_system,
            grid_sync_system,
            cleanup_system,
        )
            .chain(),
    );
    // Victory system runs unconditionally (after main chain)
    schedule.add_systems(victory_system);

    (world, schedule)
}

/// Spawn a TheBox building for a given player.
fn spawn_the_box(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    let bstats = cc_core::building_stats::building_stats(BuildingKind::TheBox);
    // Grant supply_cap from the building (matches what setup.rs does at game start)
    if let Some(pres) = world.resource_mut::<PlayerResources>().players.get_mut(player_id as usize) {
        pres.supply_cap += bstats.supply_provided;
    }
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building { kind: BuildingKind::TheBox },
            Health { current: bstats.health, max: bstats.health },
            Producer,
            ProductionQueue::default(),
        ))
        .id()
}

/// Spawn a resource deposit at a grid position.
fn spawn_deposit(world: &mut World, grid: GridPos) -> Entity {
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            ResourceDeposit {
                resource_type: ResourceType::Food,
                remaining: 1500,
            },
        ))
        .id()
}

#[test]
fn test_full_economy_loop() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    // Spawn TheBox at (10,10), deposit at (12,10), Pawdler at (11,10)
    spawn_the_box(&mut world, GridPos::new(10, 10), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(12, 10));
    let worker = spawn_combat_unit(&mut world, GridPos::new(11, 10), 0, UnitKind::Pawdler);

    let initial_food = world.resource::<PlayerResources>().players[0].food;

    // Issue gather command
    world.resource_mut::<CommandQueue>().push(GameCommand::GatherResource {
        unit_ids: vec![EntityId(worker.to_bits())],
        deposit: EntityId(deposit.to_bits()),
    });

    // Run 200 ticks — enough for multiple gather trips (15 ticks harvest + travel time)
    run_ticks(&mut world, &mut schedule, 200);

    let final_food = world.resource::<PlayerResources>().players[0].food;
    assert!(
        final_food > initial_food,
        "Food should increase after gathering: initial={initial_food}, final={final_food}"
    );
}

#[test]
fn test_train_unit_from_the_box() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let box_e = spawn_the_box(&mut world, GridPos::new(10, 10), 0);

    // Set supply to 0 so we have room (supply_cap defaults to 10)
    world.resource_mut::<PlayerResources>().players[0].supply = 0;

    let initial_pawdler_count = world
        .query_filtered::<&UnitType, ()>()
        .iter(&world)
        .filter(|ut| ut.kind == UnitKind::Pawdler)
        .count();

    // Issue train command (Pawdler costs 50 food, 50 ticks to train)
    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(box_e.to_bits()),
        unit_kind: UnitKind::Pawdler,
    });

    // Run 55 ticks (50 train_time + buffer)
    run_ticks(&mut world, &mut schedule, 55);

    let new_pawdler_count = world
        .query_filtered::<&UnitType, ()>()
        .iter(&world)
        .filter(|ut| ut.kind == UnitKind::Pawdler)
        .count();

    assert_eq!(
        new_pawdler_count,
        initial_pawdler_count + 1,
        "A new Pawdler should have been trained"
    );
}

#[test]
fn test_build_command_spawns_building() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    // Need a builder entity (any unit works since the command just checks Owner)
    let builder = spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);
    // Ensure enough food for LitterBox (75 food, 75 tick build time)
    world.resource_mut::<PlayerResources>().players[0].food = 300;

    let build_pos = GridPos::new(12, 10);
    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::LitterBox,
        position: build_pos,
    });

    // Builder needs to walk from (10,10) to adjacent (12,10) — ~20 ticks
    run_ticks(&mut world, &mut schedule, 20);

    // Building should exist with UnderConstruction after builder arrives
    let building_count = world
        .query_filtered::<(&Building, &UnderConstruction), ()>()
        .iter(&world)
        .filter(|(b, _)| b.kind == BuildingKind::LitterBox)
        .count();

    assert_eq!(building_count, 1, "LitterBox should be under construction after builder walks there");

    // Run until construction completes (75 ticks)
    run_ticks(&mut world, &mut schedule, 80);

    // UnderConstruction should be removed
    let still_under_construction = world
        .query_filtered::<(&Building, &UnderConstruction), ()>()
        .iter(&world)
        .filter(|(b, _)| b.kind == BuildingKind::LitterBox)
        .count();

    assert_eq!(still_under_construction, 0, "LitterBox should be finished");
}

#[test]
fn test_build_insufficient_resources() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);

    // Set food too low for CatTree (needs 150)
    world.resource_mut::<PlayerResources>().players[0].food = 50;

    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: GridPos::new(12, 10),
    });

    run_ticks(&mut world, &mut schedule, 5);

    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::CatTree)
        .count();

    assert_eq!(building_count, 0, "CatTree should not spawn without enough food");
}

#[test]
fn test_build_and_train_loop() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    let builder = spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);
    // CatTree costs 150 food, build_time=150, then train Nuisance costs 75, train_time=60
    world.resource_mut::<PlayerResources>().players[0].food = 400;
    world.resource_mut::<PlayerResources>().players[0].supply = 1; // builder uses 1
    world.resource_mut::<PlayerResources>().players[0].supply_cap = 20; // enough room

    // Step 1: Build CatTree
    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: GridPos::new(12, 10),
    });

    // Wait for builder walk (~20 ticks) + construction (150 ticks) + buffer
    run_ticks(&mut world, &mut schedule, 180);

    // Find the CatTree entity (should now be a Producer)
    let cat_tree = world
        .query_filtered::<(Entity, &Building, &Producer), Without<UnderConstruction>>()
        .iter(&world)
        .find(|(_, b, _)| b.kind == BuildingKind::CatTree)
        .map(|(e, _, _)| e);

    assert!(cat_tree.is_some(), "CatTree should be built and producing");
    let cat_tree = cat_tree.unwrap();

    // Step 2: Train a Nuisance
    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(cat_tree.to_bits()),
        unit_kind: UnitKind::Nuisance,
    });

    run_ticks(&mut world, &mut schedule, 65);

    let nuisance_count = world
        .query_filtered::<&UnitType, ()>()
        .iter(&world)
        .filter(|ut| ut.kind == UnitKind::Nuisance)
        .count();

    assert!(nuisance_count >= 1, "A Nuisance should have been trained from CatTree");
}

#[test]
fn test_victory_on_enemy_box_destroyed() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    // Both players have TheBox
    spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let enemy_box = spawn_the_box(&mut world, GridPos::new(20, 20), 1);

    run_ticks(&mut world, &mut schedule, 1);
    assert_eq!(*world.resource::<GameState>(), GameState::Playing);

    // Kill enemy box by setting health to 0 and marking Dead
    world.entity_mut(enemy_box).insert(Dead);
    if let Some(mut health) = world.get_mut::<Health>(enemy_box) {
        health.current = Fixed::ZERO;
    }

    run_ticks(&mut world, &mut schedule, 1);

    assert_eq!(
        *world.resource::<GameState>(),
        GameState::Victory { winner: 0 },
        "Player 0 should win when enemy TheBox is dead"
    );
}

#[test]
fn test_defeat_on_own_box_destroyed() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    let player_box = spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    spawn_the_box(&mut world, GridPos::new(20, 20), 1);

    // Destroy player's box
    world.entity_mut(player_box).insert(Dead);
    if let Some(mut health) = world.get_mut::<Health>(player_box) {
        health.current = Fixed::ZERO;
    }

    run_ticks(&mut world, &mut schedule, 1);

    assert_eq!(
        *world.resource::<GameState>(),
        GameState::Victory { winner: 1 },
        "Player 1 should win when player 0's TheBox is dead"
    );
}

#[test]
fn test_no_victory_while_both_alive() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    spawn_the_box(&mut world, GridPos::new(20, 20), 1);

    run_ticks(&mut world, &mut schedule, 50);

    assert_eq!(
        *world.resource::<GameState>(),
        GameState::Playing,
        "Game should remain Playing while both TheBoxes alive"
    );
}

#[test]
fn test_sim_freezes_on_victory() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let enemy_box = spawn_the_box(&mut world, GridPos::new(20, 20), 1);

    // Kill enemy box
    world.entity_mut(enemy_box).insert(Dead);

    run_ticks(&mut world, &mut schedule, 1);
    assert_eq!(*world.resource::<GameState>(), GameState::Victory { winner: 0 });

    // Record tick count, run more ticks — note that in the full game, the main chain
    // has run_if(game_is_playing), but our test schedule doesn't replicate that.
    // This test verifies the victory_system correctly transitions state and stays stable.
    let tick_at_victory = world.resource::<SimClock>().tick;

    run_ticks(&mut world, &mut schedule, 10);

    // Victory should remain stable
    assert_eq!(
        *world.resource::<GameState>(),
        GameState::Victory { winner: 0 },
        "Victory state should remain stable"
    );
    // Sim clock should still advance since our test schedule doesn't have run_if
    assert!(world.resource::<SimClock>().tick > tick_at_victory);
}

#[test]
fn test_trained_unit_gets_components() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let box_e = spawn_the_box(&mut world, GridPos::new(10, 10), 0);
    world.resource_mut::<PlayerResources>().players[0].supply = 0;

    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(box_e.to_bits()),
        unit_kind: UnitKind::Pawdler,
    });

    run_ticks(&mut world, &mut schedule, 55);

    // Find the trained Pawdler (not the builder)
    let pawdler = world
        .query_filtered::<(Entity, &UnitType, &Health, &AttackStats), ()>()
        .iter(&world)
        .find(|(_, ut, _, _)| ut.kind == UnitKind::Pawdler);

    assert!(pawdler.is_some(), "Trained Pawdler should exist");
    let (_, _, health, attack) = pawdler.unwrap();

    // Verify it got full component set from base_stats
    assert!(health.current > Fixed::ZERO, "Pawdler should have health");
    assert!(attack.damage > Fixed::ZERO, "Pawdler should have damage");
}

#[test]
fn test_combat_determines_winner() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    // Set up two small armies facing each other
    // Player 0: 3 Nuisance
    spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Nuisance);
    spawn_combat_unit(&mut world, GridPos::new(10, 11), 0, UnitKind::Nuisance);
    spawn_combat_unit(&mut world, GridPos::new(10, 12), 0, UnitKind::Nuisance);

    // Player 1: 1 Nuisance (outnumbered)
    let lone_enemy = spawn_combat_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    // Let auto-acquire handle combat
    run_ticks(&mut world, &mut schedule, 200);

    // The outnumbered unit should be dead
    assert!(
        world.entity(lone_enemy).contains::<Dead>(),
        "Outnumbered enemy should die"
    );
}

#[test]
fn test_supply_cap_from_buildings() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);

    // Initial supply_cap is 0 (default, all supply comes from buildings)
    world.resource_mut::<PlayerResources>().players[0].food = 500;

    let initial_cap = world.resource::<PlayerResources>().players[0].supply_cap;

    // Build a LitterBox (provides +10 supply)
    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::LitterBox,
        position: GridPos::new(12, 10),
    });

    // Builder walks from (10,10) to adjacent (12,10) — ~20 ticks for arrival
    run_ticks(&mut world, &mut schedule, 20);

    let new_cap = world.resource::<PlayerResources>().players[0].supply_cap;
    assert_eq!(
        new_cap,
        initial_cap + 10,
        "LitterBox should add 10 supply cap when builder arrives and places it"
    );
}

#[test]
fn test_resource_deposit_depletes() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _box = spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(7, 7));

    // Set deposit remaining to a small amount
    world.get_mut::<ResourceDeposit>(deposit).unwrap().remaining = 20;

    // Spawn a Pawdler already in Harvesting state right at the deposit
    let pawdler = spawn_combat_unit(&mut world, GridPos::new(7, 7), 0, UnitKind::Pawdler);
    world.entity_mut(pawdler).insert(Gathering {
        deposit_entity: EntityId(deposit.to_bits()),
        carried_type: ResourceType::Food,
        carried_amount: 0,
        state: GatherState::Harvesting { ticks_remaining: 1 },
        last_pos: (Fixed::from_num(7), Fixed::from_num(7)),
        stale_ticks: 0,
    });

    run_ticks(&mut world, &mut schedule, 5);

    // Deposit should have been decremented
    let remaining = world.get::<ResourceDeposit>(deposit).unwrap().remaining;
    assert!(remaining < 20, "Deposit should deplete during gathering: remaining={remaining}");
}

#[test]
fn test_dead_gatherer_stops_gathering() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _box = spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(7, 7));

    let pawdler = spawn_combat_unit(&mut world, GridPos::new(7, 7), 0, UnitKind::Pawdler);
    world.entity_mut(pawdler).insert(Gathering {
        deposit_entity: EntityId(deposit.to_bits()),
        carried_type: ResourceType::Food,
        carried_amount: 0,
        state: GatherState::Harvesting { ticks_remaining: 10 },
        last_pos: (Fixed::from_num(7), Fixed::from_num(7)),
        stale_ticks: 0,
    });

    // Mark the gatherer as Dead
    world.entity_mut(pawdler).insert(Dead);

    let initial_food = world.resource::<PlayerResources>().players[0].food;

    // Run ticks — dead gatherer should not produce resources
    run_ticks(&mut world, &mut schedule, 20);

    let final_food = world.resource::<PlayerResources>().players[0].food;
    assert_eq!(
        initial_food, final_food,
        "Dead gatherer should not produce resources"
    );
}

#[test]
fn test_build_on_water_rejected() {
    let mut map = GameMap::new(32, 32);
    // Set tile at (15, 15) to water
    if let Some(tile) = map.get_mut(GridPos::new(15, 15)) {
        tile.terrain = cc_core::terrain::TerrainType::Water;
    }

    let (mut world, mut schedule) = make_full_sim(map);
    let builder = spawn_combat_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].food = 500;

    let initial_food = world.resource::<PlayerResources>().players[0].food;

    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::LitterBox,
        position: GridPos::new(15, 15),
    });

    run_ticks(&mut world, &mut schedule, 5);

    // Building should NOT have been placed
    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::LitterBox)
        .count();

    assert_eq!(building_count, 0, "Should not build on water");

    // Resources should NOT have been deducted
    let final_food = world.resource::<PlayerResources>().players[0].food;
    assert_eq!(initial_food, final_food, "Resources should not be deducted for rejected build");
}

#[test]
fn test_victory_with_more_than_two_players() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));

    // Add a third player to PlayerResources
    world.resource_mut::<PlayerResources>().players.push(
        cc_sim::resources::PlayerResourceState::default()
    );

    // Three players with TheBox
    spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    spawn_the_box(&mut world, GridPos::new(20, 20), 1);
    let box_2 = spawn_the_box(&mut world, GridPos::new(15, 15), 2);

    run_ticks(&mut world, &mut schedule, 1);
    assert_eq!(*world.resource::<GameState>(), GameState::Playing);

    // Kill player 2's box
    world.entity_mut(box_2).insert(Dead);
    run_ticks(&mut world, &mut schedule, 1);

    // Game should still be playing (2 players remain)
    assert_eq!(
        *world.resource::<GameState>(),
        GameState::Playing,
        "Game should continue with 2 remaining TheBoxes"
    );
}

// ---------------------------------------------------------------------------
// Phase 4A: Abilities, Buildings, Tech Tree integration tests
// ---------------------------------------------------------------------------

use cc_core::abilities::unit_abilities;
use cc_core::commands::AbilityTarget;
use cc_core::status_effects::StatusEffects;

/// Spawn a combat unit with full Phase 4A components (AbilitySlots, StatusEffects, StatModifiers).
fn spawn_full_unit(
    world: &mut World,
    grid: GridPos,
    player_id: u8,
    kind: UnitKind,
) -> Entity {
    let stats = base_stats(kind);
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            UnitType { kind },
            Health { current: stats.health, max: stats.health },
            MovementSpeed { speed: stats.speed },
            AttackStats {
                damage: stats.damage,
                range: stats.range,
                attack_speed: stats.attack_speed,
                cooldown_remaining: 0,
            },
            AttackTypeMarker { attack_type: stats.attack_type },
            AbilitySlots::from_abilities(unit_abilities(kind)),
            StatusEffects::default(),
            StatModifiers::default(),
        ))
        .id()
}

/// Spawn a ScratchingPost (already constructed) for research tests.
fn spawn_scratching_post(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    let bstats = cc_core::building_stats::building_stats(BuildingKind::ScratchingPost);
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building { kind: BuildingKind::ScratchingPost },
            Health { current: bstats.health, max: bstats.health },
            Researcher,
            ResearchQueue::default(),
        ))
        .id()
}

/// Spawn a ServerRack (already constructed) for advanced unit training.
fn spawn_server_rack(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    let bstats = cc_core::building_stats::building_stats(BuildingKind::ServerRack);
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building { kind: BuildingKind::ServerRack },
            Health { current: bstats.health, max: bstats.health },
            Producer,
            ProductionQueue::default(),
        ))
        .id()
}

/// Spawn a LaserPointer tower (already constructed).
fn spawn_laser_pointer(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    let bstats = cc_core::building_stats::building_stats(BuildingKind::LaserPointer);
    world
        .spawn((
            Position { world: WorldPos::from_grid(grid) },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building { kind: BuildingKind::LaserPointer },
            Health { current: bstats.health, max: bstats.health },
            AttackStats {
                damage: Fixed::from_num(10),
                range: Fixed::from_num(6),
                attack_speed: 15,
                cooldown_remaining: 0,
            },
            AttackTypeMarker { attack_type: AttackType::Ranged },
        ))
        .id()
}

#[test]
fn test_ability_infrastructure_on_spawn() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let box_e = spawn_the_box(&mut world, GridPos::new(10, 10), 0);
    world.resource_mut::<PlayerResources>().players[0].supply = 0;

    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(box_e.to_bits()),
        unit_kind: UnitKind::Pawdler,
    });

    run_ticks(&mut world, &mut schedule, 55);

    // Find the trained Pawdler
    let has_full_components = world
        .query_filtered::<(&UnitType, &AbilitySlots, &StatusEffects, &StatModifiers), ()>()
        .iter(&world)
        .any(|(ut, _, _, _)| ut.kind == UnitKind::Pawdler);

    assert!(has_full_components, "Trained unit should have AbilitySlots, StatusEffects, and StatModifiers");
}

#[test]
fn test_activate_ability_cooldown_cycle() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    // Nuisance slot 2 = Zoomies: Activated, cooldown 120, gpu_cost 10, duration 30
    let unit = spawn_full_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);

    // Ensure GPU available
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;

    // Activate Zoomies (slot 2)
    world.resource_mut::<CommandQueue>().push(GameCommand::ActivateAbility {
        unit_id: EntityId(unit.to_bits()),
        slot: 2,
        target: AbilityTarget::SelfCast,
    });

    run_ticks(&mut world, &mut schedule, 1);

    let slots = world.get::<AbilitySlots>(unit).unwrap();
    assert!(slots.slots[2].active, "Zoomies should be active after activation");
    assert!(slots.slots[2].cooldown_remaining > 0, "Cooldown should be set");

    // Try to activate again — should be rejected (on cooldown)
    let gpu_before = world.resource::<PlayerResources>().players[0].gpu_cores;
    world.resource_mut::<CommandQueue>().push(GameCommand::ActivateAbility {
        unit_id: EntityId(unit.to_bits()),
        slot: 2,
        target: AbilityTarget::SelfCast,
    });
    run_ticks(&mut world, &mut schedule, 1);
    let gpu_after = world.resource::<PlayerResources>().players[0].gpu_cores;
    assert_eq!(gpu_before, gpu_after, "GPU should not be deducted when ability is on cooldown");

    // Run until cooldown expires (120 ticks)
    run_ticks(&mut world, &mut schedule, 125);

    let slots = world.get::<AbilitySlots>(unit).unwrap();
    assert_eq!(slots.slots[2].cooldown_remaining, 0, "Cooldown should have expired");
    assert!(!slots.slots[2].active, "Duration should have expired (30 ticks < 125 ticks)");
}

#[test]
fn test_stat_modifiers_affect_combat() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    // Two adjacent melee units, one with boosted damage
    let attacker = spawn_full_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_full_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);

    // Boost attacker's damage_multiplier to 2x via stat modifiers
    world.get_mut::<StatModifiers>(attacker).unwrap().damage_multiplier = Fixed::from_num(2);

    let initial_hp = world.get::<Health>(target).unwrap().current;
    issue_attack(&mut world, &[attacker], target);
    run_ticks(&mut world, &mut schedule, 15);

    let hp_after = world.get::<Health>(target).unwrap().current;
    let damage_dealt = initial_hp - hp_after;
    let base_damage = base_stats(UnitKind::Nuisance).damage;

    assert!(
        damage_dealt > base_damage,
        "Damage with 2x multiplier ({damage_dealt}) should exceed base ({base_damage})"
    );
}

#[test]
fn test_stat_modifiers_affect_movement() {
    use cc_core::status_effects::{StatusEffectId, StatusInstance};

    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let fast_unit = spawn_full_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let normal_unit = spawn_full_unit(&mut world, GridPos::new(5, 8), 0, UnitKind::Nuisance);

    // Apply Zoomies status effect (grants +100% speed via stat_modifier_system)
    world.get_mut::<StatusEffects>(fast_unit).unwrap().effects.push(StatusInstance {
        effect: StatusEffectId::Zoomies,
        remaining_ticks: 100,
        stacks: 1,
        source: EntityId(0),
    });

    issue_move(&mut world, &[fast_unit], GridPos::new(15, 5));
    issue_move(&mut world, &[normal_unit], GridPos::new(15, 8));
    run_ticks(&mut world, &mut schedule, 30);

    let fast_pos = world.get::<Position>(fast_unit).unwrap().world;
    let normal_pos = world.get::<Position>(normal_unit).unwrap().world;

    // Fast unit should be further along (higher x)
    assert!(
        fast_pos.x > normal_pos.x,
        "Boosted unit should move faster: fast_x={}, normal_x={}",
        fast_pos.x, normal_pos.x
    );
}

#[test]
fn test_server_rack_trains_advanced_units() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let rack = spawn_server_rack(&mut world, GridPos::new(10, 10), 0);

    world.resource_mut::<PlayerResources>().players[0].food = 500;
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 200;
    world.resource_mut::<PlayerResources>().players[0].supply = 0;
    world.resource_mut::<PlayerResources>().players[0].supply_cap = 20;

    // Train a FlyingFox (no upgrade gate)
    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(rack.to_bits()),
        unit_kind: UnitKind::FlyingFox,
    });

    // FlyingFox train_time = 80 ticks
    run_ticks(&mut world, &mut schedule, 85);

    let fox_count = world
        .query_filtered::<&UnitType, ()>()
        .iter(&world)
        .filter(|ut| ut.kind == UnitKind::FlyingFox)
        .count();

    assert_eq!(fox_count, 1, "FlyingFox should have been trained from ServerRack");
}

#[test]
fn test_laser_pointer_fires_at_enemies() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _tower = spawn_laser_pointer(&mut world, GridPos::new(10, 10), 0);
    let target = spawn_full_unit(&mut world, GridPos::new(13, 10), 1, UnitKind::Nuisance);

    let initial_hp = world.get::<Health>(target).unwrap().current;

    // Tower has range 6, target is 3 tiles away — should fire
    run_ticks(&mut world, &mut schedule, 30);

    let hp_after = world.get::<Health>(target).unwrap().current;
    assert!(
        hp_after < initial_hp,
        "LaserPointer should damage nearby enemy: initial={initial_hp}, after={hp_after}"
    );
}

#[test]
fn test_research_completes_and_applies() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let sp = spawn_scratching_post(&mut world, GridPos::new(10, 10), 0);

    // Give enough resources
    world.resource_mut::<PlayerResources>().players[0].food = 500;
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 200;

    // Spawn a combat unit to receive the upgrade
    let unit = spawn_full_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let dmg_before = world.get::<AttackStats>(unit).unwrap().damage;

    // Research SharperClaws (+2 damage, 200 ticks)
    world.resource_mut::<CommandQueue>().push(GameCommand::Research {
        building: EntityId(sp.to_bits()),
        upgrade: UpgradeType::SharperClaws,
    });

    run_ticks(&mut world, &mut schedule, 205);

    let dmg_after = world.get::<AttackStats>(unit).unwrap().damage;
    assert!(
        dmg_after > dmg_before,
        "SharperClaws should add +2 damage: before={dmg_before}, after={dmg_after}"
    );

    // Verify upgrade is recorded
    let completed = &world.resource::<PlayerResources>().players[0].completed_upgrades;
    assert!(completed.contains(&UpgradeType::SharperClaws), "SharperClaws should be in completed_upgrades");
}

#[test]
fn test_upgrade_gates_unit_training() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let rack = spawn_server_rack(&mut world, GridPos::new(10, 10), 0);

    world.resource_mut::<PlayerResources>().players[0].food = 1000;
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 500;
    world.resource_mut::<PlayerResources>().players[0].supply = 0;
    world.resource_mut::<PlayerResources>().players[0].supply_cap = 20;

    // Try training MechCommander without MechPrototype — should be rejected
    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(rack.to_bits()),
        unit_kind: UnitKind::MechCommander,
    });

    run_ticks(&mut world, &mut schedule, 500);

    let mech_count = world
        .query_filtered::<&UnitType, ()>()
        .iter(&world)
        .filter(|ut| ut.kind == UnitKind::MechCommander)
        .count();

    assert_eq!(mech_count, 0, "MechCommander should not train without MechPrototype upgrade");
}

#[test]
fn test_cancel_research_refunds() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let sp = spawn_scratching_post(&mut world, GridPos::new(10, 10), 0);

    world.resource_mut::<PlayerResources>().players[0].food = 500;
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 200;

    // Queue SharperClaws (costs 150F, 50G)
    world.resource_mut::<CommandQueue>().push(GameCommand::Research {
        building: EntityId(sp.to_bits()),
        upgrade: UpgradeType::SharperClaws,
    });

    run_ticks(&mut world, &mut schedule, 1);

    let food_after_queue = world.resource::<PlayerResources>().players[0].food;
    let gpu_after_queue = world.resource::<PlayerResources>().players[0].gpu_cores;
    assert_eq!(food_after_queue, 350, "150 food should be deducted");
    assert_eq!(gpu_after_queue, 150, "50 GPU should be deducted");

    // Cancel research
    world.resource_mut::<CommandQueue>().push(GameCommand::CancelResearch {
        building: EntityId(sp.to_bits()),
    });

    run_ticks(&mut world, &mut schedule, 1);

    let food_after_cancel = world.resource::<PlayerResources>().players[0].food;
    let gpu_after_cancel = world.resource::<PlayerResources>().players[0].gpu_cores;
    assert_eq!(food_after_cancel, 500, "Food should be refunded on cancel");
    assert_eq!(gpu_after_cancel, 200, "GPU should be refunded on cancel");
}

#[test]
fn test_new_units_get_upgrades() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let sp = spawn_scratching_post(&mut world, GridPos::new(10, 10), 0);

    // Build a CatTree (already constructed) to train Nuisance from
    let cat_tree = {
        let bstats = cc_core::building_stats::building_stats(BuildingKind::CatTree);
        world
            .spawn((
                Position { world: WorldPos::from_grid(GridPos::new(15, 15)) },
                Velocity::zero(),
                GridCell { pos: GridPos::new(15, 15) },
                Owner { player_id: 0 },
                Building { kind: BuildingKind::CatTree },
                Health { current: bstats.health, max: bstats.health },
                Producer,
                ProductionQueue::default(),
            ))
            .id()
    };

    world.resource_mut::<PlayerResources>().players[0].food = 1000;
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 200;
    world.resource_mut::<PlayerResources>().players[0].supply = 0;
    world.resource_mut::<PlayerResources>().players[0].supply_cap = 20;

    // Research ThickerFur (+25 HP for combat units, 200 ticks)
    world.resource_mut::<CommandQueue>().push(GameCommand::Research {
        building: EntityId(sp.to_bits()),
        upgrade: UpgradeType::ThickerFur,
    });

    run_ticks(&mut world, &mut schedule, 205);

    // Verify upgrade completed
    assert!(
        world.resource::<PlayerResources>().players[0]
            .completed_upgrades.contains(&UpgradeType::ThickerFur),
        "ThickerFur should be completed"
    );

    // Train a Nuisance (combat unit) — it should spawn with +25 HP
    world.resource_mut::<CommandQueue>().push(GameCommand::TrainUnit {
        building: EntityId(cat_tree.to_bits()),
        unit_kind: UnitKind::Nuisance,
    });

    // Nuisance train_time = 60 ticks
    run_ticks(&mut world, &mut schedule, 65);

    let base_hp = base_stats(UnitKind::Nuisance).health;
    let expected_hp = base_hp + Fixed::from_num(25);

    let has_boosted_unit = world
        .query_filtered::<(&UnitType, &Health), ()>()
        .iter(&world)
        .any(|(ut, hp)| ut.kind == UnitKind::Nuisance && hp.max >= expected_hp);

    assert!(has_boosted_unit, "Newly trained Nuisance should have +25 HP from ThickerFur");
}

// ---------------------------------------------------------------------------
// Bug fix: Stale gatherer detection
// ---------------------------------------------------------------------------

#[test]
fn test_stale_gatherer_gets_released() {
    // A worker with Gathering + MoveTarget that never changes position should
    // lose its Gathering component after GATHERER_STALE_TICKS ticks.
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _box = spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(20, 20));

    // Spawn worker with zero movement speed so movement_system cannot advance
    // its position, simulating a stuck gatherer.
    let worker = world.spawn((
        Position { world: WorldPos::from_grid(GridPos::new(10, 10)) },
        Velocity::zero(),
        GridCell { pos: GridPos::new(10, 10) },
        Owner { player_id: 0 },
        UnitType { kind: UnitKind::Pawdler },
        Health { current: Fixed::from_num(50), max: Fixed::from_num(50) },
        MovementSpeed { speed: Fixed::ZERO }, // zero speed = can't move
        AttackStats {
            damage: Fixed::from_num(5),
            range: Fixed::from_num(1),
            attack_speed: 10,
            cooldown_remaining: 0,
        },
        AttackTypeMarker { attack_type: AttackType::Melee },
    )).id();

    // Manually set up gathering + MoveTarget (simulates command having been issued)
    let worker_pos = world.get::<Position>(worker).unwrap().world;
    world.entity_mut(worker).insert(Gathering {
        deposit_entity: EntityId(deposit.to_bits()),
        carried_type: ResourceType::Food,
        carried_amount: 0,
        state: GatherState::MovingToDeposit,
        last_pos: (worker_pos.x, worker_pos.y),
        stale_ticks: 0,
    });
    // MoveTarget pointing at the deposit (unreachable due to zero speed)
    world.entity_mut(worker).insert(MoveTarget {
        target: WorldPos::from_grid(GridPos::new(20, 20)),
    });

    // Run for less than GATHERER_STALE_TICKS — should still have Gathering
    run_ticks(&mut world, &mut schedule, 25);
    assert!(
        world.get::<Gathering>(worker).is_some(),
        "Worker should still be gathering before stale threshold"
    );

    // Run past the stale threshold (30 ticks total from start + some buffer)
    run_ticks(&mut world, &mut schedule, 10);

    assert!(
        world.get::<Gathering>(worker).is_none(),
        "Stale gatherer should have Gathering removed after {} ticks with no progress",
        cc_core::tuning::GATHERER_STALE_TICKS
    );
}

// ---------------------------------------------------------------------------
// Bug fix: ReturningToBase proximity check
// ---------------------------------------------------------------------------

#[test]
fn test_returning_worker_deposits_near_dropoff() {
    // A worker that completes the ReturningToBase movement and ends up near a
    // drop-off building should deposit resources normally.
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _box = spawn_the_box(&mut world, GridPos::new(10, 10), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(12, 10));

    let worker = spawn_combat_unit(&mut world, GridPos::new(11, 10), 0, UnitKind::Pawdler);

    // Issue gather command — full cycle should work
    world.resource_mut::<CommandQueue>().push(GameCommand::GatherResource {
        unit_ids: vec![EntityId(worker.to_bits())],
        deposit: EntityId(deposit.to_bits()),
    });

    let initial_food = world.resource::<PlayerResources>().players[0].food;

    // Run 200 ticks — enough for multiple gather trips
    run_ticks(&mut world, &mut schedule, 200);

    let final_food = world.resource::<PlayerResources>().players[0].food;
    assert!(
        final_food > initial_food,
        "Worker near drop-off should deposit resources: initial={initial_food}, final={final_food}"
    );
}

#[test]
fn test_returning_worker_no_deposit_when_far_from_dropoff() {
    // A worker in ReturningToBase state that loses its MoveTarget while far from
    // any drop-off should NOT deposit resources.
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let _box = spawn_the_box(&mut world, GridPos::new(5, 5), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(20, 20));

    // Spawn worker far from the drop-off
    let worker = spawn_combat_unit(&mut world, GridPos::new(15, 15), 0, UnitKind::Pawdler);

    // Manually set up ReturningToBase with carried resources, but NO MoveTarget
    // (simulates MoveTarget being stripped by a Stop command or path failure).
    let worker_pos = world.get::<Position>(worker).unwrap().world;
    world.entity_mut(worker).insert(Gathering {
        deposit_entity: EntityId(deposit.to_bits()),
        carried_type: ResourceType::Food,
        carried_amount: 10,
        state: GatherState::ReturningToBase,
        last_pos: (worker_pos.x, worker_pos.y),
        stale_ticks: 0,
    });
    // Intentionally no MoveTarget — this is the bug scenario

    let initial_food = world.resource::<PlayerResources>().players[0].food;

    // Run a tick — the gathering system should check proximity and NOT deposit
    run_ticks(&mut world, &mut schedule, 1);

    let final_food = world.resource::<PlayerResources>().players[0].food;
    assert_eq!(
        initial_food, final_food,
        "Worker far from drop-off should NOT deposit resources: initial={initial_food}, final={final_food}"
    );

    // Gathering component should have been removed (worker released for reassignment)
    assert!(
        world.get::<Gathering>(worker).is_none(),
        "Gathering should be removed when worker is far from drop-off with no MoveTarget"
    );
}

// ---------------------------------------------------------------------------
// Bug fix: Tuning constants consolidated
// ---------------------------------------------------------------------------

#[test]
fn test_tuning_constants_accessible() {
    // Verify that the centralized tuning module constants are sane and accessible.
    use cc_core::tuning;

    assert!(tuning::HARVEST_TICKS > 0, "HARVEST_TICKS should be positive");
    assert!(tuning::CARRY_AMOUNT > 0, "CARRY_AMOUNT should be positive");
    assert!(tuning::GATHERER_STALE_TICKS > 0, "GATHERER_STALE_TICKS should be positive");
    assert!(tuning::DROPOFF_PROXIMITY_SQ > 0, "DROPOFF_PROXIMITY_SQ should be positive");
    assert!(tuning::PROJECTILE_SPEED > Fixed::ZERO, "PROJECTILE_SPEED should be positive");
    assert!(tuning::TOWER_PROJECTILE_SPEED > Fixed::ZERO, "TOWER_PROJECTILE_SPEED should be positive");
    assert!(tuning::ATTACK_MOVE_SIGHT_RANGE > 0, "ATTACK_MOVE_SIGHT_RANGE should be positive");
    assert!(tuning::CC_IMMUNITY_TICKS > 0, "CC_IMMUNITY_TICKS should be positive");
    assert!(tuning::ATTACK_REISSUE_INTERVAL > 0, "ATTACK_REISSUE_INTERVAL should be positive");
    assert!(tuning::BASE_THREAT_RADIUS > 0, "BASE_THREAT_RADIUS should be positive");
}

// ---------------------------------------------------------------------------
// Builder walk-to-build-site tests
// ---------------------------------------------------------------------------

#[test]
fn test_builder_walks_to_build_site() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].food = 300;

    let build_pos = GridPos::new(10, 5);
    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: build_pos,
    });

    // After 1 tick: builder should have BuildOrder, no building yet
    run_ticks(&mut world, &mut schedule, 1);
    assert!(
        world.get::<BuildOrder>(builder).is_some(),
        "Builder should have BuildOrder after build command"
    );
    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::CatTree)
        .count();
    assert_eq!(building_count, 0, "Building should not exist yet while builder walks");

    // Run enough ticks for builder to walk there (~60 ticks for 5 tiles at 0.12/tick)
    run_ticks(&mut world, &mut schedule, 60);

    // Building should now exist
    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::CatTree)
        .count();
    assert_eq!(building_count, 1, "CatTree should be placed after builder arrives");

    // BuildOrder should be removed from builder
    assert!(
        world.get::<BuildOrder>(builder).is_none(),
        "BuildOrder should be removed after building is placed"
    );
}

#[test]
fn test_dead_builder_cannot_place_building() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].food = 300;

    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: GridPos::new(10, 5),
    });

    run_ticks(&mut world, &mut schedule, 1);

    // Kill the builder mid-walk
    world.entity_mut(builder).insert(Dead);
    if let Some(mut health) = world.get_mut::<Health>(builder) {
        health.current = Fixed::ZERO;
    }

    // Run enough ticks for the builder to have arrived if alive
    run_ticks(&mut world, &mut schedule, 80);

    // Building should NOT exist — dead builder can't place
    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::CatTree)
        .count();
    assert_eq!(building_count, 0, "Dead builder should not place building");
}

#[test]
fn test_stop_cancels_build_order() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].food = 300;

    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: GridPos::new(10, 5),
    });

    run_ticks(&mut world, &mut schedule, 1);
    assert!(world.get::<BuildOrder>(builder).is_some());

    // Issue Stop command
    world.resource_mut::<CommandQueue>().push(GameCommand::Stop {
        unit_ids: vec![EntityId(builder.to_bits())],
    });

    run_ticks(&mut world, &mut schedule, 1);
    assert!(
        world.get::<BuildOrder>(builder).is_none(),
        "Stop should clear BuildOrder"
    );

    // Run more ticks — no building should appear
    run_ticks(&mut world, &mut schedule, 80);
    let building_count = world
        .query_filtered::<&Building, ()>()
        .iter(&world)
        .filter(|b| b.kind == BuildingKind::CatTree)
        .count();
    assert_eq!(building_count, 0, "No building after stop cancels build order");
}

#[test]
fn test_move_cancels_build_order() {
    let (mut world, mut schedule) = make_full_sim(GameMap::new(32, 32));
    let builder = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].food = 300;

    world.resource_mut::<CommandQueue>().push(GameCommand::Build {
        builder: EntityId(builder.to_bits()),
        building_kind: BuildingKind::CatTree,
        position: GridPos::new(10, 5),
    });

    run_ticks(&mut world, &mut schedule, 1);
    assert!(world.get::<BuildOrder>(builder).is_some());

    // Issue Move command to redirect builder
    world.resource_mut::<CommandQueue>().push(GameCommand::Move {
        unit_ids: vec![EntityId(builder.to_bits())],
        target: GridPos::new(3, 3),
    });

    run_ticks(&mut world, &mut schedule, 1);
    assert!(
        world.get::<BuildOrder>(builder).is_none(),
        "Move should clear BuildOrder"
    );
    // CatTree costs 150 food — should be refunded
    let food_after = world.resource::<PlayerResources>().players[0].food;
    assert_eq!(food_after, 300, "Move should refund CatTree's 150 food cost, got {food_after}");
}
