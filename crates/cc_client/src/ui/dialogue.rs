use std::collections::HashMap;

use bevy::prelude::*;

use cc_core::mission::{DialogueLine, VoiceStyle};
use cc_sim::campaign::triggers::DialogueEvent;

/// Lazy-loaded portrait image handles, keyed by portrait asset key.
#[derive(Resource, Default)]
pub struct PortraitHandles {
    pub handles: HashMap<String, Handle<Image>>,
}

/// Dialogue UI state — manages the typewriter effect and line queue.
#[derive(Resource)]
pub struct DialogueState {
    pub queue: Vec<DialogueLine>,
    pub current_index: usize,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub active: bool,
    /// First unique speaker detected in queue → left portrait.
    pub left_speaker: Option<String>,
    /// Second unique speaker detected in queue → right portrait.
    pub right_speaker: Option<String>,
    /// Portrait key for left speaker.
    pub left_portrait: Option<String>,
    /// Portrait key for right speaker.
    pub right_portrait: Option<String>,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            current_index: 0,
            chars_revealed: 0,
            char_timer: 0.0,
            active: false,
            left_speaker: None,
            right_speaker: None,
            left_portrait: None,
            right_portrait: None,
        }
    }
}

const TYPEWRITER_SPEED: f32 = 40.0;

/// Scan dialogue lines to assign speakers to left/right positions.
/// First unique speaker → left, second unique speaker → right.
pub fn detect_speakers(lines: &[DialogueLine]) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut left_speaker: Option<String> = None;
    let mut right_speaker: Option<String> = None;
    let mut left_portrait: Option<String> = None;
    let mut right_portrait: Option<String> = None;

    for line in lines {
        if left_speaker.is_none() {
            left_speaker = Some(line.speaker.clone());
            if !line.portrait.is_empty() {
                left_portrait = Some(line.portrait.clone());
            }
        } else if left_speaker.as_deref() != Some(&line.speaker) && right_speaker.is_none() {
            right_speaker = Some(line.speaker.clone());
            if !line.portrait.is_empty() {
                right_portrait = Some(line.portrait.clone());
            }
            break;
        }
    }

    (left_speaker, right_speaker, left_portrait, right_portrait)
}

/// Returns true if any line in the slice has a non-empty portrait key.
pub fn has_portraits(lines: &[DialogueLine]) -> bool {
    lines.iter().any(|l| !l.portrait.is_empty())
}

/// Read incoming DialogueEvents and queue them for display.
pub fn dialogue_event_reader(
    mut events: MessageReader<DialogueEvent>,
    mut state: ResMut<DialogueState>,
) {
    for event in events.read() {
        if state.active {
            state.queue.extend(event.lines.iter().cloned());
        } else {
            state.queue = event.lines.clone();
            state.current_index = 0;
            state.chars_revealed = 0;
            state.char_timer = 0.0;
            state.active = true;
        }
        // Re-detect speakers whenever new lines arrive
        let (ls, rs, lp, rp) = detect_speakers(&state.queue);
        state.left_speaker = ls;
        state.right_speaker = rs;
        state.left_portrait = lp;
        state.right_portrait = rp;
    }
}

/// Marker for the dialogue box root.
#[derive(Component)]
pub struct DialogueRoot;

/// Marker for the dialogue speaker name.
#[derive(Component)]
pub struct DialogueSpeaker;

/// Marker for the dialogue text content.
#[derive(Component)]
pub struct DialogueText;

/// Marker for the advance prompt text.
#[derive(Component)]
pub struct DialoguePrompt;

/// Marker for the left portrait image.
#[derive(Component)]
pub struct LeftPortrait;

/// Marker for the right portrait image.
#[derive(Component)]
pub struct RightPortrait;

pub fn spawn_dialogue(mut commands: Commands) {
    commands
        .spawn((
            DialogueRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-400.0)),
                width: Val::Px(800.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                align_items: AlignItems::FlexStart,
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Left portrait (96x96)
            parent.spawn((
                LeftPortrait,
                ImageNode::default(),
                Node {
                    width: Val::Px(96.0),
                    height: Val::Px(96.0),
                    flex_shrink: 0.0,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.5)),
            ));

            // Center column: speaker + text + prompt
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        DialogueSpeaker,
                        Text::new(""),
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                    ));
                    col.spawn((
                        DialogueText,
                        Text::new(""),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                    ));
                    col.spawn((
                        DialoguePrompt,
                        Text::new(""),
                        TextColor(Color::srgb(0.6, 0.6, 0.6)),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        Node {
                            align_self: AlignSelf::FlexEnd,
                            ..default()
                        },
                    ));
                });

            // Right portrait (96x96)
            parent.spawn((
                RightPortrait,
                ImageNode::default(),
                Node {
                    width: Val::Px(96.0),
                    height: Val::Px(96.0),
                    flex_shrink: 0.0,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.5)),
            ));
        });
}

pub fn update_dialogue(
    mut state: ResMut<DialogueState>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut portraits: ResMut<PortraitHandles>,
    mut root_vis: Query<
        &mut Visibility,
        (With<DialogueRoot>, Without<DialogueSpeaker>, Without<DialogueText>, Without<DialoguePrompt>),
    >,
    mut speaker_q: Query<
        (&mut Text, &mut TextColor),
        (With<DialogueSpeaker>, Without<DialogueRoot>, Without<DialogueText>, Without<DialoguePrompt>),
    >,
    mut text_q: Query<
        &mut Text,
        (With<DialogueText>, Without<DialogueRoot>, Without<DialogueSpeaker>, Without<DialoguePrompt>),
    >,
    mut prompt_q: Query<
        &mut Text,
        (With<DialoguePrompt>, Without<DialogueRoot>, Without<DialogueSpeaker>, Without<DialogueText>),
    >,
    mut left_q: Query<
        (&mut ImageNode, &mut Node, &mut BackgroundColor),
        (With<LeftPortrait>, Without<RightPortrait>),
    >,
    mut right_q: Query<
        (&mut ImageNode, &mut Node, &mut BackgroundColor),
        (With<RightPortrait>, Without<LeftPortrait>),
    >,
) {
    if !state.active || state.queue.is_empty() {
        for mut vis in root_vis.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    for mut vis in root_vis.iter_mut() {
        *vis = Visibility::Inherited;
    }

    let dt = time.delta_secs();

    let Some(line) = state.queue.get(state.current_index) else {
        state.active = false;
        return;
    };
    let line = line.clone();

    let total_chars = line.text.chars().count();
    let fully_revealed = state.chars_revealed >= total_chars;

    // Handle Space input
    if keys.just_pressed(KeyCode::Space) {
        if fully_revealed {
            let next = state.current_index + 1;
            if next >= state.queue.len() {
                state.active = false;
                return;
            }
            state.current_index = next;
            state.chars_revealed = 0;
            state.char_timer = 0.0;
        } else {
            state.chars_revealed = total_chars;
        }
    }

    // Typewriter timer
    if !fully_revealed {
        state.char_timer += dt;
        let chars_to_add = (state.char_timer * TYPEWRITER_SPEED) as usize;
        if chars_to_add > 0 {
            state.chars_revealed = (state.chars_revealed + chars_to_add).min(total_chars);
            state.char_timer = 0.0;
        }
    }

    // Re-read current line
    let Some(current_line) = state.queue.get(state.current_index) else {
        state.active = false;
        return;
    };

    let visible_text: String = current_line
        .text
        .chars()
        .take(state.chars_revealed)
        .collect();

    let speaker_color = match current_line.voice_style {
        VoiceStyle::Normal => Color::srgb(1.0, 1.0, 1.0),
        VoiceStyle::AiVoice => Color::srgb(0.4, 1.0, 0.4),
        VoiceStyle::Whisper => Color::srgb(0.7, 0.7, 0.8),
        VoiceStyle::Shout => Color::srgb(1.0, 0.4, 0.4),
    };

    if let Ok((mut text, mut color)) = speaker_q.single_mut() {
        text.0 = current_line.speaker.clone();
        color.0 = speaker_color;
    }

    if let Ok(mut text) = text_q.single_mut() {
        text.0 = match current_line.voice_style {
            VoiceStyle::Shout => visible_text.to_uppercase(),
            _ => visible_text,
        };
    }

    let fully_done = state.chars_revealed >= current_line.text.chars().count();
    if let Ok(mut prompt) = prompt_q.single_mut() {
        if fully_done {
            let is_last = state.current_index + 1 >= state.queue.len();
            prompt.0 = if is_last {
                "[Space] Close".to_string()
            } else {
                "[Space] Continue".to_string()
            };
        } else {
            prompt.0 = String::new();
        }
    }

    // --- Portrait rendering ---
    let show_portraits = state.left_portrait.is_some() || state.right_portrait.is_some();
    let is_left_speaker = state.left_speaker.as_deref() == Some(&current_line.speaker);

    // Helper: load or retrieve a portrait handle
    let load_portrait = |key: &str, handles: &mut PortraitHandles, server: &AssetServer| -> Handle<Image> {
        handles.handles.entry(key.to_string()).or_insert_with(|| {
            server.load(format!("portraits/{key}.png"))
        }).clone()
    };

    // Left portrait
    if let Ok((mut img, mut node, mut bg)) = left_q.single_mut() {
        if show_portraits {
            if let Some(ref key) = state.left_portrait {
                node.display = Display::Flex;
                img.image = load_portrait(key, &mut portraits, &asset_server);
                let alpha = if is_left_speaker { 1.0 } else { 0.4 };
                img.color = Color::srgba(1.0, 1.0, 1.0, alpha);
                bg.0 = Color::srgba(0.2, 0.2, 0.3, 0.5 * alpha);
            } else {
                node.display = Display::None;
            }
        } else {
            node.display = Display::None;
        }
    }

    // Right portrait
    if let Ok((mut img, mut node, mut bg)) = right_q.single_mut() {
        if show_portraits {
            if let Some(ref key) = state.right_portrait {
                node.display = Display::Flex;
                img.image = load_portrait(key, &mut portraits, &asset_server);
                let alpha = if !is_left_speaker { 1.0 } else { 0.4 };
                img.color = Color::srgba(1.0, 1.0, 1.0, alpha);
                bg.0 = Color::srgba(0.2, 0.2, 0.3, 0.5 * alpha);
            } else {
                node.display = Display::None;
            }
        } else {
            node.display = Display::None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_line(speaker: &str, portrait: &str) -> DialogueLine {
        DialogueLine {
            speaker: speaker.into(),
            text: "Test".into(),
            voice_style: VoiceStyle::Normal,
            portrait: portrait.into(),
        }
    }

    #[test]
    fn detect_speakers_two_speakers() {
        let lines = vec![
            make_line("Alice", "portrait_alice"),
            make_line("Bob", "portrait_bob"),
            make_line("Alice", "portrait_alice"),
        ];
        let (ls, rs, lp, rp) = detect_speakers(&lines);
        assert_eq!(ls.as_deref(), Some("Alice"));
        assert_eq!(rs.as_deref(), Some("Bob"));
        assert_eq!(lp.as_deref(), Some("portrait_alice"));
        assert_eq!(rp.as_deref(), Some("portrait_bob"));
    }

    #[test]
    fn detect_speakers_one_speaker() {
        let lines = vec![
            make_line("Alice", "portrait_alice"),
            make_line("Alice", "portrait_alice"),
        ];
        let (ls, rs, _, _) = detect_speakers(&lines);
        assert_eq!(ls.as_deref(), Some("Alice"));
        assert!(rs.is_none());
    }

    #[test]
    fn has_portraits_detects_presence() {
        let with = vec![make_line("A", "portrait_a")];
        let without = vec![make_line("A", "")];
        assert!(has_portraits(&with));
        assert!(!has_portraits(&without));
    }

    #[test]
    fn has_portraits_empty_queue() {
        assert!(!has_portraits(&[]));
    }
}
