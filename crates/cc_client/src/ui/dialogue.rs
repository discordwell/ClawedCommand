use bevy::prelude::*;

use cc_core::mission::{DialogueLine, VoiceStyle};
use cc_sim::campaign::triggers::DialogueEvent;

/// Dialogue UI state — manages the typewriter effect and line queue.
#[derive(Resource)]
pub struct DialogueState {
    pub queue: Vec<DialogueLine>,
    pub current_index: usize,
    pub chars_revealed: usize,
    pub char_timer: f32,
    pub active: bool,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            current_index: 0,
            chars_revealed: 0,
            char_timer: 0.0,
            active: false,
        }
    }
}

const TYPEWRITER_SPEED: f32 = 40.0;

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

pub fn spawn_dialogue(mut commands: Commands) {
    commands
        .spawn((
            DialogueRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-300.0)),
                width: Val::Px(600.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                DialogueSpeaker,
                Text::new(""),
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
            ));
            parent.spawn((
                DialogueText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));
            parent.spawn((
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
}

pub fn update_dialogue(
    mut state: ResMut<DialogueState>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
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
}
