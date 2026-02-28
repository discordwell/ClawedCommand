use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::UiState;

/// Display toast notifications that fade over time.
pub fn notification_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Tick down notification timers
    for (_, remaining) in ui_state.notifications.iter_mut() {
        *remaining -= dt;
    }
    ui_state.notifications.retain(|(_, remaining)| *remaining > 0.0);

    if ui_state.notifications.is_empty() {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Area::new(egui::Id::new("notifications"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 40.0))
        .show(ctx, |ui| {
            for (msg, remaining) in &ui_state.notifications {
                let alpha = (*remaining).min(1.0);
                let color = egui::Color32::from_rgba_unmultiplied(255, 255, 200, (alpha * 255.0) as u8);
                ui.colored_label(color, msg);
            }
        });
}
