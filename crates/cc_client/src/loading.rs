//! Asset loading progress tracker for the WASM play page.
//!
//! Tracks Bevy asset loading and exposes progress to JavaScript
//! via a `#[wasm_bindgen]` export. The play.html loading bar uses
//! this to show real asset loading progress (50%-100% range).

use bevy::prelude::*;
use bevy::asset::UntypedAssetId;

#[cfg(target_arch = "wasm32")]
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Atomic progress value accessible from JS (stores f32 bits).
/// 0.0 = no assets loaded, 1.0 = all loaded.
#[cfg(target_arch = "wasm32")]
static LOADING_PROGRESS: AtomicU32 = AtomicU32::new(0);

/// JS-callable: get the current asset loading progress (0.0 to 1.0).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn cc_get_loading_progress() -> f32 {
    f32::from_bits(LOADING_PROGRESS.load(Ordering::Relaxed))
}

/// Resource that tracks asset handles to monitor loading progress.
#[derive(Resource, Default)]
pub struct LoadingTracker {
    /// Asset IDs to track. Once all are loaded, progress = 1.0.
    pub assets: Vec<UntypedAssetId>,
    /// Set to true once all assets report loaded.
    pub complete: bool,
}

impl LoadingTracker {
    /// Register an asset handle for tracking.
    pub fn track<A: Asset>(&mut self, handle: &Handle<A>) {
        self.assets.push(handle.id().untyped());
    }
}

/// System that checks asset loading progress each frame.
fn track_loading_system(
    asset_server: Res<AssetServer>,
    mut tracker: ResMut<LoadingTracker>,
) {
    if tracker.complete || tracker.assets.is_empty() {
        return;
    }

    let total = tracker.assets.len();
    let loaded = tracker
        .assets
        .iter()
        .filter(|id| {
            matches!(
                asset_server.get_load_state(**id),
                Some(bevy::asset::LoadState::Loaded)
            )
        })
        .count();

    let progress = loaded as f32 / total as f32;

    #[cfg(target_arch = "wasm32")]
    LOADING_PROGRESS.store(progress.to_bits(), Ordering::Relaxed);

    if loaded == total {
        tracker.complete = true;
        info!("All {} tracked assets loaded", total);
    }
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadingTracker>()
            .add_systems(Update, track_loading_system);
    }
}
