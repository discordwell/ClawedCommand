use bevy::prelude::*;

use cc_sim::campaign::state::{CampaignPhase, CampaignState};

use super::cinematic::{FadeIn, act_display_name, faction_accent_color, faction_hero_portrait};
use super::dialogue::PortraitHandles;

/// Auto-advance timer for the act title card.
#[derive(Resource)]
pub struct ActTitleTimer(pub Timer);

impl Default for ActTitleTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Once))
    }
}

/// Marker for the act title card root.
#[derive(Component)]
pub struct ActTitleRoot;

/// Marker for the act title text.
#[derive(Component)]
pub struct ActTitleText;

/// Marker for the act title portrait.
#[derive(Component)]
pub struct ActTitlePortrait;

pub fn spawn_act_title_card(mut commands: Commands) {
    commands
        .spawn((
            ActTitleRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.95)),
            Visibility::Hidden,
            ZIndex(110),
        ))
        .with_children(|parent| {
            // Portrait
            parent.spawn((
                ActTitlePortrait,
                ImageNode::default(),
                Node {
                    width: Val::Px(128.0),
                    height: Val::Px(128.0),
                    ..default()
                },
            ));

            // Act title text
            parent.spawn((
                ActTitleText,
                Text::new(""),
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                FadeIn::new(1.0),
            ));

            // Skip hint
            parent.spawn((
                Text::new("[Space/Enter] Skip"),
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
        });
}

/// Update the act title card visibility and content.
pub fn update_act_title_card(
    time: Res<Time>,
    campaign: Res<CampaignState>,
    mut timer: ResMut<ActTitleTimer>,
    mut root_vis: Query<
        (&mut Visibility, &mut BackgroundColor),
        (With<ActTitleRoot>, Without<ActTitleText>),
    >,
    mut title_q: Query<
        (&mut Text, &mut TextColor),
        (With<ActTitleText>, Without<ActTitleRoot>, Without<ActTitlePortrait>),
    >,
    mut portrait_q: Query<
        &mut ImageNode,
        (With<ActTitlePortrait>, Without<ActTitleRoot>, Without<ActTitleText>),
    >,
    asset_server: Res<AssetServer>,
    mut portraits: ResMut<PortraitHandles>,
) {
    let show = campaign.phase == CampaignPhase::ActTitleCard;

    for (mut vis, _) in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        timer.0.reset();
        return;
    }

    let act = campaign.entering_act.unwrap_or(0);
    let accent = faction_accent_color(act);

    // Tint background with faction color
    for (_, mut bg) in root_vis.iter_mut() {
        let srgba = accent.to_srgba();
        bg.0 = Color::srgba(srgba.red * 0.15, srgba.green * 0.15, srgba.blue * 0.15, 0.95);
    }

    // Set title text
    if let Ok((mut text, mut color)) = title_q.single_mut() {
        text.0 = act_display_name(act).to_string();
        color.0 = accent;
    }

    // Set portrait
    if let Ok(mut img) = portrait_q.single_mut() {
        let key = faction_hero_portrait(act);
        let handle = portraits
            .handles
            .entry(key.to_string())
            .or_insert_with(|| asset_server.load(format!("portraits/{key}.png")))
            .clone();
        img.image = handle;
    }

    // Auto-advance timer
    timer.0.tick(time.delta());
}

/// Handle input to skip the act title card.
pub fn act_title_input(
    keys: Res<ButtonInput<KeyCode>>,
    timer: Res<ActTitleTimer>,
    mut campaign: ResMut<CampaignState>,
) {
    if campaign.phase != CampaignPhase::ActTitleCard {
        return;
    }

    let skip = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    if skip || timer.0.just_finished() {
        campaign.phase = CampaignPhase::Briefing;
    }
}
