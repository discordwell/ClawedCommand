use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_agent::agent_bridge::AgentBridge;

/// Collapsible side panel for agent chat history.
pub fn agent_chat_system(
    mut contexts: EguiContexts,
    bridge: Res<AgentBridge>,
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

            // Placeholder for chat history — messages come through AgentBridge
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label("Agent responses will appear here...");
                    // In a full implementation, we'd store and display
                    // the response log from poll_agent_responses
                });

            ui.separator();

            // Quick command buttons
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Scout").clicked() {
                    let _ = bridge.request_tx.send(
                        cc_agent::agent_bridge::AgentRequest {
                            player_id: 0,
                            prompt: "Scout the map and report enemy positions".into(),
                            tier: cc_agent::tool_tier::ToolTier::Advanced,
                        },
                    );
                }
                if ui.small_button("Defend").clicked() {
                    let _ = bridge.request_tx.send(
                        cc_agent::agent_bridge::AgentRequest {
                            player_id: 0,
                            prompt: "Defend our base from incoming threats".into(),
                            tier: cc_agent::tool_tier::ToolTier::Advanced,
                        },
                    );
                }
                if ui.small_button("Attack").clicked() {
                    let _ = bridge.request_tx.send(
                        cc_agent::agent_bridge::AgentRequest {
                            player_id: 0,
                            prompt: "Launch an attack on the enemy base".into(),
                            tier: cc_agent::tool_tier::ToolTier::Advanced,
                        },
                    );
                }
            });
        });
}
