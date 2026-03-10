use bevy::prelude::*;

/// Fade-in animation component. Interpolates BackgroundColor/TextColor alpha.
#[derive(Component)]
pub struct FadeIn {
    pub timer: Timer,
    pub from_alpha: f32,
    pub to_alpha: f32,
}

impl FadeIn {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            from_alpha: 0.0,
            to_alpha: 1.0,
        }
    }
}

/// Slide-in animation component. Interpolates Node left position.
#[derive(Component)]
pub struct SlideIn {
    pub timer: Timer,
    pub from_x: f32,
    pub to_x: f32,
}

/// System: animate FadeIn components by interpolating BackgroundColor alpha.
pub fn animate_fade_in(
    time: Res<Time>,
    mut query: Query<(
        &mut FadeIn,
        Option<&mut BackgroundColor>,
        Option<&mut TextColor>,
    )>,
) {
    for (mut fade, bg, text_color) in query.iter_mut() {
        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();
        let alpha = fade.from_alpha + (fade.to_alpha - fade.from_alpha) * t;

        if let Some(mut bg) = bg {
            let c = bg.0.to_srgba();
            bg.0 = Color::srgba(c.red, c.green, c.blue, alpha);
        }
        if let Some(mut tc) = text_color {
            let c = tc.0.to_srgba();
            tc.0 = Color::srgba(c.red, c.green, c.blue, alpha);
        }
    }
}

/// System: animate SlideIn components by interpolating Node left position.
pub fn animate_slide_in(time: Res<Time>, mut query: Query<(&mut SlideIn, &mut Node)>) {
    for (mut slide, mut node) in query.iter_mut() {
        slide.timer.tick(time.delta());
        let t = slide.timer.fraction();
        let x = slide.from_x + (slide.to_x - slide.from_x) * t;
        node.left = Val::Px(x);
    }
}

/// Returns the faction accent color for a given act number.
pub fn faction_accent_color(act: u32) -> Color {
    match act {
        0 => Color::srgb(0.0, 0.8, 0.7), // Neutral teal
        1 => Color::srgb(1.0, 0.7, 0.2), // catGPT amber
        2 => Color::srgb(0.3, 0.7, 0.3), // Seekers green
        3 => Color::srgb(0.6, 0.3, 0.8), // Murder purple
        4 => Color::srgb(0.9, 0.6, 0.2), // LLAMA orange
        5 => Color::srgb(0.2, 0.7, 0.7), // Croak teal-green
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}

/// Returns the hero portrait asset key for a given act number.
pub fn faction_hero_portrait(act: u32) -> &'static str {
    match act {
        0 => "hero_kelpie",
        1 => "hero_felix_nine",
        2 => "hero_mother_granite",
        3 => "hero_rex_solstice",
        4 => "hero_king_ringtail",
        5 => "hero_the_eternal",
        _ => "hero_kelpie",
    }
}

/// Returns the display name for a given act number.
pub fn act_display_name(act: u32) -> &'static str {
    match act {
        0 => "PROLOGUE",
        1 => "ACT 1: AMONG CATS",
        2 => "ACT 2: THE MOUNTAIN'S PATIENCE",
        3 => "ACT 3: THE LIE THAT SEES",
        4 => "ACT 4: GARBAGE AND GLORY",
        5 => "ACT 5: STILL WATERS",
        _ => "UNKNOWN ACT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_colors_all_acts() {
        for act in 0..=5 {
            let color = faction_accent_color(act);
            let srgba = color.to_srgba();
            assert!(srgba.red >= 0.0 && srgba.red <= 1.0);
        }
    }

    #[test]
    fn act_names_all_acts() {
        assert_eq!(act_display_name(0), "PROLOGUE");
        assert_eq!(act_display_name(1), "ACT 1: AMONG CATS");
        assert_eq!(act_display_name(5), "ACT 5: STILL WATERS");
        assert_eq!(act_display_name(99), "UNKNOWN ACT");
    }

    #[test]
    fn hero_portraits_all_acts() {
        assert_eq!(faction_hero_portrait(0), "hero_kelpie");
        assert_eq!(faction_hero_portrait(4), "hero_king_ringtail");
    }

    #[test]
    fn fade_in_new() {
        let fade = FadeIn::new(1.0);
        assert_eq!(fade.from_alpha, 0.0);
        assert_eq!(fade.to_alpha, 1.0);
        assert!(!fade.timer.just_finished());
    }
}
