use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::renderer::screenshot::ScreenshotConfig;
use cc_sim::resources::{PlayerResources, SimClock};

const LOCAL_PLAYER: usize = 0;

/// Top bar: shows Food, GPU Cores, NFTs, Supply, clock, auto-capture indicator via egui.
pub fn resource_bar_system(
    mut contexts: EguiContexts,
    player_resources: Res<PlayerResources>,
    clock: Option<Res<SimClock>>,
    screenshot_config: Option<Res<ScreenshotConfig>>,
) {
    let Some(pres) = player_resources.players.get(LOCAL_PLAYER) else {
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::TopBottomPanel::top("resource_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;

            ui.colored_label(egui::Color32::from_rgb(255, 200, 100), format!("Food: {}", pres.food));
            ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("GPU: {}", pres.gpu_cores));
            ui.colored_label(egui::Color32::from_rgb(255, 200, 50), format!("NFTs: {}", pres.nfts));
            ui.separator();
            ui.colored_label(
                if pres.supply >= pres.supply_cap {
                    egui::Color32::RED
                } else {
                    egui::Color32::WHITE
                },
                format!("Supply: {}/{}", pres.supply, pres.supply_cap),
            );

            // Game clock
            let tick = clock.as_ref().map(|c| c.tick).unwrap_or(0);
            let total_secs = tick / 10;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            ui.separator();
            ui.colored_label(egui::Color32::LIGHT_GRAY, format!("{:02}:{:02}", mins, secs));

            // Auto-capture indicator
            if let Some(ref config) = screenshot_config {
                if let Some(interval) = config.auto_interval {
                    let label = if interval <= 10.0 {
                        "[AUTO 10s]"
                    } else {
                        "[AUTO 30s]"
                    };
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), label);
                }
            }
        });
    });
}
