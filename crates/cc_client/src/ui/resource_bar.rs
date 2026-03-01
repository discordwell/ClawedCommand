use bevy::prelude::*;

use crate::renderer::screenshot::ScreenshotConfig;
use cc_sim::resources::{PlayerResources, SimClock};

const LOCAL_PLAYER: usize = 0;

/// Marker for the resource bar root node.
#[derive(Component)]
pub struct ResourceBarRoot;

/// Marker for the resource bar text content.
#[derive(Component)]
pub struct ResourceBarText;

pub fn spawn_resource_bar(mut commands: Commands) {
    commands
        .spawn((
            ResourceBarRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(28.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ResourceBarText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));
        });
}

pub fn update_resource_bar(
    player_resources: Res<PlayerResources>,
    clock: Option<Res<SimClock>>,
    screenshot_config: Option<Res<ScreenshotConfig>>,
    mut text_q: Query<&mut Text, With<ResourceBarText>>,
) {
    let Some(pres) = player_resources.players.get(LOCAL_PLAYER) else {
        return;
    };

    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let tick = clock.as_ref().map(|c| c.tick).unwrap_or(0);
    let total_secs = tick / 10;
    let mins = total_secs / 60;
    let secs = total_secs % 60;

    let supply_warn = if pres.supply >= pres.supply_cap {
        "!"
    } else {
        ""
    };

    let mut line = format!(
        "Food: {}  |  GPU: {}  |  NFTs: {}  |  Supply: {}/{}{}  |  {:02}:{:02}",
        pres.food, pres.gpu_cores, pres.nfts, pres.supply, pres.supply_cap, supply_warn, mins, secs,
    );

    if let Some(ref config) = screenshot_config {
        if let Some(interval) = config.auto_interval {
            let label = if interval <= 10.0 {
                " [AUTO 10s]"
            } else {
                " [AUTO 30s]"
            };
            line.push_str(label);
        }
    }

    text.0 = line;
}
