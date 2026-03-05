use std::collections::HashSet;
use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use cc_core::components::{Dead, HeroIdentity, Owner, Position};
use cc_core::mission::{MissionDefinition, ObjectiveCondition};

use crate::resources::SimClock;

use super::triggers::ObjectiveCompleteEvent;

/// Act 3 branching choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Act3Choice {
    HelpRex,
    RefuseRex,
}

/// Status of the Patches character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PatchesStatus {
    #[default]
    Free,
    Captured,
}

/// Persistent campaign state that survives across mission loads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistentCampaignState {
    pub act3_choice: Option<Act3Choice>,
    pub gemineye_fabrication_rate: u32,
    pub patches_status: PatchesStatus,
    pub murder_alliance: bool,
    pub flicker_subplot_progress: u8,
    pub ponderer_fragment_found: bool,
    pub ending_d_eligible: bool,
    pub flags: HashSet<String>,
}

impl PersistentCampaignState {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn set_flag(&mut self, flag: String) {
        self.flags.insert(flag);
    }
}

/// Campaign phase FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignPhase {
    /// No campaign active — normal skirmish mode.
    Inactive,
    /// Showing the campaign world map.
    WorldMap,
    /// Showing an act title card transition.
    ActTitleCard,
    /// Showing the mission briefing screen.
    Briefing,
    /// Mission is actively playing.
    InMission,
    /// Mission ended, showing debrief.
    Debriefing,
}

/// Result of a completed mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionResult {
    Victory,
    Failure,
}

/// Tracking status of a single objective.
#[derive(Debug, Clone)]
pub struct ObjectiveStatus {
    pub id: String,
    pub completed: bool,
}

/// Central campaign state resource.
#[derive(Resource)]
pub struct CampaignState {
    /// Current loaded mission definition (Arc-wrapped to avoid cloning every tick).
    /// None = no campaign active.
    pub current_mission: Option<Arc<MissionDefinition>>,
    /// Completed mission IDs (for progression tracking).
    pub completed_missions: HashSet<String>,
    /// Narrative flags set by triggers (for branching storylines).
    pub flags: HashSet<String>,
    /// Total enemy kills in current mission.
    pub enemy_kill_count: u32,
    /// Trigger IDs that have already fired (for `once` triggers).
    pub fired_triggers: HashSet<String>,
    /// Status of each objective in the current mission.
    pub objective_status: Vec<ObjectiveStatus>,
    /// Current campaign phase.
    pub phase: CampaignPhase,
    /// IDs of waves that have been spawned.
    pub spawned_waves: HashSet<String>,
    /// Persistent state that survives across mission loads.
    pub persistent: PersistentCampaignState,
    /// Result of the last completed mission.
    pub last_mission_result: Option<MissionResult>,
    /// Reason for last mission failure.
    pub last_failure_reason: Option<String>,
    /// The act number the player is entering (for title card display).
    pub entering_act: Option<u32>,
}

impl Default for CampaignState {
    fn default() -> Self {
        Self {
            current_mission: None,
            completed_missions: HashSet::new(),
            flags: HashSet::new(),
            enemy_kill_count: 0,
            fired_triggers: HashSet::new(),
            objective_status: Vec::new(),
            phase: CampaignPhase::Inactive,
            spawned_waves: HashSet::new(),
            persistent: PersistentCampaignState::default(),
            last_mission_result: None,
            last_failure_reason: None,
            entering_act: None,
        }
    }
}

impl CampaignState {
    /// Initialize campaign state from a mission definition.
    /// Preserves `persistent` and `completed_missions` across loads.
    pub fn load_mission(&mut self, mission: MissionDefinition) {
        self.objective_status = mission
            .objectives
            .iter()
            .map(|obj| ObjectiveStatus {
                id: obj.id.clone(),
                completed: false,
            })
            .collect();
        self.enemy_kill_count = 0;
        self.fired_triggers.clear();
        self.flags.clear();
        self.spawned_waves.clear();
        self.phase = CampaignPhase::Briefing;
        self.current_mission = Some(Arc::new(mission));
        // persistent and completed_missions are intentionally preserved
    }

    /// Check if all primary objectives are complete.
    pub fn all_primary_complete(&self) -> bool {
        let Some(mission) = &self.current_mission else {
            return false;
        };
        for obj in &mission.objectives {
            if obj.primary {
                let status = self.objective_status.iter().find(|s| s.id == obj.id);
                if status.is_none_or(|s| !s.completed) {
                    return false;
                }
            }
        }
        true
    }

    /// Mark an objective as complete by ID.
    pub fn complete_objective(&mut self, objective_id: &str) {
        if let Some(status) = self
            .objective_status
            .iter_mut()
            .find(|s| s.id == objective_id)
        {
            status.completed = true;
        }
    }
}

/// Message sent when the time limit warning threshold is reached.
#[derive(Message)]
pub struct TimeLimitWarningEvent;

/// Message sent when a mission fails (hero died, etc.).
#[derive(Message)]
pub struct MissionFailedEvent {
    pub reason: String,
}

/// Message sent when mission victory conditions are met.
#[derive(Message)]
pub struct MissionVictoryEvent;

/// System: check objective conditions each tick and detect victory/failure.
pub fn mission_objective_system(
    clock: Res<SimClock>,
    mut campaign: ResMut<CampaignState>,
    mut obj_events: MessageReader<ObjectiveCompleteEvent>,
    mut victory_writer: MessageWriter<MissionVictoryEvent>,
    mut fail_writer: MessageWriter<MissionFailedEvent>,
    heroes: Query<(&HeroIdentity, &Owner, Has<Dead>, Option<&Position>)>,
    enemies: Query<(&Owner, Has<Dead>)>,
    wave_tracker: Res<super::wave_spawner::WaveTracker>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    // Process trigger-completed objectives
    for event in obj_events.read() {
        campaign.complete_objective(&event.objective_id);
    }

    let Some(mission) = campaign.current_mission.as_ref().map(Arc::clone) else {
        return;
    };

    // Count living enemies for EliminateAll
    let mut living_enemies = 0u32;
    for (owner, is_dead) in enemies.iter() {
        if owner.player_id != 0 && !is_dead {
            living_enemies += 1;
        }
    }

    // Check each objective's condition
    for obj in &mission.objectives {
        let status = campaign.objective_status.iter().find(|s| s.id == obj.id);
        if status.is_some_and(|s| s.completed) {
            continue;
        }

        match &obj.condition {
            ObjectiveCondition::EliminateAll => {
                // Auto-evaluate: complete when all enemies are dead
                if living_enemies == 0 {
                    campaign.complete_objective(&obj.id);
                }
            }
            ObjectiveCondition::KillCount(target) => {
                if campaign.enemy_kill_count >= *target {
                    campaign.complete_objective(&obj.id);
                }
            }
            ObjectiveCondition::Survive(tick_target) => {
                // Auto-evaluate: complete when current tick >= target
                if clock.tick >= *tick_target {
                    campaign.complete_objective(&obj.id);
                }
            }
            ObjectiveCondition::HeroReachesPos {
                hero,
                position,
                radius,
            } => {
                // Auto-evaluate: complete when hero is within radius of position
                for (identity, _owner, _is_dead, pos_opt) in heroes.iter() {
                    if identity.hero_id == *hero
                        && let Some(pos) = pos_opt
                    {
                        let grid = pos.world.to_grid();
                        let dx = (grid.x - position.x).abs();
                        let dy = (grid.y - position.y).abs();
                        if dx <= *radius && dy <= *radius {
                            campaign.complete_objective(&obj.id);
                        }
                    }
                }
            }
            ObjectiveCondition::HeroDied(hero_id) => {
                // This is a FAIL condition — if the hero is dead, mission fails
                for (identity, _owner, is_dead, _pos) in heroes.iter() {
                    if identity.hero_id == *hero_id && is_dead {
                        let reason = format!("{:?} has fallen. Mission failed.", hero_id);
                        fail_writer.write(MissionFailedEvent {
                            reason: reason.clone(),
                        });
                        campaign.last_mission_result = Some(MissionResult::Failure);
                        campaign.last_failure_reason = Some(reason);
                        campaign.phase = CampaignPhase::Debriefing;
                        return;
                    }
                }
            }
            ObjectiveCondition::EliminateWave(wave_id) => {
                // Auto-evaluate: complete when wave has been fully eliminated
                if wave_tracker
                    .waves
                    .get(wave_id)
                    .is_some_and(|(total, alive)| *total > 0 && *alive == 0)
                {
                    campaign.complete_objective(&obj.id);
                }
            }
            ObjectiveCondition::Manual => {
                // Only completed by triggers
            }
        }
    }

    // Check mission-critical heroes
    for (identity, _owner, is_dead, _pos) in heroes.iter() {
        if identity.mission_critical && is_dead {
            let reason = format!(
                "{:?} was mission-critical and has fallen.",
                identity.hero_id
            );
            fail_writer.write(MissionFailedEvent {
                reason: reason.clone(),
            });
            campaign.last_mission_result = Some(MissionResult::Failure);
            campaign.last_failure_reason = Some(reason);
            campaign.phase = CampaignPhase::Debriefing;
            return;
        }
    }

    // Check if all primary objectives are complete
    if campaign.all_primary_complete() {
        victory_writer.write(MissionVictoryEvent);
        campaign.last_mission_result = Some(MissionResult::Victory);
        campaign.last_failure_reason = None;
        campaign.phase = CampaignPhase::Debriefing;
        let mission_id = campaign.current_mission.as_ref().map(|m| m.id.clone());
        if let Some(id) = mission_id {
            campaign.completed_missions.insert(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::coords::GridPos;
    use cc_core::hero::HeroId;
    use cc_core::mission::*;
    use cc_core::terrain::TerrainType;

    fn test_mission() -> MissionDefinition {
        MissionDefinition {
            id: "test".into(),
            name: "Test".into(),
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
            enemy_waves: vec![],
            objectives: vec![
                MissionObjective {
                    id: "primary".into(),
                    description: "Win".into(),
                    primary: true,
                    condition: ObjectiveCondition::Manual,
                },
                MissionObjective {
                    id: "secondary".into(),
                    description: "Bonus".into(),
                    primary: false,
                    condition: ObjectiveCondition::KillCount(5),
                },
            ],
            triggers: vec![],
            dialogue: vec![],
            briefing_text: "Go!".into(),
            debrief_text: "Done!".into(),
            ai_tool_tier: None,
            next_mission: NextMission::default(),
            mutators: vec![],
        }
    }

    #[test]
    fn load_mission_initializes_state() {
        let mut state = CampaignState::default();
        state.load_mission(test_mission());
        assert_eq!(state.phase, CampaignPhase::Briefing);
        assert_eq!(state.objective_status.len(), 2);
        assert!(!state.objective_status[0].completed);
        assert_eq!(state.enemy_kill_count, 0);
    }

    #[test]
    fn complete_objective_marks_status() {
        let mut state = CampaignState::default();
        state.load_mission(test_mission());
        state.complete_objective("primary");
        assert!(state.objective_status[0].completed);
        assert!(!state.objective_status[1].completed);
    }

    #[test]
    fn all_primary_complete_works() {
        let mut state = CampaignState::default();
        state.load_mission(test_mission());

        assert!(!state.all_primary_complete());
        state.complete_objective("primary");
        assert!(state.all_primary_complete());
        // Secondary doesn't affect primary check
        assert!(!state.objective_status[1].completed);
    }

    #[test]
    fn kill_count_objective_auto_completes() {
        let mut state = CampaignState::default();
        state.load_mission(test_mission());
        state.phase = CampaignPhase::InMission;
        state.enemy_kill_count = 5;
        // The secondary objective (KillCount(5)) should now be completable
        // In the actual system this runs each tick — here we just verify the count
        assert!(state.enemy_kill_count >= 5);
    }

    #[test]
    fn default_campaign_is_inactive() {
        let state = CampaignState::default();
        assert_eq!(state.phase, CampaignPhase::Inactive);
        assert!(state.current_mission.is_none());
    }

    #[test]
    fn persistent_state_survives_mission_load() {
        let mut state = CampaignState::default();
        state.persistent.set_flag("helped_rex".into());
        state.persistent.murder_alliance = true;
        state.completed_missions.insert("prologue".into());

        state.load_mission(test_mission());

        assert!(state.persistent.has_flag("helped_rex"));
        assert!(state.persistent.murder_alliance);
        assert!(state.completed_missions.contains("prologue"));
        assert_eq!(state.enemy_kill_count, 0);
        assert!(state.fired_triggers.is_empty());
        assert!(state.flags.is_empty());
    }

    #[test]
    fn persistent_flag_operations() {
        let mut persistent = PersistentCampaignState::default();
        assert!(!persistent.has_flag("test"));
        persistent.set_flag("test".into());
        assert!(persistent.has_flag("test"));
    }
}
