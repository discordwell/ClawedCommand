//! Phase 4C: Integration tests for 10 abilities + 2 TDL fixes.

use bevy::prelude::*;
use cc_core::abilities::unit_abilities;
use cc_core::commands::AbilityTarget;
use cc_core::commands::{EntityId, GameCommand};
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
    let mut entity_cmds = world.spawn((
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

    // Insert passive components that production_system would add
    if kind == UnitKind::Chonk {
        entity_cmds.insert((
            Aura {
                aura_type: AuraType::GravitationalChonk,
                radius: Fixed::from_bits(3 << 16),
                active: true,
            },
            NineLivesTracker::default(),
        ));
    }
    if kind == UnitKind::Catnapper {
        entity_cmds.insert(DreamSiegeTimer::default());
    }

    entity_cmds.id()
}

fn run_ticks(world: &mut World, schedule: &mut Schedule, n: usize) {
    for _ in 0..n {
        schedule.run(world);
    }
}

fn issue_attack(world: &mut World, attackers: &[Entity], target: Entity) {
    let ids = attackers.iter().map(|e| EntityId(e.to_bits())).collect();
    world.resource_mut::<CommandQueue>().push(GameCommand::Attack {
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

fn spawn_building(
    world: &mut World,
    grid: GridPos,
    player_id: u8,
    kind: BuildingKind,
) -> Entity {
    let bstats = cc_core::building_stats::building_stats(kind);
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(grid),
            },
            Velocity::zero(),
            GridCell { pos: grid },
            Owner { player_id },
            Building { kind },
            Health {
                current: bstats.health,
                max: bstats.health,
            },
        ))
        .id()
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
// TDL Fix Tests
// ---------------------------------------------------------------------------

/// T1: 5 Annoyed stacks convert to Tilted CC
#[test]
fn test_tilted_cc_triggers_at_five_annoyed_stacks() {
    let (mut world, mut schedule) = make_sim();
    let target = spawn_unit(&mut world, GridPos::new(10, 10), 1, UnitKind::Chonk);
    // Give target massive HP so it doesn't die
    world.get_mut::<Health>(target).unwrap().current = Fixed::from_num(5000);
    world.get_mut::<Health>(target).unwrap().max = Fixed::from_num(5000);

    // Directly inject 5 Annoyed stacks
    {
        let mut effects = world.get_mut::<StatusEffects>(target).unwrap();
        effects.effects.push(cc_core::status_effects::StatusInstance {
            effect: StatusEffectId::Annoyed,
            remaining_ticks: 100,
            stacks: 5,
            source: EntityId(0),
        });
    }

    // Run a tick to let status_effect_system convert Annoyed → Tilted
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(target).unwrap();
    let has_tilted = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::Tilted && e.remaining_ticks > 0);
    let has_annoyed = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::Annoyed);
    assert!(has_tilted, "Should have Tilted CC after 5 Annoyed stacks");
    assert!(!has_annoyed, "Annoyed stacks should be removed after Tilted conversion");
}

/// T2: DreamSiege timer resets when Catnapper takes damage
#[test]
fn test_dream_siege_resets_on_catnapper_taking_damage() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);

    // Set up DreamSiegeTimer as if it had been attacking a target
    {
        let hp = world.get::<Health>(catnapper).unwrap().current;
        let mut timer = world.get_mut::<DreamSiegeTimer>(catnapper).unwrap();
        timer.ticks_on_target = 50;
        timer.current_target = Some(EntityId(999));
        timer.last_hp = hp;
    }

    // Run a tick to set last_hp
    run_ticks(&mut world, &mut schedule, 1);

    // Simulate damage by reducing HP
    let hp_before = world.get::<Health>(catnapper).unwrap().current;
    world.get_mut::<Health>(catnapper).unwrap().current = hp_before - Fixed::from_num(5);

    // Run another tick — ability_effect_system should detect HP decrease
    run_ticks(&mut world, &mut schedule, 1);

    let timer = world.get::<DreamSiegeTimer>(catnapper).unwrap();
    assert_eq!(timer.ticks_on_target, 0, "DreamSiege timer should reset on damage");
    // current_target is preserved — only ticks reset on damage.
    // Target changes are handled by combat_system when attacking a different entity.
}

// ---------------------------------------------------------------------------
// Ability Tests
// ---------------------------------------------------------------------------

/// GravitationalChonk: enemies within 3 tiles get pulled toward Chonk
#[test]
fn test_gravitational_chonk_pulls_enemies() {
    let (mut world, mut schedule) = make_sim();
    let _chonk = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Chonk);
    let enemy = spawn_unit(&mut world, GridPos::new(12, 10), 1, UnitKind::Nuisance);

    let initial_x = world.get::<Position>(enemy).unwrap().world.x;

    // Run several ticks for aura to pull
    run_ticks(&mut world, &mut schedule, 10);

    let final_x = world.get::<Position>(enemy).unwrap().world.x;
    assert!(
        final_x < initial_x,
        "Enemy should be pulled toward Chonk (x decreased): initial={}, final={}",
        initial_x,
        final_x,
    );
}

/// GravitationalChonk: allies should NOT be pulled
#[test]
fn test_gravitational_chonk_ignores_allies() {
    let (mut world, mut schedule) = make_sim();
    let _chonk = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Chonk);
    let ally = spawn_unit(&mut world, GridPos::new(12, 10), 0, UnitKind::Nuisance);

    let initial_pos = world.get::<Position>(ally).unwrap().world;

    run_ticks(&mut world, &mut schedule, 10);

    let final_pos = world.get::<Position>(ally).unwrap().world;
    assert_eq!(
        initial_pos.x, final_pos.x,
        "Ally X position should not change from gravitational pull"
    );
    assert_eq!(
        initial_pos.y, final_pos.y,
        "Ally Y position should not change from gravitational pull"
    );
}

/// NineLives: Chonk revives at 30% HP on lethal damage (with GPU)
#[test]
fn test_nine_lives_revives_chonk() {
    let (mut world, mut schedule) = make_sim();
    let chonk = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Chonk);
    // Player 0 starts with 50 GPU by default — enough for NineLives (25 GPU cost)

    let gpu_before = world.resource::<PlayerResources>().players[0].gpu_cores;

    // Set HP to 0 directly to trigger NineLives in cleanup_system
    world.get_mut::<Health>(chonk).unwrap().current = Fixed::ZERO;

    run_ticks(&mut world, &mut schedule, 1);

    // Chonk should NOT be dead (NineLives triggered)
    let is_dead = world.get::<Dead>(chonk).is_some();
    assert!(!is_dead, "Chonk should survive via NineLives");

    let hp = world.get::<Health>(chonk).unwrap().current;
    assert!(
        hp > Fixed::ZERO,
        "Chonk should have HP after NineLives revive"
    );

    // Check GPU was deducted
    let gpu_after = world.resource::<PlayerResources>().players[0].gpu_cores;
    assert_eq!(
        gpu_after,
        gpu_before - 25,
        "GPU should be deducted by 25 for NineLives"
    );
}

/// NineLives: no GPU means no revive — Chonk dies
#[test]
fn test_nine_lives_no_gpu_no_revive() {
    let (mut world, mut schedule) = make_sim();
    let chonk = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Chonk);
    // Drain player 0's GPU to 0 (default is 50)
    world.resource_mut::<PlayerResources>().players[0].gpu_cores = 0;

    // Set HP to 0 directly
    world.get_mut::<Health>(chonk).unwrap().current = Fixed::ZERO;

    run_ticks(&mut world, &mut schedule, 1);

    let is_dead = world.get::<Dead>(chonk).is_some();
    assert!(is_dead, "Chonk should die without GPU for NineLives");
}

/// NineLives: cooldown prevents double trigger
#[test]
fn test_nine_lives_cooldown_prevents_double_trigger() {
    let (mut world, mut schedule) = make_sim();
    let chonk = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Chonk);
    give_gpu(&mut world, 0, 100);

    // Trigger NineLives once by setting HP to 0
    world.get_mut::<Health>(chonk).unwrap().current = Fixed::ZERO;
    run_ticks(&mut world, &mut schedule, 1);

    // Chonk should have survived (first trigger)
    assert!(
        world.get::<Dead>(chonk).is_none(),
        "Chonk should survive first NineLives trigger"
    );

    // Now set HP to 0 again immediately (within cooldown)
    world.get_mut::<Health>(chonk).unwrap().current = Fixed::ZERO;
    run_ticks(&mut world, &mut schedule, 1);

    // Should be dead — cooldown hasn't expired
    let is_dead = world.get::<Dead>(chonk).is_some();
    assert!(is_dead, "Chonk should die on second lethal within NineLives cooldown");
}

/// ContagiousYawning: AoE CC applies Drowsed to enemies in range
#[test]
fn test_contagious_yawning_aoe_cc() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Activate ContagiousYawning (slot 1 for Catnapper)
    issue_ability(&mut world, catnapper, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    let has_drowsed = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::Drowsed);
    assert!(has_drowsed, "Enemy should have Drowsed after ContagiousYawning");
}

/// ContagiousYawning: CC immune enemies unaffected
#[test]
fn test_contagious_yawning_respects_cc_immunity() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Give enemy CC immunity
    {
        let mut effects = world.get_mut::<StatusEffects>(enemy).unwrap();
        effects.cc_immunity_remaining = 999;
    }

    issue_ability(&mut world, catnapper, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    let has_drowsed = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::Drowsed);
    assert!(!has_drowsed, "CC immune enemy should NOT get Drowsed");
}

/// PowerNap: generates GPU while active
#[test]
fn test_power_nap_generates_gpu() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);
    give_gpu(&mut world, 0, 50); // Need GPU to pay ability cost

    let gpu_before = world.resource::<PlayerResources>().players[0].gpu_cores;

    // Activate PowerNap (slot 2 for Catnapper)
    issue_ability(&mut world, catnapper, 2);
    run_ticks(&mut world, &mut schedule, 20);

    let gpu_after = world.resource::<PlayerResources>().players[0].gpu_cores;
    // PowerNap costs 10 GPU but generates 1 GPU every 2 ticks
    // Over 20 ticks that's ~10 GPU generated, minus 10 cost = net ~0
    // But we just need to check it generated SOME GPU beyond what was left after cost
    let gpu_after_cost = gpu_before - 10; // 10 GPU ability cost
    assert!(
        gpu_after > gpu_after_cost,
        "PowerNap should generate GPU: after_cost={}, final={}",
        gpu_after_cost,
        gpu_after,
    );
}

/// PowerNap: immobilizes and prevents attack
#[test]
fn test_power_nap_immobilizes_and_prevents_attack() {
    let (mut world, mut schedule) = make_sim();
    let catnapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Catnapper);
    give_gpu(&mut world, 0, 50);

    // Activate PowerNap (slot 2)
    issue_ability(&mut world, catnapper, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let modifiers = world.get::<StatModifiers>(catnapper).unwrap();
    assert!(modifiers.immobilized, "PowerNap should immobilize");
    assert!(modifiers.cannot_attack, "PowerNap should prevent attack");
}

/// TacticalUplink: allies in range get TacticalLink (cooldown reduction)
#[test]
fn test_tactical_uplink_buffs_allies() {
    let (mut world, mut schedule) = make_sim();
    let mech = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::MechCommander);
    let ally = spawn_unit(&mut world, GridPos::new(11, 10), 0, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Activate TacticalUplink (slot 0 for MechCommander, toggle)
    issue_ability(&mut world, mech, 0);
    run_ticks(&mut world, &mut schedule, 5);

    let effects = world.get::<StatusEffects>(ally).unwrap();
    let has_link = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::TacticalLink);
    assert!(has_link, "Ally should have TacticalLink from TacticalUplink aura");

    let modifiers = world.get::<StatModifiers>(ally).unwrap();
    assert!(
        modifiers.cooldown_multiplier < Fixed::from_num(1),
        "TacticalLink should reduce cooldowns: {}",
        modifiers.cooldown_multiplier,
    );
}

/// TacticalUplink: enemies should NOT get TacticalLink
#[test]
fn test_tactical_uplink_no_effect_on_enemies() {
    let (mut world, mut schedule) = make_sim();
    let mech = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::MechCommander);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    issue_ability(&mut world, mech, 0);
    run_ticks(&mut world, &mut schedule, 5);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    let has_link = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::TacticalLink);
    assert!(!has_link, "Enemy should NOT have TacticalLink");
}

/// Hairball: spawns an obstacle entity
#[test]
fn test_hairball_spawns_obstacle() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Count HairballObstacle entities before
    let count_before = world
        .query::<&HairballObstacle>()
        .iter(&world)
        .count();

    // Activate Hairball (slot 1 for Nuisance)
    issue_ability(&mut world, nuisance, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let count_after = world
        .query::<&HairballObstacle>()
        .iter(&world)
        .count();

    assert!(
        count_after > count_before,
        "Hairball should spawn a HairballObstacle entity"
    );
}

/// Hairball: despawns after duration
#[test]
fn test_hairball_despawns_after_duration() {
    let (mut world, mut schedule) = make_sim();
    let nuisance = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    issue_ability(&mut world, nuisance, 1);
    run_ticks(&mut world, &mut schedule, 3);

    // Verify hairball exists
    let count_mid = world
        .query::<&HairballObstacle>()
        .iter(&world)
        .count();
    assert!(count_mid > 0, "Hairball should exist after spawning");

    // Run past duration (100 ticks)
    run_ticks(&mut world, &mut schedule, 110);

    let count_after = world
        .query::<&HairballObstacle>()
        .iter(&world)
        .count();
    assert_eq!(count_after, 0, "Hairball should despawn after 100 ticks");
}

/// Disoriented (FlyingFox): AoE CC applies Disoriented to enemies
#[test]
fn test_disoriented_flyingfox_aoe_cc() {
    let (mut world, mut schedule) = make_sim();
    let fox = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::FlyingFox);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Activate Disoriented (slot 2 for FlyingFox)
    issue_ability(&mut world, fox, 2);
    run_ticks(&mut world, &mut schedule, 3);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    let has_disoriented = effects
        .effects
        .iter()
        .any(|e| e.effect == StatusEffectId::Disoriented);
    assert!(has_disoriented, "Enemy should have Disoriented after FlyingFox ability");
}

/// DisgustMortar: AoE damage hits enemies near source
#[test]
fn test_disgust_mortar_aoe_damage() {
    let (mut world, mut schedule) = make_sim();
    let hisser = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Hisser);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    let hp_before = world.get::<Health>(enemy).unwrap().current;

    // Activate DisgustMortar (slot 1 for Hisser)
    issue_ability(&mut world, hisser, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let hp_after = world.get::<Health>(enemy).unwrap().current;
    assert!(
        hp_after < hp_before,
        "Enemy should take AoE damage from DisgustMortar: before={}, after={}",
        hp_before,
        hp_after,
    );
}

/// DisgustMortar: no friendly fire
#[test]
fn test_disgust_mortar_no_friendly_fire() {
    let (mut world, mut schedule) = make_sim();
    let hisser = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Hisser);
    let ally = spawn_unit(&mut world, GridPos::new(11, 10), 0, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    let hp_before = world.get::<Health>(ally).unwrap().current;

    issue_ability(&mut world, hisser, 1);
    run_ticks(&mut world, &mut schedule, 3);

    let hp_after = world.get::<Health>(ally).unwrap().current;
    assert_eq!(
        hp_before, hp_after,
        "Ally should NOT take damage from DisgustMortar"
    );
}

/// ShapedCharge: buildings take 3x damage
#[test]
fn test_shaped_charge_building_bonus() {
    let (mut world, mut schedule) = make_sim();
    let sapper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::FerretSapper);
    let building = spawn_building(&mut world, GridPos::new(11, 10), 1, BuildingKind::TheBox);
    let unit = spawn_unit(&mut world, GridPos::new(11, 11), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    let bld_hp_before = world.get::<Health>(building).unwrap().current;
    let unit_hp_before = world.get::<Health>(unit).unwrap().current;

    // Activate ShapedCharge (slot 0 for FerretSapper)
    issue_ability(&mut world, sapper, 0);
    run_ticks(&mut world, &mut schedule, 3);

    let bld_hp_after = world.get::<Health>(building).unwrap().current;
    let unit_hp_after = world.get::<Health>(unit).unwrap().current;

    let bld_dmg = bld_hp_before - bld_hp_after;
    let unit_dmg = unit_hp_before - unit_hp_after;

    // Both should take damage if in range
    if unit_dmg > Fixed::ZERO {
        assert!(
            bld_dmg > unit_dmg,
            "Building should take more damage than unit from ShapedCharge: bld_dmg={}, unit_dmg={}",
            bld_dmg,
            unit_dmg,
        );
    } else {
        // Unit was out of range, just check building took damage
        assert!(
            bld_dmg > Fixed::ZERO,
            "Building should take damage from ShapedCharge"
        );
    }
}

/// EcholocationPulse: reveals enemies with VisibleThroughFog
#[test]
fn test_echolocation_reveals_enemies() {
    let (mut world, mut schedule) = make_sim();
    let fox = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::FlyingFox);
    let enemy = spawn_unit(&mut world, GridPos::new(12, 10), 1, UnitKind::Nuisance);
    give_gpu(&mut world, 0, 50);

    // Verify no VTF before
    assert!(
        world.get::<VisibleThroughFog>(enemy).is_none(),
        "Enemy should not have VTF before echolocation"
    );

    // Activate EcholocationPulse (slot 0 for FlyingFox)
    issue_ability(&mut world, fox, 0);
    run_ticks(&mut world, &mut schedule, 3);

    let vtf = world.get::<VisibleThroughFog>(enemy);
    assert!(vtf.is_some(), "Enemy should have VisibleThroughFog after EcholocationPulse");
    assert!(
        vtf.unwrap().remaining_ticks > 0,
        "VTF should have remaining ticks"
    );
}
