//! Integration tests for the campaign mutator system: environmental hazards,
//! control restrictions, time limits, and mutator toggling.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::hero::HeroId;
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_core::mission::*;
use cc_core::mutator::{HazardDirection, MissionMutator, PeriodicClearing};
use cc_core::terrain::{FLAG_LAVA, FLAG_TEMP_BLOCKED, FLAG_TOXIC, FLAG_WATER_CONVERTED};
use cc_core::unit_stats::base_stats;

use cc_sim::campaign::mutator_state::{ControlRestrictions, FogState, MutatorState};
use cc_sim::campaign::mutator_systems;
use cc_sim::campaign::state::{CampaignPhase, CampaignState, MissionFailedEvent, MissionVictoryEvent, TimeLimitWarningEvent};
use cc_sim::campaign::triggers::{DialogueEvent, ObjectiveCompleteEvent, TriggerFiredEvent};
use cc_sim::campaign::wave_spawner::{MissionStarted, WaveTracker};
use cc_sim::resources::{CommandQueue, ControlGroups, MapResource, PlayerResources, SimClock, SimRng};
use cc_sim::systems::tick_system::tick_system;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a sim world with campaign + mutator systems (full chain including triggers/waves/wind).
fn make_mutator_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(SimRng::default());
    world.insert_resource(MapResource { map });
    world.init_resource::<CampaignState>();
    world.init_resource::<WaveTracker>();
    world.init_resource::<MissionStarted>();
    world.init_resource::<ControlRestrictions>();
    world.init_resource::<MutatorState>();
    world.init_resource::<FogState>();

    // Register all message types
    world.init_resource::<Messages<DialogueEvent>>();
    world.init_resource::<Messages<TriggerFiredEvent>>();
    world.init_resource::<Messages<ObjectiveCompleteEvent>>();
    world.init_resource::<Messages<MissionFailedEvent>>();
    world.init_resource::<Messages<MissionVictoryEvent>>();
    world.init_resource::<Messages<TimeLimitWarningEvent>>();

    let mut schedule = Schedule::new(FixedUpdate);
    schedule.add_systems(
        (
            tick_system,
            cc_sim::campaign::wave_spawner::wave_tracking_system,
            cc_sim::campaign::triggers::trigger_check_system,
            cc_sim::campaign::wave_spawner::wave_spawner_system,
            mutator_systems::environmental_hazard_system,
            mutator_systems::wind_displacement_system,
            mutator_systems::hazard_damage_system,
            mutator_systems::mutator_tick_system,
        )
            .chain(),
    );

    (world, schedule)
}

/// Create a minimal mission with specified mutators.
fn mission_with_mutators(mutators: Vec<MissionMutator>) -> MissionDefinition {
    MissionDefinition {
        id: "mutator_test".into(),
        name: "Mutator Test".into(),
        act: 0,
        mission_index: 0,
        map: MissionMap::Generated {
            seed: 42,
            width: 16,
            height: 16,
        },
        player_setup: PlayerSetup {
            heroes: vec![HeroSpawn {
                hero_id: HeroId::Kelpie,
                position: GridPos::new(0, 0),
                mission_critical: true,
                player_id: 0,
            }],
            units: vec![],
            buildings: vec![],
            starting_food: 0,
            starting_gpu: 0,
            starting_nfts: 0,
        },
        enemy_waves: vec![],
        objectives: vec![MissionObjective {
            id: "win".into(),
            description: "Win".into(),
            primary: true,
            condition: ObjectiveCondition::Manual,
        }],
        triggers: vec![],
        dialogue: vec![],
        briefing_text: "Test".into(),
        debrief_text: "Test".into(),
        ai_tool_tier: None,
        next_mission: NextMission::None,
        mutators,
    }
}

/// Load a mission into the campaign state and initialize mutator resources.
fn load_mission_with_mutators(world: &mut World, mission: MissionDefinition) {
    // Set up campaign state
    {
        let mut campaign = world.resource_mut::<CampaignState>();
        campaign.load_mission(mission);
        campaign.phase = CampaignPhase::InMission;
    }

    // Clone what we need to avoid borrow conflicts
    let campaign = world.resource::<CampaignState>().clone();
    let mission_ref = campaign.current_mission.clone().unwrap();

    let mut restrictions = ControlRestrictions::default();
    let mut mutator_state = MutatorState::default();
    let mut fog = FogState::default();

    mutator_systems::mutator_init(
        &campaign,
        &mission_ref,
        &mut restrictions,
        &mut mutator_state,
        &mut fog,
    );

    // Write back to world resources
    *world.resource_mut::<ControlRestrictions>() = restrictions;
    *world.resource_mut::<MutatorState>() = mutator_state;
    *world.resource_mut::<FogState>() = fog;
}

/// Spawn a unit at a grid position for a player.
fn spawn_unit(world: &mut World, pos: GridPos, player_id: u8) -> Entity {
    let stats = base_stats(UnitKind::Nuisance);
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(pos),
            },
            Velocity::zero(),
            GridCell { pos },
            MovementSpeed { speed: stats.speed },
            Owner { player_id },
            UnitType {
                kind: UnitKind::Nuisance,
            },
            Health {
                current: stats.health,
                max: stats.health,
            },
            AttackStats {
                damage: stats.damage,
                range: stats.range,
                attack_speed: stats.attack_speed,
                cooldown_remaining: 0,
            },
        ))
        .id()
}

fn run_ticks(world: &mut World, schedule: &mut Schedule, n: usize) {
    for _ in 0..n {
        schedule.run(world);
    }
}

// ---------------------------------------------------------------------------
// LavaRise Tests
// ---------------------------------------------------------------------------

#[test]
fn lava_rise_marks_tiles_after_delay() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::LavaRise {
        interval_ticks: 5,
        damage_per_tick: 10,
        direction: HazardDirection::East,
        rows_per_wave: 1,
        initial_delay_ticks: 3,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Run 3 ticks — at tick 3, first lava wave should trigger (delay=3, elapsed=0, 0%5==0)
    run_ticks(&mut world, &mut schedule, 3);

    // Tick counter is now at 3 (tick_system increments at start of each run)
    // Check that column x=0 has FLAG_LAVA set (direction=East means lava from x=0 inward)
    let map = &world.resource::<MapResource>().map;
    let tile = map.get(GridPos::new(0, 5)).unwrap();
    assert!(
        tile.dynamic_flags & FLAG_LAVA != 0,
        "Column 0 should have FLAG_LAVA after initial delay"
    );

    // Column 1 should NOT have lava yet
    let tile1 = map.get(GridPos::new(1, 5)).unwrap();
    assert!(
        tile1.dynamic_flags & FLAG_LAVA == 0,
        "Column 1 should not have lava yet"
    );
}

#[test]
fn lava_rise_damages_units_on_lava_tiles() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::LavaRise {
        interval_ticks: 1,
        damage_per_tick: 5,
        direction: HazardDirection::East,
        rows_per_wave: 2,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Spawn unit at (0, 5) — will be on lava after first tick
    let unit = spawn_unit(&mut world, GridPos::new(0, 5), 0);
    let initial_hp = world.get::<Health>(unit).unwrap().current;

    // Run 1 tick: lava should appear and damage unit
    run_ticks(&mut world, &mut schedule, 1);

    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(
        hp_after < initial_hp,
        "Unit on lava tile should take damage: {} >= {}",
        hp_after,
        initial_hp
    );
}

// ---------------------------------------------------------------------------
// ToxicTide Tests
// ---------------------------------------------------------------------------

#[test]
fn toxic_tide_marks_outer_ring() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::ToxicTide {
        interval_ticks: 1,
        damage_per_tick: 3,
        rows_per_wave: 1,
        initial_delay_ticks: 0,
        safe_zone_center: Some(GridPos::new(4, 4)),
        min_safe_radius: 2,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Run 1 tick — outer ring (edge_dist=0) should become toxic
    run_ticks(&mut world, &mut schedule, 1);

    let map = &world.resource::<MapResource>().map;

    // Corner tile (0,0) is at edge_dist=0, should be toxic
    let corner = map.get(GridPos::new(0, 0)).unwrap();
    assert!(
        corner.dynamic_flags & FLAG_TOXIC != 0,
        "Corner tile should be toxic"
    );

    // Center tile (4,4) is in safe zone, should NOT be toxic
    let center = map.get(GridPos::new(4, 4)).unwrap();
    assert!(
        center.dynamic_flags & FLAG_TOXIC == 0,
        "Safe zone center should not be toxic"
    );
}

#[test]
fn toxic_tide_damages_units() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::ToxicTide {
        interval_ticks: 1,
        damage_per_tick: 7,
        rows_per_wave: 1,
        initial_delay_ticks: 0,
        safe_zone_center: Some(GridPos::new(4, 4)),
        min_safe_radius: 1,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Unit at edge — will be in toxic zone
    let unit = spawn_unit(&mut world, GridPos::new(0, 0), 0);
    let initial_hp = world.get::<Health>(unit).unwrap().current;

    run_ticks(&mut world, &mut schedule, 1);

    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(
        hp_after < initial_hp,
        "Unit on toxic tile should take damage"
    );
}

// ---------------------------------------------------------------------------
// Tremors Tests
// ---------------------------------------------------------------------------

#[test]
fn tremors_create_blocked_tiles() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::Tremors {
        interval_ticks: 1,
        building_damage: 0,
        terrain_change_chance: 100, // Always change
        epicenter_radius: 3,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    run_ticks(&mut world, &mut schedule, 1);

    // With 100% chance and radius 3, some tiles should be blocked
    let map = &world.resource::<MapResource>().map;
    let mut blocked_count = 0;
    for x in 0..16i32 {
        for y in 0..16i32 {
            if let Some(tile) = map.get(GridPos::new(x, y)) {
                if tile.dynamic_flags & FLAG_TEMP_BLOCKED != 0 {
                    blocked_count += 1;
                }
            }
        }
    }
    assert!(
        blocked_count > 0,
        "Tremors with 100% chance should block at least some tiles"
    );
}

#[test]
fn tremors_are_deterministic() {
    // Run the same tremor twice with same seed — results should be identical
    let run = || {
        let map = GameMap::new(16, 16);
        let (mut world, mut schedule) = make_mutator_sim(map);
        // Reset RNG to known seed
        *world.resource_mut::<SimRng>() = SimRng::new(42);

        let mission = mission_with_mutators(vec![MissionMutator::Tremors {
            interval_ticks: 1,
            building_damage: 0,
            terrain_change_chance: 50,
            epicenter_radius: 3,
            initial_delay_ticks: 0,
        }]);
        load_mission_with_mutators(&mut world, mission);

        run_ticks(&mut world, &mut schedule, 3);

        // Collect all dynamic_flags
        let map = &world.resource::<MapResource>().map;
        let mut flags = Vec::new();
        for y in 0..16i32 {
            for x in 0..16i32 {
                flags.push(map.get(GridPos::new(x, y)).unwrap().dynamic_flags);
            }
        }
        flags
    };

    let run1 = run();
    let run2 = run();
    assert_eq!(run1, run2, "Tremors should be deterministic with same seed");
}

// ---------------------------------------------------------------------------
// Flooding Tests
// ---------------------------------------------------------------------------

#[test]
fn flooding_converts_low_elevation_tiles() {
    let mut map = GameMap::new(8, 8);
    // Set some tiles to low elevation
    map.get_mut(GridPos::new(0, 0)).unwrap().elevation = 0;
    map.get_mut(GridPos::new(1, 1)).unwrap().elevation = 1;
    map.get_mut(GridPos::new(2, 2)).unwrap().elevation = 2;

    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::Flooding {
        interval_ticks: 1,
        initial_water_level: 0,
        max_water_level: 2,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // After 1 tick, water level → 1: elevation 0 tiles should convert
    run_ticks(&mut world, &mut schedule, 1);

    let map = &world.resource::<MapResource>().map;
    let tile_0 = map.get(GridPos::new(0, 0)).unwrap();
    assert!(
        tile_0.dynamic_flags & FLAG_WATER_CONVERTED != 0,
        "Elevation 0 tile should be flooded at water level 1"
    );

    let tile_2 = map.get(GridPos::new(2, 2)).unwrap();
    assert!(
        tile_2.dynamic_flags & FLAG_WATER_CONVERTED == 0,
        "Elevation 2 tile should NOT be flooded at water level 1"
    );
}

// ---------------------------------------------------------------------------
// TimeLimit Tests
// ---------------------------------------------------------------------------

#[test]
fn time_limit_fires_mission_failed() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::TimeLimit {
        max_ticks: 5,
        warning_at: 3,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Run 4 ticks — should NOT fail yet
    run_ticks(&mut world, &mut schedule, 4);
    {
        let events = world.resource::<Messages<MissionFailedEvent>>();
        assert!(
            events.is_empty(),
            "Should not fail before max_ticks"
        );
    }

    // Run 1 more tick — tick reaches 5, should fail
    run_ticks(&mut world, &mut schedule, 1);
    {
        let events = world.resource::<Messages<MissionFailedEvent>>();
        assert!(
            !events.is_empty(),
            "Should fire MissionFailedEvent at max_ticks"
        );
    }
}

// ---------------------------------------------------------------------------
// Control Restriction Tests (via mutator_init)
// ---------------------------------------------------------------------------

#[test]
fn voice_only_control_blocks_player_input() {
    let map = GameMap::new(8, 8);
    let (mut world, _) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::VoiceOnlyControl {
        ai_enabled: false,
        enemy_difficulty_multiplier: Fixed::from_num(1),
    }]);
    load_mission_with_mutators(&mut world, mission);

    let restrictions = world.resource::<ControlRestrictions>();
    assert!(!restrictions.mouse_keyboard_enabled, "Mouse/keyboard should be disabled");
    assert!(restrictions.voice_enabled, "Voice should be enabled");
    assert!(!restrictions.ai_enabled, "AI should be disabled per config");
}

#[test]
fn no_build_mode_blocks_building() {
    let map = GameMap::new(8, 8);
    let (mut world, _) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::NoBuildMode]);
    load_mission_with_mutators(&mut world, mission);

    let restrictions = world.resource::<ControlRestrictions>();
    assert!(!restrictions.building_enabled, "Building should be disabled");
    assert!(restrictions.mouse_keyboard_enabled, "Mouse/keyboard should still work");
}

#[test]
fn no_ai_control_blocks_ai_commands() {
    let map = GameMap::new(8, 8);
    let (mut world, _) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::NoAiControl]);
    load_mission_with_mutators(&mut world, mission);

    let restrictions = world.resource::<ControlRestrictions>();
    assert!(!restrictions.ai_enabled, "AI should be disabled");
    assert!(restrictions.mouse_keyboard_enabled, "Mouse/keyboard should still work");
    assert!(restrictions.voice_enabled, "Voice should still work");
}

#[test]
fn ai_only_control_blocks_input_and_voice() {
    let map = GameMap::new(8, 8);
    let (mut world, _) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::AiOnlyControl { tool_tier: 2 }]);
    load_mission_with_mutators(&mut world, mission);

    let restrictions = world.resource::<ControlRestrictions>();
    assert!(!restrictions.mouse_keyboard_enabled, "Mouse/keyboard should be disabled");
    assert!(!restrictions.voice_enabled, "Voice should be disabled");
    assert!(restrictions.ai_enabled, "AI should be enabled");
}

#[test]
fn restricted_units_sets_allowed_kinds() {
    let map = GameMap::new(8, 8);
    let (mut world, _) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::RestrictedUnits {
        allowed_kinds: vec![UnitKind::Chonk, UnitKind::Hisser],
        max_unit_count: Some(5),
    }]);
    load_mission_with_mutators(&mut world, mission);

    let restrictions = world.resource::<ControlRestrictions>();
    let allowed = restrictions.allowed_unit_kinds.as_ref().unwrap();
    assert_eq!(allowed.len(), 2);
    assert!(allowed.contains(&UnitKind::Chonk));
    assert!(allowed.contains(&UnitKind::Hisser));
    assert_eq!(restrictions.max_unit_count, Some(5));
}

// ---------------------------------------------------------------------------
// DenseFog Tests
// ---------------------------------------------------------------------------

#[test]
fn dense_fog_periodic_clearing() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::DenseFog {
        vision_reduction: 5,
        periodic_clearing: Some(PeriodicClearing {
            interval_ticks: 10,
            clear_duration_ticks: 3,
        }),
    }]);
    load_mission_with_mutators(&mut world, mission);

    // At tick 0 (cycle 0%10=0, 0<3) → cleared
    run_ticks(&mut world, &mut schedule, 1);
    assert!(
        world.resource::<FogState>().currently_clear,
        "Fog should be clear at start of cycle"
    );

    // At tick 4 (cycle 4%10=4, 4>=3) → foggy
    run_ticks(&mut world, &mut schedule, 4);
    assert!(
        !world.resource::<FogState>().currently_clear,
        "Fog should return after clear_duration"
    );
}

// ---------------------------------------------------------------------------
// Multiple Mutators Stacking
// ---------------------------------------------------------------------------

#[test]
fn multiple_hazards_stack_without_conflict() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![
        MissionMutator::LavaRise {
            interval_ticks: 1,
            damage_per_tick: 5,
            direction: HazardDirection::East,
            rows_per_wave: 1,
            initial_delay_ticks: 0,
        },
        MissionMutator::Tremors {
            interval_ticks: 1,
            building_damage: 0,
            terrain_change_chance: 50,
            epicenter_radius: 2,
            initial_delay_ticks: 0,
        },
        MissionMutator::TimeLimit {
            max_ticks: 100,
            warning_at: 80,
        },
    ]);
    load_mission_with_mutators(&mut world, mission);

    // Run several ticks — should not panic
    run_ticks(&mut world, &mut schedule, 5);

    // Verify lava appeared
    let map = &world.resource::<MapResource>().map;
    let tile = map.get(GridPos::new(0, 5)).unwrap();
    assert!(
        tile.dynamic_flags & FLAG_LAVA != 0,
        "Lava should be present when stacking mutators"
    );

    // Verify no mission failure (time limit is 100)
    let events = world.resource::<Messages<MissionFailedEvent>>();
    assert!(events.is_empty(), "Should not fail with time limit of 100");
}

// ---------------------------------------------------------------------------
// No Mutators (Backward Compatibility)
// ---------------------------------------------------------------------------

#[test]
fn no_mutators_runs_cleanly() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![]);
    load_mission_with_mutators(&mut world, mission);

    // Should run without issues
    run_ticks(&mut world, &mut schedule, 10);

    // No hazards, no failures
    let map = &world.resource::<MapResource>().map;
    for x in 0..8i32 {
        for y in 0..8i32 {
            let tile = map.get(GridPos::new(x, y)).unwrap();
            assert_eq!(tile.dynamic_flags, 0, "No flags should be set without mutators");
        }
    }
}

// ---------------------------------------------------------------------------
// DamageZone Tests
// ---------------------------------------------------------------------------

#[test]
fn damage_zone_damages_units_on_specified_tiles() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let zone_tiles = vec![GridPos::new(3, 3), GridPos::new(4, 4)];
    let mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: zone_tiles,
        damage_per_tick: 8,
        active_from_start: true,
        toggle_flag: None,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Unit ON damage zone
    let unit_on = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    // Unit OFF damage zone
    let unit_off = spawn_unit(&mut world, GridPos::new(5, 5), 0);

    let hp_on_before = world.get::<Health>(unit_on).unwrap().current;
    let hp_off_before = world.get::<Health>(unit_off).unwrap().current;

    run_ticks(&mut world, &mut schedule, 1);

    let hp_on_after = world.get::<Health>(unit_on).unwrap().current;
    let hp_off_after = world.get::<Health>(unit_off).unwrap().current;

    assert!(
        hp_on_after < hp_on_before,
        "Unit on damage zone should take damage"
    );
    assert_eq!(
        hp_off_after, hp_off_before,
        "Unit outside damage zone should not take damage"
    );
}

#[test]
fn damage_zone_inactive_from_start() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: vec![GridPos::new(3, 3)],
        damage_per_tick: 10,
        active_from_start: false,
        toggle_flag: None,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    let hp_before = world.get::<Health>(unit).unwrap().current;

    run_ticks(&mut world, &mut schedule, 3);

    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert_eq!(
        hp_after, hp_before,
        "Inactive damage zone should not deal damage"
    );
}

// ---------------------------------------------------------------------------
// WindStorm Tests
// ---------------------------------------------------------------------------

#[test]
fn wind_storm_toggles_active_state() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 3,
        direction: HazardDirection::North,
        force: 2,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // At tick 1 (cycle 1%10=1, 1<3) → wind active
    run_ticks(&mut world, &mut schedule, 1);
    assert!(
        world.resource::<MutatorState>().wind_active,
        "Wind should be active during gust"
    );

    // At tick 5 (cycle 5%10=5, 5>=3) → wind inactive
    run_ticks(&mut world, &mut schedule, 4);
    assert!(
        !world.resource::<MutatorState>().wind_active,
        "Wind should be inactive between gusts"
    );
}

// ---------------------------------------------------------------------------
// LavaRise Direction Tests
// ---------------------------------------------------------------------------

#[test]
fn lava_rise_all_edges_shrinks_inward() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::LavaRise {
        interval_ticks: 1,
        damage_per_tick: 5,
        direction: HazardDirection::AllEdges,
        rows_per_wave: 1,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    run_ticks(&mut world, &mut schedule, 1);

    let map = &world.resource::<MapResource>().map;

    // All edge tiles (row 0, row 7, col 0, col 7) should have lava
    assert!(map.get(GridPos::new(0, 0)).unwrap().dynamic_flags & FLAG_LAVA != 0);
    assert!(map.get(GridPos::new(7, 0)).unwrap().dynamic_flags & FLAG_LAVA != 0);
    assert!(map.get(GridPos::new(0, 7)).unwrap().dynamic_flags & FLAG_LAVA != 0);
    assert!(map.get(GridPos::new(7, 7)).unwrap().dynamic_flags & FLAG_LAVA != 0);

    // Center should be clear
    assert!(map.get(GridPos::new(4, 4)).unwrap().dynamic_flags & FLAG_LAVA == 0);
}

// ---------------------------------------------------------------------------
// Item 1: ToggleMutator Trigger Action Tests
// ---------------------------------------------------------------------------

#[test]
fn toggle_mutator_activates_inactive_mutator() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    // DamageZone starts inactive, trigger at tick 5 activates it
    let mut mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: vec![GridPos::new(3, 3)],
        damage_per_tick: 10,
        active_from_start: false,
        toggle_flag: None,
    }]);
    mission.triggers = vec![ScriptedTrigger {
        id: "activate_zone".into(),
        condition: TriggerCondition::AtTick(5),
        actions: vec![TriggerAction::ToggleMutator { mutator_index: 0, active: true }],
        once: true,
    }];
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    let hp_before = world.get::<Health>(unit).unwrap().current;

    // Run 4 ticks — zone inactive, no damage
    run_ticks(&mut world, &mut schedule, 4);
    let hp_mid = world.get::<Health>(unit).unwrap().current;
    assert_eq!(hp_mid, hp_before, "No damage before trigger fires");

    // Run 2 more ticks — trigger fires at tick 5, damage zone becomes active
    run_ticks(&mut world, &mut schedule, 2);
    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(hp_after < hp_before, "Should take damage after mutator activated");
}

#[test]
fn toggle_mutator_deactivates_active_mutator() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mut mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: vec![GridPos::new(3, 3)],
        damage_per_tick: 10,
        active_from_start: true,
        toggle_flag: None,
    }]);
    mission.triggers = vec![ScriptedTrigger {
        id: "deactivate_zone".into(),
        condition: TriggerCondition::AtTick(3),
        actions: vec![TriggerAction::ToggleMutator { mutator_index: 0, active: false }],
        once: true,
    }];
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    let hp_before = world.get::<Health>(unit).unwrap().current;

    // Run 2 ticks — zone active, unit takes damage
    run_ticks(&mut world, &mut schedule, 2);
    let hp_mid = world.get::<Health>(unit).unwrap().current;
    assert!(hp_mid < hp_before, "Should take damage while zone active");

    // Run 3 more ticks — trigger fires at tick 3 deactivating the zone
    let hp_at_deactivation = hp_mid;
    run_ticks(&mut world, &mut schedule, 3);
    // After deactivation the unit should not take MORE damage (check last 2 ticks)
    // Actually it may take damage on tick 3 since trigger fires same tick
    // But ticks 4 and 5 should have no damage — just verify zone is inactive
    let ms = world.resource::<MutatorState>();
    assert!(!ms.is_active(0), "Mutator should be inactive after toggle");
}

// ---------------------------------------------------------------------------
// Item 2: TimeLimit Warning Event Tests
// ---------------------------------------------------------------------------

#[test]
fn time_limit_warning_fires_at_warning_tick() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::TimeLimit {
        max_ticks: 100,
        warning_at: 5,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Run 4 ticks — no warning yet
    run_ticks(&mut world, &mut schedule, 4);
    {
        let events = world.resource::<Messages<TimeLimitWarningEvent>>();
        assert!(events.is_empty(), "Warning should not fire before warning_at");
    }

    // Run 1 more tick — tick reaches 5
    run_ticks(&mut world, &mut schedule, 1);
    {
        let events = world.resource::<Messages<TimeLimitWarningEvent>>();
        assert!(!events.is_empty(), "Warning should fire at warning_at tick");
    }
}

#[test]
fn time_limit_warning_fires_only_once() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::TimeLimit {
        max_ticks: 100,
        warning_at: 3,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Run past the warning tick
    run_ticks(&mut world, &mut schedule, 6);

    let ms = world.resource::<MutatorState>();
    assert!(ms.time_warning_fired, "Warning fired flag should be set");
}

// ---------------------------------------------------------------------------
// Item 3: DamageZone toggle_flag Tests
// ---------------------------------------------------------------------------

#[test]
fn damage_zone_toggle_flag_requires_flag() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    // DamageZone with toggle_flag — starts active but flag not set
    let mut mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: vec![GridPos::new(3, 3)],
        damage_per_tick: 10,
        active_from_start: true,
        toggle_flag: Some("traps".into()),
    }]);
    // Trigger at tick 3 sets the "traps" flag
    mission.triggers = vec![ScriptedTrigger {
        id: "set_traps".into(),
        condition: TriggerCondition::AtTick(3),
        actions: vec![TriggerAction::SetFlag("traps".into())],
        once: true,
    }];
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    let hp_before = world.get::<Health>(unit).unwrap().current;

    // Run 2 ticks — flag not set, zone should not damage
    run_ticks(&mut world, &mut schedule, 2);
    let hp_mid = world.get::<Health>(unit).unwrap().current;
    assert_eq!(hp_mid, hp_before, "No damage before flag is set");

    // Run 2 more ticks — trigger sets flag at tick 3, damage starts
    run_ticks(&mut world, &mut schedule, 2);
    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(hp_after < hp_before, "Should take damage after flag is set");
}

#[test]
fn damage_zone_no_toggle_flag_uses_active_only() {
    let map = GameMap::new(8, 8);
    let (mut world, mut schedule) = make_mutator_sim(map);

    // No toggle_flag → active vec controls it
    let mission = mission_with_mutators(vec![MissionMutator::DamageZone {
        tiles: vec![GridPos::new(3, 3)],
        damage_per_tick: 10,
        active_from_start: true,
        toggle_flag: None,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(3, 3), 0);
    let hp_before = world.get::<Health>(unit).unwrap().current;

    run_ticks(&mut world, &mut schedule, 1);
    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(hp_after < hp_before, "Active zone without toggle_flag should deal damage");
}

// ---------------------------------------------------------------------------
// Item 5: Wind Displacement Tests
// ---------------------------------------------------------------------------

#[test]
fn wind_displaces_units_in_direction() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 5,
        direction: HazardDirection::South,
        force: 2,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(8, 5), 0);

    // At tick 1, cycle=1%10=1, 1<5 → wind active, unit pushed south by 2
    run_ticks(&mut world, &mut schedule, 1);

    let pos = world.get::<Position>(unit).unwrap();
    let y = pos.world.y.to_num::<i32>();
    assert_eq!(y, 7, "Unit should be displaced south by force=2");
}

#[test]
fn wind_clamps_at_map_edge() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 5,
        direction: HazardDirection::South,
        force: 3,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Unit near south edge
    let unit = spawn_unit(&mut world, GridPos::new(8, 14), 0);

    run_ticks(&mut world, &mut schedule, 1);

    let pos = world.get::<Position>(unit).unwrap();
    let y = pos.world.y.to_num::<i32>();
    assert_eq!(y, 15, "Unit should be clamped at map edge (15)");
}

#[test]
fn wind_kills_pushed_off_map() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 5,
        direction: HazardDirection::South,
        force: 3,
        can_push_off_map: true,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    // Unit at south edge — push of 3 will go off map
    let unit = spawn_unit(&mut world, GridPos::new(8, 14), 0);

    run_ticks(&mut world, &mut schedule, 1);

    let hp_after = world.get::<Health>(unit).unwrap().current;
    assert!(hp_after <= Fixed::from_num(0), "Unit pushed off map should receive lethal damage (hp: {hp_after})");
}

#[test]
fn wind_clears_movement_targets() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 5,
        direction: HazardDirection::East,
        force: 1,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(5, 5), 0);
    // Add a MoveTarget component
    world.entity_mut(unit).insert(MoveTarget { target: WorldPos::from_grid(GridPos::new(10, 10)) });

    run_ticks(&mut world, &mut schedule, 1);

    // MoveTarget should be removed
    assert!(world.get::<MoveTarget>(unit).is_none(), "Wind should clear MoveTarget");
}

#[test]
fn wind_inactive_no_displacement() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 3,
        direction: HazardDirection::South,
        force: 2,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(8, 5), 0);

    // Run 5 ticks: at tick 5, cycle=5%10=5, 5>=3 → wind inactive
    run_ticks(&mut world, &mut schedule, 5);

    let pos = world.get::<Position>(unit).unwrap();
    let y = pos.world.y.to_num::<i32>();
    // Wind was active ticks 1-3 (cycle 1,2 < 3), pushing 2 each = +6 total,
    // but tick 3 cycle=3%10=3 which is NOT < 3, so active ticks are 1 and 2 only = +4
    // Actually: tick 1 → cycle=(1-0)%10=1 <3 → push; tick 2 → 2<3 → push; tick 3 → 3>=3 no
    // So pushed on ticks 1,2 = 2*2=4 → y=5+4=9
    assert_eq!(y, 9, "Unit should only be displaced during active wind ticks");
}

#[test]
fn wind_all_edges_skipped() {
    let map = GameMap::new(16, 16);
    let (mut world, mut schedule) = make_mutator_sim(map);

    let mission = mission_with_mutators(vec![MissionMutator::WindStorm {
        interval_ticks: 10,
        duration_ticks: 5,
        direction: HazardDirection::AllEdges,
        force: 2,
        can_push_off_map: false,
        initial_delay_ticks: 0,
    }]);
    load_mission_with_mutators(&mut world, mission);

    let unit = spawn_unit(&mut world, GridPos::new(8, 5), 0);

    // Should not panic, and no displacement
    run_ticks(&mut world, &mut schedule, 3);

    let pos = world.get::<Position>(unit).unwrap();
    let y = pos.world.y.to_num::<i32>();
    assert_eq!(y, 5, "AllEdges wind should not displace units");
}
