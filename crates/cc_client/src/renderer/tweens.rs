use bevy::prelude::*;

use crate::renderer::animation::AnimState;
use crate::setup::{UnitMesh, unit_scale};
use cc_core::components::{Dead, Health, UnitKind};

// ---------------------------------------------------------------------------
// Easing helpers
// ---------------------------------------------------------------------------

/// Quadratic ease-out: decelerates toward 1.0.
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Quadratic ease-in-out: accelerates then decelerates.
fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

// ---------------------------------------------------------------------------
// Archetype classification (from unit_scale thresholds)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Archetype {
    Worker,
    Small,
    Medium,
    Heavy,
    Tank,
    Hero,
}

fn archetype_of(kind: UnitKind) -> Archetype {
    let s = unit_scale(kind);
    if s <= 0.17 {
        Archetype::Worker
    } else if s <= 0.20 {
        Archetype::Small
    } else if s <= 0.26 {
        Archetype::Medium
    } else if s <= 0.32 {
        Archetype::Heavy
    } else if s <= 0.40 {
        Archetype::Tank
    } else {
        Archetype::Hero
    }
}

// ---------------------------------------------------------------------------
// TweenParams — per-unit-kind animation configuration
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TweenParams {
    // Idle
    pub bob_amplitude: f32,    // pixels
    pub bob_speed: f32,        // Hz
    pub breathe_scale: f32,    // ±fraction (e.g. 0.02 = ±2%)
    pub breathe_speed: f32,    // Hz

    // Walk
    pub stride_bounce: f32,    // pixels
    pub stride_speed: f32,     // Hz
    pub lean_angle: f32,       // radians
    pub lean_speed: f32,       // Hz

    // Attack
    pub lunge_distance: f32,   // pixels
    pub lunge_duration: f32,   // seconds
    pub recoil_duration: f32,  // seconds

    // Hit reaction
    pub hit_flash_duration: f32, // seconds
    pub knockback_pixels: f32,   // pixels
    pub knockback_duration: f32, // seconds

    // Spawn
    pub spawn_pop_duration: f32, // seconds
    pub spawn_overshoot: f32,    // scale factor (e.g. 1.3 = 30% overshoot)

    // Transition blending
    pub blend_duration: f32, // seconds
}

impl TweenParams {
    fn for_archetype(arch: Archetype) -> Self {
        match arch {
            Archetype::Worker => Self {
                bob_amplitude: 0.6,
                bob_speed: 3.0,
                breathe_scale: 0.015,
                breathe_speed: 2.5,
                stride_bounce: 1.0,
                stride_speed: 6.0,
                lean_angle: 0.04,
                lean_speed: 6.0,
                lunge_distance: 1.5,
                lunge_duration: 0.12,
                recoil_duration: 0.15,
                hit_flash_duration: 0.2,
                knockback_pixels: 1.5,
                knockback_duration: 0.15,
                spawn_pop_duration: 0.3,
                spawn_overshoot: 1.25,
                blend_duration: 0.1,
            },
            Archetype::Small => Self {
                bob_amplitude: 0.8,
                bob_speed: 3.5,
                breathe_scale: 0.02,
                breathe_speed: 2.8,
                stride_bounce: 1.2,
                stride_speed: 7.0,
                lean_angle: 0.06,
                lean_speed: 7.0,
                lunge_distance: 2.0,
                lunge_duration: 0.1,
                recoil_duration: 0.12,
                hit_flash_duration: 0.2,
                knockback_pixels: 2.0,
                knockback_duration: 0.15,
                spawn_pop_duration: 0.25,
                spawn_overshoot: 1.3,
                blend_duration: 0.1,
            },
            Archetype::Medium => Self {
                bob_amplitude: 0.5,
                bob_speed: 2.5,
                breathe_scale: 0.015,
                breathe_speed: 2.0,
                stride_bounce: 0.8,
                stride_speed: 5.0,
                lean_angle: 0.04,
                lean_speed: 5.0,
                lunge_distance: 2.5,
                lunge_duration: 0.15,
                recoil_duration: 0.18,
                hit_flash_duration: 0.25,
                knockback_pixels: 1.5,
                knockback_duration: 0.18,
                spawn_pop_duration: 0.3,
                spawn_overshoot: 1.2,
                blend_duration: 0.12,
            },
            Archetype::Heavy => Self {
                bob_amplitude: 0.3,
                bob_speed: 1.8,
                breathe_scale: 0.01,
                breathe_speed: 1.5,
                stride_bounce: 0.5,
                stride_speed: 3.5,
                lean_angle: 0.025,
                lean_speed: 3.5,
                lunge_distance: 2.0,
                lunge_duration: 0.2,
                recoil_duration: 0.25,
                hit_flash_duration: 0.3,
                knockback_pixels: 1.0,
                knockback_duration: 0.2,
                spawn_pop_duration: 0.35,
                spawn_overshoot: 1.15,
                blend_duration: 0.15,
            },
            Archetype::Tank => Self {
                bob_amplitude: 0.15,
                bob_speed: 1.2,
                breathe_scale: 0.008,
                breathe_speed: 1.2,
                stride_bounce: 0.3,
                stride_speed: 2.5,
                lean_angle: 0.015,
                lean_speed: 2.5,
                lunge_distance: 1.5,
                lunge_duration: 0.3,
                recoil_duration: 0.35,
                hit_flash_duration: 0.35,
                knockback_pixels: 0.5,
                knockback_duration: 0.25,
                spawn_pop_duration: 0.4,
                spawn_overshoot: 1.1,
                blend_duration: 0.18,
            },
            Archetype::Hero => Self {
                bob_amplitude: 0.4,
                bob_speed: 2.0,
                breathe_scale: 0.018,
                breathe_speed: 1.8,
                stride_bounce: 0.6,
                stride_speed: 4.0,
                lean_angle: 0.05,
                lean_speed: 4.0,
                lunge_distance: 3.5,
                lunge_duration: 0.2,
                recoil_duration: 0.25,
                hit_flash_duration: 0.3,
                knockback_pixels: 0.8,
                knockback_duration: 0.2,
                spawn_pop_duration: 0.45,
                spawn_overshoot: 1.2,
                blend_duration: 0.15,
            },
        }
    }
}

/// Returns tween parameters for a given unit kind.
/// Derived from archetype (based on unit_scale thresholds).
fn tween_params(kind: UnitKind) -> TweenParams {
    TweenParams::for_archetype(archetype_of(kind))
}

// ---------------------------------------------------------------------------
// Pure math offset functions (testable without Bevy)
// ---------------------------------------------------------------------------

/// Idle: sin-wave bob (Y offset) + breathe (scale offset).
fn idle_offsets(elapsed: f32, params: &TweenParams) -> (f32, f32) {
    let dy = (elapsed * params.bob_speed * std::f32::consts::TAU).sin() * params.bob_amplitude;
    let scale_offset =
        (elapsed * params.breathe_speed * std::f32::consts::TAU).sin() * params.breathe_scale;
    (dy, scale_offset)
}

/// Walk: stride bounce (Y offset) + lean sway (rotation Z in radians).
fn walk_offsets(elapsed: f32, params: &TweenParams) -> (f32, f32) {
    let dy =
        (elapsed * params.stride_speed * std::f32::consts::TAU).sin().abs() * params.stride_bounce;
    let rotation_z = (elapsed * params.lean_speed * std::f32::consts::TAU).sin() * params.lean_angle;
    (dy, rotation_z)
}

/// Attack lunge: ease-out forward then ease-in-out recoil.
/// `t` is normalized progress [0..1] over (lunge_duration + recoil_duration).
fn attack_offset(t: f32, params: &TweenParams) -> f32 {
    let total = params.lunge_duration + params.recoil_duration;
    if total <= 0.0 {
        return 0.0;
    }
    let lunge_frac = params.lunge_duration / total;
    if t <= lunge_frac {
        // Lunge forward
        let lt = (t / lunge_frac).clamp(0.0, 1.0);
        ease_out(lt) * params.lunge_distance
    } else {
        // Recoil back
        let rt = ((t - lunge_frac) / (1.0 - lunge_frac)).clamp(0.0, 1.0);
        (1.0 - ease_in_out(rt)) * params.lunge_distance
    }
}

/// Spawn pop-in: 0 → overshoot → 1.0.
/// `t` is normalized [0..1] over spawn_pop_duration.
fn spawn_scale(t: f32, params: &TweenParams) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t >= 1.0 {
        return 1.0;
    }
    // Phase 1 (0..0.5): ease-out from 0 to overshoot
    // Phase 2 (0.5..1): ease-in-out from overshoot to 1.0
    if t < 0.5 {
        let p = t / 0.5;
        ease_out(p) * params.spawn_overshoot
    } else {
        let p = (t - 0.5) / 0.5;
        let from = params.spawn_overshoot;
        from + ease_in_out(p) * (1.0 - from)
    }
}

/// Hit flash factor: 1.0 at start, quadratic decay to 0.0 when expired.
fn hit_flash_factor(remaining: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return 0.0;
    }
    let t = (remaining / duration).clamp(0.0, 1.0);
    t * t
}

/// Knockback offset: max at start, ease-in-out snap back to 0.
fn knockback_offset(remaining: f32, params: &TweenParams) -> f32 {
    if params.knockback_duration <= 0.0 {
        return 0.0;
    }
    let t = (remaining / params.knockback_duration).clamp(0.0, 1.0);
    ease_in_out(t) * params.knockback_pixels
}

// ---------------------------------------------------------------------------
// TweenState — per-entity runtime component
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct TweenState {
    pub elapsed: f32,
    pub prev_state: AnimState,
    pub blend_out: f32,
    pub blend_from: AnimState,
    pub hit_timer: f32,
    pub knockback_timer: f32,
    pub prev_health: f32,
    pub spawn_timer: f32,
    pub params: TweenParams,
    /// Base scale from unit_scale(), cached to avoid re-lookup.
    pub base_scale: f32,
}

impl TweenState {
    pub fn new(kind: UnitKind) -> Self {
        let params = tween_params(kind);
        let spawn_dur = params.spawn_pop_duration;
        Self {
            elapsed: 0.0,
            prev_state: AnimState::Idle,
            blend_out: 0.0,
            blend_from: AnimState::Idle,
            hit_timer: 0.0,
            knockback_timer: 0.0,
            prev_health: f32::MAX, // avoids false flash on spawn
            spawn_timer: spawn_dur,
            params,
            base_scale: unit_scale(kind),
        }
    }
}

// ---------------------------------------------------------------------------
// Main system: apply_unit_tweens
// ---------------------------------------------------------------------------

/// Transform-based tween animation layered on top of sync_unit_sprites.
/// - translation.y: additive (bob/bounce/knockback added to sim position)
/// - rotation: owned by this system (no other system sets rotation on living units)
/// - scale: owned by this system (cached base_scale × breathe × spawn_pop)
///
/// Hit flash modulates Sprite.color toward white; render_selection_indicators
/// resets color each frame before this system runs, so flash is always relative
/// to the correct base tint.
///
/// Runs after sync_unit_sprites, render_selection_indicators, and advance_animation.
/// Only active in Tactical zoom tier. Disjoint with death system (Without<Dead>).
pub fn apply_unit_tweens(
    time: Res<Time>,
    mut query: Query<
        (
            &AnimState,
            &Health,
            &mut TweenState,
            &mut Transform,
            Option<&mut Sprite>,
        ),
        (With<UnitMesh>, Without<Dead>),
    >,
) {
    let dt = time.delta_secs();

    for (anim_state, health, mut tween, mut transform, sprite) in query.iter_mut() {
        // 1. Tick elapsed clock
        tween.elapsed += dt;

        // 2. Detect health decrease → trigger hit flash + knockback
        let current_hp: f32 = health.current.to_num();
        if current_hp < tween.prev_health && tween.prev_health != f32::MAX {
            tween.hit_timer = tween.params.hit_flash_duration;
            tween.knockback_timer = tween.params.knockback_duration;
        }
        tween.prev_health = current_hp;

        // 3. Detect AnimState change → start blend transition
        if *anim_state != tween.prev_state {
            tween.blend_out = tween.params.blend_duration;
            tween.blend_from = tween.prev_state;
            tween.prev_state = *anim_state;
        }

        // 4. Compute current state offsets
        let (cur_dy, cur_scale_offset, cur_rotation) = state_offsets(
            *anim_state,
            tween.elapsed,
            &tween.params,
        );

        // 5. Blend with previous state if transitioning
        let (dy, scale_offset, rotation) = if tween.blend_out > 0.0 {
            let blend_t = 1.0 - (tween.blend_out / tween.params.blend_duration).clamp(0.0, 1.0);
            let (prev_dy, prev_scale, prev_rot) = state_offsets(
                tween.blend_from,
                tween.elapsed,
                &tween.params,
            );
            (
                prev_dy + (cur_dy - prev_dy) * blend_t,
                prev_scale + (cur_scale_offset - prev_scale) * blend_t,
                prev_rot + (cur_rotation - prev_rot) * blend_t,
            )
        } else {
            (cur_dy, cur_scale_offset, cur_rotation)
        };

        // 6. Spawn pop scale
        let spawn_scale_mul = if tween.spawn_timer > 0.0 && tween.params.spawn_pop_duration > 0.0 {
            let t = 1.0 - (tween.spawn_timer / tween.params.spawn_pop_duration).clamp(0.0, 1.0);
            spawn_scale(t, &tween.params)
        } else {
            1.0
        };

        // 7. Knockback offset
        let kb = if tween.knockback_timer > 0.0 {
            knockback_offset(tween.knockback_timer, &tween.params)
        } else {
            0.0
        };

        // 8. Apply to Transform (additive on top of sync_unit_sprites)
        transform.translation.y += dy + kb;
        transform.rotation = Quat::from_rotation_z(rotation);
        let final_scale = tween.base_scale * (1.0 + scale_offset) * spawn_scale_mul;
        transform.scale = Vec3::splat(final_scale);

        // 9. Hit flash on Sprite.color (lerp toward white, preserving alpha)
        if let Some(mut sprite) = sprite {
            if tween.hit_timer > 0.0 {
                let flash = hit_flash_factor(tween.hit_timer, tween.params.hit_flash_duration);
                let linear = sprite.color.to_linear();
                sprite.color = Color::LinearRgba(LinearRgba::new(
                    (linear.red + flash * (1.0 - linear.red)).min(1.0),
                    (linear.green + flash * (1.0 - linear.green)).min(1.0),
                    (linear.blue + flash * (1.0 - linear.blue)).min(1.0),
                    linear.alpha, // preserve alpha
                ));
            }
        }

        // 10. Tick down one-shot timers
        tween.hit_timer = (tween.hit_timer - dt).max(0.0);
        tween.knockback_timer = (tween.knockback_timer - dt).max(0.0);
        tween.spawn_timer = (tween.spawn_timer - dt).max(0.0);
        tween.blend_out = (tween.blend_out - dt).max(0.0);
    }
}

/// Compute (dy, scale_offset, rotation_z) for a given animation state.
fn state_offsets(state: AnimState, elapsed: f32, params: &TweenParams) -> (f32, f32, f32) {
    match state {
        AnimState::Idle => {
            let (dy, scale) = idle_offsets(elapsed, params);
            (dy, scale, 0.0)
        }
        AnimState::Walk => {
            let (dy, rot) = walk_offsets(elapsed, params);
            (dy, 0.0, rot)
        }
        AnimState::Attack => {
            let total = params.lunge_duration + params.recoil_duration;
            let t = if total > 0.0 {
                (elapsed % total) / total
            } else {
                0.0
            };
            let dy = attack_offset(t, params);
            (dy, 0.0, 0.0)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Easing ---

    #[test]
    fn ease_out_boundaries() {
        assert!((ease_out(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_out(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_out_monotonic() {
        let mut prev = ease_out(0.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let val = ease_out(t);
            assert!(val >= prev, "ease_out not monotonic at t={t}");
            prev = val;
        }
    }

    #[test]
    fn ease_in_out_boundaries() {
        assert!((ease_in_out(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_in_out(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ease_in_out_monotonic() {
        let mut prev = ease_in_out(0.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let val = ease_in_out(t);
            assert!(val >= prev, "ease_in_out not monotonic at t={t}");
            prev = val;
        }
    }

    // --- Idle offsets ---

    #[test]
    fn idle_offsets_bounded() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        for i in 0..1000 {
            let elapsed = i as f32 * 0.01;
            let (dy, scale) = idle_offsets(elapsed, &params);
            assert!(dy.abs() <= params.bob_amplitude + 0.001);
            assert!(scale.abs() <= params.breathe_scale + 0.001);
        }
    }

    // --- Walk offsets ---

    #[test]
    fn walk_offsets_bounded() {
        let params = TweenParams::for_archetype(Archetype::Small);
        for i in 0..1000 {
            let elapsed = i as f32 * 0.01;
            let (dy, rot) = walk_offsets(elapsed, &params);
            assert!(dy >= -0.001, "walk dy should be non-negative (abs sin)");
            assert!(dy <= params.stride_bounce + 0.001);
            assert!(rot.abs() <= params.lean_angle + 0.001);
        }
    }

    // --- Attack lunge ---

    #[test]
    fn attack_lunge_starts_at_zero() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        let offset = attack_offset(0.0, &params);
        assert!(offset.abs() < 0.01, "attack should start near 0, got {offset}");
    }

    #[test]
    fn attack_lunge_peaks_at_lunge_distance() {
        let params = TweenParams::for_archetype(Archetype::Hero);
        let total = params.lunge_duration + params.recoil_duration;
        let lunge_frac = params.lunge_duration / total;
        let peak = attack_offset(lunge_frac, &params);
        assert!(
            (peak - params.lunge_distance).abs() < 0.1,
            "peak should be near lunge_distance ({}) but got {peak}",
            params.lunge_distance
        );
    }

    #[test]
    fn attack_lunge_returns_to_zero() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        let offset = attack_offset(1.0, &params);
        assert!(offset.abs() < 0.01, "attack should end near 0, got {offset}");
    }

    // --- Spawn scale ---

    #[test]
    fn spawn_scale_starts_near_zero() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        let s = spawn_scale(0.0, &params);
        assert!(s < 0.1, "spawn should start near 0, got {s}");
    }

    #[test]
    fn spawn_scale_overshoots() {
        let params = TweenParams::for_archetype(Archetype::Small);
        // At the midpoint, scale should be at overshoot
        let s = spawn_scale(0.5, &params);
        assert!(
            (s - params.spawn_overshoot).abs() < 0.05,
            "spawn should overshoot to {}, got {s}",
            params.spawn_overshoot
        );
    }

    #[test]
    fn spawn_scale_settles_to_one() {
        let params = TweenParams::for_archetype(Archetype::Tank);
        let s = spawn_scale(1.0, &params);
        assert!((s - 1.0).abs() < f32::EPSILON, "spawn should settle to 1.0, got {s}");
    }

    // --- Hit flash ---

    #[test]
    fn hit_flash_full_at_start() {
        let factor = hit_flash_factor(0.3, 0.3);
        assert!((factor - 1.0).abs() < 0.01, "flash should be ~1.0 at start, got {factor}");
    }

    #[test]
    fn hit_flash_zero_when_expired() {
        let factor = hit_flash_factor(0.0, 0.3);
        assert!(factor.abs() < f32::EPSILON, "flash should be 0.0 when expired, got {factor}");
    }

    // --- Knockback ---

    #[test]
    fn knockback_max_at_start() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        let kb = knockback_offset(params.knockback_duration, &params);
        assert!(
            (kb - params.knockback_pixels).abs() < 0.01,
            "knockback should be max at start, got {kb}"
        );
    }

    #[test]
    fn knockback_zero_when_expired() {
        let params = TweenParams::for_archetype(Archetype::Medium);
        let kb = knockback_offset(0.0, &params);
        assert!(kb.abs() < f32::EPSILON, "knockback should be 0 when expired, got {kb}");
    }

    // --- All unit kinds have valid params ---

    #[test]
    fn all_unit_kinds_have_positive_params() {
        let all_kinds = [
            // Cat
            UnitKind::Pawdler, UnitKind::Nuisance, UnitKind::Chonk, UnitKind::FlyingFox,
            UnitKind::Hisser, UnitKind::Yowler, UnitKind::Mouser, UnitKind::Catnapper,
            UnitKind::FerretSapper, UnitKind::MechCommander,
            // Murder
            UnitKind::MurderScrounger, UnitKind::Sentinel, UnitKind::Rookclaw,
            UnitKind::Magpike, UnitKind::Magpyre, UnitKind::Jaycaller, UnitKind::Jayflicker,
            UnitKind::Dusktalon, UnitKind::Hootseer, UnitKind::CorvusRex,
            // Seekers
            UnitKind::Delver, UnitKind::Ironhide, UnitKind::Cragback, UnitKind::Warden,
            UnitKind::Sapjaw, UnitKind::Wardenmother, UnitKind::SeekerTunneler,
            UnitKind::Embermaw, UnitKind::Dustclaw, UnitKind::Gutripper,
            // Clawed
            UnitKind::Nibblet, UnitKind::Swarmer, UnitKind::Gnawer, UnitKind::Shrieker,
            UnitKind::Tunneler, UnitKind::Sparks, UnitKind::Quillback, UnitKind::Whiskerwitch,
            UnitKind::Plaguetail, UnitKind::WarrenMarshal,
            // Croak
            UnitKind::Ponderer, UnitKind::Regeneron, UnitKind::Broodmother, UnitKind::Gulper,
            UnitKind::Eftsaber, UnitKind::Croaker, UnitKind::Leapfrog, UnitKind::Shellwarden,
            UnitKind::Bogwhisper, UnitKind::MurkCommander,
            // LLAMA
            UnitKind::Scrounger, UnitKind::Bandit, UnitKind::HeapTitan, UnitKind::GlitchRat,
            UnitKind::PatchPossum, UnitKind::GreaseMonkey, UnitKind::DeadDropUnit,
            UnitKind::Wrecker, UnitKind::DumpsterDiver, UnitKind::JunkyardKing,
        ];
        assert_eq!(all_kinds.len(), 60, "should cover all 60 unit kinds");

        for kind in all_kinds {
            let p = tween_params(kind);
            assert!(p.bob_amplitude > 0.0, "{kind:?} bob_amplitude");
            assert!(p.bob_speed > 0.0, "{kind:?} bob_speed");
            assert!(p.breathe_scale > 0.0, "{kind:?} breathe_scale");
            assert!(p.breathe_speed > 0.0, "{kind:?} breathe_speed");
            assert!(p.stride_bounce > 0.0, "{kind:?} stride_bounce");
            assert!(p.lunge_distance > 0.0, "{kind:?} lunge_distance");
            assert!(p.hit_flash_duration > 0.0, "{kind:?} hit_flash_duration");
            assert!(p.knockback_pixels > 0.0, "{kind:?} knockback_pixels");
            assert!(p.spawn_pop_duration > 0.0, "{kind:?} spawn_pop_duration");
            assert!(p.spawn_overshoot > 1.0, "{kind:?} spawn_overshoot");
            assert!(p.blend_duration > 0.0, "{kind:?} blend_duration");
        }
    }

    // --- Blend decrement ---

    #[test]
    fn blend_reaches_zero_within_duration() {
        let params = TweenParams::for_archetype(Archetype::Heavy);
        let dt = 0.016; // ~60fps
        let mut remaining = params.blend_duration;
        let steps = (params.blend_duration / dt).ceil() as usize + 1;
        for _ in 0..steps {
            remaining = (remaining - dt).max(0.0);
        }
        assert!(
            remaining.abs() < f32::EPSILON,
            "blend should reach 0 within duration, got {remaining}"
        );
    }

    // --- Archetype classification ---

    #[test]
    fn archetype_classification_sanity() {
        assert_eq!(archetype_of(UnitKind::Pawdler), Archetype::Worker);
        assert_eq!(archetype_of(UnitKind::Nuisance), Archetype::Small);
        assert_eq!(archetype_of(UnitKind::Hisser), Archetype::Medium);
        assert_eq!(archetype_of(UnitKind::Yowler), Archetype::Heavy);
        assert_eq!(archetype_of(UnitKind::Chonk), Archetype::Tank);
        assert_eq!(archetype_of(UnitKind::MechCommander), Archetype::Hero);
    }

    // --- Easing clamping ---

    #[test]
    fn easing_clamps_out_of_range() {
        assert!((ease_out(-1.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_out(2.0) - 1.0).abs() < f32::EPSILON);
        assert!((ease_in_out(-1.0) - 0.0).abs() < f32::EPSILON);
        assert!((ease_in_out(2.0) - 1.0).abs() < f32::EPSILON);
    }
}
