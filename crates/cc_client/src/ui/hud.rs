use std::collections::HashMap;

use bevy::prelude::*;

use crate::renderer::screenshot::ScreenshotConfig;
use crate::setup::UnitMesh;
use cc_core::components::{AttackStats, Health, MovementSpeed, Owner, Selected, UnitKind, UnitType};
use cc_sim::resources::{PlayerResources, SimClock};

/// Marker for the top resource bar text.
#[derive(Component)]
pub struct TopBarText;

/// Marker for the bottom info bar text.
#[derive(Component)]
pub struct BottomBarText;

/// Set up the HUD overlay: top bar + bottom bar.
pub fn setup_hud(mut commands: Commands) {
    // Top bar: semi-transparent strip with resource text
    commands
        .spawn((
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
                TopBarText,
                Text::new("Food: 200 | GPU: 50 | NFTs: 0 | Supply: 0/10 | 00:00"),
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
                right: Val::Px(0.0),
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

/// Update HUD: top bar resources + clock, bottom bar selected unit info.
pub fn update_hud(
    resources: Option<Res<PlayerResources>>,
    clock: Option<Res<SimClock>>,
    screenshot_config: Option<Res<ScreenshotConfig>>,
    selected: Query<
        (&UnitType, &Health, &AttackStats, &MovementSpeed, &Owner),
        (With<Selected>, With<UnitMesh>),
    >,
    mut top_text: Query<&mut Text, (With<TopBarText>, Without<BottomBarText>)>,
    mut bottom_text: Query<&mut Text, (With<BottomBarText>, Without<TopBarText>)>,
) {
    // --- Top bar: resources + clock ---
    if let Ok(mut text) = top_text.single_mut() {
        let (food, gpu, nfts, supply, supply_cap) = if let Some(ref res) = resources {
            if let Some(pr) = res.players.first() {
                (pr.food, pr.gpu_cores, pr.nfts, pr.supply, pr.supply_cap)
            } else {
                (0, 0, 0, 0, 0)
            }
        } else {
            (0, 0, 0, 0, 0)
        };

        let tick = clock.map(|c| c.tick).unwrap_or(0);
        let total_secs = tick / 10;
        let mins = total_secs / 60;
        let secs = total_secs % 60;

        let auto_indicator = match screenshot_config.as_ref().and_then(|c| c.auto_interval) {
            Some(t) if t <= 10.0 => " [AUTO 10s]",
            Some(_) => " [AUTO 30s]",
            None => "",
        };

        **text = format!(
            "Food: {} | GPU: {} | NFTs: {} | Supply: {}/{} | {:02}:{:02}{}",
            food, gpu, nfts, supply, supply_cap, mins, secs, auto_indicator,
        );
    }

    // --- Bottom bar: selected unit info ---
    let Ok(mut text) = bottom_text.single_mut() else {
        return;
    };

    let count = selected.iter().count();
    if count == 0 {
        **text = "No unit selected".into();
        return;
    }

    if count == 1 {
        if let Some((unit_type, health, attack, speed, _owner)) = selected.iter().next() {
            let hp_cur: f32 = health.current.to_num();
            let hp_max: f32 = health.max.to_num();
            let dmg: f32 = attack.damage.to_num();
            let rng: f32 = attack.range.to_num();
            let spd: f32 = speed.speed.to_num();
            **text = format!(
                "{:?}  |  HP: {:.0}/{:.0}  |  ATK: {:.0}  |  RNG: {:.1}  |  SPD: {:.1}",
                unit_type.kind, hp_cur, hp_max, dmg, rng, spd
            );
        }
    } else {
        // Multi-select: show unit type breakdown
        let mut type_counts: HashMap<UnitKind, u32> = HashMap::new();
        for (unit_type, _health, _attack, _speed, _owner) in selected.iter() {
            *type_counts.entry(unit_type.kind).or_insert(0) += 1;
        }

        let mut parts: Vec<String> = type_counts
            .iter()
            .map(|(kind, count)| format!("{}x {:?}", count, kind))
            .collect();
        parts.sort();

        **text = format!("{} selected: {}", count, parts.join(", "));
    }
}
