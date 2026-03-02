//! Integration tests for the voice intent system.
//!
//! These tests run `voice_intent_system` against a headless Bevy World with
//! spawned entities, injected voice events, and verify the resulting GameCommands.

use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::unit_stats::base_stats;
use cc_sim::resources::{CommandQueue, MapResource, VoiceOverride};
use cc_voice::events::VoiceCommandEvent;
use cc_voice::intent::voice_intent_system;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Minimal resource to queue voice keywords for injection.
#[derive(Resource, Default)]
struct VoiceInjectionQueue {
    keywords: Vec<(String, f32)>,
}

/// System that drains VoiceInjectionQueue into VoiceCommandEvent messages.
fn inject_voice_events(
    mut queue: ResMut<VoiceInjectionQueue>,
    mut writer: MessageWriter<VoiceCommandEvent>,
) {
    for (keyword, confidence) in queue.keywords.drain(..) {
        writer.write(VoiceCommandEvent {
            keyword,
            confidence,
        });
    }
}

/// Create a headless World + Schedule with the voice intent system wired up.
fn make_voice_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(VoiceOverride::default());
    world.insert_resource(MapResource { map });
    world.insert_resource(CursorGridPos::default());
    world.insert_resource(VoiceInjectionQueue::default());
    world.init_resource::<Messages<VoiceCommandEvent>>();

    let mut schedule = Schedule::new(Update);
    schedule.add_systems((inject_voice_events, voice_intent_system).chain());

    (world, schedule)
}

fn spawn_owned_unit(
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
            MovementSpeed { speed: stats.speed },
            AttackStats {
                damage: stats.damage,
                range: stats.range,
                attack_speed: stats.attack_speed,
                cooldown_remaining: 0,
            },
            AttackTypeMarker {
                attack_type: stats.attack_type,
            },
            StatModifiers::default(),
        ))
        .id()
}

fn spawn_building(
    world: &mut World,
    grid: GridPos,
    player_id: u8,
    kind: BuildingKind,
) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Owner { player_id },
            Building { kind },
        ))
        .id()
}

fn say(world: &mut World, keyword: &str) {
    world
        .resource_mut::<VoiceInjectionQueue>()
        .keywords
        .push((keyword.into(), 0.95));
}

fn say_many(world: &mut World, keywords: &[&str]) {
    let mut queue = world.resource_mut::<VoiceInjectionQueue>();
    for kw in keywords {
        queue.keywords.push(((*kw).into(), 0.95));
    }
}

fn drain_commands(world: &mut World) -> Vec<GameCommand> {
    world
        .resource_mut::<CommandQueue>()
        .commands
        .drain(..)
        .map(|qc| qc.command)
        .collect()
}

fn tick(world: &mut World, schedule: &mut Schedule) {
    schedule.run(world);
}

// ---------------------------------------------------------------------------
// Tests: Stop / Hold / Defend
// ---------------------------------------------------------------------------

#[test]
fn voice_stop_issues_stop_command() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    let _u1 = spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    let _u2 = spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Hisser);

    say(&mut world, "stop");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1, "should produce exactly one command");
    assert!(
        matches!(&cmds[0], GameCommand::Stop { unit_ids } if unit_ids.len() == 2),
        "should be a Stop command with 2 units, got: {:?}",
        cmds[0]
    );
}

#[test]
fn voice_hold_issues_hold_position() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);

    say(&mut world, "hold");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], GameCommand::HoldPosition { .. }));
}

#[test]
fn voice_defend_maps_to_hold_position() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);

    say(&mut world, "defend");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    assert!(matches!(&cmds[0], GameCommand::HoldPosition { .. }));
}

// ---------------------------------------------------------------------------
// Tests: Unit filter
// ---------------------------------------------------------------------------

#[test]
fn voice_unit_filter_then_stop() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Hisser);
    spawn_owned_unit(&mut world, GridPos::new(7, 7), 0, UnitKind::Hisser);

    // "hisser stop" — should only stop hissers (2 units), not the chonk
    say_many(&mut world, &["hisser", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 2, "should target only the 2 hissers");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Selector keywords
// ---------------------------------------------------------------------------

#[test]
fn voice_army_selector_excludes_workers() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Pawdler);
    spawn_owned_unit(&mut world, GridPos::new(7, 7), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(8, 8), 0, UnitKind::Hisser);

    say_many(&mut world, &["army", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 2, "army = non-workers: chonk + hisser");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

#[test]
fn voice_workers_selector_includes_only_workers() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Pawdler);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(7, 7), 0, UnitKind::Hisser);

    say_many(&mut world, &["workers", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 1, "workers = only pawdler");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Nearby selector
// ---------------------------------------------------------------------------

#[test]
fn voice_nearby_filters_by_cursor_distance() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));

    // Near cursor (within 10 tiles of (10,10))
    spawn_owned_unit(&mut world, GridPos::new(8, 8), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(12, 12), 0, UnitKind::Hisser);
    // Far from cursor
    spawn_owned_unit(&mut world, GridPos::new(50, 50), 0, UnitKind::Chonk);

    world.resource_mut::<CursorGridPos>().pos = Some(GridPos::new(10, 10));

    say_many(&mut world, &["nearby", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(
                unit_ids.len(),
                2,
                "nearby should include 2 units within 10 tiles, not the far one"
            );
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

#[test]
fn voice_nearby_without_cursor_falls_back_to_all() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(25, 25), 0, UnitKind::Hisser);

    // No cursor position set (default is None)
    say_many(&mut world, &["nearby", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 2, "no cursor → fallback to all units");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

#[test]
fn voice_nearby_boundary_distance_10_included() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));

    // Exactly at Chebyshev distance 10 — should be included
    spawn_owned_unit(&mut world, GridPos::new(20, 10), 0, UnitKind::Chonk);
    // At distance 11 — should be excluded
    spawn_owned_unit(&mut world, GridPos::new(21, 10), 0, UnitKind::Hisser);

    world.resource_mut::<CursorGridPos>().pos = Some(GridPos::new(10, 10));

    say_many(&mut world, &["nearby", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 1, "distance 10 included, distance 11 excluded");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Enemy filtering (voice commands only affect player's units)
// ---------------------------------------------------------------------------

#[test]
fn voice_commands_exclude_enemy_units() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Chonk); // enemy

    say(&mut world, "stop");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 1, "should only include player 0's unit");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Attack command
// ---------------------------------------------------------------------------

#[test]
fn voice_attack_targets_enemy_centroid() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);

    // Two enemies at (40,40) and (50,50) → centroid (45,45)
    spawn_owned_unit(&mut world, GridPos::new(40, 40), 1, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(50, 50), 1, UnitKind::Chonk);

    say(&mut world, "attack");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::AttackMove { unit_ids, target } => {
            assert_eq!(unit_ids.len(), 1);
            assert_eq!(*target, GridPos::new(45, 45), "should target enemy centroid");
        }
        other => panic!("expected AttackMove, got: {other:?}"),
    }
}

#[test]
fn voice_attack_with_no_enemies_targets_map_center() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);

    say(&mut world, "attack");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::AttackMove { target, .. } => {
            assert_eq!(*target, GridPos::new(32, 32), "should target map center");
        }
        other => panic!("expected AttackMove, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Retreat command
// ---------------------------------------------------------------------------

#[test]
fn voice_retreat_targets_player_hq() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Hisser);
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheBox);

    say(&mut world, "retreat");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            assert_eq!(*target, GridPos::new(5, 5), "should retreat to HQ");
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

#[test]
fn voice_retreat_targets_non_catgpt_hq() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Hisser);
    // Non-catGPT HQ + a non-HQ building — should retreat to HQ, not the other building
    spawn_building(&mut world, GridPos::new(20, 20), 0, BuildingKind::CatTree);
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheGrotto);

    say(&mut world, "retreat");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            assert_eq!(*target, GridPos::new(5, 5), "should retreat to TheGrotto HQ, not CatTree");
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Move with direction
// ---------------------------------------------------------------------------

#[test]
fn voice_north_move_offsets_from_map_center() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Chonk);

    // "north move" → center(32,32) + (-15,-15) = (17,17)
    say_many(&mut world, &["north", "move"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            assert_eq!(*target, GridPos::new(17, 17));
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

#[test]
fn voice_south_move_offsets_from_map_center() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Chonk);

    say_many(&mut world, &["south", "move"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            // center(32,32) + (15,15) = (47,47)
            assert_eq!(*target, GridPos::new(47, 47));
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

#[test]
fn voice_west_move_offsets_from_map_center() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Chonk);

    // center(32,32) + (-15, +15) = (17, 47)
    say_many(&mut world, &["west", "move"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            assert_eq!(*target, GridPos::new(17, 47));
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

#[test]
fn voice_move_without_direction_goes_toward_enemy() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(50, 50), 1, UnitKind::Chonk);

    say(&mut world, "move");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            assert_eq!(*target, GridPos::new(50, 50), "move toward enemy centroid");
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Charge (attack-move)
// ---------------------------------------------------------------------------

#[test]
fn voice_charge_issues_attack_move() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(50, 50), 1, UnitKind::Chonk);

    say(&mut world, "charge");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    assert!(
        matches!(&cmds[0], GameCommand::AttackMove { .. }),
        "charge should produce AttackMove, got: {:?}",
        cmds[0]
    );
}

// ---------------------------------------------------------------------------
// Tests: Build command
// ---------------------------------------------------------------------------

#[test]
fn voice_build_tower_finds_worker_and_site() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    // Player's HQ (needed for faction inference)
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheBox);
    // Player's worker
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Pawdler);

    world.resource_mut::<CursorGridPos>().pos = Some(GridPos::new(10, 10));

    say_many(&mut world, &["tower", "build"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Build {
            building_kind,
            position,
            ..
        } => {
            assert_eq!(
                *building_kind,
                BuildingKind::LaserPointer,
                "tower → LaserPointer for catGPT"
            );
            // Should be near cursor (10,10)
            let dist = (position.x - 10).abs().max((position.y - 10).abs());
            assert!(dist <= 1, "build site should be near cursor, got {position:?}");
        }
        other => panic!("expected Build, got: {other:?}"),
    }
}

#[test]
fn voice_build_without_building_keyword_is_ignored() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheBox);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Pawdler);

    // Just "build" with no building name
    say(&mut world, "build");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert!(cmds.is_empty(), "build without building keyword should produce no command");
}

#[test]
fn voice_build_uses_faction_buildings() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    // Croak HQ → should use Croak buildings
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheGrotto);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Ponderer);

    world.resource_mut::<CursorGridPos>().pos = Some(GridPos::new(10, 10));

    say_many(&mut world, &["barracks", "build"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Build { building_kind, .. } => {
            assert_eq!(
                *building_kind,
                BuildingKind::SpawningPools,
                "barracks → SpawningPools for Croak"
            );
        }
        other => panic!("expected Build, got: {other:?}"),
    }
}

#[test]
fn voice_build_selects_nearest_worker() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_building(&mut world, GridPos::new(5, 5), 0, BuildingKind::TheBox);
    let far_worker = spawn_owned_unit(&mut world, GridPos::new(25, 25), 0, UnitKind::Pawdler);
    let near_worker = spawn_owned_unit(&mut world, GridPos::new(9, 9), 0, UnitKind::Pawdler);

    world.resource_mut::<CursorGridPos>().pos = Some(GridPos::new(10, 10));

    say_many(&mut world, &["tower", "build"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Build { builder, .. } => {
            assert_eq!(
                *builder,
                EntityId(near_worker.to_bits()),
                "should pick the nearest worker (9,9), not the far one (25,25)"
            );
            assert_ne!(*builder, EntityId(far_worker.to_bits()));
        }
        other => panic!("expected Build, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Cancel clears pending state
// ---------------------------------------------------------------------------

#[test]
fn voice_cancel_clears_pending_state() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Hisser);

    // "hisser cancel stop" — cancel should clear the hisser filter,
    // so stop targets all units
    say_many(&mut world, &["hisser", "cancel", "stop"]);
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 2, "cancel should clear filter → all units");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Multi-tick keyword accumulation
// ---------------------------------------------------------------------------

#[test]
fn voice_keywords_accumulate_across_ticks() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Hisser);

    // Tick 1: say "hisser" (sets filter)
    say(&mut world, "hisser");
    tick(&mut world, &mut schedule);
    assert!(drain_commands(&mut world).is_empty(), "filter alone produces no command");

    // Tick 2: say "stop" (executes with pending filter)
    say(&mut world, "stop");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 1, "hisser filter should persist across ticks");
        }
        other => panic!("expected Stop, got: {other:?}"),
    }
}

#[test]
fn voice_direction_persists_across_ticks() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(64, 64));
    spawn_owned_unit(&mut world, GridPos::new(30, 30), 0, UnitKind::Chonk);

    // Tick 1: say "east"
    say(&mut world, "east");
    tick(&mut world, &mut schedule);
    assert!(drain_commands(&mut world).is_empty());

    // Tick 2: say "move"
    say(&mut world, "move");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Move { target, .. } => {
            // east on 64x64: center(32,32) + (15,-15) = (47,17)
            assert_eq!(*target, GridPos::new(47, 17));
        }
        other => panic!("expected Move, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests: Pending state clears after command execution
// ---------------------------------------------------------------------------

#[test]
fn voice_pending_filter_clears_after_command() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    spawn_owned_unit(&mut world, GridPos::new(6, 6), 0, UnitKind::Hisser);

    // "hisser stop" → targets only hissers
    say_many(&mut world, &["hisser", "stop"]);
    tick(&mut world, &mut schedule);
    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => assert_eq!(unit_ids.len(), 1),
        _ => panic!("expected Stop"),
    }

    // Next "stop" without filter → targets all units
    say(&mut world, "stop");
    tick(&mut world, &mut schedule);
    let cmds = drain_commands(&mut world);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        GameCommand::Stop { unit_ids } => {
            assert_eq!(unit_ids.len(), 2, "filter should be cleared after first command");
        }
        _ => panic!("expected Stop"),
    }
}

// ---------------------------------------------------------------------------
// Tests: No units = no command
// ---------------------------------------------------------------------------

#[test]
fn voice_command_with_no_units_produces_nothing() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    // No units spawned

    say(&mut world, "stop");
    tick(&mut world, &mut schedule);

    let cmds = drain_commands(&mut world);
    assert!(cmds.is_empty(), "no units → no command");
}

// ---------------------------------------------------------------------------
// Tests: Player ID tagging
// ---------------------------------------------------------------------------

#[test]
fn voice_commands_tagged_with_player_0() {
    let (mut world, mut schedule) = make_voice_sim(GameMap::new(32, 32));
    spawn_owned_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);

    say(&mut world, "stop");
    tick(&mut world, &mut schedule);

    let raw = &world.resource::<CommandQueue>().commands;
    assert_eq!(raw.len(), 1);
    assert_eq!(
        raw[0].player_id,
        Some(0),
        "voice commands should be tagged with player_id 0"
    );
}
