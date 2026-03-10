use bevy::prelude::*;

use cc_core::mission::MissionDefinition;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

use super::campaign_save;

/// Resource: available campaign missions loaded from RON files.
#[derive(Resource, Default)]
pub struct AvailableMissions {
    pub missions: Vec<MissionDefinition>,
}

/// Resource: controls whether the campaign menu is open.
#[derive(Resource, Default)]
pub struct CampaignMenuOpen(pub bool);

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

/// Marker for the "NEW CAMPAIGN" button.
#[derive(Component)]
pub struct NewCampaignButton;

/// Marker for the "CONTINUE" button.
#[derive(Component)]
pub struct ContinueCampaignButton;

pub fn spawn_campaign_menu(mut commands: Commands) {
    commands
        .spawn((
            CampaignMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-180.0),
                    top: Val::Px(-120.0),
                    ..default()
                },
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(32.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.94)),
            Visibility::Hidden,
            ZIndex(120),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("CAMPAIGN"),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            // NEW CAMPAIGN button
            parent
                .spawn((
                    NewCampaignButton,
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.4, 0.35, 0.9)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("NEW CAMPAIGN"),
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                    ));
                });

            // CONTINUE button (only visible if save exists)
            parent
                .spawn((
                    ContinueCampaignButton,
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.9)),
                    Visibility::Hidden,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("CONTINUE"),
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                    ));
                });

            // Hint text
            parent.spawn((
                Text::new("[Esc] Close"),
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
            ));
        });
}

pub fn update_campaign_menu(
    mut campaign: ResMut<CampaignState>,
    mut menu_open: ResMut<CampaignMenuOpen>,
    mut root_vis: Query<&mut Visibility, (With<CampaignMenuRoot>, Without<ContinueCampaignButton>)>,
    mut continue_vis: Query<
        &mut Visibility,
        (With<ContinueCampaignButton>, Without<CampaignMenuRoot>),
    >,
    new_btn: Query<(&NewCampaignButton, &Interaction), Changed<Interaction>>,
    continue_btn: Query<
        (&ContinueCampaignButton, &Interaction),
        (Changed<Interaction>, Without<NewCampaignButton>),
    >,
) {
    let show = menu_open.0 && campaign.phase == CampaignPhase::Inactive;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Show CONTINUE button only if save exists
    let has_save = campaign_save::save_exists();
    for mut vis in continue_vis.iter_mut() {
        *vis = if show && has_save {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        return;
    }

    // Handle NEW CAMPAIGN button
    for (_, interaction) in new_btn.iter() {
        if *interaction == Interaction::Pressed {
            // Reset campaign state
            *campaign = CampaignState::default();
            campaign.phase = CampaignPhase::WorldMap;
            menu_open.0 = false;
        }
    }

    // Handle CONTINUE button
    for (_, interaction) in continue_btn.iter() {
        if *interaction == Interaction::Pressed {
            match campaign_save::load_campaign() {
                Ok(data) => {
                    campaign.completed_missions = data.completed_missions;
                    campaign.persistent = data.persistent;
                    campaign.phase = CampaignPhase::WorldMap;
                    menu_open.0 = false;
                    info!(
                        "Loaded campaign save ({} completed)",
                        campaign.completed_missions.len()
                    );
                }
                Err(e) => {
                    warn!("Failed to load campaign save: {e}");
                }
            }
        }
    }
}
