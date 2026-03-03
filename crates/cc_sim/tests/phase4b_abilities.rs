//! Phase 4B: Integration tests for the 10 abilities.

use bevy::prelude::*;
use cc_core::abilities::unit_abilities;
use cc_core::commands::AbilityTarget;
use cc_core::unit_stats::base_stats;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_core::status_effects::{StatusEffectId, StatusEffects};
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
            AbilitySlots::from_abilities(unit_abilities(kind)),
            StatusEffects::default(),
            StatModifiers::default(),
        ))
        .id()
}

fn run_ticks(world: &mut World, schedule: &mut Schedule, n: usize) {
    for _ in 0..n {
        schedule.run(world);
    }
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

fn issue_ability(world: &mut World, unit: Entity, slot: u8) {
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::ActivateAbility {
            unit_id: EntityId(unit.to_bits()),
            slot,
            target: AbilityTarget::SelfCast,
        });
}

fn spawn_deposit(world: &mut World, grid: GridPos) -> Entity {
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            ResourceDeposit {
                resource_type: ResourceType::Food,
                remaining: 500,
            },
        ))
        .id()
}

fn spawn_the_box(world: &mut World, grid: GridPos, player_id: u8) -> Entity {
    let bstats = cc_core::building_stats::building_stats(BuildingKind::TheBox);
    if let Some(pres) = world
        .resource_mut::<PlayerResources>()
        .players
        .get_mut(player_id as usize)
    {
        pres.supply_cap += bstats.supply_provided;
    }
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building {
                kind: BuildingKind::TheBox,
            },
            Health {
                current: bstats.health,
                max: bstats.health,
            },
            Producer,
            ProductionQueue::default(),
        ))
        .id()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_annoyance_stacks_on_nuisance_attack() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);
    issue_attack(&mut world, &[nuisance], target);
    run_ticks(&mut world, &mut schedule, 15);
    let effects = world.get::<StatusEffects>(target).unwrap();
    let annoyed = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Annoyed);
    assert!(
        annoyed.is_some(),
        "Target should have Annoyed status effect after Nuisance attack"
    );
}

#[test]
fn test_annoyance_stacks_cap_at_five() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);
    world.get_mut::<Health>(target).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(target).unwrap().max = Fixed::from_num(5000);
    issue_attack(&mut world, &[nuisance], target);
    run_ticks(&mut world, &mut schedule, 150);
    let effects = world.get::<StatusEffects>(target).unwrap();
    let annoyed = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Annoyed);
    let tilted = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Tilted);
    // At 5 stacks, Annoyed converts to Tilted CC — so either capped Annoyed or Tilted should exist
    assert!(
        annoyed.is_some() || tilted.is_some(),
        "Target should have Annoyed stacks or Tilted (converted from 5 stacks)"
    );
    if let Some(annoyed) = annoyed {
        assert!(
            annoyed.stacks <= 5,
            "Annoyed stacks should cap at 5, got {}",
            annoyed.stacks
        );
    }
}

#[test]
fn test_corrosive_spit_on_hisser_attack() {
    let (mut world, mut schedule) = make_sim();
    let hisser = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Hisser);
    let target = spawn_unit(&mut world, GridPos::new(8, 5), 1, UnitKind::Chonk);
    issue_attack(&mut world, &[hisser], target);
    run_ticks(&mut world, &mut schedule, 30);
    let effects = world.get::<StatusEffects>(target).unwrap();
    let corroded = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Corroded);
    assert!(
        corroded.is_some(),
        "Target should have Corroded after Hisser attack"
    );
}

#[test]
fn test_zoomies_grants_speed_and_invulnerability() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;
    issue_ability(&mut world, nuisance, 2);
    run_ticks(&mut world, &mut schedule, 2);
    let mods = world.get::<StatModifiers>(nuisance).unwrap();
    assert!(
        mods.invulnerable,
        "Nuisance with Zoomies should be invulnerable"
    );
    assert!(
        mods.speed_multiplier > Fixed::ONE,
        "Nuisance with Zoomies should have speed boost"
    );
}

#[test]
fn test_zoomies_prevents_attack() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Nuisance);
    let target = spawn_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;
    let initial_hp = world.get::<Health>(target).unwrap().current;
    issue_ability(&mut world, nuisance, 2);
    run_ticks(&mut world, &mut schedule, 1);
    issue_attack(&mut world, &[nuisance], target);
    run_ticks(&mut world, &mut schedule, 20);
    let hp_after = world.get::<Health>(target).unwrap().current;
    assert_eq!(
        hp_after, initial_hp,
        "Nuisance with Zoomies should not deal damage (cannot_attack)"
    );
}

#[test]
fn test_loaf_mode_immobilizes_and_reduces_damage() {
    let (mut world, mut schedule) = make_sim();
    let chonk = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Chonk);
    issue_ability(&mut world, chonk, 1);
    run_ticks(&mut world, &mut schedule, 2);
    let mods = world.get::<StatModifiers>(chonk).unwrap();
    assert!(mods.immobilized, "LoafMode should set immobilized flag");
    assert!(
        mods.damage_reduction < Fixed::ONE,
        "LoafMode should reduce damage_reduction below 1.0 (got {})",
        mods.damage_reduction
    );
}

#[test]
fn test_harmonic_resonance_buffs_allies() {
    let (mut world, mut schedule) = make_sim();
    let yowler = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Yowler);
    let ally = spawn_unit(&mut world, GridPos::new(6, 5), 0, UnitKind::Nuisance);
    issue_ability(&mut world, yowler, 0);
    run_ticks(&mut world, &mut schedule, 3);
    let effects = world.get::<StatusEffects>(ally).unwrap();
    let buff = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::HarmonicBuff);
    assert!(
        buff.is_some(),
        "Ally near Yowler with HarmonicResonance should have HarmonicBuff"
    );
}

#[test]
fn test_lullaby_debuffs_enemies() {
    let (mut world, mut schedule) = make_sim();
    let yowler = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Yowler);
    let enemy = spawn_unit(&mut world, GridPos::new(7, 5), 1, UnitKind::Nuisance);
    issue_ability(&mut world, yowler, 2);
    run_ticks(&mut world, &mut schedule, 3);
    let effects = world.get::<StatusEffects>(enemy).unwrap();
    let debuff = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::LullabyDebuff);
    assert!(
        debuff.is_some(),
        "Enemy near Yowler with Lullaby should have LullabyDebuff"
    );
}

#[test]
fn test_lullaby_harmonic_mutual_exclusivity() {
    let (mut world, mut schedule) = make_sim();
    let yowler = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Yowler);
    issue_ability(&mut world, yowler, 0);
    run_ticks(&mut world, &mut schedule, 2);
    let slots = world.get::<AbilitySlots>(yowler).unwrap();
    assert!(slots.slots[0].active, "HarmonicResonance should be active");
    issue_ability(&mut world, yowler, 2);
    run_ticks(&mut world, &mut schedule, 2);
    let slots = world.get::<AbilitySlots>(yowler).unwrap();
    assert!(slots.slots[2].active, "Lullaby should now be active");
    assert!(
        !slots.slots[0].active,
        "HarmonicResonance should be deactivated by mutual exclusivity"
    );
}

#[test]
fn test_dissonant_screech_aoe_cc() {
    let (mut world, mut schedule) = make_sim();
    let yowler = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Yowler);
    let enemy1 = spawn_unit(&mut world, GridPos::new(7, 5), 1, UnitKind::Nuisance);
    let enemy2 = spawn_unit(&mut world, GridPos::new(5, 7), 1, UnitKind::Nuisance);
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;
    issue_ability(&mut world, yowler, 1);
    run_ticks(&mut world, &mut schedule, 2);
    let e1_effects = world.get::<StatusEffects>(enemy1).unwrap();
    let e2_effects = world.get::<StatusEffects>(enemy2).unwrap();
    let e1_cc = e1_effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Disoriented);
    let e2_cc = e2_effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::Disoriented);
    assert!(
        e1_cc.is_some(),
        "Enemy1 within range should have Disoriented CC"
    );
    assert!(
        e2_cc.is_some(),
        "Enemy2 within range should have Disoriented CC"
    );
}

#[test]
fn test_spite_carry_boosts_gather_speed() {
    let (mut world, mut schedule) = make_sim();
    let _box = spawn_the_box(&mut world, GridPos::new(10, 10), 0);
    let deposit = spawn_deposit(&mut world, GridPos::new(12, 10));
    let pawdler = spawn_unit(&mut world, GridPos::new(11, 10), 0, UnitKind::Pawdler);
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::GatherResource {
            unit_ids: vec![EntityId(pawdler.to_bits())],
            deposit: EntityId(deposit.to_bits()),
        });
    run_ticks(&mut world, &mut schedule, 5);
    issue_ability(&mut world, pawdler, 1);
    run_ticks(&mut world, &mut schedule, 2);
    let mods = world.get::<StatModifiers>(pawdler).unwrap();
    assert!(
        mods.gather_speed_multiplier > Fixed::ONE,
        "SpiteCarry should boost gather speed multiplier"
    );
}

#[test]
fn test_revulsion_pushes_enemies() {
    let (mut world, mut schedule) = make_sim();
    let pawdler = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Pawdler);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 100;
    let enemy_pos_before = world.get::<Position>(enemy).unwrap().world;
    issue_ability(&mut world, pawdler, 2);
    run_ticks(&mut world, &mut schedule, 2);
    let enemy_pos_after = world.get::<Position>(enemy).unwrap().world;
    let dx = enemy_pos_after.x - enemy_pos_before.x;
    assert!(
        dx > Fixed::ZERO,
        "Enemy should be pushed away (positive x direction), got dx={dx}"
    );
}

#[test]
fn test_dream_siege_ramps_damage() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Catnapper);
    // Add DreamSiegeTimer manually since spawn_unit doesn't include it
    world
        .entity_mut(catnapper)
        .insert(DreamSiegeTimer::default());
    let target = spawn_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);
    world.get_mut::<Health>(target).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(target).unwrap().max = Fixed::from_num(5000);
    issue_attack(&mut world, &[catnapper], target);
    run_ticks(&mut world, &mut schedule, 35);
    let hp_after_first = world.get::<Health>(target).unwrap().current;
    let first_damage = Fixed::from_num(5000) - hp_after_first;
    run_ticks(&mut world, &mut schedule, 200);
    let hp_after_many = world.get::<Health>(target).unwrap().current;
    let total_damage = Fixed::from_num(5000) - hp_after_many;
    assert!(
        total_damage > first_damage * Fixed::from_num(3),
        "DreamSiege should ramp damage over time. first={first_damage}, total={total_damage}"
    );
}

#[test]
fn test_dream_siege_resets_on_target_change() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Catnapper);
    world
        .entity_mut(catnapper)
        .insert(DreamSiegeTimer::default());
    // Give Catnapper high HP so T2 damage-reset doesn't interfere
    world.get_mut::<Health>(catnapper).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(catnapper).unwrap().max = Fixed::from_num(5000);
    let target1 = spawn_unit(&mut world, GridPos::new(6, 5), 1, UnitKind::Chonk);
    world.get_mut::<Health>(target1).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(target1).unwrap().max = Fixed::from_num(5000);
    // Remove target's ability to fight back so Catnapper doesn't take damage
    world.get_mut::<AttackStats>(target1).unwrap().damage = Fixed::ZERO;
    let target2 = spawn_unit(&mut world, GridPos::new(6, 6), 1, UnitKind::Chonk);
    world.get_mut::<Health>(target2).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(target2).unwrap().max = Fixed::from_num(5000);
    world.get_mut::<AttackStats>(target2).unwrap().damage = Fixed::ZERO;
    issue_attack(&mut world, &[catnapper], target1);
    run_ticks(&mut world, &mut schedule, 100);
    let timer = world.get::<DreamSiegeTimer>(catnapper).unwrap();
    assert!(
        timer.ticks_on_target > 0,
        "DreamSiegeTimer should have ticks after attacking target1"
    );
    issue_attack(&mut world, &[catnapper], target2);
    run_ticks(&mut world, &mut schedule, 35);
    let timer = world.get::<DreamSiegeTimer>(catnapper).unwrap();
    assert!(
        timer.ticks_on_target < 10,
        "DreamSiegeTimer should reset when switching targets, got {}",
        timer.ticks_on_target
    );
}

#[test]
fn test_aura_no_effect_out_of_range() {
    let (mut world, mut schedule) = make_sim();
    let yowler = spawn_unit(&mut world, GridPos::new(5, 5), 0, UnitKind::Yowler);
    let ally_far = spawn_unit(&mut world, GridPos::new(15, 15), 0, UnitKind::Nuisance);
    issue_ability(&mut world, yowler, 0);
    run_ticks(&mut world, &mut schedule, 3);
    let effects = world.get::<StatusEffects>(ally_far).unwrap();
    let buff = effects
        .effects
        .iter()
        .find(|e| e.effect == StatusEffectId::HarmonicBuff);
    assert!(
        buff.is_none(),
        "Ally far from Yowler should NOT have HarmonicBuff"
    );
}
