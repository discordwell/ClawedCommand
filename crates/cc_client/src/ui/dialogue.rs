use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_core::mission::{DialogueLine, VoiceStyle};
use cc_sim::campaign::triggers::DialogueEvent;

/// Dialogue UI state — manages the typewriter effect and line queue.
#[derive(Resource)]
pub struct DialogueState {
    /// Queue of dialogue lines waiting to be displayed.
    pub queue: Vec<DialogueLine>,
    /// Index of the currently displayed line.
    pub current_index: usize,
    /// How many characters of the current line are visible (typewriter).
    pub chars_revealed: usize,
    /// Timer for typewriter effect.
    pub char_timer: f32,
    /// If true, dialogue box is visible.
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

/// Characters revealed per second for typewriter effect.
const TYPEWRITER_SPEED: f32 = 40.0;

/// Read incoming DialogueEvents and queue them for display.
pub fn dialogue_event_reader(
    mut events: MessageReader<DialogueEvent>,
    mut state: ResMut<DialogueState>,
) {
    for event in events.read() {
        // If already showing dialogue, append to queue
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

/// Render the dialogue box at the bottom of the screen.
pub fn dialogue_system(
    mut contexts: EguiContexts,
    mut state: ResMut<DialogueState>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !state.active || state.queue.is_empty() {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let dt = time.delta_secs();

    // Get current line
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
            // Advance to next line
            let next = state.current_index + 1;
            if next >= state.queue.len() {
                state.active = false;
                return;
            }
            state.current_index = next;
            state.chars_revealed = 0;
            state.char_timer = 0.0;
        } else {
            // Skip to end of current line
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

    // Re-read current line (may have advanced)
    let Some(current_line) = state.queue.get(state.current_index) else {
        state.active = false;
        return;
    };

    let visible_text: String = current_line
        .text
        .chars()
        .take(state.chars_revealed)
        .collect();

    // Speaker name color based on voice style
    let speaker_color = match current_line.voice_style {
        VoiceStyle::Normal => egui::Color32::WHITE,
        VoiceStyle::AiVoice => egui::Color32::from_rgb(100, 255, 100),
        VoiceStyle::Whisper => egui::Color32::from_rgb(180, 180, 200),
        VoiceStyle::Shout => egui::Color32::from_rgb(255, 100, 100),
    };

    // Dialogue box at bottom of screen
    let screen_rect = ctx.screen_rect();
    let box_width = 600.0_f32.min(screen_rect.width() - 40.0);
    let box_x = (screen_rect.width() - box_width) / 2.0;
    let box_y = screen_rect.height() - 160.0;

    egui::Area::new(egui::Id::new("dialogue_box"))
        .fixed_pos(egui::pos2(box_x, box_y))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200))
                .inner_margin(egui::Margin::same(16))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_max_width(box_width);

                    // Portrait placeholder + speaker name
                    ui.horizontal(|ui| {
                        // Faction-colored square as portrait placeholder
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(48.0, 48.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            4.0,
                            speaker_color.linear_multiply(0.5),
                        );

                        ui.vertical(|ui| {
                            ui.colored_label(speaker_color, &current_line.speaker);
                            // Text styling based on voice
                            match current_line.voice_style {
                                VoiceStyle::Whisper => {
                                    ui.label(
                                        egui::RichText::new(&visible_text)
                                            .italics()
                                            .size(12.0)
                                            .color(egui::Color32::LIGHT_GRAY),
                                    );
                                }
                                VoiceStyle::Shout => {
                                    ui.label(
                                        egui::RichText::new(visible_text.to_uppercase())
                                            .strong()
                                            .size(16.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                }
                                _ => {
                                    ui.label(
                                        egui::RichText::new(&visible_text)
                                            .size(14.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                }
                            }
                        });
                    });

                    // Advance prompt
                    let fully_done = state.chars_revealed >= current_line.text.chars().count();
                    if fully_done {
                        let is_last = state.current_index + 1 >= state.queue.len();
                        let prompt = if is_last {
                            "[Space] Close"
                        } else {
                            "[Space] Continue"
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label(
                                egui::RichText::new(prompt)
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                            );
                        });
                    }
                });
        });
}
