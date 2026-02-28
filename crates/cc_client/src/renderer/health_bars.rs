use bevy::prelude::*;

use cc_core::components::{Dead, Health, UnitType};

/// Marker for health bar background sprite.
#[derive(Component)]
pub struct HealthBarBg;

/// Marker for health bar foreground sprite (the colored fill).
#[derive(Component)]
pub struct HealthBarFg;

const BAR_WIDTH: f32 = 22.0;
const BAR_HEIGHT: f32 = 3.0;
const BAR_Y_OFFSET: f32 = 14.0; // Above the unit sprite

/// Spawn health bar child entities for units that don't have one yet.
pub fn spawn_health_bars(
    mut commands: Commands,
    units: Query<Entity, (With<UnitType>, With<Health>, Without<HealthBarBg>)>,
) {
    for entity in units.iter() {
        // Background (dark)
        let bg = commands
            .spawn((
                HealthBarBg,
                Sprite {
                    color: Color::srgba(0.1, 0.1, 0.1, 0.8),
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.1),
            ))
            .id();

        // Foreground (colored fill)
        let fg = commands
            .spawn((
                HealthBarFg,
                Sprite {
                    color: Color::srgb(0.2, 0.9, 0.2), // Green
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.2),
            ))
            .id();

        commands.entity(entity).add_children(&[bg, fg]);
    }
}

/// Update health bar fill width and color based on current HP.
pub fn update_health_bars(
    units: Query<(&Health, &Children), (With<UnitType>, Without<Dead>)>,
    mut bars: Query<(&mut Sprite, &mut Transform), With<HealthBarFg>>,
) {
    for (health, children) in units.iter() {
        let ratio: f32 = if health.max > cc_core::math::FIXED_ZERO {
            (health.current / health.max).to_num::<f32>().clamp(0.0, 1.0)
        } else {
            0.0
        };

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = bars.get_mut(child) {
                let fill_width = BAR_WIDTH * ratio;
                sprite.custom_size = Some(Vec2::new(fill_width, BAR_HEIGHT));

                // Offset so bar shrinks from right
                let x_offset = (fill_width - BAR_WIDTH) / 2.0;
                transform.translation.x = x_offset;

                // Color gradient: green > yellow > red
                sprite.color = if ratio > 0.6 {
                    Color::srgb(0.2, 0.9, 0.2) // Green
                } else if ratio > 0.3 {
                    Color::srgb(0.9, 0.9, 0.2) // Yellow
                } else {
                    Color::srgb(0.9, 0.2, 0.2) // Red
                };
            }
        }
    }
}
