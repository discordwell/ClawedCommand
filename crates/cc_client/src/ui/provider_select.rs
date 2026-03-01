//! Provider selection overlay — shown on WASM builds when AgentStatus == Unconfigured.
//!
//! Lets the user choose their AI backend before starting the agent loop.
//! Follows the same full-screen overlay pattern as `briefing.rs`.

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use cc_agent::llm_client::{AgentStatus, LlmBackend, LlmConfig};
use cc_agent::wasm_runner::ProviderSelection;

// ── Marker components ───────────────────────────────────────────────

#[derive(Component)]
pub struct ProviderSelectRoot;

#[derive(Component)]
pub struct ProviderSelectTitle;

#[derive(Component)]
pub struct ProviderOptionText;

/// Tag attached to each option row and its label child, carries the option index.
#[derive(Component)]
pub struct ProviderOptionRow(pub usize);

#[derive(Component)]
pub struct ProviderProgressBar;

#[derive(Component)]
pub struct ProviderProgressFill;

#[derive(Component)]
pub struct ProviderStatusLine;

#[derive(Component)]
pub struct ApiKeyInput;

// ── State resource ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOption {
    WebGpu,
    Ollama,
    RemoteApi,
    Skip,
}

impl ProviderOption {
    const ALL: [ProviderOption; 4] = [
        ProviderOption::WebGpu,
        ProviderOption::Ollama,
        ProviderOption::RemoteApi,
        ProviderOption::Skip,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::WebGpu => "In-Browser (WebGPU)",
            Self::Ollama => "Local Server (Ollama)",
            Self::RemoteApi => "Remote API",
            Self::Skip => "Skip (No AI)",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::WebGpu => "Run AI directly in your browser — requires WebGPU",
            Self::Ollama => "Connect to Ollama on localhost:11434",
            Self::RemoteApi => "Use a remote OpenAI-compatible API endpoint",
            Self::Skip => "Start without AI commander — mock responses only",
        }
    }

    fn index(&self) -> usize {
        Self::ALL.iter().position(|o| o == self).unwrap()
    }
}

/// Whether the provider selection overlay is currently blocking game input.
/// Other input systems can check for this resource to avoid conflicts.
#[derive(Resource, Default)]
pub struct ProviderOverlayActive(pub bool);

#[derive(Resource, Debug)]
pub struct ProviderSelectState {
    pub selected: ProviderOption,
    pub webgpu_available: bool,
    pub api_key_buf: String,
    pub api_url_buf: String,
    /// True when the user has confirmed and we're waiting for init.
    pub confirmed: bool,
}

impl Default for ProviderSelectState {
    fn default() -> Self {
        Self {
            selected: ProviderOption::WebGpu,
            webgpu_available: false,
            api_key_buf: String::new(),
            api_url_buf: "https://api.mistral.ai".into(),
            confirmed: false,
        }
    }
}

// ── Colors ──────────────────────────────────────────────────────────

const BG: Color = Color::srgba(0.02, 0.02, 0.06, 0.95);
const ACCENT: Color = Color::srgb(0.3, 0.8, 1.0);
const DIM: Color = Color::srgb(0.35, 0.35, 0.4);
const TEXT: Color = Color::srgb(0.85, 0.85, 0.85);
const ERROR_COLOR: Color = Color::srgb(1.0, 0.35, 0.35);
const PROGRESS_BG: Color = Color::srgb(0.15, 0.15, 0.2);
const PROGRESS_FG: Color = Color::srgb(0.3, 0.8, 1.0);

// ── Spawn ───────────────────────────────────────────────────────────

pub fn spawn_provider_select(mut commands: Commands) {
    // Detect WebGPU availability at spawn time (WASM only)
    #[cfg(target_arch = "wasm32")]
    let webgpu = cc_agent::webllm_client::webgpu_available();
    #[cfg(not(target_arch = "wasm32"))]
    let webgpu = false;

    commands.insert_resource(ProviderSelectState {
        webgpu_available: webgpu,
        ..default()
    });
    commands.insert_resource(ProviderOverlayActive(true));

    // Root overlay — full screen, centered
    commands
        .spawn((
            ProviderSelectRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GlobalZIndex(100),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // Inner panel
            root.spawn((
                Node {
                    width: Val::Px(480.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(BG),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((
                    ProviderSelectTitle,
                    Text::new("AI COMMANDER SETUP"),
                    TextColor(ACCENT),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Subtitle
                panel.spawn((
                    Text::new("Choose how Minstral connects:"),
                    TextColor(TEXT),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                ));

                // Option rows
                for (i, opt) in ProviderOption::ALL.iter().enumerate() {
                    panel
                        .spawn((
                            ProviderOptionRow(i),
                            Node {
                                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                        ))
                        .with_children(|row| {
                            row.spawn((
                                ProviderOptionText,
                                ProviderOptionRow(i),
                                Text::new(format!("  {}", opt.label())),
                                TextColor(TEXT),
                                TextFont {
                                    font_size: 15.0,
                                    ..default()
                                },
                            ));
                            row.spawn((
                                Text::new(format!("  {}", opt.description())),
                                TextColor(DIM),
                                TextFont {
                                    font_size: 11.0,
                                    ..default()
                                },
                            ));
                        });
                }

                // API key input area (hidden unless Remote API selected)
                panel.spawn((
                    ApiKeyInput,
                    Text::new(""),
                    TextColor(TEXT),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ));

                // Progress bar container
                panel
                    .spawn((
                        ProviderProgressBar,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(8.0),
                            margin: UiRect::top(Val::Px(8.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(PROGRESS_BG),
                        Visibility::Hidden,
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            ProviderProgressFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(PROGRESS_FG),
                        ));
                    });

                // Status line (errors, hints)
                panel.spawn((
                    ProviderStatusLine,
                    Text::new(""),
                    TextColor(DIM),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ));

                // Controls hint
                panel.spawn((
                    Text::new("[Up/Down] Navigate  [Enter] Confirm"),
                    TextColor(DIM),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    Node {
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                ));
            });
        });
}

// ── Overlay visibility + active flag ────────────────────────────────

pub fn toggle_provider_overlay(
    agent_status: Res<AgentStatus>,
    mut overlay_active: ResMut<ProviderOverlayActive>,
    mut root_vis: Query<&mut Visibility, With<ProviderSelectRoot>>,
) {
    let show = matches!(
        *agent_status,
        AgentStatus::Unconfigured | AgentStatus::Initializing(_) | AgentStatus::Error(_)
    );

    overlay_active.0 = show;

    for mut vis in root_vis.iter_mut() {
        *vis = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// ── Option highlighting ─────────────────────────────────────────────

pub fn update_provider_options(
    state: Res<ProviderSelectState>,
    agent_status: Res<AgentStatus>,
    mut option_rows: Query<
        (&ProviderOptionRow, &mut BackgroundColor),
        Without<ProviderOptionText>,
    >,
    mut option_texts: Query<
        (&ProviderOptionRow, &mut TextColor),
        With<ProviderOptionText>,
    >,
) {
    if !matches!(
        *agent_status,
        AgentStatus::Unconfigured | AgentStatus::Initializing(_) | AgentStatus::Error(_)
    ) {
        return;
    }

    let selected = state.selected.index();

    for (row, mut bg) in option_rows.iter_mut() {
        *bg = if row.0 == selected {
            BackgroundColor(Color::srgba(0.3, 0.8, 1.0, 0.12))
        } else {
            BackgroundColor(Color::NONE)
        };
    }

    for (row, mut color) in option_texts.iter_mut() {
        let is_disabled = row.0 == ProviderOption::WebGpu.index() && !state.webgpu_available;
        color.0 = if is_disabled {
            Color::srgb(0.25, 0.25, 0.3)
        } else if row.0 == selected {
            ACCENT
        } else {
            TEXT
        };
    }
}

// ── Progress bar + status line + API key display ────────────────────

pub fn update_provider_status(
    state: Res<ProviderSelectState>,
    agent_status: Res<AgentStatus>,
    mut api_key_q: Query<
        (&mut Text, &mut Visibility),
        (With<ApiKeyInput>, Without<ProviderProgressBar>),
    >,
    mut progress_vis: Query<
        &mut Visibility,
        (With<ProviderProgressBar>, Without<ApiKeyInput>),
    >,
    mut progress_fill: Query<&mut Node, With<ProviderProgressFill>>,
    mut status_line: Query<
        (&mut Text, &mut TextColor),
        (With<ProviderStatusLine>, Without<ApiKeyInput>),
    >,
) {
    if !matches!(
        *agent_status,
        AgentStatus::Unconfigured | AgentStatus::Initializing(_) | AgentStatus::Error(_)
    ) {
        return;
    }

    // API key input
    let show_api = state.selected == ProviderOption::RemoteApi && !state.confirmed;
    if let Ok((mut text, mut vis)) = api_key_q.single_mut() {
        *vis = if show_api {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if show_api {
            let masked_key = if state.api_key_buf.is_empty() {
                "API Key: (type to enter)".to_string()
            } else {
                let visible = state.api_key_buf.len().min(4);
                let masked = "*".repeat(state.api_key_buf.len().saturating_sub(visible));
                format!(
                    "API Key: {}{}",
                    masked,
                    &state.api_key_buf[state.api_key_buf.len().saturating_sub(visible)..]
                )
            };
            text.0 = format!("URL: {}\n{}", state.api_url_buf, masked_key);
        }
    }

    // Progress bar
    if let Ok(mut vis) = progress_vis.single_mut() {
        *vis = if matches!(*agent_status, AgentStatus::Initializing(_)) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let AgentStatus::Initializing(pct) = *agent_status {
        if let Ok(mut node) = progress_fill.single_mut() {
            node.width = Val::Percent(pct * 100.0);
        }
    }

    // Status line
    if let Ok((mut text, mut color)) = status_line.single_mut() {
        match &*agent_status {
            AgentStatus::Initializing(pct) => {
                text.0 = format!("Downloading model... {:.0}%", pct * 100.0);
                color.0 = ACCENT;
            }
            AgentStatus::Error(msg) => {
                let hint = if msg.contains("CORS") || msg.contains("cors") {
                    " (check CORS headers on your API server)"
                } else {
                    ""
                };
                text.0 = format!("Error: {}{}", msg.chars().take(60).collect::<String>(), hint);
                color.0 = ERROR_COLOR;
            }
            AgentStatus::Unconfigured => {
                if !state.webgpu_available && state.selected == ProviderOption::WebGpu {
                    text.0 = "WebGPU not available in this browser".into();
                    color.0 = Color::srgb(0.8, 0.6, 0.2);
                } else {
                    text.0 = String::new();
                    color.0 = DIM;
                }
            }
            _ => {
                text.0 = String::new();
            }
        }
    }
}

// ── Input handling ──────────────────────────────────────────────────

pub fn provider_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut kb_events: MessageReader<KeyboardInput>,
    mut state: ResMut<ProviderSelectState>,
    agent_status: Res<AgentStatus>,
    mut commands: Commands,
) {
    // On error, allow retry by resetting confirmed flag
    if matches!(*agent_status, AgentStatus::Error(_)) && state.confirmed {
        state.confirmed = false;
    }

    // Only handle input when unconfigured or error (allow retry)
    if !matches!(*agent_status, AgentStatus::Unconfigured | AgentStatus::Error(_)) {
        return;
    }

    if state.confirmed {
        return;
    }

    let option_count = ProviderOption::ALL.len();
    let selected_idx = state.selected.index();

    // Navigation
    if keys.just_pressed(KeyCode::ArrowUp) {
        let new_idx = if selected_idx > 0 {
            selected_idx - 1
        } else {
            option_count - 1
        };
        state.selected = ProviderOption::ALL[new_idx];
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.selected = ProviderOption::ALL[(selected_idx + 1) % option_count];
    }

    // API key text input using KeyboardInput events for proper character handling
    if state.selected == ProviderOption::RemoteApi {
        for event in kb_events.read() {
            if event.state != ButtonState::Pressed {
                continue;
            }
            if event.key_code == KeyCode::Backspace && !state.api_key_buf.is_empty() {
                state.api_key_buf.pop();
            } else if let Some(ref text) = event.text {
                // Filter out control characters; only append printable text
                for ch in text.chars() {
                    if !ch.is_control() {
                        state.api_key_buf.push(ch);
                    }
                }
            }
        }
    }

    // Confirm
    if keys.just_pressed(KeyCode::Enter) {
        let option = state.selected;

        // Block WebGPU if unavailable
        if option == ProviderOption::WebGpu && !state.webgpu_available {
            return;
        }

        state.confirmed = true;

        let config = match option {
            ProviderOption::WebGpu => LlmConfig {
                backend: LlmBackend::WebLlm,
                ..default()
            },
            ProviderOption::Ollama => LlmConfig {
                backend: LlmBackend::OpenAiCompatible,
                base_url: "http://localhost:11434".into(),
                ..default()
            },
            ProviderOption::RemoteApi => LlmConfig {
                backend: LlmBackend::OpenAiCompatible,
                base_url: state.api_url_buf.clone(),
                api_key: state.api_key_buf.clone(),
                ..default()
            },
            ProviderOption::Skip => LlmConfig {
                backend: LlmBackend::Mock,
                ..default()
            },
        };

        let backend = config.backend.clone();
        commands.insert_resource(config);
        commands.insert_resource(ProviderSelection { backend });
    }
}
