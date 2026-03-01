use bevy::prelude::*;

use super::UiState;

/// Marker for the notification area root node.
#[derive(Component)]
pub struct NotificationRoot;

/// Marker for individual notification text entries.
#[derive(Component)]
pub struct NotificationEntry {
    pub remaining: f32,
}

pub fn spawn_notifications(mut commands: Commands) {
    commands.spawn((
        NotificationRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
    ));
}

pub fn update_notifications(
    mut commands: Commands,
    time: Res<Time>,
    mut ui_state: ResMut<UiState>,
    root_q: Query<Entity, With<NotificationRoot>>,
    mut entries: Query<(Entity, &mut NotificationEntry, &mut TextColor)>,
) {
    let dt = time.delta_secs();

    // Tick down and fade existing entries
    for (entity, mut entry, mut color) in entries.iter_mut() {
        entry.remaining -= dt;
        if entry.remaining <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            let alpha = entry.remaining.min(1.0);
            color.0 = Color::srgba(1.0, 1.0, 0.8, alpha);
        }
    }

    // Spawn new notifications from UiState
    if ui_state.notifications.is_empty() {
        return;
    }

    let Ok(root) = root_q.single() else { return };

    let new_notifs: Vec<(String, f32)> = ui_state.notifications.drain(..).collect();
    for (msg, remaining) in new_notifs {
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                NotificationEntry { remaining },
                Text::new(msg),
                TextColor(Color::srgba(1.0, 1.0, 0.8, 1.0)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        });
    }
}
