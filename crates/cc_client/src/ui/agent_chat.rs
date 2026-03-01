use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_agent::agent_bridge::{AgentBridge, AgentChatLog, AgentRequest, AgentSource};
use cc_agent::tool_tier::FactionToolStates;

/// Collapsible side panel for agent chat history.
pub fn agent_chat_system(
    mut contexts: EguiContexts,
    bridge: Res<AgentBridge>,
    chat_log: Res<AgentChatLog>,
    tool_states: Res<FactionToolStates>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::SidePanel::right("agent_chat")
        .resizable(true)
        .default_width(250.0)
        .max_width(400.0)
        .show(ctx, |ui| {
            ui.heading("AI Agent");
            ui.separator();

            // Show connection status
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(egui::Color32::GREEN, "Connected");
            });

            ui.separator();

            // Display response log
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if chat_log.entries.is_empty() {
                        ui.label("No agent activity yet.");
                    }
                    for entry in &chat_log.entries {
                        if let Some(err) = &entry.error {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 100, 100),
                                format!("Error: {err}"),
                            );
                        }
                        if !entry.content.is_empty() {
                            ui.label(&entry.content);
                        }
                        ui.add_space(4.0);
                    }
                });

            ui.separator();

            // Quick command buttons
            let tier = tool_states.tier_for(0);
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Scout").clicked() {
                    let _ = bridge.request_tx.send(AgentRequest {
                        player_id: 0,
                        prompt: "Scout the map and report enemy positions".into(),
                        tier,
                        source: AgentSource::QuickCommand,
                        chat_history: None,
                    });
                }
                if ui.small_button("Defend").clicked() {
                    let _ = bridge.request_tx.send(AgentRequest {
                        player_id: 0,
                        prompt: "Defend our base from incoming threats".into(),
                        tier,
                        source: AgentSource::QuickCommand,
                        chat_history: None,
                    });
                }
                if ui.small_button("Attack").clicked() {
                    let _ = bridge.request_tx.send(AgentRequest {
                        player_id: 0,
                        prompt: "Launch an attack on the enemy base".into(),
                        tier,
                        source: AgentSource::QuickCommand,
                        chat_history: None,
                    });
                }
            });
        });
}
