use serde::{Deserialize, Serialize};

use crate::components::UnitKind;
use crate::coords::GridPos;
use crate::math::Fixed;

// ---------------------------------------------------------------------------
// Mission Mutators — RON-serializable gameplay modifiers for campaign missions
// ---------------------------------------------------------------------------

/// A gameplay modifier applied to a campaign mission.
/// Each variant is data-driven with timing/intensity parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionMutator {
    // -- Environmental Hazards --
    /// Lava advances from a map edge, blocking tiles and damaging units.
    LavaRise {
        interval_ticks: u64,
        damage_per_tick: u32,
        direction: HazardDirection,
        rows_per_wave: u32,
        initial_delay_ticks: u64,
    },
    /// Periodic wind gusts displace units in a direction.
    WindStorm {
        interval_ticks: u64,
        duration_ticks: u64,
        direction: HazardDirection,
        force: u32,
        can_push_off_map: bool,
        initial_delay_ticks: u64,
    },
    /// Toxic tide shrinks the playable area inward toward a safe zone.
    ToxicTide {
        interval_ticks: u64,
        damage_per_tick: u32,
        rows_per_wave: u32,
        initial_delay_ticks: u64,
        safe_zone_center: Option<GridPos>,
        min_safe_radius: u32,
    },
    /// Earthquakes damage buildings and randomly alter terrain.
    Tremors {
        interval_ticks: u64,
        building_damage: u32,
        terrain_change_chance: u32,
        epicenter_radius: u32,
        initial_delay_ticks: u64,
    },
    /// Rising water level converts low-elevation tiles to water.
    Flooding {
        interval_ticks: u64,
        initial_water_level: u8,
        max_water_level: u8,
        initial_delay_ticks: u64,
    },
    /// Reduced vision range; optionally clears periodically.
    DenseFog {
        vision_reduction: u32,
        periodic_clearing: Option<PeriodicClearing>,
    },
    /// Specific tiles deal damage to units standing on them.
    DamageZone {
        tiles: Vec<GridPos>,
        damage_per_tick: u32,
        active_from_start: bool,
        toggle_flag: Option<String>,
    },

    // -- Control Restrictions --
    /// Only voice commands allowed (mouse/keyboard disabled for unit control).
    VoiceOnlyControl {
        ai_enabled: bool,
        enemy_difficulty_multiplier: Fixed,
    },
    /// AI agent commands disabled.
    NoAiControl,
    /// Building placement disabled.
    NoBuildMode,
    /// Only AI controls units (player input disabled).
    AiOnlyControl { tool_tier: u8 },
    /// Only specific unit types allowed; optional population cap.
    RestrictedUnits {
        allowed_kinds: Vec<UnitKind>,
        max_unit_count: Option<u32>,
    },

    // -- Gameplay Modifiers --
    /// Mission fails after max_ticks; warning shown at warning_at.
    TimeLimit { max_ticks: u64, warning_at: u64 },
    /// Resource gather rates are multiplied.
    ResourceScarcity {
        food_multiplier: Fixed,
        gpu_multiplier: Fixed,
    },
    /// Damage dealt is multiplied per side.
    DamageMultiplier {
        player_multiplier: Fixed,
        enemy_multiplier: Fixed,
    },
    /// Movement speed is multiplied per side.
    SpeedMultiplier {
        player_multiplier: Fixed,
        enemy_multiplier: Fixed,
    },

    // -- Dream Sequence --
    /// Activates dream sequence mode with special UI and gameplay overrides.
    DreamSequence {
        /// Skip the standard briefing screen (auto-advance to InMission).
        skip_briefing: bool,
        /// Skip the standard debrief screen (auto-advance to next mission).
        skip_debrief: bool,
        /// Which dream sub-scene this mission represents.
        scene_type: DreamSceneType,
    },
}

/// Dream sequence sub-scene identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DreamSceneType {
    /// Military office desk grind loop.
    Office,
    /// Lake walk to Claude of the Lake.
    Lake,
}

/// Check if a list of mutators indicates an active dream mission.
pub fn is_dream_mission(mutators: &[MissionMutator]) -> bool {
    mutators
        .iter()
        .any(|m| matches!(m, MissionMutator::DreamSequence { .. }))
}

/// Direction from which a hazard advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardDirection {
    North,
    South,
    East,
    West,
    AllEdges,
}

/// Configuration for periodic fog clearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicClearing {
    pub interval_ticks: u64,
    pub clear_duration_ticks: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::FIXED_ONE;

    #[test]
    fn ron_round_trip_lava_rise() {
        let m = MissionMutator::LavaRise {
            interval_ticks: 50,
            damage_per_tick: 10,
            direction: HazardDirection::East,
            rows_per_wave: 2,
            initial_delay_ticks: 100,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::LavaRise {
                interval_ticks: 50,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_wind_storm() {
        let m = MissionMutator::WindStorm {
            interval_ticks: 30,
            duration_ticks: 10,
            direction: HazardDirection::West,
            force: 3,
            can_push_off_map: true,
            initial_delay_ticks: 20,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::WindStorm {
                can_push_off_map: true,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_toxic_tide() {
        let m = MissionMutator::ToxicTide {
            interval_ticks: 40,
            damage_per_tick: 5,
            rows_per_wave: 1,
            initial_delay_ticks: 60,
            safe_zone_center: Some(GridPos::new(32, 32)),
            min_safe_radius: 8,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::ToxicTide {
                min_safe_radius: 8,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_tremors() {
        let m = MissionMutator::Tremors {
            interval_ticks: 100,
            building_damage: 25,
            terrain_change_chance: 10,
            epicenter_radius: 5,
            initial_delay_ticks: 50,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::Tremors {
                building_damage: 25,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_flooding() {
        let m = MissionMutator::Flooding {
            interval_ticks: 60,
            initial_water_level: 0,
            max_water_level: 3,
            initial_delay_ticks: 30,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::Flooding {
                max_water_level: 3,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_dense_fog() {
        let m = MissionMutator::DenseFog {
            vision_reduction: 4,
            periodic_clearing: Some(PeriodicClearing {
                interval_ticks: 100,
                clear_duration_ticks: 20,
            }),
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::DenseFog {
                vision_reduction: 4,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_dense_fog_no_clearing() {
        let m = MissionMutator::DenseFog {
            vision_reduction: 6,
            periodic_clearing: None,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::DenseFog {
                periodic_clearing: None,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_damage_zone() {
        let m = MissionMutator::DamageZone {
            tiles: vec![GridPos::new(5, 5), GridPos::new(6, 5)],
            damage_per_tick: 3,
            active_from_start: false,
            toggle_flag: Some("activate_traps".into()),
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::DamageZone {
                active_from_start: false,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_voice_only_control() {
        let m = MissionMutator::VoiceOnlyControl {
            ai_enabled: false,
            enemy_difficulty_multiplier: FIXED_ONE,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::VoiceOnlyControl {
                ai_enabled: false,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_no_ai_control() {
        let m = MissionMutator::NoAiControl;
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(parsed, MissionMutator::NoAiControl));
    }

    #[test]
    fn ron_round_trip_no_build_mode() {
        let m = MissionMutator::NoBuildMode;
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(parsed, MissionMutator::NoBuildMode));
    }

    #[test]
    fn ron_round_trip_ai_only_control() {
        let m = MissionMutator::AiOnlyControl { tool_tier: 2 };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::AiOnlyControl { tool_tier: 2 }
        ));
    }

    #[test]
    fn ron_round_trip_restricted_units() {
        let m = MissionMutator::RestrictedUnits {
            allowed_kinds: vec![UnitKind::Mouser, UnitKind::Hisser],
            max_unit_count: Some(6),
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::RestrictedUnits {
                max_unit_count: Some(6),
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_time_limit() {
        let m = MissionMutator::TimeLimit {
            max_ticks: 3000,
            warning_at: 2500,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::TimeLimit {
                max_ticks: 3000,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_resource_scarcity() {
        let half = Fixed::from_bits(32768); // 0.5
        let m = MissionMutator::ResourceScarcity {
            food_multiplier: half,
            gpu_multiplier: half,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(parsed, MissionMutator::ResourceScarcity { .. }));
    }

    #[test]
    fn ron_round_trip_damage_multiplier() {
        let m = MissionMutator::DamageMultiplier {
            player_multiplier: FIXED_ONE,
            enemy_multiplier: Fixed::from_bits(98304), // 1.5
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(parsed, MissionMutator::DamageMultiplier { .. }));
    }

    #[test]
    fn ron_round_trip_speed_multiplier() {
        let m = MissionMutator::SpeedMultiplier {
            player_multiplier: FIXED_ONE,
            enemy_multiplier: Fixed::from_bits(78643), // 1.2
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(parsed, MissionMutator::SpeedMultiplier { .. }));
    }

    #[test]
    fn ron_round_trip_hazard_direction_all_variants() {
        for dir in [
            HazardDirection::North,
            HazardDirection::South,
            HazardDirection::East,
            HazardDirection::West,
            HazardDirection::AllEdges,
        ] {
            let s = ron::to_string(&dir).unwrap();
            let parsed: HazardDirection = ron::from_str(&s).unwrap();
            assert_eq!(parsed, dir);
        }
    }

    #[test]
    fn ron_round_trip_periodic_clearing() {
        let pc = PeriodicClearing {
            interval_ticks: 200,
            clear_duration_ticks: 50,
        };
        let s = ron::to_string(&pc).unwrap();
        let parsed: PeriodicClearing = ron::from_str(&s).unwrap();
        assert_eq!(parsed, pc);
    }

    #[test]
    fn vec_of_mutators_round_trip() {
        let mutators = vec![
            MissionMutator::TimeLimit {
                max_ticks: 1000,
                warning_at: 800,
            },
            MissionMutator::NoBuildMode,
            MissionMutator::DenseFog {
                vision_reduction: 3,
                periodic_clearing: None,
            },
        ];
        let s = ron::to_string(&mutators).unwrap();
        let parsed: Vec<MissionMutator> = ron::from_str(&s).unwrap();
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn ron_round_trip_dream_sequence_office() {
        let m = MissionMutator::DreamSequence {
            skip_briefing: false,
            skip_debrief: true,
            scene_type: DreamSceneType::Office,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::DreamSequence {
                skip_briefing: false,
                skip_debrief: true,
                scene_type: DreamSceneType::Office,
            }
        ));
    }

    #[test]
    fn ron_round_trip_dream_sequence_lake() {
        let m = MissionMutator::DreamSequence {
            skip_briefing: true,
            skip_debrief: true,
            scene_type: DreamSceneType::Lake,
        };
        let s = ron::to_string(&m).unwrap();
        let parsed: MissionMutator = ron::from_str(&s).unwrap();
        assert!(matches!(
            parsed,
            MissionMutator::DreamSequence {
                scene_type: DreamSceneType::Lake,
                ..
            }
        ));
    }

    #[test]
    fn ron_round_trip_dream_scene_type() {
        for scene in [DreamSceneType::Office, DreamSceneType::Lake] {
            let s = ron::to_string(&scene).unwrap();
            let parsed: DreamSceneType = ron::from_str(&s).unwrap();
            assert_eq!(parsed, scene);
        }
    }
}
