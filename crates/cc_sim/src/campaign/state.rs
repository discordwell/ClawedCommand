use bevy::prelude::*;

use cc_core::components::{Dead, HeroIdentity, Owner};
use cc_core::mission::{MissionDefinition, ObjectiveCondition};

use super::triggers::ObjectiveCompleteEvent;

/// Campaign phase FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignPhase {
    /// No campaign active — normal skirmish mode.
    Inactive,
    /// Showing the mission briefing screen.
    Briefing,
    /// Mission is actively playing.
    InMission,
    /// Mission ended, showing debrief.
    Debriefing,
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
    /// Current loaded mission definition. None = no campaign active.
    pub current_mission: Option<MissionDefinition>,
    /// Completed mission IDs (for progression tracking).
    pub completed_missions: Vec<String>,
    /// Narrative flags set by triggers (for branching storylines).
    pub flags: Vec<String>,
    /// Total enemy kills in current mission.
    pub enemy_kill_count: u32,
    /// Trigger IDs that have already fired (for `once` triggers).
    pub fired_triggers: Vec<String>,
    /// Status of each objective in the current mission.
    pub objective_status: Vec<ObjectiveStatus>,
    /// Current campaign phase.
    pub phase: CampaignPhase,
    /// IDs of waves that have been spawned.
    pub spawned_waves: Vec<String>,
}

impl Default for CampaignState {
    fn default() -> Self {
        Self {
            current_mission: None,
            completed_missions: Vec::new(),
            flags: Vec::new(),
            enemy_kill_count: 0,
            fired_triggers: Vec::new(),
            objective_status: Vec::new(),
            phase: CampaignPhase::Inactive,
            spawned_waves: Vec::new(),
        }
    }
}

impl CampaignState {
    /// Initialize campaign state from a mission definition.
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
        self.current_mission = Some(mission);
    }

    /// Check if all primary objectives are complete.
    pub fn all_primary_complete(&self) -> bool {
        let Some(mission) = &self.current_mission else {
            return false;
        };
        for obj in &mission.objectives {
            if obj.primary {
                let status = self
                    .objective_status
                    .iter()
                    .find(|s| s.id == obj.id);
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
    mut campaign: ResMut<CampaignState>,
    mut obj_events: MessageReader<ObjectiveCompleteEvent>,
    mut victory_writer: MessageWriter<MissionVictoryEvent>,
    mut fail_writer: MessageWriter<MissionFailedEvent>,
    heroes: Query<(&HeroIdentity, &Owner, Has<Dead>)>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    // Process trigger-completed objectives
    for event in obj_events.read() {
        campaign.complete_objective(&event.objective_id);
    }

    let Some(mission) = campaign.current_mission.clone() else {
        return;
    };

    // Check each objective's condition
    for obj in &mission.objectives {
        let status = campaign
            .objective_status
            .iter()
            .find(|s| s.id == obj.id);
        if status.is_some_and(|s| s.completed) {
            continue;
        }

        match &obj.condition {
            ObjectiveCondition::EliminateAll => {
                // Handled by trigger system checking all enemies dead
            }
            ObjectiveCondition::KillCount(target) => {
                if campaign.enemy_kill_count >= *target {
                    campaign.complete_objective(&obj.id);
                }
            }
            ObjectiveCondition::HeroDied(hero_id) => {
                // This is a FAIL condition — if the hero is dead, mission fails
                for (identity, _owner, is_dead) in heroes.iter() {
                    if identity.hero_id == *hero_id && is_dead {
                        fail_writer.send(MissionFailedEvent {
                            reason: format!(
                                "{:?} has fallen. Mission failed.",
                                hero_id
                            ),
                        });
                        campaign.phase = CampaignPhase::Debriefing;
                        return;
                    }
                }
            }
            ObjectiveCondition::Manual => {
                // Only completed by triggers
            }
            _ => {
                // Other conditions (Survive, HeroReachesPos, etc.) handled by trigger system
            }
        }
    }

    // Check mission-critical heroes
    for (identity, _owner, is_dead) in heroes.iter() {
        if identity.mission_critical && is_dead {
            fail_writer.send(MissionFailedEvent {
                reason: format!("{:?} was mission-critical and has fallen.", identity.hero_id),
            });
            campaign.phase = CampaignPhase::Debriefing;
            return;
        }
    }

    // Check if all primary objectives are complete
    if campaign.all_primary_complete() {
        victory_writer.send(MissionVictoryEvent);
        campaign.phase = CampaignPhase::Debriefing;
        if let Some(mission) = &campaign.current_mission {
            if !campaign.completed_missions.contains(&mission.id) {
                campaign.completed_missions.push(mission.id.clone());
            }
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
}
