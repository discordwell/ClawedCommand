use std::collections::HashSet;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use cc_sim::campaign::state::{CampaignPhase, CampaignState, PersistentCampaignState};

/// Serializable campaign save data.
#[derive(Serialize, Deserialize, Default)]
pub struct CampaignSaveData {
    pub version: u32,
    pub completed_missions: HashSet<String>,
    pub current_mission_id: Option<String>,
    pub persistent: PersistentCampaignState,
}

/// Returns the save file path: `~/.clawed_command/campaign_save.ron`.
pub fn save_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".clawed_command")
            .join("campaign_save.ron")
    })
}

/// Returns true if a save file exists.
pub fn save_exists() -> bool {
    save_path().is_some_and(|p| p.exists())
}

/// Save campaign state to disk.
pub fn save_campaign(state: &CampaignState) -> Result<(), String> {
    let path = save_path().ok_or("Could not determine save path")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create save dir: {e}"))?;
    }
    let data = CampaignSaveData {
        version: 1,
        completed_missions: state.completed_missions.clone(),
        current_mission_id: state.current_mission.as_ref().map(|m| m.id.clone()),
        persistent: state.persistent.clone(),
    };
    let ron_str = ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default())
        .map_err(|e| format!("Failed to serialize save: {e}"))?;
    std::fs::write(&path, ron_str).map_err(|e| format!("Failed to write save: {e}"))?;
    info!("Campaign saved to {}", path.display());
    Ok(())
}

/// Load campaign state from disk.
pub fn load_campaign() -> Result<CampaignSaveData, String> {
    let path = save_path().ok_or("Could not determine save path")?;
    let ron_str =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read save: {e}"))?;
    let data: CampaignSaveData =
        ron::from_str(&ron_str).map_err(|e| format!("Failed to parse save: {e}"))?;
    Ok(data)
}

/// Tracks the previous campaign phase to detect actual transitions.
#[derive(Resource)]
pub struct PreviousCampaignPhase(pub CampaignPhase);

impl Default for PreviousCampaignPhase {
    fn default() -> Self {
        Self(CampaignPhase::Inactive)
    }
}

/// System: auto-save when transitioning into WorldMap (not every frame).
pub fn auto_save_campaign(
    campaign: Res<CampaignState>,
    mut prev_phase: ResMut<PreviousCampaignPhase>,
) {
    let current = campaign.phase;
    if current == prev_phase.0 {
        return;
    }
    let old = prev_phase.0;
    prev_phase.0 = current;

    // Only save on transition into WorldMap (from Debriefing or elsewhere)
    if current == CampaignPhase::WorldMap && old != CampaignPhase::Inactive {
        if let Err(e) = save_campaign(&campaign) {
            warn!("Auto-save failed: {e}");
        }
    }
}

/// Startup system: load save if it exists and populate CampaignState.
pub fn load_campaign_save(mut campaign: ResMut<CampaignState>) {
    if !save_exists() {
        return;
    }
    match load_campaign() {
        Ok(data) => {
            campaign.completed_missions = data.completed_missions;
            campaign.persistent = data.persistent;
            info!("Campaign save loaded ({} missions completed)", campaign.completed_missions.len());
        }
        Err(e) => {
            warn!("Failed to load campaign save: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_data_round_trip() {
        let mut data = CampaignSaveData::default();
        data.version = 1;
        data.completed_missions.insert("prologue".into());
        data.current_mission_id = Some("act1_m1".into());
        data.persistent.murder_alliance = true;

        let ron_str = ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default()).unwrap();
        let parsed: CampaignSaveData = ron::from_str(&ron_str).unwrap();

        assert_eq!(parsed.version, 1);
        assert!(parsed.completed_missions.contains("prologue"));
        assert_eq!(parsed.current_mission_id.as_deref(), Some("act1_m1"));
        assert!(parsed.persistent.murder_alliance);
    }

    #[test]
    fn previous_phase_defaults_to_inactive() {
        let prev = PreviousCampaignPhase::default();
        assert_eq!(prev.0, CampaignPhase::Inactive);
    }

    #[test]
    fn save_path_returns_some() {
        // HOME should be set in any test environment
        let path = save_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.ends_with("campaign_save.ron"));
    }
}
