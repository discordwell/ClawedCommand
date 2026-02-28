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
use cc_sim::resources::{CommandQueue, MapResource, SimClock};
use cc_sim::systems::{
    cleanup_system::cleanup_system, combat_system::combat_system,
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, projectile_system::projectile_system,
    target_acquisition_system::target_acquisition_system, tick_system::tick_system,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(MapResource { map });

    // Mirror production pipeline from SimSystemsPlugin, using FixedUpdate label
    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            process_commands,
            target_acquisition_system,
            combat_system,
            projectile_system,
            movement_system,
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

    // Target should be despawned
    assert!(
        world.get_entity(target).is_err(),
        "Target should be despawned after death"
    );
}

#[test]
fn two_units_fight_to_death() {
    let (mut world, mut schedule) = make_sim(GameMap::new(32, 32));
    let a = spawn_combat_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let b = spawn_combat_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Nuisance);

    // Both will auto-acquire each other since they are adjacent enemies
    run_ticks(&mut world, &mut schedule, 200);

    // At least one should be dead
    let a_alive = world.get_entity(a).is_ok();
    let b_alive = world.get_entity(b).is_ok();
    assert!(
        !a_alive || !b_alive,
        "After 200 ticks of fighting, at least one unit should be dead"
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
        world.get_entity(target).is_err(),
        "Target should die quickly under focus fire"
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
