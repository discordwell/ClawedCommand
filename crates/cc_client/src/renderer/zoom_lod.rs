use bevy::prelude::*;

use crate::renderer::health_bars::{HealthBarBg, HealthBarFg};
use crate::renderer::props::Prop;
use crate::renderer::selection::SelectionRing;
use crate::setup::UnitMesh;

/// Zoom level-of-detail tier. Controls which visual elements are shown.
///
/// - **Tactical**: Full sprites, health bars, props, terrain borders, water animation.
/// - **Strategic**: Simplified colored-dot icons, most detail hidden for readability.
#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum ZoomTier {
    #[default]
    Tactical,
    Strategic,
}

/// Threshold camera scale at which we switch TO Strategic view (zooming out).
const STRATEGIC_THRESHOLD: f32 = 2.0;
/// Threshold camera scale at which we switch BACK to Tactical view (zooming in).
/// Lower than STRATEGIC_THRESHOLD to prevent flickering near the boundary.
const TACTICAL_THRESHOLD: f32 = 1.8;

/// Marker component for the simplified strategic-zoom icon child entity.
#[derive(Component)]
pub struct StrategicIcon;

/// Strategic icon radius in world units.
pub const ICON_RADIUS: f32 = 4.0;
/// Z-offset for strategic icon above the unit sprite.
const ICON_Z_OFFSET: f32 = 0.1;

/// Run condition: returns true when the current zoom tier is Tactical.
pub fn is_tactical(tier: Res<ZoomTier>) -> bool {
    *tier == ZoomTier::Tactical
}

/// Spawn a `StrategicIcon` child entity for a unit and attach it as a child.
/// The icon uses an inverse scale to cancel out the parent unit's transform scale,
/// so all icons appear at a uniform size regardless of unit type.
pub fn spawn_strategic_icon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    parent_entity: Entity,
    parent_scale: f32,
    team_color: Color,
    tier: &ZoomTier,
) {
    let icon_mesh = meshes.add(Circle::new(ICON_RADIUS));
    let icon_mat = materials.add(ColorMaterial::from_color(team_color));
    let inverse_scale = 1.0 / parent_scale;
    let icon_vis = if *tier == ZoomTier::Strategic {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let icon = commands
        .spawn((
            StrategicIcon,
            Mesh2d(icon_mesh),
            MeshMaterial2d(icon_mat),
            Transform::from_xyz(0.0, 0.0, ICON_Z_OFFSET).with_scale(Vec3::splat(inverse_scale)),
            icon_vis,
        ))
        .id();
    commands.entity(parent_entity).add_children(&[icon]);
}

/// Reads camera orthographic scale and updates `ZoomTier` with hysteresis.
pub fn detect_zoom_tier(camera: Single<&Projection, With<Camera2d>>, mut tier: ResMut<ZoomTier>) {
    let Projection::Orthographic(ref ortho) = **camera else {
        return;
    };
    let scale = ortho.scale;

    match *tier {
        ZoomTier::Tactical => {
            if scale >= STRATEGIC_THRESHOLD {
                *tier = ZoomTier::Strategic;
            }
        }
        ZoomTier::Strategic => {
            if scale <= TACTICAL_THRESHOLD {
                *tier = ZoomTier::Tactical;
            }
        }
    }
}

/// Toggles visibility of child visual elements when `ZoomTier` changes.
///
/// Parent entities (with UnitMesh + Sprite) stay `Visibility::Inherited` always so
/// children remain renderable. The parent sprite alpha is handled separately by
/// `render_selection_indicators` which checks ZoomTier when setting colors.
///
/// Uses `Has<T>` accessors instead of separate queries to avoid combinatorial
/// `Without<>` filters and simplify adding new child component types.
pub fn toggle_lod_visuals(
    tier: Res<ZoomTier>,
    unit_query: Query<&Children, With<UnitMesh>>,
    mut child_query: Query<
        (
            &mut Visibility,
            Has<StrategicIcon>,
            Has<HealthBarBg>,
            Has<HealthBarFg>,
            Has<SelectionRing>,
        ),
        Without<Prop>,
    >,
    mut prop_query: Query<&mut Visibility, (With<Prop>, Without<UnitMesh>)>,
) {
    let is_strategic = *tier == ZoomTier::Strategic;

    for children in unit_query.iter() {
        for child in children.iter() {
            if let Ok((mut vis, is_icon, is_hb_bg, is_hb_fg, is_ring)) = child_query.get_mut(child)
            {
                if is_icon {
                    *vis = if is_strategic {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                } else if is_hb_bg || is_hb_fg || is_ring {
                    *vis = if is_strategic {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    };
                }
            }
        }
    }

    // Toggle standalone prop entities
    let prop_vis = if is_strategic {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in prop_query.iter_mut() {
        *vis = prop_vis;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_tier_default_is_tactical() {
        assert_eq!(ZoomTier::default(), ZoomTier::Tactical);
    }

    #[test]
    fn hysteresis_constants_are_valid() {
        assert!(TACTICAL_THRESHOLD < STRATEGIC_THRESHOLD);
        assert!(TACTICAL_THRESHOLD > 0.0);
        assert!(STRATEGIC_THRESHOLD > 0.0);
    }

    #[test]
    fn hysteresis_gap_prevents_flicker() {
        let gap = STRATEGIC_THRESHOLD - TACTICAL_THRESHOLD;
        assert!(
            gap >= 0.1,
            "Hysteresis gap {gap} is too small, risk of flicker"
        );
    }

    #[test]
    fn zoom_tier_equality() {
        assert_eq!(ZoomTier::Tactical, ZoomTier::Tactical);
        assert_eq!(ZoomTier::Strategic, ZoomTier::Strategic);
        assert_ne!(ZoomTier::Tactical, ZoomTier::Strategic);
    }

    #[test]
    fn zoom_tier_copy_clone() {
        let tier = ZoomTier::Strategic;
        let copied = tier;
        let cloned = tier.clone();
        assert_eq!(tier, copied);
        assert_eq!(tier, cloned);
    }

    #[test]
    fn icon_radius_is_positive() {
        assert!(ICON_RADIUS > 0.0);
    }

    /// Simulate the hysteresis logic without Bevy ECS.
    fn simulate_tier_transition(current: ZoomTier, scale: f32) -> ZoomTier {
        match current {
            ZoomTier::Tactical => {
                if scale >= STRATEGIC_THRESHOLD {
                    ZoomTier::Strategic
                } else {
                    ZoomTier::Tactical
                }
            }
            ZoomTier::Strategic => {
                if scale <= TACTICAL_THRESHOLD {
                    ZoomTier::Tactical
                } else {
                    ZoomTier::Strategic
                }
            }
        }
    }

    #[test]
    fn tactical_to_strategic_at_threshold() {
        assert_eq!(
            simulate_tier_transition(ZoomTier::Tactical, STRATEGIC_THRESHOLD),
            ZoomTier::Strategic
        );
    }

    #[test]
    fn tactical_stays_below_threshold() {
        assert_eq!(
            simulate_tier_transition(ZoomTier::Tactical, STRATEGIC_THRESHOLD - 0.01),
            ZoomTier::Tactical
        );
    }

    #[test]
    fn strategic_to_tactical_at_threshold() {
        assert_eq!(
            simulate_tier_transition(ZoomTier::Strategic, TACTICAL_THRESHOLD),
            ZoomTier::Tactical
        );
    }

    #[test]
    fn strategic_stays_above_threshold() {
        assert_eq!(
            simulate_tier_transition(ZoomTier::Strategic, TACTICAL_THRESHOLD + 0.01),
            ZoomTier::Strategic
        );
    }

    #[test]
    fn hysteresis_prevents_flicker_in_gap() {
        let gap_scale = (TACTICAL_THRESHOLD + STRATEGIC_THRESHOLD) / 2.0;

        assert_eq!(
            simulate_tier_transition(ZoomTier::Tactical, gap_scale),
            ZoomTier::Tactical,
            "Scale {gap_scale} in hysteresis gap should not trigger Tactical->Strategic"
        );

        assert_eq!(
            simulate_tier_transition(ZoomTier::Strategic, gap_scale),
            ZoomTier::Strategic,
            "Scale {gap_scale} in hysteresis gap should not trigger Strategic->Tactical"
        );
    }

    #[test]
    fn full_zoom_cycle() {
        let mut tier = ZoomTier::Tactical;

        tier = simulate_tier_transition(tier, 1.0);
        assert_eq!(tier, ZoomTier::Tactical);

        tier = simulate_tier_transition(tier, 1.5);
        assert_eq!(tier, ZoomTier::Tactical);

        tier = simulate_tier_transition(tier, 1.9); // In gap
        assert_eq!(tier, ZoomTier::Tactical);

        tier = simulate_tier_transition(tier, 2.0); // Hit threshold
        assert_eq!(tier, ZoomTier::Strategic);

        tier = simulate_tier_transition(tier, 1.9); // In gap -- stays Strategic
        assert_eq!(tier, ZoomTier::Strategic);

        tier = simulate_tier_transition(tier, 1.8); // Hit lower threshold
        assert_eq!(tier, ZoomTier::Tactical);

        tier = simulate_tier_transition(tier, 1.0);
        assert_eq!(tier, ZoomTier::Tactical);
    }
}
