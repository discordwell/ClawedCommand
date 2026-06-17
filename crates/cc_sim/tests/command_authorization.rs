use bevy::prelude::*;

use cc_core::abilities::unit_abilities;
use cc_core::building_stats::building_stats;
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{
    AbilitySlots, BuildOrder, Building, BuildingKind, GatherState, Gathering, GridCell,
    HoldPosition, MoveTarget, Owner, Position, Producer, ProductionQueue, ResourceDeposit,
    ResourceType, StatModifiers, UnitKind,
};
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::math::Fixed;
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

// ---------------------------------------------------------------------------
// Re-tasking a builder cancels its pending build order
//
// `Build` deducts the building cost up front and attaches a `BuildOrder`. When
// the player re-tasks that builder, the order must be cancelled: the cost is
// refunded and the `BuildOrder` removed. Otherwise the spent resources leak and
// `builder_system` (which acts on any entity carrying a `BuildOrder` near the
// site) can later spawn a phantom building the player told it to abandon.
// ---------------------------------------------------------------------------

fn spawn_builder_with_order(
    world: &mut World,
    grid: GridPos,
    player_id: u8,
    kind: BuildingKind,
    build_pos: GridPos,
) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            GridCell { pos: grid },
            Owner { player_id },
            BuildOrder {
                building_kind: kind,
                position: build_pos,
            },
        ))
        .id()
}

fn assert_retask_cancels_build_order(make_cmd: impl FnOnce(&mut World, EntityId) -> GameCommand) {
    let (mut world, mut schedule) = make_sim();
    // ServerRack costs both food and gpu, so we exercise both refund paths.
    let kind = BuildingKind::ServerRack;
    let bstats = building_stats(kind);
    let builder =
        spawn_builder_with_order(&mut world, GridPos::new(4, 4), 0, kind, GridPos::new(6, 6));
    {
        let players = &mut world.resource_mut::<PlayerResources>().players;
        players[0].food = 0;
        players[0].gpu_cores = 0;
    }

    let cmd = make_cmd(&mut world, EntityId(builder.to_bits()));
    world.resource_mut::<CommandQueue>().push_for_player(0, cmd);
    run_command_system(&mut world, &mut schedule);

    assert!(
        world.get::<BuildOrder>(builder).is_none(),
        "re-tasking a builder must remove its pending BuildOrder"
    );
    let players = &world.resource::<PlayerResources>().players;
    assert_eq!(
        players[0].food, bstats.food_cost,
        "food cost must be refunded when the build order is cancelled"
    );
    assert_eq!(
        players[0].gpu_cores, bstats.gpu_cost,
        "gpu cost must be refunded when the build order is cancelled"
    );
}

#[test]
fn attack_cancels_pending_build_order() {
    assert_retask_cancels_build_order(|_w, id| GameCommand::Attack {
        unit_ids: vec![id],
        // any id other than the builder itself (self-targeting is rejected)
        target: EntityId(id.0 ^ 1),
    });
}

#[test]
fn attack_move_cancels_pending_build_order() {
    assert_retask_cancels_build_order(|_w, id| GameCommand::AttackMove {
        unit_ids: vec![id],
        target: GridPos::new(8, 8),
    });
}

#[test]
fn hold_position_cancels_pending_build_order() {
    assert_retask_cancels_build_order(|_w, id| GameCommand::HoldPosition { unit_ids: vec![id] });
}

#[test]
fn gather_cancels_pending_build_order() {
    assert_retask_cancels_build_order(|world, id| {
        let deposit = world
            .spawn((
                Position {
                    world: WorldPos::from_grid(GridPos::new(5, 5)),
                },
                ResourceDeposit {
                    resource_type: ResourceType::Food,
                    remaining: 100,
                },
            ))
            .id();
        GameCommand::GatherResource {
            unit_ids: vec![id],
            deposit: EntityId(deposit.to_bits()),
        }
    });
}

#[test]
fn hold_position_stops_active_gatherer() {
    let (mut world, mut schedule) = make_sim();
    let worker = spawn_unit(&mut world, GridPos::new(4, 4), 0, UnitKind::Pawdler);
    world.entity_mut(worker).insert(Gathering {
        deposit_entity: EntityId(0),
        carried_type: ResourceType::Food,
        carried_amount: 0,
        state: GatherState::MovingToDeposit,
        last_pos: (Fixed::from_num(4), Fixed::from_num(4)),
        stale_ticks: 0,
    });

    world.resource_mut::<CommandQueue>().push_for_player(
        0,
        GameCommand::HoldPosition {
            unit_ids: vec![EntityId(worker.to_bits())],
        },
    );
    run_command_system(&mut world, &mut schedule);

    assert!(
        world.get::<Gathering>(worker).is_none(),
        "hold position must stop an active gather loop"
    );
    assert!(
        world.get::<HoldPosition>(worker).is_some(),
        "hold position should mark the unit as holding"
    );
}
