use bevy::prelude::*;

use cc_sim::campaign::state::{CampaignPhase, CampaignState};

use super::cinematic::{FadeIn, act_display_name, faction_accent_color, faction_hero_portrait};
use super::dialogue::PortraitHandles;

const TYPEWRITER_SPEED: f32 = 40.0;

/// Briefing typewriter state.
#[derive(Resource, Default)]
pub struct BriefingTypewriter {
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub active: bool,
    pub full_text: String,
}

/// Marker for the briefing overlay root.
#[derive(Component)]
pub struct BriefingRoot;

/// Marker for the faction-colored left border.
#[derive(Component)]
pub struct BriefingBorder;

/// Marker for the act header text.
#[derive(Component)]
pub struct BriefingActHeader;

/// Marker for the mission name text.
#[derive(Component)]
pub struct BriefingMissionName;

/// Marker for the briefing text content.
#[derive(Component)]
pub struct BriefingText;

/// Marker for the objectives section.
#[derive(Component)]
pub struct BriefingObjectives;

/// Marker for the BEGIN MISSION button.
#[derive(Component)]
pub struct BriefingStartButton;

/// Marker for the portrait image.
#[derive(Component)]
pub struct BriefingPortrait;

pub fn spawn_briefing(mut commands: Commands) {
    commands
        .spawn((
            BriefingRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-325.0),
                    top: Val::Px(-250.0),
                    ..default()
                },
                width: Val::Px(650.0),
                flex_direction: FlexDirection::Row,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.94)),
            Visibility::Hidden,
            ZIndex(105),
        ))
        .with_children(|parent| {
            // Left faction-colored border strip
            parent.spawn((
                BriefingBorder,
                Node {
                    width: Val::Px(4.0),
                    height: Val::Percent(100.0),
                    border_radius: BorderRadius {
                        top_left: Val::Px(12.0),
                        bottom_left: Val::Px(12.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 0.8, 0.7)),
            ));

            // Main content column
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(28.0)),
                    row_gap: Val::Px(8.0),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|col| {
                    // Top row: act header + portrait
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|top| {
                        // Left: act header + mission name
                        top.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|left| {
                            left.spawn((
                                BriefingActHeader,
                                Text::new(""),
                                TextColor(Color::srgb(0.0, 0.8, 0.7)),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                            ));
                            left.spawn((
                                BriefingMissionName,
                                Text::new(""),
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                                TextFont {
                                    font_size: 22.0,
                                    ..default()
                                },
                            ));
                        });

                        // Right: portrait (96x96)
                        top.spawn((
                            BriefingPortrait,
                            ImageNode::default(),
                            Node {
                                width: Val::Px(96.0),
                                height: Val::Px(96.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.5)),
                            FadeIn::new(0.8),
                        ));
                    });

                    // Briefing text (typewriter)
                    col.spawn((
                        BriefingText,
                        Text::new(""),
                        TextColor(Color::srgb(0.85, 0.85, 0.85)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::top(Val::Px(12.0)),
                            ..default()
                        },
                    ));

                    // Objectives section
                    col.spawn((
                        BriefingObjectives,
                        Text::new(""),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                    ));

                    // BEGIN MISSION button
                    col.spawn((
                        BriefingStartButton,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            align_self: AlignSelf::Center,
                            margin: UiRect::top(Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.5, 0.4, 0.9)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("BEGIN MISSION"),
                            TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                        ));
                    });
                });
        });
}

pub fn update_briefing(
    time: Res<Time>,
    campaign: Res<CampaignState>,
    mut typewriter: ResMut<BriefingTypewriter>,
    mut root_vis: Query<&mut Visibility, (With<BriefingRoot>, Without<BriefingText>)>,
    mut border_q: Query<
        &mut BackgroundColor,
        (With<BriefingBorder>, Without<BriefingRoot>, Without<BriefingText>),
    >,
    mut header_q: Query<
        (&mut Text, &mut TextColor),
        (With<BriefingActHeader>, Without<BriefingRoot>, Without<BriefingText>),
    >,
    mut name_q: Query<
        &mut Text,
        (With<BriefingMissionName>, Without<BriefingActHeader>, Without<BriefingText>),
    >,
    mut text_q: Query<
        &mut Text,
        (With<BriefingText>, Without<BriefingMissionName>, Without<BriefingActHeader>),
    >,
    mut obj_q: Query<
        &mut Text,
        (With<BriefingObjectives>, Without<BriefingText>, Without<BriefingMissionName>, Without<BriefingActHeader>),
    >,
    mut portrait_q: Query<
        (&mut ImageNode, &mut FadeIn),
        (With<BriefingPortrait>, Without<BriefingRoot>),
    >,
    asset_server: Res<AssetServer>,
    mut portraits: ResMut<PortraitHandles>,
) {
    let show = campaign.phase == CampaignPhase::Briefing;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        if typewriter.active {
            typewriter.active = false;
            // Reset portrait fade-in for next briefing
            if let Ok((_, mut fade)) = portrait_q.single_mut() {
                fade.timer.reset();
            }
        }
        return;
    }

    let Some(mission) = &campaign.current_mission else {
        return;
    };

    // Initialize typewriter on first frame
    if !typewriter.active {
        typewriter.full_text = mission.briefing_text.clone();
        typewriter.chars_revealed = 0;
        typewriter.char_timer = 0.0;
        typewriter.active = true;
    }

    let act = mission.act;
    let accent = faction_accent_color(act);

    // Faction-colored border
    for mut bg in border_q.iter_mut() {
        bg.0 = accent;
    }

    // Act header
    if let Ok((mut text, mut color)) = header_q.single_mut() {
        text.0 = act_display_name(act).to_string();
        color.0 = accent;
    }

    // Mission name
    if let Ok(mut text) = name_q.single_mut() {
        text.0 = mission.name.clone();
    }

    // Typewriter text
    let total_chars = typewriter.full_text.chars().count();
    if typewriter.chars_revealed < total_chars {
        typewriter.char_timer += time.delta_secs();
        let chars_to_add = (typewriter.char_timer * TYPEWRITER_SPEED) as usize;
        if chars_to_add > 0 {
            typewriter.chars_revealed = (typewriter.chars_revealed + chars_to_add).min(total_chars);
            typewriter.char_timer = 0.0;
        }
    }

    if let Ok(mut text) = text_q.single_mut() {
        text.0 = typewriter
            .full_text
            .chars()
            .take(typewriter.chars_revealed)
            .collect();
    }

    // Objectives
    if let Ok(mut obj_text) = obj_q.single_mut() {
        let mut lines = vec!["OBJECTIVES".to_string()];
        for obj in &mission.objectives {
            let bullet = if obj.primary { ">>" } else { "  " };
            let suffix = if obj.primary { "" } else { " (optional)" };
            lines.push(format!("{} {}{}", bullet, obj.description, suffix));
        }
        obj_text.0 = lines.join("\n");
    }

    // Portrait
    if let Ok((mut img, _)) = portrait_q.single_mut() {
        let key = faction_hero_portrait(act);
        let handle = portraits
            .handles
            .entry(key.to_string())
            .or_insert_with(|| asset_server.load(format!("portraits/{key}.png")))
            .clone();
        img.image = handle;
    }
}

/// Transition from Briefing to InMission via Enter/Space or button click.
/// Space also skips the typewriter if it's still running.
pub fn briefing_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut campaign: ResMut<CampaignState>,
    mut typewriter: ResMut<BriefingTypewriter>,
    interactions: Query<(&BriefingStartButton, &Interaction), Changed<Interaction>>,
) {
    if campaign.phase != CampaignPhase::Briefing {
        return;
    }

    let button_pressed = interactions
        .iter()
        .any(|(_, i)| *i == Interaction::Pressed);

    let key_pressed = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);

    // If typewriter is still running, Space/Enter skips it first
    if key_pressed && typewriter.active {
        let total = typewriter.full_text.chars().count();
        if typewriter.chars_revealed < total {
            typewriter.chars_revealed = total;
            return;
        }
    }

    if key_pressed || button_pressed {
        campaign.phase = CampaignPhase::InMission;
    }
}
