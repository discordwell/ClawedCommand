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
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, tick_system::tick_system,
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
            apply_deferred,
            movement_system,
            apply_deferred,
            grid_sync_system,
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
        map.get_mut(GridPos::new(10, y)).unwrap().passable = false;
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
    map.get_mut(GridPos::new(10, 10)).unwrap().passable = false;

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
                map.get_mut(GridPos::new(x, y)).unwrap().passable = false;
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

        if let Some(path) = pathfinding::find_path(&map, start, end) {
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
