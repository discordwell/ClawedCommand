use bevy::prelude::*;

use cc_core::mission::NextMission;
use cc_core::mutator::MissionMutator;
use cc_sim::campaign::state::{CampaignPhase, CampaignState, MissionResult};

const TYPEWRITER_SPEED: f32 = 40.0;

/// Debrief typewriter + UI state.
#[derive(Resource)]
pub struct DebriefState {
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub active: bool,
    pub full_text: String,
}

impl Default for DebriefState {
    fn default() -> Self {
        Self {
            chars_revealed: 0,
            char_timer: 0.0,
            active: false,
            full_text: String::new(),
        }
    }
}

/// Marker for debrief overlay root.
#[derive(Component)]
pub struct DebriefRoot;

/// Marker for the VICTORY/DEFEAT title text.
#[derive(Component)]
pub struct DebriefTitle;

/// Marker for the mission name subtitle.
#[derive(Component)]
pub struct DebriefSubtitle;

/// Marker for the debrief body text (typewriter).
#[derive(Component)]
pub struct DebriefText;

/// Marker for objective checklist rows.
#[derive(Component)]
pub struct ObjectiveRow {
    pub index: usize,
}

/// Marker for debrief buttons.
#[derive(Component)]
pub struct DebriefButton {
    pub action: DebriefAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebriefAction {
    NextMission,
    Retry,
    ReturnToMap,
}

pub fn spawn_debrief(mut commands: Commands) {
    commands
        .spawn((
            DebriefRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
            // Render on top of game
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Central panel
            parent
                .spawn(Node {
                    width: Val::Px(600.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    ..default()
                })
                .with_children(|panel| {
                    // Title: VICTORY / MISSION FAILED
                    panel.spawn((
                        DebriefTitle,
                        Text::new(""),
                        TextColor(Color::srgb(1.0, 0.84, 0.0)),
                        TextFont {
                            font_size: 42.0,
                            ..default()
                        },
                    ));

                    // Mission name subtitle
                    panel.spawn((
                        DebriefSubtitle,
                        Text::new(""),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                    ));

                    // Debrief text (typewriter)
                    panel.spawn((
                        DebriefText,
                        Text::new(""),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                    ));

                    // Objective checklist area (will be populated dynamically)
                    // We use marker components on the text children

                    // Button row
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(16.0),
                            margin: UiRect::top(Val::Px(20.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            for (label, action) in [
                                ("NEXT MISSION", DebriefAction::NextMission),
                                ("RETRY", DebriefAction::Retry),
                                ("RETURN TO MAP", DebriefAction::ReturnToMap),
                            ] {
                                row.spawn((
                                    DebriefButton { action },
                                    Button,
                                    Node {
                                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                                        border_radius: BorderRadius::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                                ))
                                .with_child((
                                    Text::new(label),
                                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                ));
                            }
                        });
                });
        });
}

/// Dream sequence auto-skip: immediately load next mission without showing debrief UI.
pub fn dream_debrief_auto_skip(mut campaign: ResMut<CampaignState>) {
    if campaign.phase != CampaignPhase::Debriefing {
        return;
    }

    let Some(mission) = &campaign.current_mission else {
        return;
    };

    let should_skip = mission.mutators.iter().any(|m| {
        matches!(
            m,
            MissionMutator::DreamSequence {
                skip_debrief: true,
                ..
            }
        )
    });

    if !should_skip {
        return;
    }

    // Replicate NextMission resolution from debrief_interaction
    let next_id = match &mission.next_mission {
        NextMission::Fixed(id) => Some(id.clone()),
        NextMission::Branching {
            flag,
            on_true,
            on_false,
        } => {
            if campaign.persistent.has_flag(flag) {
                Some(on_true.clone())
            } else {
                Some(on_false.clone())
            }
        }
        NextMission::None => None,
    };

    if let Some(next_id) = next_id {
        let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/campaign")
            .join(format!("{next_id}.ron"));
        match std::fs::read_to_string(&ron_path) {
            Ok(ron_str) => {
                match ron::from_str::<cc_core::mission::MissionDefinition>(&ron_str) {
                    Ok(next_mission) => {
                        let new_act = next_mission.act;
                        let old_act = campaign
                            .current_mission
                            .as_ref()
                            .map(|m| m.act)
                            .unwrap_or(0);
                        campaign.load_mission(next_mission);
                        if new_act != old_act {
                            campaign.entering_act = Some(new_act);
                            campaign.phase = CampaignPhase::ActTitleCard;
                        }
                    }
                    Err(e) => {
                        warn!("Dream auto-skip: failed to parse {next_id}.ron: {e}");
                        campaign.phase = CampaignPhase::WorldMap;
                    }
                }
            }
            Err(e) => {
                warn!("Dream auto-skip: failed to read {next_id}.ron: {e}");
                campaign.phase = CampaignPhase::WorldMap;
            }
        }
    } else {
        campaign.phase = CampaignPhase::WorldMap;
    }
}

/// Update debrief visibility and content.
pub fn update_debrief(
    campaign: Res<CampaignState>,
    mut debrief_state: ResMut<DebriefState>,
    mut root_vis: Query<&mut Visibility, (With<DebriefRoot>, Without<DebriefTitle>)>,
    mut title_q: Query<
        (&mut Text, &mut TextColor),
        (
            With<DebriefTitle>,
            Without<DebriefRoot>,
            Without<DebriefSubtitle>,
            Without<DebriefText>,
        ),
    >,
    mut subtitle_q: Query<
        &mut Text,
        (
            With<DebriefSubtitle>,
            Without<DebriefRoot>,
            Without<DebriefTitle>,
            Without<DebriefText>,
        ),
    >,
    mut next_btn_q: Query<
        (&DebriefButton, &mut Visibility),
        (Without<DebriefRoot>, Without<DebriefTitle>),
    >,
) {
    let show = campaign.phase == CampaignPhase::Debriefing;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        if debrief_state.active {
            debrief_state.active = false;
            debrief_state.chars_revealed = 0;
            debrief_state.char_timer = 0.0;
        }
        return;
    }

    // Initialize typewriter on first frame
    if !debrief_state.active {
        if let Some(mission) = &campaign.current_mission {
            debrief_state.full_text = mission.debrief_text.clone();
        } else {
            debrief_state.full_text = String::new();
        }
        debrief_state.chars_revealed = 0;
        debrief_state.char_timer = 0.0;
        debrief_state.active = true;
    }

    let is_victory = campaign.last_mission_result == Some(MissionResult::Victory);

    // Title
    if let Ok((mut text, mut color)) = title_q.single_mut() {
        if is_victory {
            text.0 = "VICTORY".to_string();
            color.0 = Color::srgb(1.0, 0.84, 0.0);
        } else {
            text.0 = "MISSION FAILED".to_string();
            color.0 = Color::srgb(1.0, 0.23, 0.23);
        }
    }

    // Subtitle — mission name
    if let Ok(mut text) = subtitle_q.single_mut() {
        if let Some(mission) = &campaign.current_mission {
            text.0 = mission.name.clone();
        }
    }

    // Show/hide Next Mission button based on victory + next exists
    let has_next = campaign
        .current_mission
        .as_ref()
        .is_some_and(|m| !matches!(m.next_mission, NextMission::None));

    for (btn, mut vis) in next_btn_q.iter_mut() {
        match btn.action {
            DebriefAction::NextMission => {
                *vis = if is_victory && has_next {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            DebriefAction::Retry => {
                *vis = if !is_victory {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            DebriefAction::ReturnToMap => {
                // Always visible
            }
        }
    }
}

/// Typewriter effect for debrief text.
pub fn debrief_typewriter(
    time: Res<Time>,
    campaign: Res<CampaignState>,
    mut state: ResMut<DebriefState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut text_q: Query<
        &mut Text,
        (
            With<DebriefText>,
            Without<DebriefTitle>,
            Without<DebriefSubtitle>,
        ),
    >,
) {
    if campaign.phase != CampaignPhase::Debriefing || !state.active {
        return;
    }

    let total_chars = state.full_text.chars().count();

    // Space to skip typewriter
    if keys.just_pressed(KeyCode::Space) && state.chars_revealed < total_chars {
        state.chars_revealed = total_chars;
    }

    if state.chars_revealed < total_chars {
        state.char_timer += time.delta_secs();
        let chars_to_add = (state.char_timer * TYPEWRITER_SPEED) as usize;
        if chars_to_add > 0 {
            state.chars_revealed = (state.chars_revealed + chars_to_add).min(total_chars);
            state.char_timer = 0.0;
        }
    }

    if let Ok(mut text) = text_q.single_mut() {
        // Build objectives section
        let mut display = String::new();

        // Debrief text (typewriter)
        let visible: String = state.full_text.chars().take(state.chars_revealed).collect();
        display.push_str(&visible);

        // Objectives (show immediately, below text)
        if state.chars_revealed >= total_chars {
            if let Some(mission) = &campaign.current_mission {
                display.push_str("\n\nOBJECTIVES\n");
                for (i, obj) in mission.objectives.iter().enumerate() {
                    let status = campaign.objective_status.get(i);
                    let icon = if status.is_some_and(|s| s.completed) {
                        "[OK]"
                    } else {
                        "[  ]"
                    };
                    let primary = if obj.primary { "" } else { " (optional)" };
                    display.push_str(&format!("{} {}{}\n", icon, obj.description, primary));
                }
            }
        }

        text.0 = display;
    }
}

/// Handle debrief button interactions.
pub fn debrief_interaction(
    mut campaign: ResMut<CampaignState>,
    interactions: Query<(&DebriefButton, &Interaction), Changed<Interaction>>,
) {
    for (btn, interaction) in interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match btn.action {
            DebriefAction::NextMission => {
                // Resolve next mission and transition
                let next_id =
                    campaign
                        .current_mission
                        .as_ref()
                        .and_then(|m| match &m.next_mission {
                            NextMission::Fixed(id) => Some(id.clone()),
                            NextMission::Branching {
                                flag,
                                on_true,
                                on_false,
                            } => {
                                if campaign.persistent.has_flag(flag) {
                                    Some(on_true.clone())
                                } else {
                                    Some(on_false.clone())
                                }
                            }
                            NextMission::None => None,
                        });

                if let Some(next_id) = next_id {
                    // Load the next mission RON file
                    let ron_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../assets/campaign")
                        .join(format!("{next_id}.ron"));
                    match std::fs::read_to_string(&ron_path) {
                        Ok(ron_str) => {
                            match ron::from_str::<cc_core::mission::MissionDefinition>(&ron_str) {
                                Ok(mission) => {
                                    let new_act = mission.act;
                                    let old_act = campaign
                                        .current_mission
                                        .as_ref()
                                        .map(|m| m.act)
                                        .unwrap_or(0);
                                    campaign.load_mission(mission);
                                    if new_act != old_act {
                                        campaign.entering_act = Some(new_act);
                                        campaign.phase = CampaignPhase::ActTitleCard;
                                    }
                                    // else stays in Briefing (set by load_mission)
                                }
                                Err(e) => {
                                    warn!("Failed to parse {next_id}.ron: {e}");
                                    campaign.phase = CampaignPhase::WorldMap;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read {next_id}.ron: {e}");
                            campaign.phase = CampaignPhase::WorldMap;
                        }
                    }
                }
            }
            DebriefAction::Retry => {
                // Reload current mission
                if let Some(mission) = campaign.current_mission.clone() {
                    let mission_def = (*mission).clone();
                    campaign.load_mission(mission_def);
                    // load_mission sets phase to Briefing
                }
            }
            DebriefAction::ReturnToMap => {
                campaign.phase = CampaignPhase::WorldMap;
            }
        }
    }
}
