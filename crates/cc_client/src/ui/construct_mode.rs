use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_agent::agent_bridge::{AgentBridge, AgentRequest, AgentSource};
use cc_agent::construct_mode::{ConstructModeState, ScriptLibrary, LuaScript, ScriptTestResult};
use cc_agent::llm_client::ChatMessage;
use cc_agent::tool_tier::FactionToolStates;

/// System prompt for construct mode — instructs the LLM to generate Lua scripts.
const CONSTRUCT_MODE_SYSTEM_PROMPT: &str = r#"You are Minstral, an AI assistant in the RTS game ClawedCommand. The player is asking you to create or modify a Lua script for army automation.

Generate a Lua script using the ctx API. Available methods:

Queries:
- ctx:my_units(kind?) -> [{id, kind, x, y, idle, moving, attacking, gathering, hp, hp_max, speed, damage, range, owner}]
- ctx:enemy_units() -> same format
- ctx:my_buildings(kind?) -> [{id, kind, x, y, under_construction}]
- ctx:enemy_buildings() -> same format
- ctx:get_resources() -> {food, gpu_cores, nfts, supply, supply_cap}
- ctx:resource_deposits() -> [{id, x, y, remaining, resource_type}]

Commands:
- ctx:move_units(ids_table, x, y)
- ctx:attack(ids_table, target_id)
- ctx:attack_move(ids_table, x, y)
- ctx:stop(ids_table)
- ctx:hold(ids_table)
- ctx:gather(ids_table, deposit_id)
- ctx:build(builder_id, building_type_string, x, y)
- ctx:train(building_id, unit_type_string)

Unit kinds: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, Catnapper, FerretSapper, MechCommander
Building kinds: TheBox, CatTree, FishMarket, ServerRack, ScratchingPost, LitterBox, CatFlap, LaserPointer

Format your response with the script in a ```lua code block.
Start the script with: -- script_name: Short description
Add: -- Intents: comma, separated, voice, triggers

Keep scripts concise and focused on one behavior."#;

/// Toggle construct mode with Tab key.
pub fn construct_mode_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ConstructModeState>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        state.active = !state.active;
    }
}

/// Construct mode UI: script library + code editor + LLM chat.
pub fn construct_mode_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ConstructModeState>,
    mut library: ResMut<ScriptLibrary>,
    bridge: Res<AgentBridge>,
    tool_states: Res<FactionToolStates>,
) {
    if !state.active {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Construct Mode")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .resizable(true)
        .default_width(800.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left panel: script library
                ui.vertical(|ui| {
                    ui.set_min_width(140.0);
                    ui.heading("Scripts");
                    ui.separator();
                    let script_names: Vec<(usize, String)> = library
                        .scripts
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (i, s.name.clone()))
                        .collect();
                    for (i, name) in &script_names {
                        let selected = state
                            .current_script
                            .as_ref()
                            .is_some_and(|s| s.name == *name);
                        if ui.selectable_label(selected, name).clicked() {
                            let script = library.scripts[*i].clone();
                            state.editable_source = script.source.clone();
                            state.current_script = Some(script);
                            state.test_result = None;
                        }
                        if *i < script_names.len() - 1 {
                            ui.separator();
                        }
                    }
                    if script_names.is_empty() {
                        ui.label("No scripts yet");
                    }
                });

                ui.separator();

                // Center panel: editable code editor
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.heading("Code");
                    ui.separator();
                    if state.current_script.is_some() || !state.editable_source.is_empty() {
                        egui::ScrollArea::vertical()
                            .max_height(350.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut state.editable_source)
                                        .code_editor()
                                        .desired_width(300.0),
                                );
                            });
                        if let Some(script) = &state.current_script {
                            if !script.intents.is_empty() {
                                ui.label(format!("Intents: {}", script.intents.join(", ")));
                            }
                        }
                    } else {
                        ui.label("Select a script or ask the AI to create one");
                    }

                    // Test result display
                    if let Some(result) = &state.test_result {
                        let color = if result.success {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.colored_label(color, &result.message);
                    }
                });

                ui.separator();

                // Right panel: LLM chat
                ui.vertical(|ui| {
                    ui.set_min_width(200.0);
                    ui.heading("AI Chat");
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for msg in &state.chat_history {
                                if msg.role == "user" {
                                    ui.colored_label(
                                        egui::Color32::LIGHT_BLUE,
                                        format!("You: {}", msg.content),
                                    );
                                } else {
                                    ui.label(format!("Minstral: {}", msg.content));
                                }
                                ui.add_space(4.0);
                            }
                            if state.waiting_for_response {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Minstral is thinking...");
                                });
                            }
                        });

                    ui.separator();

                    let send_enabled = !state.waiting_for_response;
                    ui.horizontal(|ui| {
                        if !send_enabled {
                            ui.disable();
                        }
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut state.chat_input)
                                .desired_width(150.0)
                                .hint_text("Describe behavior..."),
                        );

                        let send_clicked = ui.button("Send").clicked();
                        let enter_pressed = response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        if (send_clicked || enter_pressed)
                            && !state.chat_input.is_empty()
                            && send_enabled
                        {
                            let user_msg = ChatMessage {
                                role: "user".to_string(),
                                content: state.chat_input.clone(),
                            };
                            state.chat_history.push(user_msg);

                            // Build full message list with system prompt
                            let mut messages = vec![ChatMessage {
                                role: "system".to_string(),
                                content: CONSTRUCT_MODE_SYSTEM_PROMPT.to_string(),
                            }];
                            messages.extend(state.chat_history.clone());

                            let tier = tool_states.tier_for(0);
                            let _ = bridge.request_tx.send(AgentRequest {
                                player_id: 0,
                                prompt: state.chat_input.clone(),
                                tier,
                                source: AgentSource::ConstructMode,
                                chat_history: Some(messages),
                            });

                            state.chat_input.clear();
                            state.waiting_for_response = true;
                        }
                    });
                });
            });

            ui.separator();

            // Bottom buttons
            ui.horizontal(|ui| {
                // Save Script
                if ui.button("Save Script").clicked() && !state.editable_source.is_empty() {
                    let intents =
                        cc_agent::agent_bridge::extract_intents_from_source(&state.editable_source);
                    let name = cc_agent::agent_bridge::extract_name_from_source(&state.editable_source)
                        .unwrap_or_else(|| format!("script_{}", library.scripts.len()));

                    let script = LuaScript {
                        name: name.clone(),
                        source: state.editable_source.clone(),
                        intents,
                        description: state
                            .current_script
                            .as_ref()
                            .map(|s| s.description.clone())
                            .unwrap_or_default(),
                    };

                    // Upsert by name
                    if let Some(idx) = library.scripts.iter().position(|s| s.name == name) {
                        library.scripts[idx] = script.clone();
                    } else {
                        library.scripts.push(script.clone());
                    }
                    state.current_script = Some(script);
                }

                // Test Script
                if ui.button("Test Script").clicked() && !state.editable_source.is_empty() {
                    match cc_agent::lua_runtime::execute_script(&state.editable_source, 0) {
                        Ok(commands) => {
                            state.test_result = Some(ScriptTestResult {
                                success: true,
                                message: format!(
                                    "Script OK — {} commands generated",
                                    commands.len()
                                ),
                                command_count: commands.len(),
                            });
                        }
                        Err(e) => {
                            state.test_result = Some(ScriptTestResult {
                                success: false,
                                message: format!("Error: {e}"),
                                command_count: 0,
                            });
                        }
                    }
                }

                if ui.button("Close (Tab)").clicked() {
                    state.active = false;
                }
            });
        });
}
