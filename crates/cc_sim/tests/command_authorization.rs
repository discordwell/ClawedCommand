use bevy::prelude::*;

use cc_core::abilities::unit_abilities;
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{
    AbilitySlots, Building, BuildingKind, GridCell, MoveTarget, Owner, Position, Producer,
    ProductionQueue, StatModifiers, UnitKind,
};
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_sim::resources::{
    CommandQueue, ControlGroups, MapResource, PlayerResources, SimClock, VoiceOverride,
};
use cc_sim::systems::command_system::process_commands;

fn make_sim() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(VoiceOverride::default());
    world.insert_resource(MapResource {
        map: GameMap::new(16, 16),
    });

    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(process_commands);
    (world, schedule)
}

fn run_command_system(world: &mut World, schedule: &mut Schedule) {
    schedule.run(world);
}

fn spawn_unit(world: &mut World, grid: GridPos, player_id: u8, kind: UnitKind) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            GridCell { pos: grid },
            Owner { player_id },
            AbilitySlots::from_abilities(unit_abilities(kind)),
            StatModifiers::default(),
        ))
        .id()
}

fn spawn_producer(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            GridCell { pos: grid },
            Owner { player_id },
            Building {
                kind: BuildingKind::TheBox,
            },
            Producer,
            ProductionQueue::default(),
        ))
        .id()
}

#[test]
fn sourced_move_rejects_enemy_unit_but_unknown_issuer_keeps_legacy_behavior() {
    let (mut world, mut schedule) = make_sim();
    let enemy_unit = spawn_unit(&mut world, GridPos::new(4, 4), 1, UnitKind::Pawdler);
    let target = GridPos::new(6, 4);

    world.resource_mut::<CommandQueue>().push_for_player(
        0,
        GameCommand::Move {
            unit_ids: vec![EntityId(enemy_unit.to_bits())],
            target,
        },
    );
    run_command_system(&mut world, &mut schedule);

    assert!(
        world.get::<MoveTarget>(enemy_unit).is_none(),
        "known player 0 issuer must not move player 1 unit"
    );

    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::Move {
            unit_ids: vec![EntityId(enemy_unit.to_bits())],
            target,
        });
    run_command_system(&mut world, &mut schedule);

    assert!(
        world.get::<MoveTarget>(enemy_unit).is_some(),
        "unknown issuer should retain existing permissive behavior for now"
    );
}

#[test]
fn sourced_train_rejects_enemy_building_without_spending_enemy_resources() {
    let (mut world, mut schedule) = make_sim();
    let enemy_box = spawn_producer(&mut world, GridPos::new(5, 5), 1);

    world.resource_mut::<PlayerResources>().players[1].supply_cap = 10;
    let initial_food = world.resource::<PlayerResources>().players[1].food;
    let initial_supply = world.resource::<PlayerResources>().players[1].supply;

    world.resource_mut::<CommandQueue>().push_for_player(
        0,
        GameCommand::TrainUnit {
            building: EntityId(enemy_box.to_bits()),
            unit_kind: UnitKind::Pawdler,
        },
    );
    run_command_system(&mut world, &mut schedule);

    let resources = world.resource::<PlayerResources>();
    assert_eq!(resources.players[1].food, initial_food);
    assert_eq!(resources.players[1].supply, initial_supply);
    assert!(
        world
            .get::<ProductionQueue>(enemy_box)
            .unwrap()
            .queue
            .is_empty(),
        "known player 0 issuer must not queue production at player 1 building"
    );
}

#[test]
fn sourced_ability_rejects_enemy_caster_without_spending_enemy_gpu() {
    let (mut world, mut schedule) = make_sim();
    let enemy_worker = spawn_unit(&mut world, GridPos::new(4, 4), 1, UnitKind::Pawdler);
    let initial_gpu = world.resource::<PlayerResources>().players[1].gpu_cores;

    world.resource_mut::<CommandQueue>().push_for_player(
        0,
        GameCommand::ActivateAbility {
            unit_id: EntityId(enemy_worker.to_bits()),
            slot: 1,
            target: AbilityTarget::SelfCast,
        },
    );
    run_command_system(&mut world, &mut schedule);

    assert_eq!(
        world.resource::<PlayerResources>().players[1].gpu_cores,
        initial_gpu
    );

    let ability_slots = world.get::<AbilitySlots>(enemy_worker).unwrap();
    assert_eq!(ability_slots.slots[1].cooldown_remaining, 0);
    assert!(!ability_slots.slots[1].active);
}
