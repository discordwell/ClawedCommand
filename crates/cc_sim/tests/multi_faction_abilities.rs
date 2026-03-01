//! Integration tests for multi-faction ability implementations.

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
    SpawnPositions,
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
    world.insert_resource(MapResource {
        map: GameMap::new(32, 32),
    });

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

fn issue_ability(world: &mut World, unit: Entity, slot: u8) {
    world
        .resource_mut::<CommandQueue>()
        .push(GameCommand::ActivateAbility {
            unit_id: EntityId(unit.to_bits()),
            slot,
            target: AbilityTarget::SelfCast,
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
// The Clawed (Mice) — Toggle abilities
// ---------------------------------------------------------------------------

/// RallyTheSwarm toggle creates an Aura component.
#[test]
fn clawed_rally_the_swarm_creates_aura() {
    let (mut world, mut schedule) = make_sim();
    let marshal = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::WarrenMarshal);

    // Slot 0 = RallyTheSwarm (Toggle, 0 GPU)
    issue_ability(&mut world, marshal, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let aura = world.get::<Aura>(marshal);
    assert!(aura.is_some(), "WarrenMarshal should have Aura after toggle");
    let aura = aura.unwrap();
    assert_eq!(aura.aura_type, AuraType::RallyTheSwarm);
    assert!(aura.active);
}

/// ChewThrough toggle applies DamageBuff via ability_effect_system.
#[test]
fn clawed_chew_through_gives_damage_buff() {
    let (mut world, mut schedule) = make_sim();
    let gnawer = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Gnawer);

    // Slot 1 = ChewThrough (Toggle, 0 GPU)
    issue_ability(&mut world, gnawer, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(gnawer).unwrap();
    assert!(
        effects.has(StatusEffectId::DamageBuff),
        "Gnawer should have DamageBuff from ChewThrough toggle"
    );
}

/// SpineWall toggle applies ArmorBuff.
#[test]
fn clawed_spine_wall_gives_armor_buff() {
    let (mut world, mut schedule) = make_sim();
    let quillback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Quillback);

    // Slot 0 = SpineWall (Toggle, 0 GPU)
    issue_ability(&mut world, quillback, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(quillback).unwrap();
    assert!(
        effects.has(StatusEffectId::ArmorBuff),
        "Quillback should have ArmorBuff from SpineWall toggle"
    );
}

// ---------------------------------------------------------------------------
// The Clawed (Mice) — Activated abilities
// ---------------------------------------------------------------------------

/// SonicSpit applies Stunned to nearby enemies.
#[test]
fn clawed_sonic_spit_stuns_enemies() {
    let (mut world, mut schedule) = make_sim();
    let shrieker = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Shrieker);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 0 = SonicSpit
    issue_ability(&mut world, shrieker, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Stunned),
        "Enemy should be Stunned by SonicSpit"
    );
}

/// QuillBurst deals AoE damage.
#[test]
fn clawed_quill_burst_deals_damage() {
    let (mut world, mut schedule) = make_sim();
    let quillback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Quillback);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    // Give Nuisance lots of HP to survive
    world.get_mut::<Health>(enemy).unwrap().current = Fixed::from_num(500);
    world.get_mut::<Health>(enemy).unwrap().max = Fixed::from_num(500);

    give_gpu(&mut world, 0, 50);
    // Slot 1 = QuillBurst
    issue_ability(&mut world, quillback, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let hp = world.get::<Health>(enemy).unwrap().current;
    assert!(
        hp < Fixed::from_num(500),
        "Enemy should take damage from QuillBurst, HP={hp}"
    );
}

// ---------------------------------------------------------------------------
// Seekers of the Deep (Badgers) — Toggle abilities
// ---------------------------------------------------------------------------

/// Entrench toggle applies Entrenched (immobile + damage reduction + damage boost).
#[test]
fn seekers_entrench_applies_entrenched() {
    let (mut world, mut schedule) = make_sim();
    let cragback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Cragback);

    // Slot 1 = Entrench (Toggle, 0 GPU)
    issue_ability(&mut world, cragback, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(cragback).unwrap();
    assert!(
        effects.has(StatusEffectId::Entrenched),
        "Cragback should be Entrenched after toggle"
    );

    // Entrenched should set immobilized
    let mods = world.get::<StatModifiers>(cragback).unwrap();
    assert!(mods.immobilized, "Entrenched should immobilize");
}

/// SeismicSlam applies Stunned AoE.
#[test]
fn seekers_seismic_slam_stuns() {
    let (mut world, mut schedule) = make_sim();
    let cragback = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Cragback);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 2 = SeismicSlam
    issue_ability(&mut world, cragback, 2);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Stunned),
        "Enemy should be Stunned by SeismicSlam"
    );
}

/// ScorchedEarth deals AoE damage.
#[test]
fn seekers_scorched_earth_deals_damage() {
    let (mut world, mut schedule) = make_sim();
    let embermaw = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Embermaw);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    world.get_mut::<Health>(enemy).unwrap().current = Fixed::from_num(500);
    world.get_mut::<Health>(enemy).unwrap().max = Fixed::from_num(500);

    give_gpu(&mut world, 0, 50);
    // Slot 2 = ScorchedEarth
    issue_ability(&mut world, embermaw, 2);
    run_ticks(&mut world, &mut schedule, 1);

    let hp = world.get::<Health>(enemy).unwrap().current;
    assert!(
        hp < Fixed::from_num(500),
        "Enemy should take damage from ScorchedEarth, HP={hp}"
    );
}

// ---------------------------------------------------------------------------
// The Murder (Corvids) — Activated abilities
// ---------------------------------------------------------------------------

/// Cacophony applies Stunned AoE.
#[test]
fn murder_cacophony_stuns() {
    let (mut world, mut schedule) = make_sim();
    let jaycaller = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Jaycaller);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 2 = Cacophony
    issue_ability(&mut world, jaycaller, 2);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Stunned),
        "Enemy should be Stunned by Cacophony"
    );
}

/// GlitterBomb applies Disoriented.
#[test]
fn murder_glitter_bomb_disorients() {
    let (mut world, mut schedule) = make_sim();
    let magpike = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Magpike);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 1 = GlitterBomb
    issue_ability(&mut world, magpike, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Disoriented),
        "Enemy should be Disoriented by GlitterBomb"
    );
}

/// SignalJam applies Silenced.
#[test]
fn murder_signal_jam_silences() {
    let (mut world, mut schedule) = make_sim();
    let magpyre = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Magpyre);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 0 = SignalJam
    issue_ability(&mut world, magpyre, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Silenced),
        "Enemy should be Silenced by SignalJam"
    );
}

/// PanopticGaze toggle creates Aura.
#[test]
fn murder_panoptic_gaze_creates_aura() {
    let (mut world, mut schedule) = make_sim();
    let hootseer = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Hootseer);

    // Slot 0 = PanopticGaze (Toggle, 0 GPU)
    issue_ability(&mut world, hootseer, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let aura = world.get::<Aura>(hootseer);
    assert!(aura.is_some(), "Hootseer should have Aura after PanopticGaze toggle");
    assert_eq!(aura.unwrap().aura_type, AuraType::PanopticGaze);
}

// ---------------------------------------------------------------------------
// LLAMA (Raccoons) — Activated abilities
// ---------------------------------------------------------------------------

/// WreckBall deals AoE damage.
#[test]
fn llama_wreck_ball_deals_damage() {
    let (mut world, mut schedule) = make_sim();
    let heap_titan = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::HeapTitan);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    world.get_mut::<Health>(enemy).unwrap().current = Fixed::from_num(500);
    world.get_mut::<Health>(enemy).unwrap().max = Fixed::from_num(500);

    give_gpu(&mut world, 0, 50);
    // Slot 1 = WreckBall
    issue_ability(&mut world, heap_titan, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let hp = world.get::<Health>(enemy).unwrap().current;
    assert!(
        hp < Fixed::from_num(500),
        "Enemy should take damage from WreckBall, HP={hp}"
    );
}

/// SignalScramble applies Silenced.
#[test]
fn llama_signal_scramble_silences() {
    let (mut world, mut schedule) = make_sim();
    let glitch_rat = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::GlitchRat);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 1 = SignalScramble
    issue_ability(&mut world, glitch_rat, 1);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Silenced),
        "Enemy should be Silenced by SignalScramble"
    );
}

/// PlayDead applies PlayingDead (invulnerable + immobile).
#[test]
fn llama_play_dead_makes_invulnerable() {
    let (mut world, mut schedule) = make_sim();
    let scrounger = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Scrounger);

    // Slot 2 = PlayDead (0 GPU)
    issue_ability(&mut world, scrounger, 2);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(scrounger).unwrap();
    assert!(
        effects.has(StatusEffectId::PlayingDead),
        "Scrounger should be PlayingDead"
    );

    let mods = world.get::<StatModifiers>(scrounger).unwrap();
    assert!(mods.invulnerable, "PlayingDead should grant invulnerability");
    assert!(mods.immobilized, "PlayingDead should immobilize");
}

/// Getaway applies SpeedBuff.
#[test]
fn llama_getaway_gives_speed_buff() {
    let (mut world, mut schedule) = make_sim();
    let bandit = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Bandit);

    // Slot 2 = Getaway (0 GPU)
    issue_ability(&mut world, bandit, 2);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(bandit).unwrap();
    assert!(
        effects.has(StatusEffectId::SpeedBuff),
        "Bandit should have SpeedBuff from Getaway"
    );
}

// ---------------------------------------------------------------------------
// Croak (Axolotls) — Abilities
// ---------------------------------------------------------------------------

/// HunkerAbility toggle applies LoafModeActive (immobile + 50% DR).
#[test]
fn croak_hunker_applies_loaf_mode() {
    let (mut world, mut schedule) = make_sim();
    let shellwarden = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Shellwarden);

    // Slot 0 = HunkerAbility (Toggle, 0 GPU)
    issue_ability(&mut world, shellwarden, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(shellwarden).unwrap();
    assert!(
        effects.has(StatusEffectId::LoafModeActive),
        "Shellwarden should have LoafModeActive after Hunker toggle"
    );
}

/// MireCurse applies Waterlogged.
#[test]
fn croak_mire_curse_applies_waterlogged() {
    let (mut world, mut schedule) = make_sim();
    let bogwhisper = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Bogwhisper);
    let enemy = spawn_unit(&mut world, GridPos::new(11, 10), 1, UnitKind::Nuisance);

    give_gpu(&mut world, 0, 50);
    // Slot 0 = MireCurse
    issue_ability(&mut world, bogwhisper, 0);
    run_ticks(&mut world, &mut schedule, 1);

    let effects = world.get::<StatusEffects>(enemy).unwrap();
    assert!(
        effects.has(StatusEffectId::Waterlogged),
        "Enemy should be Waterlogged from MireCurse"
    );
}

/// Hop activates without error (instant dash, 0 duration).
#[test]
fn croak_hop_activates() {
    let (mut world, mut schedule) = make_sim();
    let leapfrog = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Leapfrog);

    // Slot 0 = Hop (0 GPU, 0 duration — instant dash)
    issue_ability(&mut world, leapfrog, 0);
    run_ticks(&mut world, &mut schedule, 1);

    // Hop should activate (set cooldown) without error
    let slots = world.get::<AbilitySlots>(leapfrog).unwrap();
    assert!(
        slots.slots[0].cooldown_remaining > 0,
        "Hop should be on cooldown after activation"
    );
}

// ---------------------------------------------------------------------------
// Cross-faction: StatusEffect stat modifiers
// ---------------------------------------------------------------------------

/// Stunned status applies immobilized + cannot_attack + silenced.
#[test]
fn stunned_status_applies_full_stun() {
    let (mut world, mut schedule) = make_sim();
    let unit = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Nuisance);

    // Directly inject Stunned effect
    {
        let mut effects = world.get_mut::<StatusEffects>(unit).unwrap();
        effects
            .effects
            .push(cc_core::status_effects::StatusInstance {
                effect: StatusEffectId::Stunned,
                remaining_ticks: 20,
                stacks: 1,
                source: EntityId(0),
            });
    }

    run_ticks(&mut world, &mut schedule, 1);

    let mods = world.get::<StatModifiers>(unit).unwrap();
    assert!(mods.immobilized, "Stunned should immobilize");
    assert!(mods.cannot_attack, "Stunned should prevent attacks");
    assert!(mods.silenced, "Stunned should silence");
}

/// Stunned counts as CC for immunity purposes.
#[test]
fn stunned_is_cc() {
    assert!(cc_core::status_effects::is_cc(StatusEffectId::Stunned));
    // Verify existing CCs still work
    assert!(cc_core::status_effects::is_cc(StatusEffectId::Disoriented));
    assert!(cc_core::status_effects::is_cc(StatusEffectId::Drowsed));
    assert!(cc_core::status_effects::is_cc(StatusEffectId::Tilted));
    // Non-CC
    assert!(!cc_core::status_effects::is_cc(StatusEffectId::Silenced));
    assert!(!cc_core::status_effects::is_cc(StatusEffectId::SpeedBuff));
}

/// Entrenched gives immobile + damage reduction + damage boost.
#[test]
fn entrenched_status_modifies_stats() {
    let (mut world, mut schedule) = make_sim();
    let unit = spawn_unit(&mut world, GridPos::new(10, 10), 0, UnitKind::Nuisance);

    {
        let mut effects = world.get_mut::<StatusEffects>(unit).unwrap();
        effects
            .effects
            .push(cc_core::status_effects::StatusInstance {
                effect: StatusEffectId::Entrenched,
                remaining_ticks: 20,
                stacks: 1,
                source: EntityId(0),
            });
    }

    run_ticks(&mut world, &mut schedule, 1);

    let mods = world.get::<StatModifiers>(unit).unwrap();
    assert!(mods.immobilized, "Entrenched should immobilize");
    assert!(
        mods.damage_multiplier > Fixed::ONE,
        "Entrenched should boost damage"
    );
    assert!(
        mods.damage_reduction < Fixed::ONE,
        "Entrenched should reduce damage taken"
    );
}
