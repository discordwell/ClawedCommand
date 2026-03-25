//! Prompt overlay UI — opened with `/` key, sends prompts to the LLM backend.
//!
//! Features:
//! - Collapsible script manager sidebar (Tab toggles)
//! - Enable/disable scripts, manual trigger, undo toast
//! - Shares state with ConstructModeState so Tab and `/` views show the same
//!   script/chat data.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use cc_agent::agent_bridge::{AgentBridge, AgentRequest, AgentSource, UndoScriptActivation};
use cc_agent::construct_mode::{ConstructModeState, ScriptLibrary, ScriptTestResult};
use cc_agent::events::ActivationMode;
use cc_agent::script_registry::ScriptRegistry;
use cc_agent::tool_tier::ToolTier;
use cc_sim::resources::GameState;

use super::LocalPlayer;
use crate::input::InputMode;

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

/// Marker for the script sidebar container.
#[derive(Component)]
pub struct ScriptSidebarRoot;

/// Marker for the script sidebar list text.
#[derive(Component)]
pub struct ScriptSidebarList;

/// Marker for the undo toast node.
#[derive(Component)]
pub struct UndoToastNode;

/// Tracks whether the script manager sidebar is expanded.
#[derive(Resource, Default)]
pub struct ScriptManagerExpanded(pub bool);

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
                    left: Val::Px(-450.0), // wider to accommodate sidebar
                    top: Val::Px(-250.0),
                    ..default()
                },
                width: Val::Px(900.0),
                height: Val::Px(500.0),
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
                Text::new("LE CHAT PROMPT"),
                TextColor(Color::srgb(0.95, 0.8, 0.3)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));

            // Main content area: sidebar + prompt area
            parent
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Script sidebar (hidden by default)
                    parent
                        .spawn((
                            ScriptSidebarRoot,
                            Node {
                                width: Val::Px(200.0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                overflow: Overflow::clip_y(),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)),
                            Visibility::Hidden,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("SCRIPTS"),
                                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                                TextFont {
                                    font_size: 13.0,
                                    ..default()
                                },
                                Node {
                                    margin: UiRect::bottom(Val::Px(6.0)),
                                    ..default()
                                },
                            ));
                            parent.spawn((
                                ScriptSidebarList,
                                Text::new("No scripts"),
                                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                            ));
                        });

                    // Right side: input + response
                    parent
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|parent| {
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
                        });
                });

            // Status bar
            parent.spawn((
                PromptStatusText,
                Text::new("Enter=submit | Tab=scripts | Esc=close"),
                TextColor(Color::srgb(0.5, 0.5, 0.6)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
            ));
        });

    // Undo toast — floating at bottom-center, always on top
    commands.spawn((
        UndoToastNode,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(60.0),
            left: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-180.0),
                ..default()
            },
            width: Val::Px(360.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.4, 0.15, 0.95)),
        Visibility::Hidden,
        ZIndex(200),
        Text::new(""),
        TextColor(Color::srgb(0.9, 0.95, 0.9)),
        TextFont {
            font_size: 13.0,
            ..default()
        },
    ));
}

/// Show/hide overlay based on InputMode, and toggle GameState pause.
pub fn prompt_overlay_visibility(
    input_mode: Res<InputMode>,
    mut game_state: ResMut<GameState>,
    mut root_vis: Query<&mut Visibility, With<PromptOverlayRoot>>,
    sidebar_expanded: Res<ScriptManagerExpanded>,
    mut sidebar_vis: Query<&mut Visibility, (With<ScriptSidebarRoot>, Without<PromptOverlayRoot>)>,
) {
    let show = *input_mode == InputMode::Prompt;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // Show/hide sidebar based on expanded state
    for mut vis in sidebar_vis.iter_mut() {
        *vis = if show && sidebar_expanded.0 {
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
    mut sidebar_expanded: ResMut<ScriptManagerExpanded>,
    mut undo: ResMut<UndoScriptActivation>,
    #[cfg(not(target_arch = "wasm32"))] mut manual_triggers: ResMut<
        cc_agent::runner::ManualScriptTriggers,
    >,
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

            // Tab — toggle script sidebar
            Key::Tab => {
                sidebar_expanded.0 = !sidebar_expanded.0;
            }

            // Z — undo last script activation (while toast is visible)
            Key::Character(c)
                if c.as_str() == "z"
                    && construct_state.chat_input.is_empty()
                    && undo.pending.is_some() =>
            {
                if let Some((ref name, _)) = undo.pending {
                    registry.set_enabled(name, false);
                    bevy::log::info!("Undid activation of script '{}'", name);
                }
                undo.pending = None;
            }

            // Number keys 1-9 — toggle enable/disable when sidebar visible and input empty
            Key::Character(c)
                if sidebar_expanded.0
                    && construct_state.chat_input.is_empty()
                    && c.len() == 1
                    && c.as_bytes()[0].is_ascii_digit()
                    && c.as_str() != "0" =>
            {
                let idx = (c.as_bytes()[0] - b'1') as usize;
                let player_scripts: Vec<String> = registry
                    .scripts_for_player(local_player.0)
                    .iter()
                    .map(|s| s.name.clone())
                    .collect();
                if let Some(name) = player_scripts.get(idx) {
                    if let Some(script) = registry.find(name) {
                        if script.activation_mode == ActivationMode::Manual {
                            // Manual scripts: fire once
                            #[cfg(not(target_arch = "wasm32"))]
                            manual_triggers.triggered.push(name.clone());
                        } else {
                            // Auto scripts: toggle enabled
                            let new_state = !script.enabled;
                            registry.set_enabled(name, new_state);
                        }
                    }
                }
            }

            // Enter — submit prompt
            Key::Enter => {
                let prompt = construct_state.chat_input.trim().to_string();
                if prompt.is_empty() || construct_state.waiting_for_response {
                    continue;
                }

                // Clear previous script so streaming tokens fill a fresh buffer
                construct_state.editable_source.clear();
                construct_state.current_script = None;
                construct_state.test_result = None;

                // Add user message to chat history
                construct_state
                    .chat_history
                    .push(cc_agent::llm_client::ChatMessage {
                        role: "user".to_string(),
                        content: prompt.clone(),
                    });
                construct_state.waiting_for_response = true;

                // Send request through AgentBridge
                let request = AgentRequest {
                    player_id: local_player.0,
                    prompt,
                    tier: ToolTier::Basic,
                    source: AgentSource::Prompt,
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

            // S — save script to disk & library (only when not typing and a script exists)
            Key::Character(c) if c.as_str() == "s" && construct_state.chat_input.is_empty() => {
                if let Some(script) = &construct_state.current_script {
                    // Save to disk (native) or localStorage (WASM)
                    #[cfg(not(target_arch = "wasm32"))]
                    match cc_agent::script_persistence::save_script(script) {
                        Ok(path) => {
                            bevy::log::info!("Script saved to {}", path.display());
                        }
                        Err(e) => {
                            bevy::log::warn!("Failed to save script: {e}");
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    if let Err(e) = cc_agent::wasm_persistence::save_script(script) {
                        bevy::log::warn!("Failed to save script to localStorage: {e}");
                    }

                    // Ensure registered (idempotent — may already be auto-registered)
                    registry.register_from_source(&script.source, &script.name, local_player.0);

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
    registry: Res<ScriptRegistry>,
    local_player: Res<LocalPlayer>,
    sidebar_expanded: Res<ScriptManagerExpanded>,
    mut input_q: Query<
        &mut Text,
        (
            With<PromptInputText>,
            Without<PromptResponseArea>,
            Without<PromptStatusText>,
            Without<ScriptSidebarList>,
        ),
    >,
    mut response_q: Query<
        &mut Text,
        (
            With<PromptResponseArea>,
            Without<PromptInputText>,
            Without<PromptStatusText>,
            Without<ScriptSidebarList>,
        ),
    >,
    mut status_q: Query<
        &mut Text,
        (
            With<PromptStatusText>,
            Without<PromptInputText>,
            Without<PromptResponseArea>,
            Without<ScriptSidebarList>,
        ),
    >,
    mut sidebar_q: Query<
        &mut Text,
        (
            With<ScriptSidebarList>,
            Without<PromptInputText>,
            Without<PromptResponseArea>,
            Without<PromptStatusText>,
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
        if construct_state.waiting_for_response && construct_state.editable_source.is_empty() {
            text.0 = "Le Chat is thinking...".to_string();
        } else if construct_state.waiting_for_response
            && !construct_state.editable_source.is_empty()
        {
            // Streaming in progress — show tokens as they arrive
            text.0 = format!("{}|", construct_state.editable_source);
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
            text.0 =
                "Type a request and press Enter.\nExample: \"make my ranged units kite enemies\""
                    .to_string();
        }
    }

    // Update status line
    if let Ok(mut text) = status_q.single_mut() {
        if construct_state.waiting_for_response {
            text.0 = "Waiting for Le Chat...".to_string();
        } else if construct_state.current_script.is_some() {
            text.0 =
                "Enter=submit | Tab=scripts | T=test | S=save | Z=undo | Esc=close".to_string();
        } else if sidebar_expanded.0 {
            text.0 = "1-9=toggle | Tab=hide scripts | Enter=submit | Esc=close".to_string();
        } else {
            text.0 = "Enter=submit | Tab=scripts | Esc=close".to_string();
        }
    }

    // Update script sidebar list
    if sidebar_expanded.0 {
        if let Ok(mut text) = sidebar_q.single_mut() {
            let player_scripts = registry.scripts_for_player(local_player.0);
            if player_scripts.is_empty() {
                text.0 = "No scripts registered".to_string();
            } else {
                let lines: Vec<String> = player_scripts
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let indicator = match s.activation_mode {
                            ActivationMode::Manual => "[M]".to_string(),
                            _ => {
                                if s.enabled {
                                    "[*]".to_string()
                                } else {
                                    "[ ]".to_string()
                                }
                            }
                        };
                        format!("{} {} {}", i + 1, indicator, s.name)
                    })
                    .collect();
                text.0 = lines.join("\n");
            }
        }
    }
}

/// System: update the undo toast countdown and handle Z key dismissal.
pub fn update_undo_toast(
    time: Res<Time>,
    mut undo: ResMut<UndoScriptActivation>,
    mut toast_q: Query<(&mut Text, &mut Visibility), With<UndoToastNode>>,
) {
    if let Ok((mut text, mut vis)) = toast_q.single_mut() {
        if let Some((ref name, ref mut remaining)) = undo.pending {
            *remaining -= time.delta_secs();
            if *remaining <= 0.0 {
                // Timeout — script stays active
                undo.pending = None;
                *vis = Visibility::Hidden;
            } else {
                text.0 = format!(
                    "\"{}\" activated — press Z to undo ({:.0}s)",
                    name, remaining
                );
                *vis = Visibility::Inherited;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
}
