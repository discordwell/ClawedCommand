use serde::{Deserialize, Serialize};

use crate::components::{BuildingKind, UnitKind};
use crate::coords::GridPos;
use crate::hero::HeroId;
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
}

/// Map source for a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionMap {
    /// Procedurally generated from a seed.
    Generated {
        seed: u64,
        width: u32,
        height: u32,
    },
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
    /// Speaker name (e.g. "Kelpie", "Minstral").
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
        let wave_ids: Vec<&str> = self.enemy_waves.iter().map(|w| w.wave_id.as_str()).collect();
        let obj_ids: Vec<&str> = self.objectives.iter().map(|o| o.id.as_str()).collect();

        // Single pass over triggers: check dialogue indices, wave refs, and objective refs
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
                    TriggerAction::SetFlag(_)
                    | TriggerAction::PanCamera(_)
                    | TriggerAction::SetPersistentFlag(_) => {}
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
            tiles: vec![TerrainType::Grass; 4], // should be 9
            elevation: vec![0; 9],
        };
        let errs = mission.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("Tile count")));
    }

    #[test]
    fn voice_style_serializes() {
        let line = DialogueLine {
            speaker: "Minstral".into(),
            text: "Hello!".into(),
            voice_style: VoiceStyle::AiVoice,
            portrait: "portrait_minstral".into(),
        };
        let ron_str = ron::to_string(&line).unwrap();
        let parsed: DialogueLine = ron::from_str(&ron_str).unwrap();
        assert_eq!(parsed.voice_style, VoiceStyle::AiVoice);
    }

    #[test]
    fn validate_single_pass_catches_all_action_errors() {
        // Verify all three error types (dialogue, wave, objective) are caught
        // in a single pass rather than requiring separate iterations.
        let mut mission = minimal_mission();
        mission.triggers = vec![ScriptedTrigger {
            id: "multi_error".into(),
            condition: TriggerCondition::AtTick(1),
            actions: vec![
                TriggerAction::ShowDialogue(vec![42]),           // bad dialogue
                TriggerAction::SpawnWave("phantom_wave".into()), // bad wave
                TriggerAction::CompleteObjective("phantom_obj".into()), // bad objective
            ],
            once: true,
        }];
        let errs = mission.validate().unwrap_err();
        // All three errors should be caught
        assert!(errs.iter().any(|e| e.contains("dialogue index 42")));
        assert!(errs.iter().any(|e| e.contains("unknown wave 'phantom_wave'")));
        assert!(errs.iter().any(|e| e.contains("unknown objective 'phantom_obj'")));
        assert_eq!(errs.len(), 3, "Should find exactly 3 errors from the single trigger");
    }

    #[test]
    fn next_mission_ron_round_trip() {
        let mut mission = minimal_mission();
        mission.next_mission = NextMission::Fixed("act1_m2".into());
        let ron_str =
            ron::ser::to_string_pretty(&mission, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: MissionDefinition = ron::from_str(&ron_str).unwrap();
        assert!(matches!(parsed.next_mission, NextMission::Fixed(ref id) if id == "act1_m2"));
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
            NextMission::Branching { flag, on_true, on_false } => {
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
        assert!(matches!(parsed, TriggerCondition::PersistentFlag(ref s) if s == "murder_alliance"));
    }

    #[test]
    fn set_persistent_flag_action_serializes() {
        let action = TriggerAction::SetPersistentFlag("helped_rex".into());
        let ron_str = ron::to_string(&action).unwrap();
        let parsed: TriggerAction = ron::from_str(&ron_str).unwrap();
        assert!(matches!(parsed, TriggerAction::SetPersistentFlag(ref s) if s == "helped_rex"));
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
}
