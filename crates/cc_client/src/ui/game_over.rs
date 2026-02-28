use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_sim::resources::GameState;

const LOCAL_PLAYER: u8 = 0;

/// Full-screen overlay when the game ends.
pub fn game_over_system(mut contexts: EguiContexts, game_state: Res<GameState>) {
    let winner = match *game_state {
        GameState::Playing => return,
        GameState::Victory { winner } => winner,
    };

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let (title, color) = if winner == LOCAL_PLAYER {
        ("VICTORY!", egui::Color32::from_rgb(255, 215, 0))
    } else {
        ("DEFEAT!", egui::Color32::from_rgb(255, 60, 60))
    };

    egui::Area::new(egui::Id::new("game_over_overlay"))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                .inner_margin(egui::Margin::same(40))
                .corner_radius(12.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(title)
                                .size(64.0)
                                .color(color)
                                .strong(),
                        );
                        ui.add_space(16.0);
                        let reason = if winner == LOCAL_PLAYER {
                            "Enemy base destroyed!"
                        } else {
                            "Your base has been destroyed!"
                        };
                        ui.label(
                            egui::RichText::new(reason)
                                .size(24.0)
                                .color(egui::Color32::WHITE),
                        );
                    });
                });
        });
}
