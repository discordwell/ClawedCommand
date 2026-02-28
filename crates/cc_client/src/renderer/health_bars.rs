use bevy::prelude::*;

use crate::setup::{BuildingMesh, UnitMesh};
use cc_core::components::{Building, BuildingKind, Dead, Health, Owner, UnitKind, UnitType};

/// Local player ID for showing enemy health bars.
const LOCAL_PLAYER: u8 = 0;

/// Marker added to parent unit once health bars have been spawned.
#[derive(Component)]
pub struct HasHealthBar;

/// Marker for health bar background sprite.
#[derive(Component)]
pub struct HealthBarBg;

/// Marker for health bar foreground sprite (the colored fill).
#[derive(Component)]
pub struct HealthBarFg;

/// World-space constants for unit health bars.
/// Child entities use inverse parent scale, so these map directly to screen pixels.
const UNIT_BAR_HEIGHT: f32 = 3.0;
const UNIT_BAR_Y_OFFSET: f32 = 12.0;

/// Constants for building health bars (parent scale = 1.0, no adjustment needed).
const BUILDING_BAR_HEIGHT: f32 = 6.0;
const BUILDING_BAR_Y_OFFSET: f32 = 40.0;

/// World-space bar width per unit kind. Inverse parent scale on the child entity
/// cancels the parent's unit_scale, so these values are the desired screen size.
fn bar_width_for_kind(kind: UnitKind) -> f32 {
    match kind {
        UnitKind::Pawdler | UnitKind::Mouser => 16.0,
        UnitKind::Nuisance | UnitKind::FerretSapper => 20.0,
        UnitKind::Hisser | UnitKind::Yowler | UnitKind::FlyingFox => 22.0,
        UnitKind::Catnapper => 26.0,
        UnitKind::Chonk => 30.0,
        UnitKind::MechCommander => 34.0,
    }
}

/// Bar width for buildings (not scaled by unit_scale, so no doubling needed).
fn bar_width_for_building(kind: BuildingKind) -> f32 {
    match kind {
        BuildingKind::TheBox => 30.0,
        BuildingKind::CatTree | BuildingKind::ServerRack => 28.0,
        BuildingKind::FishMarket | BuildingKind::ScratchingPost | BuildingKind::CatFlap => 24.0,
        BuildingKind::LitterBox | BuildingKind::LaserPointer => 20.0,
    }
}

/// Spawn health bar child entities for units and buildings that don't have one yet.
/// Unit health bars use inverse parent scale for consistent world-space sizing.
pub fn spawn_health_bars(
    mut commands: Commands,
    units: Query<(Entity, &Transform, Option<&UnitType>, Option<&Building>), (Or<(With<UnitMesh>, With<BuildingMesh>)>, With<Health>, Without<HasHealthBar>)>,
) {
    for (entity, parent_transform, unit_type, building) in units.iter() {
        let parent_scale = parent_transform.scale.x.max(0.01);

        let (bar_width, bar_height, bar_y_offset, inverse_scale) = if unit_type.is_some() {
            let kind = unit_type.unwrap().kind;
            (
                bar_width_for_kind(kind),
                UNIT_BAR_HEIGHT,
                UNIT_BAR_Y_OFFSET,
                1.0 / parent_scale,
            )
        } else if let Some(b) = building {
            (bar_width_for_building(b.kind), BUILDING_BAR_HEIGHT, BUILDING_BAR_Y_OFFSET, 1.0)
        } else {
            (20.0, BUILDING_BAR_HEIGHT, BUILDING_BAR_Y_OFFSET, 1.0)
        };

        // Y offset is in parent-local space; divide by parent_scale for world-space positioning
        let local_y = bar_y_offset * inverse_scale;

        // Background (dark)
        let bg = commands
            .spawn((
                HealthBarBg,
                Sprite {
                    color: Color::srgba(0.1, 0.1, 0.1, 0.8),
                    custom_size: Some(Vec2::new(bar_width, bar_height)),
                    ..default()
                },
                Transform::from_xyz(0.0, local_y, 0.1)
                    .with_scale(Vec3::splat(inverse_scale)),
                Visibility::Hidden,
            ))
            .id();

        // Foreground (colored fill)
        let fg = commands
            .spawn((
                HealthBarFg,
                Sprite {
                    color: Color::srgb(0.2, 0.9, 0.2),
                    custom_size: Some(Vec2::new(bar_width, bar_height)),
                    ..default()
                },
                Transform::from_xyz(0.0, local_y, 0.2)
                    .with_scale(Vec3::splat(inverse_scale)),
                Visibility::Hidden,
            ))
            .id();

        commands
            .entity(entity)
            .insert(HasHealthBar)
            .add_children(&[bg, fg]);
    }
}

/// Hide health bars when a unit dies.
pub fn hide_dead_health_bars(
    dead_units: Query<&Children, (Added<Dead>, With<HasHealthBar>)>,
    mut bg_bars: Query<&mut Visibility, (With<HealthBarBg>, Without<HealthBarFg>)>,
    mut fg_bars: Query<&mut Visibility, (With<HealthBarFg>, Without<HealthBarBg>)>,
) {
    for children in dead_units.iter() {
        for child in children.iter() {
            if let Ok(mut vis) = bg_bars.get_mut(child) {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut vis) = fg_bars.get_mut(child) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Update health bar fill width, color gradient, and visibility based on current HP.
/// Shows bars always for enemy units/buildings (not just when damaged).
pub fn update_health_bars(
    units: Query<
        (&Health, &Transform, Option<&UnitType>, Option<&Building>, &Owner, &Children),
        (Or<(With<UnitMesh>, With<BuildingMesh>)>, Without<Dead>),
    >,
    mut bg_bars: Query<(&mut Sprite, &mut Visibility), (With<HealthBarBg>, Without<HealthBarFg>)>,
    mut fg_bars: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        (With<HealthBarFg>, Without<HealthBarBg>),
    >,
) {
    for (health, parent_transform, unit_type, building, owner, children) in units.iter() {
        let ratio: f32 = if health.max > cc_core::math::FIXED_ZERO {
            (health.current / health.max).to_num::<f32>().clamp(0.0, 1.0)
        } else {
            0.0
        };

        let parent_scale = parent_transform.scale.x.max(0.01);
        let is_unit = unit_type.is_some();

        let (bar_width, bar_height) = if let Some(ut) = unit_type {
            (bar_width_for_kind(ut.kind), UNIT_BAR_HEIGHT)
        } else if let Some(b) = building {
            (bar_width_for_building(b.kind), BUILDING_BAR_HEIGHT)
        } else {
            (20.0, BUILDING_BAR_HEIGHT)
        };

        let is_enemy = owner.player_id != LOCAL_PLAYER;
        let damaged = ratio < 1.0;
        let should_show = damaged || is_enemy;

        for child in children.iter() {
            if let Ok((mut bg_sprite, mut vis)) = bg_bars.get_mut(child) {
                *vis = if should_show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                bg_sprite.custom_size = Some(Vec2::new(bar_width, bar_height));
            }

            if let Ok((mut sprite, mut transform, mut vis)) = fg_bars.get_mut(child) {
                *vis = if should_show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };

                let fill_width = bar_width * ratio;
                sprite.custom_size = Some(Vec2::new(fill_width, bar_height));

                // X offset is in parent-local space; for units with inverse scale,
                // divide by parent_scale so the offset aligns in world space.
                let x_offset = if is_unit {
                    (fill_width - bar_width) / (2.0 * parent_scale)
                } else {
                    (fill_width - bar_width) / 2.0
                };
                transform.translation.x = x_offset;

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
