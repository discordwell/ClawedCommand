use bevy::prelude::*;

use crate::setup::UnitMesh;
use cc_core::components::{AttackStats, Health, Selected, UnitType};

/// Marker for the top resource bar.
#[derive(Component)]
pub struct TopBar;

/// Marker for the bottom info bar text.
#[derive(Component)]
pub struct BottomBarText;

/// Set up the HUD overlay: top bar + bottom bar.
pub fn setup_hud(mut commands: Commands) {
    // Top bar: semi-transparent strip with placeholder resource text
    commands
        .spawn((
            TopBar,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Food: 0  |  GPU: 0  |  NFTs: 0"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 0.8)),
            ));
        });

    // Bottom bar: semi-transparent strip showing selected unit info
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(220.0), // Leave room for minimap
                right: Val::Px(0.0),  // Stretch to right edge
                height: Val::Px(32.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            parent.spawn((
                BottomBarText,
                Text::new("No unit selected"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.8)),
            ));
        });
}

/// Update bottom bar text with selected unit info.
pub fn update_hud(
    selected: Query<(&UnitType, &Health, &AttackStats), (With<Selected>, With<UnitMesh>)>,
    mut text_query: Query<&mut Text, With<BottomBarText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let count = selected.iter().count();
    if count == 0 {
        **text = "No unit selected".into();
        return;
    }

    if count == 1 {
        let Ok((unit_type, health, attack)) = selected.single() else {
            return;
        };
        let hp_cur: f32 = health.current.to_num();
        let hp_max: f32 = health.max.to_num();
        let dmg: f32 = attack.damage.to_num();
        let rng: f32 = attack.range.to_num();
        **text = format!(
            "{:?}  |  HP: {:.0}/{:.0}  |  ATK: {:.0}  |  RNG: {:.0}",
            unit_type.kind, hp_cur, hp_max, dmg, rng
        );
    } else {
        **text = format!("{} units selected", count);
    }
}
