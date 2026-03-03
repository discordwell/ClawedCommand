use bevy::prelude::*;

use cc_sim::campaign::state::{CampaignPhase, CampaignState};

/// Marker for the briefing overlay root.
#[derive(Component)]
pub struct BriefingRoot;

/// Marker for the briefing text content.
#[derive(Component)]
pub struct BriefingText;

pub fn spawn_briefing(mut commands: Commands) {
    commands
        .spawn((
            BriefingRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-250.0),
                    top: Val::Px(-200.0),
                    ..default()
                },
                width: Val::Px(500.0),
                padding: UiRect::all(Val::Px(30.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.9)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                BriefingText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));
        });
}

pub fn update_briefing(
    campaign: Res<CampaignState>,
    mut root_vis: Query<&mut Visibility, (With<BriefingRoot>, Without<BriefingText>)>,
    mut text_q: Query<&mut Text, With<BriefingText>>,
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
        return;
    }

    let Some(mission) = &campaign.current_mission else {
        return;
    };

    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let mut lines = Vec::new();

    // Act title
    let act_text = if mission.act == 0 {
        "PROLOGUE".to_string()
    } else {
        format!("ACT {}", mission.act)
    };
    lines.push(act_text);
    lines.push(String::new());
    lines.push(mission.name.clone());
    lines.push(String::new());
    lines.push(mission.briefing_text.clone());
    lines.push(String::new());
    lines.push("OBJECTIVES".to_string());

    for obj in &mission.objectives {
        let prefix = if obj.primary { ">> " } else { "   " };
        let suffix = if obj.primary { "" } else { " (optional)" };
        lines.push(format!("{}{}{}", prefix, obj.description, suffix));
    }

    lines.push(String::new());
    lines.push("[Enter/Space] BEGIN MISSION".to_string());

    text.0 = lines.join("\n");
}

/// Transition from Briefing to InMission when the player presses Enter or Space.
pub fn briefing_input_system(keys: Res<ButtonInput<KeyCode>>, mut campaign: ResMut<CampaignState>) {
    if campaign.phase != CampaignPhase::Briefing {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        campaign.phase = CampaignPhase::InMission;
    }
}
