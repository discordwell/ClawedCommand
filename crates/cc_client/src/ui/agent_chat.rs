use bevy::prelude::*;

use cc_agent::agent_bridge::{AgentBridge, AgentChatLog, AgentRequest, AgentSource};
use cc_agent::llm_client::AgentStatus;
use cc_agent::tool_tier::FactionToolStates;

/// Marker for agent chat panel root.
#[derive(Component)]
pub struct AgentChatRoot;

/// Marker for agent chat text.
#[derive(Component)]
pub struct AgentChatText;

/// Marker for agent status display.
#[derive(Component)]
pub struct AgentStatusText;

pub fn spawn_agent_chat(mut commands: Commands) {
    commands
        .spawn((
            AgentChatRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(34.0),
                right: Val::Px(8.0),
                width: Val::Px(240.0),
                max_height: Val::Px(400.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("AI AGENT"),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));
            parent.spawn((
                AgentStatusText,
                Text::new("Status: Ready"),
                TextColor(Color::srgb(0.4, 1.0, 0.4)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
            ));
            parent.spawn((
                AgentChatText,
                Text::new("No agent activity yet."),
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
            ));
        });
}

pub fn update_agent_chat(
    chat_log: Res<AgentChatLog>,
    agent_status: Option<Res<AgentStatus>>,
    mut status_q: Query<
        (&mut Text, &mut TextColor),
        (With<AgentStatusText>, Without<AgentChatText>),
    >,
    mut text_q: Query<&mut Text, (With<AgentChatText>, Without<AgentStatusText>)>,
) {
    // Update status text
    if let Ok((mut status_text, mut status_color)) = status_q.single_mut() {
        let (label, color) = match agent_status.as_deref() {
            Some(AgentStatus::Ready) => ("Status: Ready", Color::srgb(0.4, 1.0, 0.4)),
            Some(AgentStatus::Initializing(pct)) => {
                status_text.0 = format!("Initializing... {:.0}%", pct * 100.0);
                status_color.0 = Color::srgb(1.0, 0.8, 0.2);
                return;
            }
            Some(AgentStatus::Error(msg)) => {
                status_text.0 = format!("Error: {}", msg.chars().take(40).collect::<String>());
                status_color.0 = Color::srgb(1.0, 0.3, 0.3);
                return;
            }
            _ => ("Status: Unconfigured", Color::srgb(0.5, 0.5, 0.5)),
        };
        status_text.0 = label.to_string();
        status_color.0 = color;
    }

    // Update chat log
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    if chat_log.entries.is_empty() {
        text.0 = "No agent activity yet.".to_string();
        return;
    }

    // Show last ~10 entries
    let entries: Vec<String> = chat_log
        .entries
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|entry| {
            if let Some(err) = &entry.error {
                format!("[!] {}", err)
            } else {
                entry.content.chars().take(200).collect()
            }
        })
        .collect();

    text.0 = entries.join("\n\n");
}

/// Quick command buttons system — handles keyboard shortcuts for agent commands.
pub fn agent_quick_commands(
    keys: Res<ButtonInput<KeyCode>>,
    bridge: Res<AgentBridge>,
    tool_states: Res<FactionToolStates>,
) {
    let tier = tool_states.tier_for(0);

    // F5=Scout, F6=Defend, F7=Attack
    if keys.just_pressed(KeyCode::F5) {
        let _ = bridge.request_tx.try_send(AgentRequest {
            player_id: 0,
            prompt: "Scout the map and report enemy positions".into(),
            tier,
            source: AgentSource::QuickCommand,
            chat_history: None,
        });
    }
    if keys.just_pressed(KeyCode::F6) {
        let _ = bridge.request_tx.try_send(AgentRequest {
            player_id: 0,
            prompt: "Defend our base from incoming threats".into(),
            tier,
            source: AgentSource::QuickCommand,
            chat_history: None,
        });
    }
    if keys.just_pressed(KeyCode::F7) {
        let _ = bridge.request_tx.try_send(AgentRequest {
            player_id: 0,
            prompt: "Launch an attack on the enemy base".into(),
            tier,
            source: AgentSource::QuickCommand,
            chat_history: None,
        });
    }
}
