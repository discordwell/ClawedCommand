//! Prompt overlay UI — opened with `/` key, sends prompts to the LLM backend.
//!
//! Shares state with ConstructModeState so Tab and `/` views show the same
//! script/chat data.

use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use cc_agent::agent_bridge::{AgentBridge, AgentRequest, AgentSource};
use cc_agent::construct_mode::{ConstructModeState, ScriptLibrary, ScriptTestResult};
use cc_agent::events::ScriptRegistration;
use cc_agent::runner::ScriptRegistry;
use cc_agent::tool_tier::ToolTier;
use cc_sim::resources::GameState;

use crate::input::InputMode;
use super::LocalPlayer;

/// Marker for the prompt overlay root node.
#[derive(Component)]
pub struct PromptOverlayRoot;

/// Marker for the text input display.
#[derive(Component)]
pub struct PromptInputText;

/// Marker for the response/script display area.
#[derive(Component)]
pub struct PromptResponseArea;

/// Marker for the status line.
#[derive(Component)]
pub struct PromptStatusText;

/// Spawn the prompt overlay (hidden initially).
pub fn spawn_prompt_overlay(mut commands: Commands) {
    commands
        .spawn((
            PromptOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-350.0),
                    top: Val::Px(-200.0),
                    ..default()
                },
                width: Val::Px(700.0),
                height: Val::Px(400.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.08, 0.96)),
            Visibility::Hidden,
            // Render on top of other UI
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Title bar
            parent.spawn((
                Text::new("MINSTRAL PROMPT  [/ to open]"),
                TextColor(Color::srgb(0.95, 0.8, 0.3)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));

            // Input field area
            parent
                .spawn(Node {
                    min_height: Val::Px(28.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                })
                .insert(BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)))
                .with_children(|parent| {
                    parent.spawn((
                        PromptInputText,
                        Text::new("> _"),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                    ));
                });

            // Response/script display (scrollable area)
            parent
                .spawn(Node {
                    flex_grow: 1.0,
                    padding: UiRect::all(Val::Px(6.0)),
                    overflow: Overflow::clip_y(),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                })
                .insert(BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.9)))
                .with_children(|parent| {
                    parent.spawn((
                        PromptResponseArea,
                        Text::new("Type a request and press Enter.\nExample: \"make my ranged units kite enemies\""),
                        TextColor(Color::srgb(0.6, 0.7, 0.6)),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });

            // Status bar
            parent.spawn((
                PromptStatusText,
                Text::new("Enter=submit | Esc=cancel | T=test | S=save"),
                TextColor(Color::srgb(0.5, 0.5, 0.6)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
            ));
        });
}

/// Show/hide overlay based on InputMode, and toggle GameState pause.
pub fn prompt_overlay_visibility(
    input_mode: Res<InputMode>,
    mut game_state: ResMut<GameState>,
    mut root_vis: Query<&mut Visibility, With<PromptOverlayRoot>>,
) {
    let show = *input_mode == InputMode::Prompt;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Pause/unpause game based on prompt overlay state
    if show && *game_state == GameState::Playing {
        *game_state = GameState::Paused;
    } else if !show && *game_state == GameState::Paused {
        *game_state = GameState::Playing;
    }
}

/// Handle text input, submission, and special keys for the prompt overlay.
pub fn prompt_text_input(
    mut input_mode: ResMut<InputMode>,
    mut construct_state: ResMut<ConstructModeState>,
    mut keyboard_events: MessageReader<KeyboardInput>,
    bridge: Res<AgentBridge>,
    local_player: Res<LocalPlayer>,
    mut registry: ResMut<ScriptRegistry>,
    mut library: ResMut<ScriptLibrary>,
) {
    if *input_mode != InputMode::Prompt {
        // Drain events so they don't accumulate
        for _ in keyboard_events.read() {}
        return;
    }

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match &event.logical_key {
            // Escape — close overlay
            Key::Escape => {
                *input_mode = InputMode::Normal;
                return;
            }

            // Enter — submit prompt
            Key::Enter => {
                let prompt = construct_state.chat_input.trim().to_string();
                if prompt.is_empty() || construct_state.waiting_for_response {
                    continue;
                }

                // Add user message to chat history
                construct_state.chat_history.push(cc_agent::llm_client::ChatMessage {
                    role: "user".to_string(),
                    content: prompt.clone(),
                });
                construct_state.waiting_for_response = true;

                // Send request through AgentBridge
                let request = AgentRequest {
                    player_id: local_player.0,
                    prompt,
                    tier: ToolTier::Basic,
                    source: AgentSource::ConstructMode,
                    chat_history: Some(construct_state.chat_history.clone()),
                    snapshot: None, // Snapshot will be built by the runner if needed
                };

                if let Err(e) = bridge.request_tx.try_send(request) {
                    bevy::log::warn!("Failed to send prompt request: {e}");
                    construct_state.waiting_for_response = false;
                }

                construct_state.chat_input.clear();
            }

            // Backspace
            Key::Backspace => {
                construct_state.chat_input.pop();
            }

            // T — test script (only when not typing and a script exists)
            Key::Character(c) if c.as_str() == "t" && construct_state.chat_input.is_empty() => {
                if !construct_state.editable_source.is_empty() {
                    #[cfg(not(target_arch = "wasm32"))]
                    match cc_agent::lua_runtime::execute_script(
                        &construct_state.editable_source,
                        local_player.0,
                    ) {
                        Ok(commands) => {
                            construct_state.test_result = Some(ScriptTestResult {
                                success: true,
                                message: format!("OK — {} commands", commands.len()),
                                command_count: commands.len(),
                            });
                        }
                        Err(e) => {
                            construct_state.test_result = Some(ScriptTestResult {
                                success: false,
                                message: format!("{e}"),
                                command_count: 0,
                            });
                        }
                    }
                } else {
                    // No script to test — treat as regular text input
                    construct_state.chat_input.push('t');
                }
            }

            // S — save script (only when not typing and a script exists)
            Key::Character(c) if c.as_str() == "s" && construct_state.chat_input.is_empty() => {
                if let Some(script) = &construct_state.current_script {
                    // Save to disk
                    #[cfg(not(target_arch = "wasm32"))]
                    match cc_agent::script_persistence::save_script(script) {
                        Ok(path) => {
                            bevy::log::info!("Script saved to {}", path.display());
                        }
                        Err(e) => {
                            bevy::log::warn!("Failed to save script: {e}");
                        }
                    }

                    // Register in ScriptRegistry for live execution
                    let mut reg = ScriptRegistration::new(
                        script.name.clone(),
                        script.source.clone(),
                        vec!["on_tick".to_string()],
                        local_player.0,
                    );
                    reg.tick_interval = 3;

                    // Remove old version if exists
                    registry.unregister(&script.name);
                    registry.register(reg);

                    // Add to library if not already there
                    if !library.scripts.iter().any(|s| s.name == script.name) {
                        library.scripts.push(script.clone());
                    }

                    construct_state.test_result = Some(ScriptTestResult {
                        success: true,
                        message: format!("Saved & registered '{}'", script.name),
                        command_count: 0,
                    });
                } else {
                    // No script to save — treat as regular text input
                    construct_state.chat_input.push('s');
                }
            }

            // Regular character input
            Key::Character(c) => {
                construct_state.chat_input.push_str(c.as_str());
            }

            Key::Space => {
                construct_state.chat_input.push(' ');
            }

            _ => {}
        }
    }
}

/// Update the overlay display text from ConstructModeState.
pub fn update_prompt_display(
    input_mode: Res<InputMode>,
    construct_state: Res<ConstructModeState>,
    mut input_q: Query<
        &mut Text,
        (
            With<PromptInputText>,
            Without<PromptResponseArea>,
            Without<PromptStatusText>,
        ),
    >,
    mut response_q: Query<
        &mut Text,
        (
            With<PromptResponseArea>,
            Without<PromptInputText>,
            Without<PromptStatusText>,
        ),
    >,
    mut status_q: Query<
        &mut Text,
        (
            With<PromptStatusText>,
            Without<PromptInputText>,
            Without<PromptResponseArea>,
        ),
    >,
) {
    if *input_mode != InputMode::Prompt {
        return;
    }

    // Update input text
    if let Ok(mut text) = input_q.single_mut() {
        if construct_state.chat_input.is_empty() {
            text.0 = "> _".to_string();
        } else {
            text.0 = format!("> {}_", construct_state.chat_input);
        }
    }

    // Update response area
    if let Ok(mut text) = response_q.single_mut() {
        if construct_state.waiting_for_response {
            text.0 = "Minstral is thinking...".to_string();
        } else if !construct_state.editable_source.is_empty() {
            let mut display = construct_state.editable_source.clone();
            if let Some(result) = &construct_state.test_result {
                let status = if result.success {
                    format!("\n\n-- {}", result.message)
                } else {
                    format!("\n\n-- FAILED: {}", result.message)
                };
                display.push_str(&status);
            }
            text.0 = display;
        } else if !construct_state.chat_history.is_empty() {
            // Show last assistant message
            let last = construct_state
                .chat_history
                .iter()
                .rev()
                .find(|m| m.role == "assistant");
            if let Some(msg) = last {
                text.0 = msg.content.chars().take(800).collect();
            }
        } else {
            text.0 = "Type a request and press Enter.\nExample: \"make my ranged units kite enemies\"".to_string();
        }
    }

    // Update status line
    if let Ok(mut text) = status_q.single_mut() {
        if construct_state.waiting_for_response {
            text.0 = "Waiting for Minstral...".to_string();
        } else if construct_state.current_script.is_some() {
            text.0 = "T=test | S=save & activate | Enter=new prompt | Esc=close".to_string();
        } else {
            text.0 = "Enter=submit | Esc=cancel".to_string();
        }
    }
}
