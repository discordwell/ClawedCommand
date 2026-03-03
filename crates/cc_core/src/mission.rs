use serde::{Deserialize, Serialize};

use crate::components::{BuildingKind, UnitKind};
use crate::coords::GridPos;
use crate::hero::HeroId;
use crate::mutator::MissionMutator;
use crate::terrain::TerrainType;

// ---------------------------------------------------------------------------
// Mission Definition — RON-serializable campaign mission format
// ---------------------------------------------------------------------------

/// A complete mission definition, loadable from RON files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDefinition {
    /// Unique mission identifier (e.g. "prologue").
    pub id: String,
    /// Display name (e.g. "The Server in the River").
    pub name: String,
    /// Act number (0 = prologue).
    pub act: u32,
    /// Mission index within the act.
    pub mission_index: u32,
    /// Map configuration.
    pub map: MissionMap,
    /// Player starting setup.
    pub player_setup: PlayerSetup,
    /// Enemy wave definitions.
    pub enemy_waves: Vec<EnemyWave>,
    /// Mission objectives (primary = required, secondary = optional).
    pub objectives: Vec<MissionObjective>,
    /// Scripted triggers that fire based on game conditions.
    pub triggers: Vec<ScriptedTrigger>,
    /// All dialogue lines referenced by triggers.
    pub dialogue: Vec<DialogueLine>,
    /// Text shown on the briefing screen.
    pub briefing_text: String,
    /// Text shown after mission completion.
    pub debrief_text: String,
    /// AI tool tier allowed in this mission (None = default/all).
    #[serde(default)]
    pub ai_tool_tier: Option<u8>,
    /// What mission comes next.
    #[serde(default)]
    pub next_mission: NextMission,
    /// Gameplay mutators active during this mission.
    #[serde(default)]
    pub mutators: Vec<MissionMutator>,
}

/// Map source for a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionMap {
    /// Procedurally generated from a seed.
    Generated { seed: u64, width: u32, height: u32 },
    /// Inline tile data (row-major, TerrainType per tile).
    Inline {
        width: u32,
        height: u32,
        tiles: Vec<TerrainType>,
        /// Per-tile elevation (0-3). Same length as tiles.
        elevation: Vec<u8>,
    },
}

/// Initial player setup when the mission loads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSetup {
    /// Hero units to spawn for the player.
    pub heroes: Vec<HeroSpawn>,
    /// Regular units to spawn for the player.
    pub units: Vec<UnitSpawn>,
    /// Pre-built buildings for the player.
    pub buildings: Vec<BuildingSpawn>,
    /// Starting resources.
    pub starting_food: u32,
    pub starting_gpu: u32,
    pub starting_nfts: u32,
}

/// Where/how to spawn a hero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroSpawn {
    pub hero_id: HeroId,
    pub position: GridPos,
    /// If true, mission fails when this hero dies.
    pub mission_critical: bool,
    /// Which player owns this hero (default 0 for backward compat).
    #[serde(default)]
    pub player_id: u8,
}

/// Where/how to spawn a regular unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitSpawn {
    pub kind: UnitKind,
    pub position: GridPos,
    pub player_id: u8,
}

/// Where/how to spawn a building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSpawn {
    pub kind: BuildingKind,
    pub position: GridPos,
    pub player_id: u8,
    /// If true, building spawns already constructed.
    pub pre_built: bool,
}

// ---------------------------------------------------------------------------
// Enemy Waves
// ---------------------------------------------------------------------------

/// A group of enemies that spawn when triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyWave {
    /// Unique wave id for trigger references.
    pub wave_id: String,
    /// What triggers this wave to spawn.
    pub trigger: WaveTrigger,
    /// Units in this wave.
    pub units: Vec<UnitSpawn>,
    /// Default AI behavior for spawned units.
    pub ai_behavior: WaveAiBehavior,
}

/// What causes a wave to spawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaveTrigger {
    /// Spawn at a specific tick.
    AtTick(u64),
    /// Spawn when a trigger fires.
    OnTrigger(String),
    /// Spawn at mission start.
    Immediate,
}

/// Default behavior for wave-spawned enemies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaveAiBehavior {
    /// Move to target, attacking along the way.
    AttackMove(GridPos),
    /// Patrol between waypoints.
    Patrol(Vec<GridPos>),
    /// Hold position, attack in range only.
    Defend,
    /// Stand still until engaged.
    Idle,
}

// ---------------------------------------------------------------------------
// Objectives
// ---------------------------------------------------------------------------

/// A mission objective the player must complete (or avoid failing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionObjective {
    /// Unique objective id.
    pub id: String,
    /// Display text (e.g. "Defeat the Pack Leader").
    pub description: String,
    /// Is this required for mission success?
    pub primary: bool,
    /// What condition completes this objective.
    pub condition: ObjectiveCondition,
}

/// Conditions that complete (or fail) an objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveCondition {
    /// All enemies from a specific wave are dead.
    EliminateWave(String),
    /// All enemies on the map are dead.
    EliminateAll,
    /// Player has killed N enemies total.
    KillCount(u32),
    /// A specific hero reaches a grid position (within radius).
    HeroReachesPos {
        hero: HeroId,
        position: GridPos,
        radius: i32,
    },
    /// Survive for N ticks.
    Survive(u64),
    /// Fail condition: a specific hero has died.
    HeroDied(HeroId),
    /// Completed by a trigger action (CompleteObjective).
    Manual,
}

// ---------------------------------------------------------------------------
// Scripted Triggers
// ---------------------------------------------------------------------------

/// A condition → action pair evaluated each tick during a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptedTrigger {
    /// Unique trigger id.
    pub id: String,
    /// Condition that activates this trigger.
    pub condition: TriggerCondition,
    /// Actions to perform when condition is met.
    pub actions: Vec<TriggerAction>,
    /// If true, trigger fires only once then deactivates.
    pub once: bool,
}

/// Conditions that activate a trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Fires at a specific tick.
    AtTick(u64),
    /// Fires when a hero is within radius of a position.
    HeroAtPos {
        hero: HeroId,
        position: GridPos,
        radius: i32,
    },
    /// Fires when enemy kill count reaches N.
    EnemyKillCount(u32),
    /// Fires when all enemies are dead.
    AllEnemiesDead,
    /// Fires when a specific wave is fully eliminated.
    WaveEliminated(String),
    /// Fires when a flag has been set.
    FlagSet(String),
    /// Fires when another trigger has fired.
    TriggerFired(String),
    /// All sub-conditions must be true.
    All(Vec<TriggerCondition>),
    /// Any sub-condition must be true.
    Any(Vec<TriggerCondition>),
    /// A specific hero's HP is below a fraction (percentage).
    HeroHpBelow {
        hero: HeroId,
        /// HP percentage threshold (e.g. 50 = 50%).
        percentage: u32,
    },
    /// Fires when a persistent campaign flag is set.
    PersistentFlag(String),
    /// Fires when a hazard reaches a certain level.
    HazardLevel { hazard_type: String, level: u32 },
    /// Fires periodically at fixed intervals.
    Periodic {
        interval_ticks: u64,
        offset_ticks: u64,
    },
}

/// Actions performed when a trigger fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerAction {
    /// Show dialogue lines (by indices into mission dialogue array).
    ShowDialogue(Vec<usize>),
    /// Spawn an enemy wave by wave_id.
    SpawnWave(String),
    /// Set a narrative flag.
    SetFlag(String),
    /// Mark an objective as complete by objective_id.
    CompleteObjective(String),
    /// Pan the camera to a position.
    PanCamera(GridPos),
    /// Set a persistent campaign flag (survives across missions).
    SetPersistentFlag(String),
    /// Toggle a mutator on or off by its index in the mission's mutators vec.
    ToggleMutator { mutator_index: usize, active: bool },
    /// Set terrain type at specific positions.
    SetTerrain {
        positions: Vec<GridPos>,
        terrain: TerrainType,
    },
    /// Deal damage to all units in an area.
    AreaDamage {
        center: GridPos,
        radius: u32,
        damage: u32,
    },
}

/// What mission comes after this one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NextMission {
    /// A specific mission ID.
    Fixed(String),
    /// Branch based on a persistent flag.
    Branching {
        flag: String,
        on_true: String,
        on_false: String,
    },
    /// No next mission (end of campaign or handled externally).
    None,
}

impl Default for NextMission {
    fn default() -> Self {
        NextMission::None
    }
}

// ---------------------------------------------------------------------------
// Dialogue
// ---------------------------------------------------------------------------

/// A single line of dialogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueLine {
    /// Speaker name (e.g. "Kelpie", "Geppity").
    pub speaker: String,
    /// The dialogue text.
    pub text: String,
    /// Voice style affects rendering.
    pub voice_style: VoiceStyle,
    /// Portrait asset key (e.g. "portrait_kelpie"). Empty string = no portrait.
    pub portrait: String,
}

/// How dialogue is rendered/styled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceStyle {
    /// Normal character speech.
    Normal,
    /// AI voice — rendered with distortion/static effect.
    AiVoice,
    /// Whispered text — smaller, italic.
    Whisper,
    /// Shouted text — larger, bold.
    Shout,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

impl MissionDefinition {
    /// Validate internal consistency of the mission definition.
    /// All trigger action checks (dialogue indices, wave refs, objective refs)
    /// are performed in a single pass over triggers.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check map dimensions
        if let MissionMap::Inline {
            width,
            height,
            tiles,
            elevation,
        } = &self.map
        {
            let expected = (*width as usize) * (*height as usize);
            if tiles.len() != expected {
                errors.push(format!(
                    "Tile count {} != {}x{}={}",
                    tiles.len(),
                    width,
                    height,
                    expected
                ));
            }
            if elevation.len() != expected {
                errors.push(format!(
                    "Elevation count {} != {}x{}={}",
                    elevation.len(),
                    width,
                    height,
                    expected
                ));
            }
        }

        // Build lookup sets once
        let wave_ids: Vec<&str> = self
            .enemy_waves
            .iter()
            .map(|w| w.wave_id.as_str())
            .collect();
        let obj_ids: Vec<&str> = self.objectives.iter().map(|o| o.id.as_str()).collect();

        // Single pass over triggers: check dialogue indices, wave refs, objective refs,
        // and mutator index bounds.
        for trigger in &self.triggers {
            for action in &trigger.actions {
                match action {
                    TriggerAction::ShowDialogue(indices) => {
                        for &idx in indices {
                            if idx >= self.dialogue.len() {
                                errors.push(format!(
                                    "Trigger '{}' references dialogue index {} but only {} lines exist",
                                    trigger.id,
                                    idx,
                                    self.dialogue.len()
                                ));
                            }
                        }
                    }
                    TriggerAction::SpawnWave(wave_id) => {
                        if !wave_ids.contains(&wave_id.as_str()) {
                            errors.push(format!(
                                "Trigger '{}' references unknown wave '{}'",
                                trigger.id, wave_id
                            ));
                        }
                    }
                    TriggerAction::CompleteObjective(obj_id) => {
                        if !obj_ids.contains(&obj_id.as_str()) {
                            errors.push(format!(
                                "Trigger '{}' references unknown objective '{}'",
                                trigger.id, obj_id
                            ));
                        }
                    }
                    TriggerAction::ToggleMutator { mutator_index, .. } => {
                        if *mutator_index >= self.mutators.len() {
                            errors.push(format!(
                                "Trigger '{}' references mutator index {} but only {} mutators exist",
                                trigger.id,
                                mutator_index,
                                self.mutators.len()
                            ));
                        }
                    }
                    TriggerAction::SetFlag(_)
                    | TriggerAction::PanCamera(_)
                    | TriggerAction::SetPersistentFlag(_)
                    | TriggerAction::SetTerrain { .. }
                    | TriggerAction::AreaDamage { .. } => {}
                }
            }
        }

        // Must have at least one primary objective
        if !self.objectives.iter().any(|o| o.primary) {
            errors.push("Mission has no primary objectives".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_mission() -> MissionDefinition {
        MissionDefinition {
            id: "test".into(),
            name: "Test Mission".into(),
            act: 0,
            mission_index: 0,
            map: MissionMap::Inline {
                width: 2,
                height: 2,
                tiles: vec![TerrainType::Grass; 4],
                elevation: vec![0; 4],
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
            enemy_waves: vec![EnemyWave {
                wave_id: "wave1".into(),
                trigger: WaveTrigger::Immediate,
                units: vec![UnitSpawn {
                    kind: UnitKind::Nuisance,
                    position: GridPos::new(1, 1),
                    player_id: 1,
                }],
                ai_behavior: WaveAiBehavior::Idle,
            }],
            objectives: vec![MissionObjective {
                id: "obj1".into(),
                description: "Test objective".into(),
                primary: true,
                condition: ObjectiveCondition::EliminateAll,
            }],
            triggers: vec![ScriptedTrigger {
                id: "t1".into(),
                condition: TriggerCondition::AtTick(10),
                actions: vec![TriggerAction::ShowDialogue(vec![0])],
                once: true,
            }],
            dialogue: vec![DialogueLine {
                speaker: "Test".into(),
                text: "Hello".into(),
                voice_style: VoiceStyle::Normal,
                portrait: String::new(),
            }],
            briefing_text: "Test briefing".into(),
            debrief_text: "Test debrief".into(),
            ai_tool_tier: None,
            next_mission: NextMission::None,
            mutators: vec![],
        }
    }

    #[test]
    fn ron_round_trip() {
        let mission = minimal_mission();
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.name, "Test Mission");
        assert_eq!(parsed.objectives.len(), 1);
        assert_eq!(parsed.dialogue.len(), 1);
        assert!(parsed.mutators.is_empty());
    }

    #[test]
    fn validation_passes_for_valid_mission() {
        let mission = minimal_mission();
        assert!(mission.validate().is_ok());
    }

    #[test]
    fn validation_catches_bad_dialogue_index() {
        let mut mission = minimal_mission();
        mission.triggers[0].actions = vec![TriggerAction::ShowDialogue(vec![99])];
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("dialogue index 99")));
    }

    #[test]
    fn validation_catches_bad_wave_reference() {
        let mut mission = minimal_mission();
        mission.triggers[0].actions = vec![TriggerAction::SpawnWave("nonexistent".into())];
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown wave")));
    }

    #[test]
    fn validation_catches_bad_objective_reference() {
        let mut mission = minimal_mission();
        mission.triggers[0].actions = vec![TriggerAction::CompleteObjective("nonexistent".into())];
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown objective")));
    }

    #[test]
    fn validation_catches_no_primary_objectives() {
        let mut mission = minimal_mission();
        mission.objectives[0].primary = false;
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("no primary objectives")));
    }

    #[test]
    fn validation_catches_wrong_tile_count() {
        let mut mission = minimal_mission();
        mission.map = MissionMap::Inline {
            width: 3,
            height: 3,
            tiles: vec![TerrainType::Grass; 4],
            elevation: vec![0; 9],
        };
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("Tile count")));
    }

    #[test]
    fn voice_style_serializes() {
        let line = DialogueLine {
            speaker: "Geppity".into(),
            text: "Hello!".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_geppity".into(),
        };
        let ron_str = ron::to_string(&line).unwrap();
        let parsed: DialogueLine = ron::from_str(&ron_str).unwrap();
        assert_eq!(parsed.voice_style, VoiceStyle::AiVoice);
    }

    #[test]
    fn validate_single_pass_catches_all_action_errors() {
        let mut mission = minimal_mission();
        mission.triggers = vec![ScriptedTrigger {
            id: "multi_error".into(),
            condition: TriggerCondition::AtTick(1),
            actions: vec![
                TriggerAction::ShowDialogue(vec![42]),
                TriggerAction::SpawnWave("phantom_wave".into()),
                TriggerAction::CompleteObjective("phantom_obj".into()),
            ],
            once: true,
        }];
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("dialogue index 42")));
        assert!(
            errs.iter()
                .any(|e| e.contains("unknown wave 'phantom_wave'"))
        );
        assert!(
            errs.iter()
                .any(|e| e.contains("unknown objective 'phantom_obj'"))
        );
        assert_eq!(errs.len(), 3);
    }

    #[test]
    fn next_mission_ron_round_trip() {
        let mut mission = minimal_mission();
        mission.next_mission = NextMission::Fixed("act1_m2".into());
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        assert!(matches!(parsed.next_mission, NextMission::Fixed(id) if id == "act1_m2"));
    }

    #[test]
    fn next_mission_branching_round_trip() {
        let mut mission = minimal_mission();
        mission.next_mission = NextMission::Branching {
            flag: "helped_rex".into(),
            on_true: "act3_ally".into(),
            on_false: "act3_rival".into(),
        };
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        match &parsed.next_mission {
            NextMission::Branching {
                flag,
                on_true,
                on_false,
            } => {
                assert_eq!(flag, "helped_rex");
                assert_eq!(on_true, "act3_ally");
                assert_eq!(on_false, "act3_rival");
            }
            _ => panic!("Expected Branching"),
        }
    }

    #[test]
    fn persistent_flag_condition_serializes() {
        let cond = TriggerCondition::PersistentFlag("murder_alliance".into());
        let ron_str = ron::to_string(&cond).unwrap();
        let parsed: TriggerCondition = ron::from_str(&ron_str).unwrap();
        assert!(matches!(parsed, TriggerCondition::PersistentFlag(s) if s == "murder_alliance"));
    }

    #[test]
    fn set_persistent_flag_action_serializes() {
        let action = TriggerAction::SetPersistentFlag("helped_rex".into());
        let ron_str = ron::to_string(&action).unwrap();
        let parsed: TriggerAction = ron::from_str(&ron_str).unwrap();
        assert!(matches!(parsed, TriggerAction::SetPersistentFlag(s) if s == "helped_rex"));
    }

    #[test]
    fn ai_tool_tier_defaults_to_none() {
        let mission = minimal_mission();
        assert!(mission.ai_tool_tier.is_none());
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        assert!(parsed.ai_tool_tier.is_none());
    }

    #[test]
    fn mutators_default_empty() {
        let ron_str = r#"(
            id: "test",
            name: "Test",
            act: 0,
            mission_index: 0,
            map: Generated(seed: 1, width: 32, height: 32),
            player_setup: (heroes: [], units: [], buildings: [], starting_food: 0, starting_gpu: 0, starting_nfts: 0),
            enemy_waves: [],
            objectives: [(id: "obj", description: "Win", primary: true, condition: EliminateAll)],
            triggers: [],
            dialogue: [],
            briefing_text: "",
            debrief_text: "",
        )"#;
        let parsed: MissionDefinition = ron::from_str(ron_str).unwrap();
        assert!(parsed.mutators.is_empty());
    }

    #[test]
    fn mission_with_mutators_round_trip() {
        use crate::mutator::MissionMutator;
        let mut mission = minimal_mission();
        mission.mutators = vec![
            MissionMutator::TimeLimit {
                max_ticks: 3000,
                warning_at: 2500,
            },
            MissionMutator::NoBuildMode,
        ];
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        assert_eq!(parsed.mutators.len(), 2);
    }

    #[test]
    fn validation_catches_bad_mutator_index() {
        use crate::mutator::MissionMutator;
        let mut mission = minimal_mission();
        mission.mutators = vec![MissionMutator::NoBuildMode];
        mission.triggers[0].actions = vec![TriggerAction::ToggleMutator {
            mutator_index: 5,
            active: false,
        }];
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("mutator index 5")));
    }

    #[test]
    fn validation_accepts_valid_mutator_index() {
        use crate::mutator::MissionMutator;
        let mut mission = minimal_mission();
        mission.mutators = vec![MissionMutator::NoBuildMode, MissionMutator::NoAiControl];
        mission.triggers[0].actions = vec![TriggerAction::ToggleMutator {
            mutator_index: 1,
            active: true,
        }];
        assert!(mission.validate().is_ok());
    }

    #[test]
    fn new_trigger_conditions_serialize() {
        let cond = TriggerCondition::Periodic {
            interval_ticks: 100,
            offset_ticks: 10,
        };
        let ron_str = ron::to_string(&cond).unwrap();
        let parsed: TriggerCondition = ron::from_str(&ron_str).unwrap();
        assert!(matches!(
            parsed,
            TriggerCondition::Periodic {
                interval_ticks: 100,
                ..
            }
        ));

        let cond2 = TriggerCondition::HazardLevel {
            hazard_type: "lava".into(),
            level: 3,
        };
        let ron_str2 = ron::to_string(&cond2).unwrap();
        let parsed2: TriggerCondition = ron::from_str(&ron_str2).unwrap();
        assert!(matches!(
            parsed2,
            TriggerCondition::HazardLevel { level: 3, .. }
        ));
    }

    #[test]
    fn new_trigger_actions_serialize() {
        let action = TriggerAction::ToggleMutator {
            mutator_index: 0,
            active: true,
        };
        let ron_str = ron::to_string(&action).unwrap();
        let parsed: TriggerAction = ron::from_str(&ron_str).unwrap();
        assert!(matches!(
            parsed,
            TriggerAction::ToggleMutator {
                mutator_index: 0,
                active: true
            }
        ));

        let action2 = TriggerAction::SetTerrain {
            positions: vec![GridPos::new(1, 2)],
            terrain: TerrainType::Rock,
        };
        let ron_str2 = ron::to_string(&action2).unwrap();
        let parsed2: TriggerAction = ron::from_str(&ron_str2).unwrap();
        assert!(matches!(parsed2, TriggerAction::SetTerrain { .. }));

        let action3 = TriggerAction::AreaDamage {
            center: GridPos::new(5, 5),
            radius: 3,
            damage: 50,
        };
        let ron_str3 = ron::to_string(&action3).unwrap();
        let parsed3: TriggerAction = ron::from_str(&ron_str3).unwrap();
        assert!(matches!(
            parsed3,
            TriggerAction::AreaDamage { damage: 50, .. }
        ));
    }

    #[test]
    fn parse_act5_m1_grotto_assembly_ron() {
        let ron_str = include_str!("../../../assets/campaign/act5_m1_grotto_assembly.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act5_m1_grotto_assembly.ron");
        assert_eq!(mission.id, "act5_m1_grotto_assembly");
        assert_eq!(mission.act, 5);
        assert_eq!(mission.mission_index, 1);
        // Two heroes: Kelpie and TheEternal
        assert_eq!(mission.player_setup.heroes.len(), 2);
        // Four enemy waves
        assert_eq!(mission.enemy_waves.len(), 4);
        // Three objectives
        assert_eq!(mission.objectives.len(), 3);
        // Two mutators: Flooding + TimeLimit
        assert_eq!(mission.mutators.len(), 2);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::Flooding { .. }
        ));
        assert!(matches!(
            mission.mutators[1],
            crate::mutator::MissionMutator::TimeLimit { .. }
        ));
        // Triggers include flood activation and wave spawning
        assert!(mission.triggers.len() >= 8);
        // Dialogue lines
        assert_eq!(mission.dialogue.len(), 21);
        // Validate internal consistency
        mission.validate().expect("Mission validation failed");
    }

    #[test]
    fn parse_demo_canyon_ron() {
        let ron_str = include_str!("../../../assets/campaign/demo_canyon.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse demo_canyon.ron");
        assert_eq!(mission.id, "demo_canyon");
        assert_eq!(mission.name, "Canyon Battle");
        // Inline map: 80x48 = 3840 tiles
        match &mission.map {
            MissionMap::Inline {
                width,
                height,
                tiles,
                elevation,
            } => {
                assert_eq!(*width, 80);
                assert_eq!(*height, 48);
                assert_eq!(tiles.len(), 80 * 48);
                assert_eq!(elevation.len(), 80 * 48);
            }
            _ => panic!("Expected Inline map"),
        }
        // No heroes, no player units
        assert!(mission.player_setup.heroes.is_empty());
        assert!(mission.player_setup.units.is_empty());
        // Two buildings (TheBox for P0, TheBurrow for P1)
        assert_eq!(mission.player_setup.buildings.len(), 2);
        assert_eq!(
            mission.player_setup.buildings[0].kind,
            crate::components::BuildingKind::TheBox
        );
        assert_eq!(mission.player_setup.buildings[0].player_id, 0);
        assert_eq!(
            mission.player_setup.buildings[1].kind,
            crate::components::BuildingKind::TheBurrow
        );
        assert_eq!(mission.player_setup.buildings[1].player_id, 1);
        // Two waves (P0 army, P1 army)
        assert_eq!(mission.enemy_waves.len(), 2);
        assert_eq!(mission.enemy_waves[0].wave_id, "p0_army");
        assert_eq!(mission.enemy_waves[1].wave_id, "p1_army");
        // P0 army: 12 cat units
        assert_eq!(mission.enemy_waves[0].units.len(), 12);
        // P1 army: 14 mouse units
        assert_eq!(mission.enemy_waves[1].units.len(), 14);
        // Single EliminateAll objective
        assert_eq!(mission.objectives.len(), 1);
        assert!(mission.objectives[0].primary);
        assert!(matches!(
            mission.objectives[0].condition,
            ObjectiveCondition::EliminateAll
        ));
        // Zero resources
        assert_eq!(mission.player_setup.starting_food, 0);
        assert_eq!(mission.player_setup.starting_gpu, 0);
        assert_eq!(mission.player_setup.starting_nfts, 0);
        // Validates successfully
        mission.validate().expect("Demo canyon validation failed");
    }

    #[test]
    fn demo_canyon_map_terrain_distribution() {
        let ron_str = include_str!("../../../assets/campaign/demo_canyon.ron");
        let mission: MissionDefinition = ron::from_str(ron_str).unwrap();
        let MissionMap::Inline {
            tiles, elevation, ..
        } = &mission.map
        else {
            panic!("Expected Inline map");
        };
        // Rock walls exist (top and bottom rows)
        let rock_count = tiles.iter().filter(|t| **t == TerrainType::Rock).count();
        assert!(
            rock_count > 400,
            "Should have substantial rock walls, got {rock_count}"
        );
        // Water river exists
        let water_count = tiles.iter().filter(|t| **t == TerrainType::Water).count();
        assert!(
            water_count > 100,
            "Should have a river, got {water_count} water tiles"
        );
        // Shallows crossings exist
        let shallows_count = tiles
            .iter()
            .filter(|t| **t == TerrainType::Shallows)
            .count();
        assert!(
            shallows_count > 10,
            "Should have ford crossings, got {shallows_count} shallows"
        );
        // Road bridges exist
        let road_count = tiles.iter().filter(|t| **t == TerrainType::Road).count();
        assert!(
            road_count > 10,
            "Should have road bridges, got {road_count} road tiles"
        );
        // Elevation levels present
        let max_elev = *elevation.iter().max().unwrap();
        assert_eq!(max_elev, 2, "Should have elevation 2 for rock walls");
        let elev_1_count = elevation.iter().filter(|e| **e == 1).count();
        assert!(
            elev_1_count > 1000,
            "Plateaus should be elevation 1, got {elev_1_count}"
        );
    }

    #[test]
    fn parse_act3_m9_jinx_ron() {
        let ron_str = include_str!("../../../assets/campaign/act3_m9_jinx.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act3_m9_jinx.ron");
        assert_eq!(mission.id, "act3_m9_jinx");
        assert_eq!(mission.act, 3);
        assert_eq!(mission.mission_index, 9);
        assert_eq!(mission.player_setup.heroes.len(), 1);
        assert_eq!(mission.enemy_waves.len(), 3);
        assert_eq!(mission.objectives.len(), 2);
        assert_eq!(mission.mutators.len(), 3);
        assert_eq!(mission.dialogue.len(), 12);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::WindStorm { .. }
        ));
        assert!(matches!(
            mission.mutators[1],
            crate::mutator::MissionMutator::RestrictedUnits { .. }
        ));
        assert!(matches!(
            mission.mutators[2],
            crate::mutator::MissionMutator::NoBuildMode
        ));
        assert!(
            matches!(mission.next_mission, NextMission::Fixed(ref id) if id == "act3_m10_llama_perimeter")
        );
        mission
            .validate()
            .expect("Mission validation failed for act3_m9_jinx");
    }

    #[test]
    fn parse_act3_m10_llama_perimeter_ron() {
        let ron_str = include_str!("../../../assets/campaign/act3_m10_llama_perimeter.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act3_m10_llama_perimeter.ron");
        assert_eq!(mission.id, "act3_m10_llama_perimeter");
        assert_eq!(mission.act, 3);
        assert_eq!(mission.mission_index, 10);
        assert_eq!(mission.player_setup.heroes.len(), 2);
        assert_eq!(mission.enemy_waves.len(), 5);
        assert_eq!(mission.objectives.len(), 3);
        assert_eq!(mission.mutators.len(), 2);
        assert_eq!(mission.dialogue.len(), 15);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::ResourceScarcity { .. }
        ));
        assert!(matches!(
            mission.mutators[1],
            crate::mutator::MissionMutator::SpeedMultiplier { .. }
        ));
        assert!(
            matches!(mission.next_mission, NextMission::Fixed(ref id) if id == "act3_m11_junkyard")
        );
        mission
            .validate()
            .expect("Mission validation failed for act3_m10_llama_perimeter");
    }

    #[test]
    fn parse_act3_m11_junkyard_ron() {
        let ron_str = include_str!("../../../assets/campaign/act3_m11_junkyard.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act3_m11_junkyard.ron");
        assert_eq!(mission.id, "act3_m11_junkyard");
        assert_eq!(mission.act, 3);
        assert_eq!(mission.mission_index, 11);
        assert_eq!(mission.player_setup.heroes.len(), 2);
        assert_eq!(mission.enemy_waves.len(), 5);
        assert_eq!(mission.objectives.len(), 3);
        assert_eq!(mission.mutators.len(), 3);
        assert_eq!(mission.dialogue.len(), 15);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::DamageZone { .. }
        ));
        assert!(matches!(
            mission.mutators[1],
            crate::mutator::MissionMutator::Tremors { .. }
        ));
        assert!(matches!(
            mission.mutators[2],
            crate::mutator::MissionMutator::AiOnlyControl { tool_tier: 2 }
        ));
        assert!(
            matches!(mission.next_mission, NextMission::Fixed(ref id) if id == "act3_m12_rexs_truth")
        );
        mission
            .validate()
            .expect("Mission validation failed for act3_m11_junkyard");
    }

    #[test]
    fn parse_act3_m12_rexs_truth_ron() {
        let ron_str = include_str!("../../../assets/campaign/act3_m12_rexs_truth.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act3_m12_rexs_truth.ron");
        assert_eq!(mission.id, "act3_m12_rexs_truth");
        assert_eq!(mission.act, 3);
        assert_eq!(mission.mission_index, 12);
        assert_eq!(mission.player_setup.heroes.len(), 2);
        assert_eq!(mission.enemy_waves.len(), 2);
        assert_eq!(mission.objectives.len(), 2);
        assert_eq!(mission.mutators.len(), 1);
        assert_eq!(mission.dialogue.len(), 20);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::VoiceOnlyControl {
                ai_enabled: true,
                ..
            }
        ));
        assert!(matches!(
            mission.next_mission,
            NextMission::Branching { .. }
        ));
        mission
            .validate()
            .expect("Mission validation failed for act3_m12_rexs_truth");
    }

    #[test]
    fn parse_act3_m13_escape_parliament_ron() {
        let ron_str = include_str!("../../../assets/campaign/act3_m13_escape_parliament.ron");
        let mission: MissionDefinition =
            ron::from_str(ron_str).expect("Failed to parse act3_m13_escape_parliament.ron");
        assert_eq!(mission.id, "act3_m13_escape_parliament");
        assert_eq!(mission.act, 3);
        assert_eq!(mission.mission_index, 13);
        assert_eq!(mission.player_setup.heroes.len(), 1);
        assert_eq!(mission.enemy_waves.len(), 4);
        assert_eq!(mission.objectives.len(), 2);
        assert_eq!(mission.mutators.len(), 2);
        assert_eq!(mission.dialogue.len(), 12);
        assert!(matches!(
            mission.mutators[0],
            crate::mutator::MissionMutator::LavaRise { .. }
        ));
        assert!(matches!(
            mission.mutators[1],
            crate::mutator::MissionMutator::SpeedMultiplier { .. }
        ));
        assert!(
            matches!(mission.next_mission, NextMission::Fixed(ref id) if id == "act4_m14_junkyard_fort")
        );
        mission
            .validate()
            .expect("Mission validation failed for act3_m13_escape_parliament");
    }

    #[test]
    fn hero_spawn_player_id_default_zero() {
        let ron_str = r#"(
            hero_id: Kelpie,
            position: (x: 5, y: 5),
            mission_critical: false,
        )"#;
        let parsed: HeroSpawn = ron::from_str(ron_str).unwrap();
        assert_eq!(parsed.player_id, 0);
    }

    #[test]
    fn hero_spawn_player_id_round_trip() {
        let spawn = HeroSpawn {
            hero_id: HeroId::KingRingtail,
            position: GridPos::new(10, 20),
            mission_critical: false,
            player_id: 1,
        };
        let ron_str = ron::to_string(&spawn).unwrap();
        let parsed: HeroSpawn = ron::from_str(&ron_str).unwrap();
        assert_eq!(parsed.player_id, 1);
        assert_eq!(parsed.hero_id, HeroId::KingRingtail);
    }

    // -----------------------------------------------------------------------
    // Campaign map tests: Prologue + Pond Defense
    // -----------------------------------------------------------------------

    /// Helper: check that a position's terrain is passable (not Rock or Water).
    fn assert_passable(tiles: &[TerrainType], width: u32, x: i32, y: i32, label: &str) {
        let idx = (y as u32 * width + x as u32) as usize;
        let t = tiles[idx];
        assert!(
            t != TerrainType::Rock && t != TerrainType::Water,
            "{label}: position ({x},{y}) is on impassable {t:?}"
        );
    }

    fn load_prologue() -> MissionDefinition {
        let ron_str = include_str!("../../../assets/campaign/prologue.ron");
        ron::from_str(ron_str).expect("Failed to parse prologue.ron")
    }

    fn load_pond_defense() -> MissionDefinition {
        let ron_str = include_str!("../../../assets/campaign/act1_m1_pond_defense.ron");
        ron::from_str(ron_str).expect("Failed to parse act1_m1_pond_defense.ron")
    }

    /// Assert all heroes, player units, and wave positions are on passable terrain.
    fn assert_all_positions_passable(mission: &MissionDefinition) {
        let MissionMap::Inline { width, tiles, .. } = &mission.map else {
            panic!("Expected Inline map");
        };
        let w = *width;
        for hero in &mission.player_setup.heroes {
            assert_passable(
                tiles,
                w,
                hero.position.x,
                hero.position.y,
                &format!("hero {:?}", hero.hero_id),
            );
        }
        for unit in &mission.player_setup.units {
            assert_passable(
                tiles,
                w,
                unit.position.x,
                unit.position.y,
                &format!("player unit {:?}", unit.kind),
            );
        }
        for wave in &mission.enemy_waves {
            for unit in &wave.units {
                assert_passable(
                    tiles,
                    w,
                    unit.position.x,
                    unit.position.y,
                    &format!("wave '{}' unit", wave.wave_id),
                );
            }
            if let WaveAiBehavior::AttackMove(pos) = &wave.ai_behavior {
                assert_passable(
                    tiles,
                    w,
                    pos.x,
                    pos.y,
                    &format!("wave '{}' attack_move target", wave.wave_id),
                );
            }
        }
    }

    #[test]
    fn parse_prologue_ron() {
        let mission = load_prologue();
        assert_eq!(mission.id, "prologue");
        assert_eq!(mission.act, 0);
        assert_eq!(mission.mission_index, 0);
        assert_eq!(mission.player_setup.heroes.len(), 1);
        assert_eq!(mission.player_setup.heroes[0].hero_id, HeroId::Kelpie);
        assert!(mission.player_setup.heroes[0].mission_critical);
        assert_eq!(mission.enemy_waves.len(), 3);
        assert_eq!(mission.enemy_waves[0].wave_id, "initial_ferals");
        assert_eq!(mission.enemy_waves[1].wave_id, "flanking_wave");
        assert_eq!(mission.enemy_waves[2].wave_id, "pack_leader");
        assert_eq!(mission.objectives.len(), 3);
        assert_eq!(mission.dialogue.len(), 30);
        assert_eq!(mission.triggers.len(), 11);
        mission.validate().expect("Prologue validation failed");
    }

    #[test]
    fn prologue_has_inline_map() {
        let mission = load_prologue();
        match &mission.map {
            MissionMap::Inline {
                width,
                height,
                tiles,
                elevation,
            } => {
                assert_eq!(*width, 48);
                assert_eq!(*height, 48);
                assert_eq!(tiles.len(), 48 * 48);
                assert_eq!(elevation.len(), 48 * 48);
            }
            _ => panic!("Expected Inline map for prologue"),
        }
    }

    #[test]
    fn prologue_terrain_distribution() {
        let mission = load_prologue();
        let MissionMap::Inline {
            tiles, elevation, ..
        } = &mission.map
        else {
            panic!("Expected Inline map");
        };
        let rock_count = tiles.iter().filter(|t| **t == TerrainType::Rock).count();
        assert!(
            rock_count > 300,
            "Should have rock border, got {rock_count}"
        );
        let water_count = tiles.iter().filter(|t| **t == TerrainType::Water).count();
        assert!(
            water_count > 50,
            "Should have river water, got {water_count}"
        );
        let shallows_count = tiles
            .iter()
            .filter(|t| **t == TerrainType::Shallows)
            .count();
        assert!(
            shallows_count > 40,
            "Should have ford crossings, got {shallows_count}"
        );
        let ruins_count = tiles
            .iter()
            .filter(|t| **t == TerrainType::TechRuins)
            .count();
        assert!(
            ruins_count >= 6,
            "Should have tech ruins, got {ruins_count}"
        );
        let max_elev = *elevation.iter().max().unwrap();
        assert_eq!(max_elev, 2, "Should have elevation 2 for rock walls");
        let elev_1_count = elevation.iter().filter(|e| **e == 1).count();
        assert!(
            elev_1_count > 400,
            "East bank should be elevation 1, got {elev_1_count}"
        );
    }

    #[test]
    fn prologue_positions_on_passable_terrain() {
        let mission = load_prologue();
        assert_all_positions_passable(&mission);
    }

    #[test]
    fn parse_pond_defense_ron() {
        let mission = load_pond_defense();
        assert_eq!(mission.id, "act1_m1_pond_defense");
        assert_eq!(mission.act, 1);
        assert_eq!(mission.mission_index, 1);
        assert_eq!(mission.player_setup.heroes.len(), 2);
        assert_eq!(mission.player_setup.units.len(), 6);
        assert_eq!(mission.enemy_waves.len(), 3);
        assert_eq!(mission.objectives.len(), 2);
        assert_eq!(mission.dialogue.len(), 18);
        assert_eq!(mission.player_setup.starting_food, 100);
        mission.validate().expect("Pond defense validation failed");
    }

    #[test]
    fn pond_defense_has_inline_map() {
        let mission = load_pond_defense();
        match &mission.map {
            MissionMap::Inline {
                width,
                height,
                tiles,
                elevation,
            } => {
                assert_eq!(*width, 48);
                assert_eq!(*height, 48);
                assert_eq!(tiles.len(), 48 * 48);
                assert_eq!(elevation.len(), 48 * 48);
            }
            _ => panic!("Expected Inline map for pond defense"),
        }
    }

    #[test]
    fn pond_defense_terrain_distribution() {
        let mission = load_pond_defense();
        let MissionMap::Inline {
            tiles, elevation, ..
        } = &mission.map
        else {
            panic!("Expected Inline map");
        };
        let rock_count = tiles.iter().filter(|t| **t == TerrainType::Rock).count();
        assert!(
            rock_count > 300,
            "Should have rock border, got {rock_count}"
        );
        let water_count = tiles.iter().filter(|t| **t == TerrainType::Water).count();
        assert!(
            water_count >= 18,
            "Should have pond water, got {water_count}"
        );
        let shallows_count = tiles
            .iter()
            .filter(|t| **t == TerrainType::Shallows)
            .count();
        assert!(
            shallows_count > 20,
            "Should have shallows rings, got {shallows_count}"
        );
        let forest_count = tiles.iter().filter(|t| **t == TerrainType::Forest).count();
        assert!(
            forest_count > 200,
            "Should have forest corridors, got {forest_count}"
        );
        let ruins_count = tiles
            .iter()
            .filter(|t| **t == TerrainType::TechRuins)
            .count();
        assert!(
            ruins_count >= 9,
            "Should have tech ruins, got {ruins_count}"
        );
        let max_elev = *elevation.iter().max().unwrap();
        assert_eq!(max_elev, 2, "Should have elevation 2 for rock walls");
        let elev_1_count = elevation.iter().filter(|e| **e == 1).count();
        assert!(
            elev_1_count > 200,
            "Player base should be elevation 1, got {elev_1_count}"
        );
    }

    #[test]
    fn pond_defense_positions_on_passable_terrain() {
        let mission = load_pond_defense();
        assert_all_positions_passable(&mission);
    }
}
