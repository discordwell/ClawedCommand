//! Integration tests for the campaign system: hero spawning, trigger evaluation,
//! mission victory/failure, and AI personality integration.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::hero::{hero_base_kind, hero_modifiers, HeroId};
use cc_core::map::GameMap;
use cc_core::mission::*;
use cc_core::unit_stats::base_stats;
use cc_sim::campaign::state::{CampaignPhase, CampaignState, MissionFailedEvent, MissionVictoryEvent};
use cc_sim::campaign::triggers::{
    trigger_check_system, DialogueEvent, ObjectiveCompleteEvent, TriggerFiredEvent,
};
use cc_sim::resources::{CommandQueue, ControlGroups, MapResource, PlayerResources, SimClock, SimRng};
use cc_sim::systems::tick_system::tick_system;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a sim world with campaign systems included in the schedule.
fn make_campaign_sim(map: GameMap) -> (World, Schedule) {
    let mut world = World::new();
    world.insert_resource(CommandQueue::default());
    world.insert_resource(SimClock::default());
    world.insert_resource(ControlGroups::default());
    world.insert_resource(PlayerResources::default());
    world.insert_resource(SimRng::default());
    world.insert_resource(MapResource { map });
    world.init_resource::<CampaignState>();

    // Register all message types used by campaign systems
    world.init_resource::<Messages<DialogueEvent>>();
    world.init_resource::<Messages<TriggerFiredEvent>>();
    world.init_resource::<Messages<ObjectiveCompleteEvent>>();
    world.init_resource::<Messages<MissionFailedEvent>>();
    world.init_resource::<Messages<MissionVictoryEvent>>();

    let mut schedule = Schedule::new(FixedUpdate);
    // Minimal system chain for campaign testing: tick → trigger → objective
    schedule.add_systems(
        (
            tick_system,
            trigger_check_system,
            cc_sim::campaign::state::mission_objective_system,
        )
            .chain(),
    );

    (world, schedule)
}

/// Spawn a hero entity with boosted stats.
fn spawn_hero(world: &mut World, hero_id: HeroId, pos: GridPos, player_id: u8, mission_critical: bool) -> Entity {
    let kind = hero_base_kind(hero_id);
    let base = base_stats(kind);
    let mods = hero_modifiers(hero_id);

    let boosted_hp = base.health + mods.health_bonus;
    let boosted_speed = base.speed * mods.speed_multiplier;
    let boosted_damage = base.damage + mods.damage_bonus;
    let boosted_range = base.range + mods.range_bonus;

    world
        .spawn((
            Position {
                world: WorldPos::from_grid(pos),
            },
            Velocity::zero(),
            GridCell { pos },
            MovementSpeed { speed: boosted_speed },
            Owner { player_id },
            UnitType { kind },
            Health {
                current: boosted_hp,
                max: boosted_hp,
            },
            AttackStats {
                damage: boosted_damage,
                range: boosted_range,
                attack_speed: base.attack_speed,
                cooldown_remaining: 0,
            },
            HeroIdentity {
                hero_id,
                mission_critical,
            },
        ))
        .id()
}

/// Spawn a basic combat unit (for enemies).
fn spawn_combat_unit(world: &mut World, kind: UnitKind, pos: GridPos, player_id: u8) -> Entity {
    let stats = base_stats(kind);
    world
        .spawn((
            Position {
                world: WorldPos::from_grid(pos),
            },
            Velocity::zero(),
            GridCell { pos },
            MovementSpeed { speed: stats.speed },
            Owner { player_id },
            UnitType { kind },
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

fn test_mission() -> MissionDefinition {
    MissionDefinition {
        id: "test_mission".into(),
        name: "Test Mission".into(),
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
                position: GridPos::new(2, 2),
                mission_critical: true,
            }],
            units: vec![],
            buildings: vec![],
            starting_food: 0,
            starting_gpu: 0,
            starting_nfts: 0,
        },
        enemy_waves: vec![EnemyWave {
            wave_id: "test_wave".into(),
            trigger: WaveTrigger::Immediate,
            units: vec![UnitSpawn {
                kind: UnitKind::Nuisance,
                position: GridPos::new(10, 10),
                player_id: 1,
            }],
            ai_behavior: WaveAiBehavior::Idle,
        }],
        objectives: vec![
            MissionObjective {
                id: "defeat_all".into(),
                description: "Eliminate all enemies".into(),
                primary: true,
                condition: ObjectiveCondition::Manual,
            },
            MissionObjective {
                id: "hero_survives".into(),
                description: "Kelpie must survive".into(),
                primary: true,
                condition: ObjectiveCondition::HeroDied(HeroId::Kelpie),
            },
        ],
        triggers: vec![
            ScriptedTrigger {
                id: "opening".into(),
                condition: TriggerCondition::AtTick(1),
                actions: vec![TriggerAction::ShowDialogue(vec![0])],
                once: true,
            },
            ScriptedTrigger {
                id: "kill_trigger".into(),
                condition: TriggerCondition::EnemyKillCount(1),
                actions: vec![
                    TriggerAction::SetFlag("first_kill".into()),
                    TriggerAction::CompleteObjective("defeat_all".into()),
                ],
                once: true,
            },
        ],
        dialogue: vec![DialogueLine {
            speaker: "Minstral".into(),
            text: "Hello!".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: String::new(),
        }],
        briefing_text: "Test briefing".into(),
        debrief_text: "Test debrief".into(),
    }
}

// ---------------------------------------------------------------------------
// Hero System Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn hero_entity_has_boosted_stats() {
    let (mut world, _) = make_campaign_sim(GameMap::new(16, 16));

    let hero = spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    let health = world.get::<Health>(hero).unwrap();
    let base_hp = base_stats(UnitKind::Nuisance).health;
    let expected_hp = base_hp + hero_modifiers(HeroId::Kelpie).health_bonus;
    assert_eq!(health.current, expected_hp);
    assert_eq!(health.max, expected_hp);

    let speed = world.get::<MovementSpeed>(hero).unwrap();
    let base_speed = base_stats(UnitKind::Nuisance).speed;
    let expected_speed = base_speed * hero_modifiers(HeroId::Kelpie).speed_multiplier;
    assert_eq!(speed.speed, expected_speed);

    let attack = world.get::<AttackStats>(hero).unwrap();
    let base_dmg = base_stats(UnitKind::Nuisance).damage;
    let expected_dmg = base_dmg + hero_modifiers(HeroId::Kelpie).damage_bonus;
    assert_eq!(attack.damage, expected_dmg);
}

#[test]
fn hero_entity_has_hero_identity() {
    let (mut world, _) = make_campaign_sim(GameMap::new(16, 16));

    let hero = spawn_hero(&mut world, HeroId::FelixNine, GridPos::new(5, 5), 0, false);

    let identity = world.get::<HeroIdentity>(hero).unwrap();
    assert_eq!(identity.hero_id, HeroId::FelixNine);
    assert!(!identity.mission_critical);
}

#[test]
fn hero_has_more_hp_than_regular_unit() {
    let (mut world, _) = make_campaign_sim(GameMap::new(16, 16));

    let hero = spawn_hero(&mut world, HeroId::MotherGranite, GridPos::new(2, 2), 0, true);
    let regular = spawn_combat_unit(&mut world, UnitKind::Chonk, GridPos::new(5, 5), 0);

    let hero_hp = world.get::<Health>(hero).unwrap().max;
    let regular_hp = world.get::<Health>(regular).unwrap().max;

    assert!(hero_hp > regular_hp, "Mother Granite hero should have more HP than regular Chonk");
}

// ---------------------------------------------------------------------------
// Trigger System Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn trigger_fires_at_tick() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    // Load mission with opening trigger at tick 1
    let mission = test_mission();
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    // Spawn the hero the mission expects
    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Run tick 1 — opening trigger should fire
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert!(
        campaign.fired_triggers.contains(&"opening".to_string()),
        "Opening trigger should have fired at tick 1"
    );
}

#[test]
fn kill_count_trigger_fires() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mission = test_mission();
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    // Spawn hero and an enemy
    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);
    let _enemy = spawn_combat_unit(&mut world, UnitKind::Nuisance, GridPos::new(10, 10), 1);

    // Simulate enemy death by setting kill count directly
    world.resource_mut::<CampaignState>().enemy_kill_count = 1;

    // Run a tick for trigger system to evaluate
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert!(
        campaign.fired_triggers.contains(&"kill_trigger".to_string()),
        "Kill trigger should have fired after enemy_kill_count=1"
    );
    assert!(
        campaign.flags.contains(&"first_kill".to_string()),
        "SetFlag action should have set 'first_kill' flag"
    );
}

#[test]
fn once_trigger_fires_only_once() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    // Create mission with a kill count trigger
    let mut mission = test_mission();
    mission.triggers = vec![ScriptedTrigger {
        id: "repeatable_check".into(),
        condition: TriggerCondition::EnemyKillCount(1),
        actions: vec![TriggerAction::SetFlag("fired".into())],
        once: true,
    }];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    // Spawn hero
    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Set kill count to trigger
    world.resource_mut::<CampaignState>().enemy_kill_count = 1;

    // Run multiple ticks — trigger should fire once
    run_ticks(&mut world, &mut schedule, 5);

    let campaign = world.resource::<CampaignState>();
    let fire_count = campaign
        .fired_triggers
        .iter()
        .filter(|t| *t == "repeatable_check")
        .count();
    assert_eq!(fire_count, 1, "Once-trigger should fire exactly once");
}

#[test]
fn flag_set_trigger_action_works() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    mission.triggers = vec![ScriptedTrigger {
        id: "set_flag_trigger".into(),
        condition: TriggerCondition::AtTick(1),
        actions: vec![TriggerAction::SetFlag("my_flag".into())],
        once: true,
    }];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    // Spawn hero
    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Run tick 1
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert!(campaign.flags.contains(&"my_flag".to_string()));
}

#[test]
fn compound_all_condition_requires_both() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    mission.triggers = vec![
        ScriptedTrigger {
            id: "first".into(),
            condition: TriggerCondition::AtTick(1),
            actions: vec![TriggerAction::SetFlag("first_done".into())],
            once: true,
        },
        ScriptedTrigger {
            id: "compound".into(),
            condition: TriggerCondition::All(vec![
                TriggerCondition::TriggerFired("first".into()),
                TriggerCondition::EnemyKillCount(2),
            ]),
            actions: vec![TriggerAction::SetFlag("compound_done".into())],
            once: true,
        },
    ];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Run tick 1 — first trigger fires, but compound needs kill count too
    run_ticks(&mut world, &mut schedule, 1);
    assert!(!world.resource::<CampaignState>().fired_triggers.contains(&"compound".to_string()),
        "Compound should NOT fire yet — kill count not met");

    // Set kill count and run again
    world.resource_mut::<CampaignState>().enemy_kill_count = 2;
    run_ticks(&mut world, &mut schedule, 1);

    assert!(world.resource::<CampaignState>().fired_triggers.contains(&"compound".to_string()),
        "Compound should fire now — both conditions met");
}

// ---------------------------------------------------------------------------
// Mission Victory/Failure Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn mission_victory_when_all_primary_objectives_complete() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    // One primary objective, manual completion
    mission.objectives = vec![MissionObjective {
        id: "win".into(),
        description: "Win".into(),
        primary: true,
        condition: ObjectiveCondition::Manual,
    }];
    mission.triggers = vec![ScriptedTrigger {
        id: "auto_win".into(),
        condition: TriggerCondition::AtTick(1),
        actions: vec![TriggerAction::CompleteObjective("win".into())],
        once: true,
    }];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Run enough ticks for trigger→objective→victory
    run_ticks(&mut world, &mut schedule, 3);

    let campaign = world.resource::<CampaignState>();
    assert_eq!(campaign.phase, CampaignPhase::Debriefing, "Mission should transition to Debriefing on victory");
    assert!(campaign.completed_missions.contains(&"test_mission".to_string()));
}

#[test]
fn mission_fails_when_mission_critical_hero_dies() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mission = test_mission();
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    let hero = spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    // Kill the hero by adding Dead component
    world.entity_mut(hero).insert(Dead);

    // Run a tick for mission_objective_system to detect the death
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert_eq!(campaign.phase, CampaignPhase::Debriefing, "Mission should transition to Debriefing on hero death");
}

#[test]
fn mission_does_not_fail_for_non_critical_hero_death() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    // Remove HeroDied fail condition so only mission_critical check matters
    mission.objectives = vec![MissionObjective {
        id: "obj".into(),
        description: "Objective".into(),
        primary: true,
        condition: ObjectiveCondition::Manual,
    }];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    // Spawn a NON-critical hero
    let hero = spawn_hero(&mut world, HeroId::Patches, GridPos::new(2, 2), 0, false);

    // Kill the non-critical hero
    world.entity_mut(hero).insert(Dead);

    run_ticks(&mut world, &mut schedule, 2);

    let campaign = world.resource::<CampaignState>();
    assert_eq!(campaign.phase, CampaignPhase::InMission, "Mission should NOT fail for non-critical hero death");
}

#[test]
fn kill_count_objective_auto_completes() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    mission.objectives = vec![MissionObjective {
        id: "kill_5".into(),
        description: "Kill 5 enemies".into(),
        primary: true,
        condition: ObjectiveCondition::KillCount(5),
    }];
    mission.triggers = vec![];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, false);

    // Set kill count below target
    world.resource_mut::<CampaignState>().enemy_kill_count = 3;
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert_eq!(campaign.phase, CampaignPhase::InMission, "Should not complete at 3 kills");

    // Set kill count to target
    world.resource_mut::<CampaignState>().enemy_kill_count = 5;
    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert_eq!(campaign.phase, CampaignPhase::Debriefing, "Should complete at 5 kills");
}

// ---------------------------------------------------------------------------
// AI Personality Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn ai_personality_profiles_are_distinct() {
    use cc_sim::ai::fsm::{faction_personality, AiPersonalityProfile};

    let factions = ["catGPT", "The Clawed", "Seekers of the Deep", "The Murder", "LLAMA", "Croak"];
    let profiles: Vec<AiPersonalityProfile> = factions.iter().map(|f| faction_personality(f)).collect();

    // All profiles should have different names
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    for (i, name) in names.iter().enumerate() {
        for (j, other) in names.iter().enumerate() {
            if i != j {
                assert_ne!(name, other, "Faction profiles should have unique names");
            }
        }
    }

    // Profiles should have varying attack_threshold (not all same)
    let thresholds: Vec<u32> = profiles.iter().map(|p| p.attack_threshold).collect();
    let all_same = thresholds.windows(2).all(|w| w[0] == w[1]);
    assert!(!all_same, "Attack thresholds should vary across factions");
}

#[test]
fn ai_personality_unit_preferences_differ() {
    use cc_sim::ai::fsm::faction_personality;

    let catgpt = faction_personality("catGPT");
    let seekers = faction_personality("Seekers of the Deep");

    // catGPT and Seekers should have different unit preferences
    assert_ne!(
        catgpt.unit_preferences, seekers.unit_preferences,
        "Different factions should have different unit preferences"
    );
}

#[test]
fn default_personality_for_unknown_faction() {
    use cc_sim::ai::fsm::faction_personality;

    let default = faction_personality("Unknown Faction");
    // Unknown factions get a fallback profile
    assert!(!default.name.is_empty(), "Default profile should have a name");
}

// ---------------------------------------------------------------------------
// Prologue Mission Data Tests
// ---------------------------------------------------------------------------

#[test]
fn prologue_mission_has_correct_structure() {
    let ron_str = include_str!("../../../assets/campaign/prologue.ron");
    let mission: MissionDefinition = ron::from_str(ron_str).expect("prologue.ron should parse");

    // Verify structure
    assert_eq!(mission.id, "prologue");
    assert_eq!(mission.act, 0);

    // Has Kelpie as hero
    assert_eq!(mission.player_setup.heroes.len(), 1);
    assert_eq!(mission.player_setup.heroes[0].hero_id, HeroId::Kelpie);
    assert!(mission.player_setup.heroes[0].mission_critical);

    // Has enemy waves
    assert!(mission.enemy_waves.len() >= 3, "Prologue should have at least 3 waves");

    // Has primary objectives
    assert!(
        mission.objectives.iter().any(|o| o.primary),
        "Prologue must have primary objectives"
    );

    // Has triggers
    assert!(mission.triggers.len() >= 8, "Prologue should have at least 8 triggers");

    // Has dialogue
    assert!(mission.dialogue.len() >= 20, "Prologue should have at least 20 dialogue lines");

    // All dialogue speakers are non-empty
    for (i, line) in mission.dialogue.iter().enumerate() {
        assert!(!line.speaker.is_empty(), "Dialogue line {i} has empty speaker");
        assert!(!line.text.is_empty(), "Dialogue line {i} has empty text");
    }

    // Validate internal consistency
    mission.validate().expect("Prologue should be internally consistent");
}

#[test]
fn prologue_trigger_chain_is_sound() {
    let ron_str = include_str!("../../../assets/campaign/prologue.ron");
    let mission: MissionDefinition = ron::from_str(ron_str).expect("prologue.ron should parse");

    // Verify opening trigger fires at tick 1
    let opening = mission.triggers.iter().find(|t| t.id == "opening_dialogue").unwrap();
    assert!(matches!(&opening.condition, TriggerCondition::AtTick(1)));
    assert!(opening.once);

    // Verify spawn_flankers fires on kill count 4
    let flankers = mission.triggers.iter().find(|t| t.id == "spawn_flankers").unwrap();
    assert!(matches!(&flankers.condition, TriggerCondition::EnemyKillCount(4)));

    // Verify pack_leader_dead fires when all enemies dead
    let dead = mission.triggers.iter().find(|t| t.id == "pack_leader_dead").unwrap();
    assert!(matches!(&dead.condition, TriggerCondition::AllEnemiesDead));

    // Verify it completes the defeat_pack_leader objective
    let completes_obj = dead.actions.iter().any(|a| {
        matches!(a, TriggerAction::CompleteObjective(id) if id == "defeat_pack_leader")
    });
    assert!(completes_obj, "pack_leader_dead trigger should complete the defeat_pack_leader objective");
}

// ---------------------------------------------------------------------------
// Campaign State Management Tests
// ---------------------------------------------------------------------------

#[test]
fn campaign_inactive_systems_are_noop() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    // Don't load any mission — phase should be Inactive
    assert_eq!(world.resource::<CampaignState>().phase, CampaignPhase::Inactive);

    // Running ticks should not panic or change state
    run_ticks(&mut world, &mut schedule, 10);

    assert_eq!(world.resource::<CampaignState>().phase, CampaignPhase::Inactive);
}

#[test]
fn wave_spawn_tracked_in_campaign_state() {
    let (mut world, mut schedule) = make_campaign_sim(GameMap::new(16, 16));

    let mut mission = test_mission();
    mission.triggers = vec![ScriptedTrigger {
        id: "spawn_test_wave".into(),
        condition: TriggerCondition::AtTick(1),
        actions: vec![TriggerAction::SpawnWave("test_wave".into())],
        once: true,
    }];
    world.resource_mut::<CampaignState>().load_mission(mission);
    world.resource_mut::<CampaignState>().phase = CampaignPhase::InMission;

    spawn_hero(&mut world, HeroId::Kelpie, GridPos::new(2, 2), 0, true);

    run_ticks(&mut world, &mut schedule, 1);

    let campaign = world.resource::<CampaignState>();
    assert!(
        campaign.spawned_waves.contains(&"test_wave".to_string()),
        "SpawnWave action should track wave in campaign state"
    );
}
