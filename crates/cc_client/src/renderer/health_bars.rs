use bevy::prelude::*;

use crate::setup::UnitMesh;
use cc_core::components::{Dead, Health, UnitType};

/// Marker added to parent unit once health bars have been spawned.
#[derive(Component)]
pub struct HasHealthBar;

/// Marker for health bar background sprite.
#[derive(Component)]
pub struct HealthBarBg;

/// Marker for health bar foreground sprite (the colored fill).
#[derive(Component)]
pub struct HealthBarFg;

const BAR_WIDTH: f32 = 22.0;
const BAR_HEIGHT: f32 = 3.0;
const BAR_Y_OFFSET: f32 = 14.0;

/// Spawn health bar child entities for units that don't have one yet.
pub fn spawn_health_bars(
    mut commands: Commands,
    units: Query<Entity, (With<UnitMesh>, With<Health>, Without<HasHealthBar>)>,
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
                Visibility::Hidden,
            ))
            .id();

        // Foreground (colored fill)
        let fg = commands
            .spawn((
                HealthBarFg,
                Sprite {
                    color: Color::srgb(0.2, 0.9, 0.2),
                    custom_size: Some(Vec2::new(BAR_WIDTH, BAR_HEIGHT)),
                    ..default()
                },
                Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.2),
                Visibility::Hidden,
            ))
            .id();

        commands
            .entity(entity)
            .insert(HasHealthBar)
            .add_children(&[bg, fg]);
    }
}

/// Update health bar fill width, color gradient, and visibility based on current HP.
pub fn update_health_bars(
    units: Query<(&Health, &Children), (With<UnitType>, Without<Dead>)>,
    mut bg_bars: Query<&mut Visibility, (With<HealthBarBg>, Without<HealthBarFg>)>,
    mut fg_bars: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        (With<HealthBarFg>, Without<HealthBarBg>),
    >,
) {
    for (health, children) in units.iter() {
        let ratio: f32 = if health.max > cc_core::math::FIXED_ZERO {
            (health.current / health.max).to_num::<f32>().clamp(0.0, 1.0)
        } else {
            0.0
        };

        let damaged = ratio < 1.0;

        for child in children.iter() {
            // Update BG visibility
            if let Ok(mut vis) = bg_bars.get_mut(child) {
                *vis = if damaged {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }

            // Update FG bar
            if let Ok((mut sprite, mut transform, mut vis)) = fg_bars.get_mut(child) {
                *vis = if damaged {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };

                let fill_width = BAR_WIDTH * ratio;
                sprite.custom_size = Some(Vec2::new(fill_width, BAR_HEIGHT));

                // Offset so bar shrinks from right
                let x_offset = (fill_width - BAR_WIDTH) / 2.0;
                transform.translation.x = x_offset;

                // Smooth color gradient: green → yellow → red
                sprite.color = if ratio > 0.5 {
                    let t = (ratio - 0.5) * 2.0;
                    Color::srgb(0.2 + 0.7 * (1.0 - t), 0.9, 0.2)
                } else {
                    let t = ratio * 2.0;
                    Color::srgb(0.9, 0.9 * t, 0.2 * t)
                };
            }
        }
    }
}
