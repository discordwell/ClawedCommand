use bevy::prelude::*;

use cc_core::commands::{CommandSource, EntityId, GameCommand};
use cc_core::components::{Dead, Owner, Selected, StatModifiers, UnitKind, UnitType, VoiceBuffed};
use cc_core::coords::GridPos;
use cc_core::mission::*;
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};
use cc_core::terrain::TerrainType;
use cc_sim::resources::{CommandQueue, MapResource, SimClock};

use crate::cutscene::CutsceneCamera;
use crate::renderer::voice_ping::spawn_voice_ping;
use crate::setup::UnitMesh;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAP_WIDTH: u32 = 40;
const MAP_HEIGHT: u32 = 30;

/// Tick thresholds for each phase transition.
const PHASE_FALLBACK_TICK: u64 = 30;
const PHASE_CHARGE_TICK: u64 = 80;
const PHASE_ATTACK_TICK: u64 = 130;
const PHASE_HOLD_TICK: u64 = 180;
const PHASE_PULLBACK_TICK: u64 = 230;

/// Duration of the SpeedBuff (effectively permanent for the demo).
const BUFF_DURATION: u32 = 9999;

/// Retreat target — behind the Chonk wall.
const RETREAT_POS: GridPos = GridPos::new(4, 15);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// FSM phases for the voice command demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VoiceDemoPhase {
    /// Establishing shot — units idle.
    Setup,
    /// Hissers fall back behind Chonks.
    Fallback,
    /// Mice charge toward cat line.
    Charge,
    /// All cats attack-move toward mice.
    Attack,
    /// Cats hold position — dig in.
    Hold,
    /// Cats retreat back to starting area.
    PullBack,
    /// Combat plays out naturally, no more scripted commands.
    Done,
}

/// Resource tracking the current voice demo phase and one-shot flags.
#[derive(Resource)]
pub struct VoiceDemoState {
    pub phase: VoiceDemoPhase,
    pub fallback_issued: bool,
    pub charge_issued: bool,
    pub attack_issued: bool,
    pub hold_issued: bool,
    pub pullback_issued: bool,
}

impl Default for VoiceDemoState {
    fn default() -> Self {
        Self {
            phase: VoiceDemoPhase::Setup,
            fallback_issued: false,
            charge_issued: false,
            attack_issued: false,
            hold_issued: false,
            pullback_issued: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Mission Builder
// ---------------------------------------------------------------------------

/// Build the voice demo mission: cats vs mice on a 40×30 map.
pub fn build_voice_demo_mission() -> MissionDefinition {
    let total = (MAP_WIDTH * MAP_HEIGHT) as usize;
    let mut tiles = vec![TerrainType::Grass; total];
    let mut elevation = vec![1u8; total];

    // Cat side (west): dirt terrain
    for y in 2..28 {
        for x in 2..14 {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Dirt, 1);
        }
    }

    // Battle line: road at x=15
    for y in 2..28 {
        set_tile(&mut tiles, &mut elevation, 15, y, TerrainType::Road, 1);
    }

    // Rock walls top/bottom
    for x in 0..MAP_WIDTH as i32 {
        for y in [0, 1, 28, 29] {
            set_tile(&mut tiles, &mut elevation, x, y, TerrainType::Rock, 2);
        }
    }

    // Cat units (player 0) — in player_setup.units so they spawn as P0
    let cat_units = vec![
        // 4 Chonks: tank wall at x=6
        UnitSpawn {
            kind: UnitKind::Chonk,
            position: GridPos::new(6, 11),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Chonk,
            position: GridPos::new(6, 14),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Chonk,
            position: GridPos::new(6, 17),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Chonk,
            position: GridPos::new(6, 20),
            player_id: 0,
        },
        // 5 Hissers: exposed in center at x=18-19
        UnitSpawn {
            kind: UnitKind::Hisser,
            position: GridPos::new(18, 10),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Hisser,
            position: GridPos::new(19, 13),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Hisser,
            position: GridPos::new(18, 15),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Hisser,
            position: GridPos::new(19, 17),
            player_id: 0,
        },
        UnitSpawn {
            kind: UnitKind::Hisser,
            position: GridPos::new(18, 20),
            player_id: 0,
        },
    ];

    // Mouse units (player 1) — in enemy_waves with Immediate trigger
    let mouse_units = vec![
        // 3 Swarmers
        UnitSpawn {
            kind: UnitKind::Swarmer,
            position: GridPos::new(32, 12),
            player_id: 1,
        },
        UnitSpawn {
            kind: UnitKind::Swarmer,
            position: GridPos::new(33, 15),
            player_id: 1,
        },
        UnitSpawn {
            kind: UnitKind::Swarmer,
            position: GridPos::new(32, 18),
            player_id: 1,
        },
        // 2 Shriekers
        UnitSpawn {
            kind: UnitKind::Shrieker,
            position: GridPos::new(34, 13),
            player_id: 1,
        },
        UnitSpawn {
            kind: UnitKind::Shrieker,
            position: GridPos::new(34, 17),
            player_id: 1,
        },
        // 2 Quillbacks
        UnitSpawn {
            kind: UnitKind::Quillback,
            position: GridPos::new(30, 14),
            player_id: 1,
        },
        UnitSpawn {
            kind: UnitKind::Quillback,
            position: GridPos::new(30, 16),
            player_id: 1,
        },
    ];

    // Dialogue — Le Chat (AI voice) announcing each command
    let dialogue = vec![
        DialogueLine {
            speaker: "Le Chat".into(),
            text: "Hissers, fall back. Get behind the Chonks.".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_le_chat".into(),
        },
        DialogueLine {
            speaker: "Le Chat".into(),
            text: "Clawed incoming. All mice, charge the line.".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_le_chat".into(),
        },
        DialogueLine {
            speaker: "Le Chat".into(),
            text: "All cats, attack. Push them back.".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_le_chat".into(),
        },
        DialogueLine {
            speaker: "Le Chat".into(),
            text: "Hold the line! Don't let them through.".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_le_chat".into(),
        },
        DialogueLine {
            speaker: "Le Chat".into(),
            text: "Pull back! Regroup at the tree line.".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_le_chat".into(),
        },
    ];

    // Triggers: show dialogue lines at the correct ticks
    let triggers = vec![
        ScriptedTrigger {
            id: "voice_fallback".into(),
            condition: TriggerCondition::AtTick(PHASE_FALLBACK_TICK),
            actions: vec![TriggerAction::ShowDialogue(vec![0])],
            once: true,
        },
        ScriptedTrigger {
            id: "voice_charge".into(),
            condition: TriggerCondition::AtTick(PHASE_CHARGE_TICK),
            actions: vec![TriggerAction::ShowDialogue(vec![1])],
            once: true,
        },
        ScriptedTrigger {
            id: "voice_attack".into(),
            condition: TriggerCondition::AtTick(PHASE_ATTACK_TICK),
            actions: vec![TriggerAction::ShowDialogue(vec![2])],
            once: true,
        },
        ScriptedTrigger {
            id: "voice_hold".into(),
            condition: TriggerCondition::AtTick(PHASE_HOLD_TICK),
            actions: vec![TriggerAction::ShowDialogue(vec![3])],
            once: true,
        },
        ScriptedTrigger {
            id: "voice_pullback".into(),
            condition: TriggerCondition::AtTick(PHASE_PULLBACK_TICK),
            actions: vec![TriggerAction::ShowDialogue(vec![4])],
            once: true,
        },
    ];

    MissionDefinition {
        id: "voice_demo".into(),
        name: "Voice Command Demo".into(),
        act: 0,
        mission_index: 0,
        map: MissionMap::Inline {
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            tiles,
            elevation,
        },
        player_setup: PlayerSetup {
            heroes: vec![],
            units: cat_units,
            buildings: vec![],
            starting_food: 9999,
            starting_gpu: 9999,
            starting_nfts: 9999,
        },
        enemy_waves: vec![EnemyWave {
            wave_id: "mice_main".into(),
            trigger: WaveTrigger::Immediate,
            units: mouse_units,
            ai_behavior: WaveAiBehavior::Idle,
        }],
        objectives: vec![MissionObjective {
            id: "voice_demo".into(),
            description: "Watch the voice command demo".into(),
            primary: true,
            condition: ObjectiveCondition::Survive(999999),
        }],
        triggers,
        dialogue,
        briefing_text: String::new(),
        debrief_text: String::new(),
        ai_tool_tier: None,
        next_mission: NextMission::None,
        mutators: vec![],
    }
}

/// Return the CutsceneCamera for the voice demo.
pub fn voice_demo_camera() -> CutsceneCamera {
    CutsceneCamera {
        focus: GridPos::new(MAP_WIDTH as i32 / 2, MAP_HEIGHT as i32 / 2),
        zoom: 0.7,
    }
}

// ---------------------------------------------------------------------------
// Demo System (FSM)
// ---------------------------------------------------------------------------

/// Phase-based FSM that issues commands at the scripted ticks.
pub fn voice_demo_system(
    mut commands: Commands,
    clock: Res<SimClock>,
    mut state: ResMut<VoiceDemoState>,
    mut cmd_queue: ResMut<CommandQueue>,
    units: Query<(Entity, &UnitType, &Owner), (With<UnitMesh>, Without<Dead>)>,
    status_query: Query<Option<&StatusEffects>, (With<UnitMesh>, Without<Dead>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    map_res: Res<MapResource>,
) {
    let tick = clock.tick;

    // Phase transitions (check highest first for correct priority)
    if tick >= PHASE_PULLBACK_TICK && state.phase < VoiceDemoPhase::PullBack {
        state.phase = VoiceDemoPhase::PullBack;
    } else if tick >= PHASE_HOLD_TICK && state.phase < VoiceDemoPhase::Hold {
        state.phase = VoiceDemoPhase::Hold;
    } else if tick >= PHASE_ATTACK_TICK && state.phase < VoiceDemoPhase::Attack {
        state.phase = VoiceDemoPhase::Attack;
    } else if tick >= PHASE_CHARGE_TICK && state.phase < VoiceDemoPhase::Charge {
        state.phase = VoiceDemoPhase::Charge;
    } else if tick >= PHASE_FALLBACK_TICK && state.phase < VoiceDemoPhase::Fallback {
        state.phase = VoiceDemoPhase::Fallback;
    }

    match state.phase {
        VoiceDemoPhase::Setup => {} // Idle, establishing shot
        VoiceDemoPhase::Fallback => {
            if !state.fallback_issued {
                state.fallback_issued = true;
                // Hissers (P0) fall back behind Chonks
                let hisser_entities: Vec<Entity> = units
                    .iter()
                    .filter(|(_, ut, owner)| ut.kind == UnitKind::Hisser && owner.player_id == 0)
                    .map(|(e, _, _)| e)
                    .collect();

                let hisser_ids: Vec<EntityId> = hisser_entities
                    .iter()
                    .map(|e| EntityId(e.to_bits()))
                    .collect();

                let target = RETREAT_POS;
                if !hisser_ids.is_empty() {
                    cmd_queue.push_sourced(
                        Some(0),
                        CommandSource::Script,
                        GameCommand::Move {
                            unit_ids: hisser_ids,
                            target,
                        },
                    );
                }

                // Apply buff to hissers
                for entity in &hisser_entities {
                    apply_voice_buff(&mut commands, *entity, &status_query);
                }

                spawn_voice_ping(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target,
                    map_res.map.elevation_at(target),
                );
            }
        }
        VoiceDemoPhase::Charge => {
            if !state.charge_issued {
                state.charge_issued = true;
                // All mice (P1) charge toward cat line
                let mouse_entities: Vec<Entity> = units
                    .iter()
                    .filter(|(_, _, owner)| owner.player_id == 1)
                    .map(|(e, _, _)| e)
                    .collect();

                let mouse_ids: Vec<EntityId> = mouse_entities
                    .iter()
                    .map(|e| EntityId(e.to_bits()))
                    .collect();

                let target = GridPos::new(8, 15);
                if !mouse_ids.is_empty() {
                    cmd_queue.push_sourced(
                        Some(1),
                        CommandSource::Script,
                        GameCommand::AttackMove {
                            unit_ids: mouse_ids,
                            target,
                        },
                    );
                }

                // Apply buff to mice
                for entity in &mouse_entities {
                    apply_voice_buff(&mut commands, *entity, &status_query);
                }

                spawn_voice_ping(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target,
                    map_res.map.elevation_at(target),
                );
            }
        }
        VoiceDemoPhase::Attack => {
            if !state.attack_issued {
                state.attack_issued = true;
                // All cats (P0) attack-move toward mice
                let cat_entities: Vec<Entity> = units
                    .iter()
                    .filter(|(_, _, owner)| owner.player_id == 0)
                    .map(|(e, _, _)| e)
                    .collect();

                let cat_ids: Vec<EntityId> =
                    cat_entities.iter().map(|e| EntityId(e.to_bits())).collect();

                let target = GridPos::new(32, 15);
                if !cat_ids.is_empty() {
                    cmd_queue.push_sourced(
                        Some(0),
                        CommandSource::Script,
                        GameCommand::AttackMove {
                            unit_ids: cat_ids,
                            target,
                        },
                    );
                }

                // Apply buff to all cats
                for entity in &cat_entities {
                    apply_voice_buff(&mut commands, *entity, &status_query);
                }

                spawn_voice_ping(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target,
                    map_res.map.elevation_at(target),
                );
            }
        }
        VoiceDemoPhase::Hold => {
            if !state.hold_issued {
                state.hold_issued = true;
                // All cats (P0) hold position
                let cat_entities: Vec<Entity> = units
                    .iter()
                    .filter(|(_, _, owner)| owner.player_id == 0)
                    .map(|(e, _, _)| e)
                    .collect();

                let cat_ids: Vec<EntityId> =
                    cat_entities.iter().map(|e| EntityId(e.to_bits())).collect();

                if !cat_ids.is_empty() {
                    cmd_queue.push_sourced(
                        Some(0),
                        CommandSource::Script,
                        GameCommand::HoldPosition { unit_ids: cat_ids },
                    );
                }

                // Apply buff to all cats
                for entity in &cat_entities {
                    apply_voice_buff(&mut commands, *entity, &status_query);
                }

                // Ping at approximate battle center
                let hold_target = GridPos::new(20, 15);
                spawn_voice_ping(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    hold_target,
                    map_res.map.elevation_at(hold_target),
                );
            }
        }
        VoiceDemoPhase::PullBack => {
            if !state.pullback_issued {
                state.pullback_issued = true;
                // All cats (P0) retreat to starting area
                let cat_entities: Vec<Entity> = units
                    .iter()
                    .filter(|(_, _, owner)| owner.player_id == 0)
                    .map(|(e, _, _)| e)
                    .collect();

                let cat_ids: Vec<EntityId> =
                    cat_entities.iter().map(|e| EntityId(e.to_bits())).collect();

                let target = RETREAT_POS;
                if !cat_ids.is_empty() {
                    cmd_queue.push_sourced(
                        Some(0),
                        CommandSource::Script,
                        GameCommand::Move {
                            unit_ids: cat_ids,
                            target,
                        },
                    );
                }

                // Apply buff to all cats
                for entity in &cat_entities {
                    apply_voice_buff(&mut commands, *entity, &status_query);
                }

                spawn_voice_ping(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    target,
                    map_res.map.elevation_at(target),
                );
            }
        }
        VoiceDemoPhase::Done => {} // Combat plays out naturally
    }
}

/// Apply the golden voice-command SpeedBuff to an entity.
fn apply_voice_buff(
    commands: &mut Commands,
    entity: Entity,
    status_query: &Query<Option<&StatusEffects>, (With<UnitMesh>, Without<Dead>)>,
) {
    // Insert VoiceBuffed marker
    commands.entity(entity).insert(VoiceBuffed);

    // Check if entity already has StatusEffects
    let has_status = status_query.get(entity).ok().flatten().is_some();

    let buff = StatusInstance {
        effect: StatusEffectId::SpeedBuff,
        remaining_ticks: BUFF_DURATION,
        stacks: 1,
        source: EntityId(0),
    };

    if has_status {
        // Entity already has StatusEffects — add the buff via deferred closure.
        // Also ensure StatModifiers exists (required by stat_modifier_system).
        commands
            .entity(entity)
            .queue(move |mut entity_world: EntityWorldMut| {
                if let Some(mut se) = entity_world.get_mut::<StatusEffects>() {
                    se.effects.push(buff);
                }
                if entity_world.get::<StatModifiers>().is_none() {
                    entity_world.insert(StatModifiers::default());
                }
            });
    } else {
        // Entity lacks StatusEffects — insert both StatusEffects and StatModifiers
        let mut effects = StatusEffects::default();
        effects.effects.push(buff);
        commands
            .entity(entity)
            .insert((effects, StatModifiers::default()));
    }
}

// ---------------------------------------------------------------------------
// Golden Tint System
// ---------------------------------------------------------------------------

/// Apply golden tint to voice-buffed units. Runs after render_selection_indicators.
/// Selected units keep their cyan selection tint.
pub fn voice_demo_buff_tint(
    mut sprite_units: Query<
        (&mut Sprite, Has<Selected>),
        (With<VoiceBuffed>, With<UnitMesh>, Without<Dead>),
    >,
) {
    for (mut sprite, is_selected) in sprite_units.iter_mut() {
        if !is_selected {
            // Preserve existing alpha (zoom LOD uses alpha=0 to hide in strategic mode)
            let alpha = sprite.color.alpha();
            sprite.color = Color::srgba(1.0, 0.85, 0.3, alpha);
        }
        // Selected units keep their cyan tint from render_selection_indicators
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convenience wrapper for set_tile with this module's map dimensions.
fn set_tile(
    tiles: &mut [TerrainType],
    elevation: &mut [u8],
    x: i32,
    y: i32,
    terrain: TerrainType,
    elev: u8,
) {
    crate::setup::set_tile(tiles, elevation, x, y, terrain, elev, MAP_WIDTH, MAP_HEIGHT);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_demo_mission_validates() {
        let mission = build_voice_demo_mission();
        mission.validate().unwrap_or_else(|e| {
            panic!("Voice demo mission validation failed: {e:?}");
        });
    }

    #[test]
    fn voice_demo_unit_counts() {
        let mission = build_voice_demo_mission();
        // 9 cat units in player_setup (4 Chonks + 5 Hissers)
        assert_eq!(mission.player_setup.units.len(), 9, "cat unit count");
        // 7 mouse units in enemy wave
        assert_eq!(mission.enemy_waves.len(), 1, "should have 1 enemy wave");
        assert_eq!(
            mission.enemy_waves[0].units.len(),
            7,
            "mouse unit count in wave"
        );
    }

    #[test]
    fn voice_demo_dialogue_and_triggers() {
        let mission = build_voice_demo_mission();
        assert_eq!(mission.dialogue.len(), 5, "should have 5 dialogue lines");
        assert_eq!(mission.triggers.len(), 5, "should have 5 triggers");

        // All dialogue from Le Chat with AiVoice style
        for line in &mission.dialogue {
            assert_eq!(line.speaker, "Le Chat");
            assert_eq!(line.voice_style, VoiceStyle::AiVoice);
        }
    }

    #[test]
    fn voice_demo_camera_centered() {
        let cam = voice_demo_camera();
        assert_eq!(cam.focus, GridPos::new(20, 15));
        assert!((cam.zoom - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn voice_demo_phase_ordering() {
        // Phases must be ordered for PartialOrd comparisons in the FSM
        assert!(VoiceDemoPhase::Setup < VoiceDemoPhase::Fallback);
        assert!(VoiceDemoPhase::Fallback < VoiceDemoPhase::Charge);
        assert!(VoiceDemoPhase::Charge < VoiceDemoPhase::Attack);
        assert!(VoiceDemoPhase::Attack < VoiceDemoPhase::Hold);
        assert!(VoiceDemoPhase::Hold < VoiceDemoPhase::PullBack);
        assert!(VoiceDemoPhase::PullBack < VoiceDemoPhase::Done);
    }
}
