use bevy::prelude::*;
use cc_sim::resources::PlayerResources;

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: usize = 0;

/// Marker for the resource HUD root node.
#[derive(Component)]
pub struct ResourceHudRoot;

/// Marker for the food text.
#[derive(Component)]
pub struct FoodText;

/// Marker for the GPU cores text.
#[derive(Component)]
pub struct GpuText;

/// Marker for the supply text.
#[derive(Component)]
pub struct SupplyText;

pub fn spawn_resource_hud(mut commands: Commands) {
    commands
        .spawn((
            ResourceHudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                column_gap: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                FoodText,
                Text::new("Food: 0"),
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));
            parent.spawn((
                GpuText,
                Text::new("GPU: 0"),
                TextColor(Color::srgb(0.3, 0.8, 0.9)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));
            parent.spawn((
                SupplyText,
                Text::new("Supply: 0/0"),
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));
        });
}

pub fn update_resource_hud(
    player_resources: Res<PlayerResources>,
    mut food_q: Query<&mut Text, (With<FoodText>, Without<GpuText>, Without<SupplyText>)>,
    mut gpu_q: Query<&mut Text, (With<GpuText>, Without<FoodText>, Without<SupplyText>)>,
    mut supply_q: Query<&mut Text, (With<SupplyText>, Without<FoodText>, Without<GpuText>)>,
) {
    let Some(pres) = player_resources.players.get(LOCAL_PLAYER) else {
        return;
    };

    if let Ok(mut text) = food_q.single_mut() {
        text.0 = format!("Food: {}", pres.food);
    }
    if let Ok(mut text) = gpu_q.single_mut() {
        text.0 = format!("GPU: {}", pres.gpu_cores);
    }
    if let Ok(mut text) = supply_q.single_mut() {
        text.0 = format!("Supply: {}/{}", pres.supply, pres.supply_cap);
    }
}
