use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_agent::construct_mode::{ConstructModeState, ScriptLibrary};

/// Toggle construct mode with Tab key.
pub fn construct_mode_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ConstructModeState>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        state.active = !state.active;
    }
}

/// Construct mode UI: script library + code display + LLM chat.
pub fn construct_mode_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ConstructModeState>,
    library: Res<ScriptLibrary>,
) {
    if !state.active {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Construct Mode")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .resizable(true)
        .default_width(700.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel: script library
                ui.vertical(|ui| {
                    ui.heading("Scripts");
                    ui.separator();
                    for (i, script) in library.scripts.iter().enumerate() {
                        let selected = state
                            .current_script
                            .as_ref()
                            .is_some_and(|s| s.name == script.name);
                        if ui.selectable_label(selected, &script.name).clicked() {
                            state.current_script = Some(script.clone());
                        }
                        if i < library.scripts.len() - 1 {
                            ui.separator();
                        }
                    }
                    if library.scripts.is_empty() {
                        ui.label("No scripts yet");
                    }
                });

                ui.separator();

                // Center panel: code display
                ui.vertical(|ui| {
                    ui.heading("Code");
                    ui.separator();
                    if let Some(script) = &state.current_script {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut script.source.as_str())
                                        .code_editor()
                                        .desired_width(300.0),
                                );
                            });
                        ui.label(format!("Intents: {}", script.intents.join(", ")));
                    } else {
                        ui.label("Select a script to view");
                    }
                });

                ui.separator();

                // Right panel: LLM chat
                ui.vertical(|ui| {
                    ui.heading("AI Chat");
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for msg in &state.chat_history {
                                let prefix = if msg.role == "user" { "You" } else { "AI" };
                                ui.label(format!("{}: {}", prefix, msg.content));
                                ui.add_space(4.0);
                            }
                        });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut state.chat_input)
                                .desired_width(180.0)
                                .hint_text("Describe behavior..."),
                        );

                        if ui.button("Send").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            if !state.chat_input.is_empty() {
                                let msg = cc_agent::llm_client::ChatMessage {
                                    role: "user".to_string(),
                                    content: state.chat_input.clone(),
                                };
                                state.chat_history.push(msg);
                                state.chat_input.clear();
                                // LLM response is handled asynchronously via AgentBridge
                            }
                        }
                    });
                });
            });

            ui.separator();

            // Bottom buttons
            ui.horizontal(|ui| {
                if ui.button("Save Script").clicked() {
                    // Save handled via AgentBridge response
                }
                if ui.button("Test Script").clicked() {
                    // Execute current script in sandbox
                }
                if ui.button("Close (Tab)").clicked() {
                    state.active = false;
                }
            });
        });
}
