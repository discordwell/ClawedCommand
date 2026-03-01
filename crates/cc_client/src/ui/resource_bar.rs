use bevy::prelude::*;

use cc_sim::resources::{PlayerResourceState, PlayerResources, SimClock};

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
    mut text_q: Query<&mut Text, With<ResourceBarText>>,
) {
    let Some(pres) = player_resources.players.get(LOCAL_PLAYER) else {
        return;
    };

    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let tick = clock.as_ref().map(|c| c.tick).unwrap_or(0);
    text.0 = format_resource_line(pres, tick);
}

/// Pure formatting for the resource bar — testable without Bevy.
fn format_resource_line(pres: &PlayerResourceState, tick: u64) -> String {
    let total_secs = tick / 10;
    let mins = total_secs / 60;
    let secs = total_secs % 60;

    let supply_warn = if pres.supply >= pres.supply_cap {
        "!"
    } else {
        ""
    };

    format!(
        "Food: {}  |  GPU: {}  |  NFTs: {}  |  Supply: {}/{}{}  |  {:02}:{:02}",
        pres.food, pres.gpu_cores, pres.nfts, pres.supply, pres.supply_cap, supply_warn, mins, secs,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_resource_line_shows_all_fields() {
        let pres = PlayerResourceState {
            food: 300,
            gpu_cores: 50,
            nfts: 2,
            supply: 5,
            supply_cap: 10,
            ..Default::default()
        };
        let line = format_resource_line(&pres, 615); // 61.5s = 01:01
        assert!(line.contains("Food: 300"));
        assert!(line.contains("GPU: 50"));
        assert!(line.contains("NFTs: 2"));
        assert!(line.contains("Supply: 5/10"));
        assert!(line.contains("01:01"));
        assert!(!line.contains('!'));
    }

    #[test]
    fn supply_warning_when_at_cap() {
        let pres = PlayerResourceState {
            food: 0,
            gpu_cores: 0,
            nfts: 0,
            supply: 10,
            supply_cap: 10,
            ..Default::default()
        };
        let line = format_resource_line(&pres, 0);
        assert!(line.contains("Supply: 10/10!"));
    }
}
