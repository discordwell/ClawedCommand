use bevy::prelude::*;

use cc_sim::campaign::state::{CampaignPhase, CampaignState};
use cc_sim::resources::GameState;

const LOCAL_PLAYER: u8 = 0;

/// Marker for the game over overlay root.
#[derive(Component)]
pub struct GameOverRoot;

/// Marker for the title text.
#[derive(Component)]
pub struct GameOverTitle;

/// Marker for the subtitle text.
#[derive(Component)]
pub struct GameOverSubtitle;

pub fn spawn_game_over(mut commands: Commands) {
    commands
        .spawn((
            GameOverRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-200.0),
                    top: Val::Px(-80.0),
                    ..default()
                },
                width: Val::Px(400.0),
                padding: UiRect::all(Val::Px(40.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                GameOverTitle,
                Text::new(""),
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
            ));
            parent.spawn((
                GameOverSubtitle,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                Node {
                    margin: UiRect::top(Val::Px(16.0)),
                    ..default()
                },
            ));
        });
}

pub fn update_game_over(
    game_state: Res<GameState>,
    campaign: Res<CampaignState>,
    mut root_q: Query<
        &mut Visibility,
        (
            With<GameOverRoot>,
            Without<GameOverTitle>,
            Without<GameOverSubtitle>,
        ),
    >,
    mut title_q: Query<
        (&mut Text, &mut TextColor),
        (
            With<GameOverTitle>,
            Without<GameOverRoot>,
            Without<GameOverSubtitle>,
        ),
    >,
    mut subtitle_q: Query<
        &mut Text,
        (
            With<GameOverSubtitle>,
            Without<GameOverRoot>,
            Without<GameOverTitle>,
        ),
    >,
) {
    // Hide game_over overlay during campaign debriefing (debrief.rs handles it)
    if campaign.phase == CampaignPhase::Debriefing
        || campaign.phase == CampaignPhase::WorldMap
        || campaign.phase == CampaignPhase::ActTitleCard
    {
        for mut vis in root_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let winner = match *game_state {
        GameState::Playing | GameState::Paused => {
            for mut vis in root_q.iter_mut() {
                *vis = Visibility::Hidden;
            }
            return;
        }
        GameState::Victory { winner } => winner,
    };

    for mut vis in root_q.iter_mut() {
        *vis = Visibility::Inherited;
    }

    let (title, color, reason) = if winner == LOCAL_PLAYER {
        (
            "VICTORY!",
            Color::srgb(1.0, 0.84, 0.0),
            "Enemy base destroyed!",
        )
    } else {
        (
            "DEFEAT!",
            Color::srgb(1.0, 0.23, 0.23),
            "Your base has been destroyed!",
        )
    };

    if let Ok((mut text, mut text_color)) = title_q.single_mut() {
        text.0 = title.to_string();
        text_color.0 = color;
    }
    if let Ok(mut text) = subtitle_q.single_mut() {
        text.0 = reason.to_string();
    }
}
