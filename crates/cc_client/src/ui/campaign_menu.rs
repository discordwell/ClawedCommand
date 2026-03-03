use bevy::prelude::*;

use cc_core::mission::MissionDefinition;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

/// Resource: available campaign missions loaded from RON files.
#[derive(Resource, Default)]
pub struct AvailableMissions {
    pub missions: Vec<MissionDefinition>,
}

/// Resource: controls whether the campaign menu is open.
#[derive(Resource)]
pub struct CampaignMenuOpen(pub bool);

impl Default for CampaignMenuOpen {
    fn default() -> Self {
        Self(false)
    }
}

/// Toggle campaign menu with Escape when no campaign is active.
pub fn campaign_menu_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu_open: ResMut<CampaignMenuOpen>,
    campaign: Res<CampaignState>,
) {
    if campaign.phase != CampaignPhase::Inactive {
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        menu_open.0 = !menu_open.0;
    }
}

/// Marker for campaign menu root.
#[derive(Component)]
pub struct CampaignMenuRoot;

/// Marker for campaign menu text.
#[derive(Component)]
pub struct CampaignMenuText;

/// Marker for individual mission buttons.
#[derive(Component)]
pub struct MissionButton {
    pub index: usize,
}

pub fn spawn_campaign_menu(mut commands: Commands) {
    commands
        .spawn((
            CampaignMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-200.0),
                    top: Val::Px(-200.0),
                    ..default()
                },
                width: Val::Px(400.0),
                padding: UiRect::all(Val::Px(24.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.12, 0.94)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                CampaignMenuText,
                Text::new("CAMPAIGN\n\nNo missions available."),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));
        });
}

pub fn update_campaign_menu(
    menu_open: Res<CampaignMenuOpen>,
    available: Res<AvailableMissions>,
    campaign: Res<CampaignState>,
    mut root_vis: Query<&mut Visibility, (With<CampaignMenuRoot>, Without<CampaignMenuText>)>,
    mut text_q: Query<&mut Text, With<CampaignMenuText>>,
) {
    let show = menu_open.0 && campaign.phase == CampaignPhase::Inactive;

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

    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let mut lines = vec!["CAMPAIGN".to_string(), String::new()];

    if available.missions.is_empty() {
        lines.push("No missions available.".to_string());
    } else {
        for (i, mission) in available.missions.iter().enumerate() {
            let completed = campaign.completed_missions.contains(&mission.id);
            let prefix = if completed { "[DONE] " } else { "" };
            let brief: String = mission.briefing_text.chars().take(50).collect();
            lines.push(format!(
                "{}. {}{} - {}...",
                i + 1,
                prefix,
                mission.name,
                brief
            ));
        }
        lines.push(String::new());
        lines.push("Press [Esc] to close".to_string());
    }

    text.0 = lines.join("\n");
}
