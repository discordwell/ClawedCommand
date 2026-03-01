use bevy::prelude::*;

use cc_agent::agent_bridge::{AgentBridge, AgentRequest, AgentSource};
use cc_agent::construct_mode::{ConstructModeState, LuaScript, ScriptLibrary, ScriptTestResult};
use cc_agent::llm_client::ChatMessage;
use cc_agent::tool_tier::FactionToolStates;

/// System prompt for construct mode.
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

/// Marker for construct mode root.
#[derive(Component)]
pub struct ConstructModeRoot;

/// Marker for the script library list text.
#[derive(Component)]
pub struct ConstructScriptList;

/// Marker for the code display text.
#[derive(Component)]
pub struct ConstructCodeDisplay;

/// Marker for the chat log text.
#[derive(Component)]
pub struct ConstructChatLog;

pub fn spawn_construct_mode(mut commands: Commands) {
    commands
        .spawn((
            ConstructModeRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-400.0),
                    top: Val::Px(-250.0),
                    ..default()
                },
                width: Val::Px(800.0),
                height: Val::Px(500.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Left panel: script library
            parent
                .spawn(Node {
                    width: Val::Px(150.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("SCRIPTS"),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        ConstructScriptList,
                        Text::new("No scripts"),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });

            // Center panel: code display
            parent
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("CODE"),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        ConstructCodeDisplay,
                        Text::new("Select a script or ask Minstral to create one"),
                        TextColor(Color::srgb(0.6, 0.8, 0.6)),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });

            // Right panel: chat log
            parent
                .spawn(Node {
                    width: Val::Px(200.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("AI CHAT  [F5-F7: quick cmd]"),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        ConstructChatLog,
                        Text::new("Type requests in future versions.\nFor now, use voice commands or F5-F7 hotkeys."),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });
        });
}

pub fn update_construct_mode(
    state: Res<ConstructModeState>,
    library: Res<ScriptLibrary>,
    mut root_vis: Query<
        &mut Visibility,
        (
            With<ConstructModeRoot>,
            Without<ConstructScriptList>,
            Without<ConstructCodeDisplay>,
            Without<ConstructChatLog>,
        ),
    >,
    mut list_q: Query<
        &mut Text,
        (
            With<ConstructScriptList>,
            Without<ConstructModeRoot>,
            Without<ConstructCodeDisplay>,
            Without<ConstructChatLog>,
        ),
    >,
    mut code_q: Query<
        &mut Text,
        (
            With<ConstructCodeDisplay>,
            Without<ConstructModeRoot>,
            Without<ConstructScriptList>,
            Without<ConstructChatLog>,
        ),
    >,
    mut chat_q: Query<
        &mut Text,
        (
            With<ConstructChatLog>,
            Without<ConstructModeRoot>,
            Without<ConstructScriptList>,
            Without<ConstructCodeDisplay>,
        ),
    >,
) {
    let show = state.active;
    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if !show {
        return;
    }

    // Update script list
    if let Ok(mut text) = list_q.single_mut() {
        if library.scripts.is_empty() {
            text.0 = "No scripts".to_string();
        } else {
            let names: Vec<String> = library
                .scripts
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let selected = state
                        .current_script
                        .as_ref()
                        .is_some_and(|c| c.name == s.name);
                    let marker = if selected { "> " } else { "  " };
                    format!("{}[{}] {}", marker, i + 1, s.name)
                })
                .collect();
            text.0 = names.join("\n");
        }
    }

    // Update code display
    if let Ok(mut text) = code_q.single_mut() {
        if !state.editable_source.is_empty() {
            let mut display = state.editable_source.clone();
            if let Some(result) = &state.test_result {
                let status = if result.success {
                    format!("\n\n-- TEST OK: {} commands", result.command_count)
                } else {
                    format!("\n\n-- TEST FAILED: {}", result.message)
                };
                display.push_str(&status);
            }
            text.0 = display;
        } else {
            text.0 = "Select a script or ask Minstral to create one".to_string();
        }
    }

    // Update chat log
    if let Ok(mut text) = chat_q.single_mut() {
        if state.chat_history.is_empty() {
            text.0 = "No chat history yet.\nUse F5-F7 for quick commands.".to_string();
        } else {
            let history: Vec<String> = state
                .chat_history
                .iter()
                .rev()
                .take(10)
                .rev()
                .map(|msg| {
                    let prefix = if msg.role == "user" { "You" } else { "Minstral" };
                    let content: String = msg.content.chars().take(100).collect();
                    format!("{}: {}", prefix, content)
                })
                .collect();
            let mut display = history.join("\n\n");
            if state.waiting_for_response {
                display.push_str("\n\n... Minstral is thinking ...");
            }
            text.0 = display;
        }
    }
}

/// Handle keyboard shortcuts for construct mode: number keys to select scripts.
pub fn construct_mode_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ConstructModeState>,
    library: Res<ScriptLibrary>,
) {
    if !state.active {
        return;
    }

    // Number keys 1-9 select scripts
    let key_map = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, idx) in &key_map {
        if keys.just_pressed(*key) {
            if let Some(script) = library.scripts.get(*idx) {
                state.editable_source = script.source.clone();
                state.current_script = Some(script.clone());
                state.test_result = None;
            }
        }
    }

    // T = test script (native only)
    #[cfg(not(target_arch = "wasm32"))]
    if keys.just_pressed(KeyCode::KeyT) && !state.editable_source.is_empty() {
        match cc_agent::lua_runtime::execute_script(&state.editable_source, 0) {
            Ok(commands) => {
                state.test_result = Some(ScriptTestResult {
                    success: true,
                    message: format!("Script OK - {} commands generated", commands.len()),
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
}
