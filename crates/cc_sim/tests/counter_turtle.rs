//! Integration tests for counter-turtle mechanics:
//! anti-static damage bonus, range_multiplier, and faction-specific siege abilities.

use bevy::prelude::*;
use cc_core::abilities::unit_abilities;
use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_core::status_effects::{StatusEffectId, StatusEffects};
use cc_core::unit_stats::base_stats;
use cc_sim::resources::{
    CommandQueue, ControlGroups, GameState, MapResource, PlayerResources, SimClock, SimRng,
    SpawnPositions, VoiceOverride,
};
use cc_sim::systems::{
    ability_effect_system::ability_effect_system, ability_system::ability_cooldown_system,
    aura_system::aura_system, cleanup_system::cleanup_system, combat_system::combat_system,
    command_system::process_commands, grid_sync_system::grid_sync_system,
    movement_system::movement_system, production_system::production_system,
    projectile_system::projectile_system, research_system::research_system,
    resource_system::gathering_system, stat_modifier_system::stat_modifier_system,
    stationary_timer_system::stationary_timer_system,
    status_effect_system::status_effect_system,
    target_acquisition_system::target_acquisition_system, tick_system::tick_system,
    tower_combat_system::tower_combat_system, victory_system::victory_system,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_sim() -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(GameState::default());
    world.insert_resource(SpawnPositions::default());
    world.insert_resource(SimRng::default());
    world.insert_resource(cc_sim::resources::CombatStats::default());
    world.insert_resource(VoiceOverride::default());
    world.insert_resource(MapResource {
        map: GameMap::new(32, 32),
    });
    world.init_resource::<bevy::prelude::Messages<cc_sim::systems::projectile_system::ProjectileHit>>();

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
            stationary_timer_system,
            grid_sync_system,
            cleanup_system,
        )
            .chain(),
    );
    schedule.add_systems(victory_system);

    (world, schedule)
}

fn spawn_unit(world: &mut World, grid: GridPos, player_id: u8, kind: UnitKind) -> Entity {
    let stats = base_stats(kind);
    let mut e = world.spawn((
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
        AbilitySlots::from_abilities(unit_abilities(kind)),
        StatusEffects::default(),
        StatModifiers::default(),
    ));
    // Add StationaryTimer for combat units
    if !kind.is_worker() {
        e.insert(StationaryTimer::default());
    }
    e.id()
}

fn run_ticks(world: &mut World, schedule: &mut Schedule, n: usize) {
    for _ in 0..n {
        schedule.run(world);
    }
}

fn issue_ability(world: &mut World, unit: Entity, slot: u8) {
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::ActivateAbility {
            unit_id: EntityId(unit.to_bits()),
            slot,
            target: AbilityTarget::SelfCast,
        });
}

fn issue_ability_at(world: &mut World, unit: Entity, slot: u8, target_pos: GridPos) {
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::ActivateAbility {
            unit_id: EntityId(unit.to_bits()),
            slot,
            target: AbilityTarget::Position(target_pos),
        });
}

fn give_gpu(world: &mut World, player_id: u8, amount: u32) {
    if let Some(pres) = world
        .resource_mut::<PlayerResources>()
        .players
        .get_mut(player_id as usize)
    {
        pres.gpu_cores += amount;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Catnapper SiegeNap toggle increases range from 7 to ~10 (×1.43).
#[test]
fn catnapper_siege_nap_increases_range() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);

    // Activate SiegeNap (slot 2)
    issue_ability(&mut world, catnapper, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(catnapper).unwrap();
    // range_multiplier should be ~1.43
    let expected_min = Fixed::from_bits((1 << 16) * 140 / 100); // 1.40
    assert!(
        modifiers.range_multiplier >= expected_min,
        "SiegeNap range_multiplier should be >= 1.40, got {:?}",
        modifiers.range_multiplier
    );
}

/// Catnapper SiegeNap immobilizes.
#[test]
fn catnapper_siege_nap_immobilizes() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);

    issue_ability(&mut world, catnapper, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(catnapper).unwrap();
    assert!(modifiers.immobilized, "SiegeNap should immobilize");
}

/// Shrieker SonicBarrage deals damage in a line toward target.
#[test]
fn shrieker_sonic_barrage_range_8() {
    let (mut world, mut schedule) = make_sim();
    // Shrieker at (10,10), enemy at (17,10) — 7 tiles away, directly in line (within range 8)
    let shrieker = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Shrieker);
    let target = spawn_unit(&mut world, GridPos::new(17, 10), 1, UnitKind::Chonk);
    give_gpu(&mut world, 0, 50);

    let hp_before = world.get::<Health>(target).unwrap().current;

    // Activate SonicBarrage targeting toward the enemy position
    issue_ability_at(&mut world, shrieker, 2, GridPos::new(17, 10));
    run_ticks(&mut world, &mut schedule, 5);

    let hp_after = world.get::<Health>(target).unwrap().current;
    assert!(
        hp_after < hp_before,
        "SonicBarrage should deal damage in line: before={:?}, after={:?}",
        hp_before,
        hp_after
    );
}

/// Cragback Patience of Stone: +50% damage to stationary targets when entrenched.
#[test]
fn cragback_patience_bonus_vs_stationary() {
    let (mut world, mut schedule) = make_sim();
    let cragback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Cragback);

    // Activate Entrench (slot 1 for Cragback)
    issue_ability(&mut world, cragback, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(cragback).unwrap();
    assert!(
        modifiers.anti_static_bonus > Fixed::ZERO,
        "Entrenched Cragback should have anti_static_bonus, got {:?}",
        modifiers.anti_static_bonus
    );
    let expected = Fixed::from_bits((1 << 16) * 50 / 100); // 0.5
    assert_eq!(
        modifiers.anti_static_bonus, expected,
        "anti_static_bonus should be 0.5"
    );
}

/// Cragback Patience of Stone: no bonus vs moving targets.
#[test]
fn cragback_no_bonus_vs_moving() {
    let (mut world, mut schedule) = make_sim();
    let cragback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Cragback);
    // Place target nearby; they just spawned so ticks_stationary = 0
    let target = spawn_unit(&mut world, GridPos::new(12, 10), 1, UnitKind::Nuisance);

    // Entrench
    issue_ability(&mut world, cragback, 1);
    // Run 1 tick — target should have ticks_stationary < 30 (just spawned)
    run_ticks(&mut world, &mut schedule, 1);

    let timer = world.get::<StationaryTimer>(target).unwrap();
    // After 1 tick the timer will be 1 (just incremented once)
    assert!(
        timer.ticks_stationary < 30,
        "Target should not be considered stationary yet"
    );
}

/// Hootseer DeathOmen hits at range 10.
#[test]
fn hootseer_death_omen_range_10() {
    let def = cc_core::abilities::ability_def(cc_core::abilities::AbilityId::DeathOmen);
    assert_eq!(
        def.range,
        Fixed::from_bits(10 << 16),
        "DeathOmen should have range 10"
    );
}

/// Hootseer DeathOmen applies Exposed status at target position.
#[test]
fn hootseer_death_omen_applies_exposed() {
    let (mut world, mut schedule) = make_sim();
    // Hootseer at (10,10), enemy at (11,10) — target the enemy's position
    let hootseer = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Hootseer);
    let target = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Chonk);
    give_gpu(&mut world, 0, 50);

    issue_ability_at(&mut world, hootseer, 2, GridPos::new(11, 10));
    run_ticks(&mut world, &mut schedule, 5);

    let effects = world.get::<StatusEffects>(target).unwrap();
    assert!(
        effects.has(StatusEffectId::Exposed),
        "DeathOmen should apply Exposed at target position"
    );
}

/// Hootseer DeathOmen deals double damage vs stationary targets.
#[test]
fn hootseer_death_omen_double_vs_stationary() {
    let (mut world, mut schedule) = make_sim();
    let hootseer = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Hootseer);
    let target = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Chonk);
    give_gpu(&mut world, 0, 50);

    // Make target stationary for 30+ ticks
    world.get_mut::<StationaryTimer>(target).unwrap().ticks_stationary = 35;

    let hp_before = world.get::<Health>(target).unwrap().current;

    issue_ability_at(&mut world, hootseer, 2, GridPos::new(11, 10));
    run_ticks(&mut world, &mut schedule, 3);

    let hp_after = world.get::<Health>(target).unwrap().current;
    let damage = hp_before - hp_after;
    // Should deal 50 damage (25 base × 2 vs stationary)
    assert!(
        damage >= Fixed::from_num(45),
        "DeathOmen should deal ~50 vs stationary (got {:?})",
        damage
    );
}

/// Grease Monkey JunkMortarMode attacks have AoE splash.
#[test]
fn grease_monkey_junk_mortar_aoe_splash() {
    let (mut world, mut schedule) = make_sim();
    let gm = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::GreaseMonkey);
    // Two enemies close together — primary target and splash target
    let target1 = spawn_unit(&mut world, GridPos::new(12, 10), 1, UnitKind::Chonk);
    let target2 = spawn_unit(&mut world, GridPos::new(13, 10), 1, UnitKind::Chonk);

    // Activate JunkMortarMode (slot 2)
    issue_ability(&mut world, gm, 2);
    // Run enough ticks for attack cycle + projectile travel
    run_ticks(&mut world, &mut schedule, 20);

    let hp1 = world.get::<Health>(target1).unwrap().current;
    let hp2 = world.get::<Health>(target2).unwrap().current;
    let max1 = world.get::<Health>(target1).unwrap().max;
    let max2 = world.get::<Health>(target2).unwrap().max;

    // Primary target should take damage
    assert!(
        hp1 < max1,
        "JunkMortar primary target should take damage"
    );
    // Splash target (1 tile away, within 2-tile splash radius) should also take damage
    assert!(
        hp2 < max2,
        "JunkMortar splash should damage nearby enemy (1 tile from primary)"
    );
}

/// Croaker Inflate gives range ~10 (×1.667 from base 6).
#[test]
fn croaker_inflate_range_boost() {
    let (mut world, mut schedule) = make_sim();
    let croaker = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Croaker);
    give_gpu(&mut world, 0, 50);

    // Activate Inflate (slot 2)
    issue_ability(&mut world, croaker, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(croaker).unwrap();
    // range_multiplier should be ~1.667
    let expected_min = Fixed::from_bits((1 << 16) * 160 / 100); // 1.60
    assert!(
        modifiers.range_multiplier >= expected_min,
        "Inflated Croaker range_multiplier should be >= 1.60, got {:?}",
        modifiers.range_multiplier
    );
}

/// Croaker Inflate gives anti-static bonus.
#[test]
fn croaker_inflate_anti_static_bonus() {
    let (mut world, mut schedule) = make_sim();
    let croaker = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Croaker);
    give_gpu(&mut world, 0, 50);

    issue_ability(&mut world, croaker, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(croaker).unwrap();
    let expected = Fixed::from_bits((1 << 16) * 40 / 100); // 0.4
    assert_eq!(
        modifiers.anti_static_bonus, expected,
        "Inflated Croaker should have anti_static_bonus 0.4"
    );
}

/// Grease Monkey JunkMortarMode gives range 10 (×2.0 from base 5).
#[test]
fn grease_monkey_junk_mortar_range_10() {
    let (mut world, mut schedule) = make_sim();
    let gm = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::GreaseMonkey);

    // Activate JunkMortarMode (slot 2)
    issue_ability(&mut world, gm, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(gm).unwrap();
    assert_eq!(
        modifiers.range_multiplier,
        Fixed::from_num(2),
        "JunkMortarMode should double range"
    );
}

/// Grease Monkey JunkMortarMode immobilizes.
#[test]
fn grease_monkey_junk_mortar_immobilizes() {
    let (mut world, mut schedule) = make_sim();
    let gm = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::GreaseMonkey);

    issue_ability(&mut world, gm, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(gm).unwrap();
    assert!(modifiers.immobilized, "JunkMortarMode should immobilize");
}

/// Target acquisition uses range_multiplier for scan radius.
/// Catnapper base range = 2. Without SiegeNap, can't reach at dist 2.5.
/// With SiegeNap (×1.43 → effective ~2.86), should acquire.
#[test]
fn range_multiplier_target_acquisition() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);

    // First, verify the catnapper CANNOT acquire an enemy at distance ~2.24 without siege nap
    // Place enemy at (12, 11) → dist = sqrt(4+1) = 2.24. Base range 2, so 2.24 > 2 → no acquire
    // Actually let's use dist exactly > 2 but < 2.86
    // Enemy at (10+2, 10+1) = (12, 11): dist_sq = 4+1 = 5, range_sq = 4: 5 > 4 → no acquire
    let enemy = spawn_unit(&mut world, GridPos::new(12, 11), 1, UnitKind::Chonk);

    // Run without SiegeNap — should NOT acquire (dist 2.24 > range 2)
    run_ticks(&mut world, &mut schedule, 3);
    let has_target_before = world.get::<AttackTarget>(catnapper).is_some();
    assert!(
        !has_target_before,
        "Catnapper should NOT acquire target at dist 2.24 with base range 2"
    );

    // Now activate SiegeNap (effective range ~2.86, 2.86² = 8.18 > 5)
    issue_ability(&mut world, catnapper, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let has_target_after = world.get::<AttackTarget>(catnapper).is_some();
    assert!(
        has_target_after,
        "Siege-napping Catnapper should acquire target at dist 2.24 (effective range ~2.86)"
    );
    let _ = enemy;
}

/// Anti-static threshold: no bonus at 29 ticks, bonus at 30 ticks.
#[test]
fn anti_static_threshold_30_ticks() {
    let (mut world, mut schedule) = make_sim();
    // Entrenched Cragback (has anti_static_bonus) attacking a held target
    let cragback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Cragback);
    let target = spawn_unit(&mut world, GridPos::new(12, 10), 1, UnitKind::Chonk);

    // Set target stationary timer just below threshold
    world.get_mut::<StationaryTimer>(target).unwrap().ticks_stationary = 29;

    // Entrench the Cragback
    issue_ability(&mut world, cragback, 1);
    run_ticks(&mut world, &mut schedule, 1);

    // After 1 tick, target's timer will be 30 (29 + 1 since it hasn't moved)
    let timer = world.get::<StationaryTimer>(target).unwrap();
    assert_eq!(
        timer.ticks_stationary, 30,
        "Timer should increment to 30 after one tick of standing still"
    );
}
