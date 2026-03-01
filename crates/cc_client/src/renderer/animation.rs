use bevy::prelude::*;

use cc_core::components::{AttackStats, Dead, UnitType, Velocity};
use cc_core::math::FIXED_ZERO;

use crate::renderer::anim_assets::AnimSheets;
use crate::renderer::unit_gen::kind_index;
use crate::setup::UnitMesh;

/// Current animation state for a unit, derived from sim state each frame.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AnimState {
    #[default]
    Idle,
    Walk,
    Attack,
}

/// Previous animation state — used to detect transitions and swap sheets.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PrevAnimState(pub AnimState);

/// Frame range for the current animation within the sprite sheet.
#[derive(Component, Clone, Copy, Debug)]
pub struct AnimIndices {
    pub first: usize,
    pub last: usize,
}

impl Default for AnimIndices {
    fn default() -> Self {
        Self { first: 0, last: 3 }
    }
}

/// Timer driving animation frame advancement.
#[derive(Component, Deref, DerefMut)]
pub struct AnimTimer(pub Timer);

impl Default for AnimTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.5, TimerMode::Repeating))
    }
}

/// Frame rates per animation state.
const IDLE_FRAME_SECS: f32 = 0.5;
const WALK_FRAME_SECS: f32 = 0.15;
const ATTACK_FRAME_SECS: f32 = 0.1;

fn frame_duration(state: AnimState) -> f32 {
    match state {
        AnimState::Idle => IDLE_FRAME_SECS,
        AnimState::Walk => WALK_FRAME_SECS,
        AnimState::Attack => ATTACK_FRAME_SECS,
    }
}

/// Derive animation state from sim components.
/// - Walking if velocity is nonzero
/// - Attacking if cooldown is active (recently fired)
/// - Idle otherwise
pub fn derive_anim_state(
    mut query: Query<
        (&Velocity, &AttackStats, &mut AnimState),
        (With<UnitMesh>, Without<Dead>),
    >,
) {
    for (vel, attack_stats, mut anim_state) in query.iter_mut() {
        let new_state = if attack_stats.cooldown_remaining > 0
            && attack_stats.cooldown_remaining >= attack_stats.attack_speed.saturating_sub(3)
        {
            // Within the first 3 ticks of cooldown = just attacked
            AnimState::Attack
        } else if vel.dx != FIXED_ZERO || vel.dy != FIXED_ZERO {
            AnimState::Walk
        } else {
            AnimState::Idle
        };

        if *anim_state != new_state {
            *anim_state = new_state;
        }
    }
}

/// Advance animation frames, swap sprite sheet on state transition.
pub fn advance_animation(
    time: Res<Time>,
    anim_sheets: Option<Res<AnimSheets>>,
    mut query: Query<(
        &UnitType,
        &AnimState,
        &mut PrevAnimState,
        &mut AnimIndices,
        &mut AnimTimer,
        &mut Sprite,
    ), With<UnitMesh>>,
) {
    for (unit_type, anim_state, mut prev_state, mut indices, mut timer, mut sprite) in query.iter_mut() {
        // Detect state transition
        if prev_state.0 != *anim_state {
            prev_state.0 = *anim_state;
            timer.set_duration(std::time::Duration::from_secs_f32(frame_duration(*anim_state)));
            timer.reset();

            // Swap sheet image + reset atlas index on transition
            if let Some(ref sheets) = anim_sheets {
                let idx = kind_index(unit_type.kind);
                match anim_state {
                    AnimState::Walk => {
                        if let Some((ref img, ref layout)) = sheets.walk[idx] {
                            sprite.image = img.clone();
                            sprite.texture_atlas = Some(TextureAtlas {
                                layout: layout.clone(),
                                index: 0,
                            });
                            indices.first = 0;
                            indices.last = 3;
                        }
                    }
                    AnimState::Attack => {
                        if let Some((ref img, ref layout)) = sheets.attack[idx] {
                            sprite.image = img.clone();
                            sprite.texture_atlas = Some(TextureAtlas {
                                layout: layout.clone(),
                                index: 0,
                            });
                            indices.first = 0;
                            indices.last = 3;
                        }
                    }
                    AnimState::Idle => {
                        // Return to idle: clear atlas (single-frame idle sprite)
                        sprite.texture_atlas = None;
                    }
                }
            }
        }

        // Advance frame timer
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                if atlas.index >= indices.last {
                    atlas.index = indices.first;
                } else {
                    atlas.index += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anim_state_default_is_idle() {
        assert_eq!(AnimState::default(), AnimState::Idle);
    }

    #[test]
    fn prev_anim_state_default_is_idle() {
        assert_eq!(PrevAnimState::default().0, AnimState::Idle);
    }

    #[test]
    fn anim_indices_default_range() {
        let indices = AnimIndices::default();
        assert_eq!(indices.first, 0);
        assert_eq!(indices.last, 3);
    }

    #[test]
    fn frame_durations_are_ordered() {
        assert!(frame_duration(AnimState::Attack) < frame_duration(AnimState::Walk));
        assert!(frame_duration(AnimState::Walk) < frame_duration(AnimState::Idle));
    }

    #[test]
    fn frame_durations_are_positive() {
        assert!(frame_duration(AnimState::Idle) > 0.0);
        assert!(frame_duration(AnimState::Walk) > 0.0);
        assert!(frame_duration(AnimState::Attack) > 0.0);
    }

    #[test]
    fn anim_state_equality() {
        assert_eq!(AnimState::Idle, AnimState::Idle);
        assert_eq!(AnimState::Walk, AnimState::Walk);
        assert_eq!(AnimState::Attack, AnimState::Attack);
        assert_ne!(AnimState::Idle, AnimState::Walk);
        assert_ne!(AnimState::Walk, AnimState::Attack);
    }

    #[test]
    fn idle_frame_duration_500ms() {
        assert!((frame_duration(AnimState::Idle) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn walk_frame_duration_150ms() {
        assert!((frame_duration(AnimState::Walk) - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn attack_frame_duration_100ms() {
        assert!((frame_duration(AnimState::Attack) - 0.1).abs() < f32::EPSILON);
    }
}
