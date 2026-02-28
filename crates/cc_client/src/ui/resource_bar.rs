use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use cc_sim::resources::PlayerResources;

const LOCAL_PLAYER: usize = 0;

/// Top bar: shows Food, GPU Cores, NFTs, Supply via egui.
pub fn resource_bar_system(mut contexts: EguiContexts, player_resources: Res<PlayerResources>) {
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
        });
    });
}
