use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use cc_sim::resources::SimClock;

/// Configuration resource for the screenshot capture pipeline.
#[derive(Resource)]
pub struct ScreenshotConfig {
    /// Directory to save screenshots.
    pub output_dir: PathBuf,
    /// Auto-capture interval in seconds (None = off).
    pub auto_interval: Option<f32>,
    /// Timer for auto-capture.
    pub auto_timer: f32,
    /// Monotonic counter for unique filenames.
    pub counter: u64,
    /// Override fog of war state for captures: None or Some(false) = disable FoW for captures.
    pub fog_override: Option<bool>,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("tools/asset_pipeline/qc/screenshots"),
            auto_interval: None,
            auto_timer: 0.0,
            counter: 0,
            fog_override: Some(false),
        }
    }
}

/// F12: Take a screenshot immediately.
#[cfg(not(target_arch = "wasm32"))]
pub fn screenshot_hotkey(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ScreenshotConfig>,
    clock: Option<Res<SimClock>>,
) {
    if keyboard.just_pressed(KeyCode::F12) {
        capture_screenshot(&mut commands, &mut config, clock.as_deref(), "manual");
    }
}

/// F11: Cycle auto-capture interval (off -> 10s -> 30s -> off).
#[cfg(not(target_arch = "wasm32"))]
pub fn screenshot_auto_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ScreenshotConfig>,
) {
    if keyboard.just_pressed(KeyCode::F11) {
        config.auto_interval = match config.auto_interval {
            None => {
                info!("Auto-capture: every 10s");
                Some(10.0)
            }
            Some(t) if t <= 10.0 => {
                info!("Auto-capture: every 30s");
                Some(30.0)
            }
            _ => {
                info!("Auto-capture: off");
                None
            }
        };
        config.auto_timer = 0.0;
    }
}

/// Timer-based auto-capture system.
#[cfg(not(target_arch = "wasm32"))]
pub fn screenshot_auto_capture(
    mut commands: Commands,
    time: Res<Time>,
    mut config: ResMut<ScreenshotConfig>,
    clock: Option<Res<SimClock>>,
) {
    let Some(interval) = config.auto_interval else {
        return;
    };

    config.auto_timer += time.delta_secs();
    if config.auto_timer >= interval {
        config.auto_timer -= interval;
        capture_screenshot(&mut commands, &mut config, clock.as_deref(), "auto");
    }
}

/// Capture a screenshot to disk with a JSON sidecar.
#[cfg(not(target_arch = "wasm32"))]
fn capture_screenshot(
    commands: &mut Commands,
    config: &mut ScreenshotConfig,
    clock: Option<&SimClock>,
    context: &str,
) {
    // Ensure output directory exists
    let _ = std::fs::create_dir_all(&config.output_dir);

    let now = chrono_timestamp();
    let counter = config.counter;
    config.counter += 1;

    let filename = format!("cc_{now}_{counter:04}_{context}.png");
    let path = config.output_dir.join(&filename);

    // Spawn screenshot entity
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));

    // Write JSON sidecar
    let tick = clock.map(|c| c.tick).unwrap_or(0);
    let sidecar = format!(
        "{{\n  \"tick\": {},\n  \"counter\": {},\n  \"context\": \"{}\",\n  \"filename\": \"{}\"\n}}\n",
        tick, counter, context, filename
    );
    let sidecar_path = path.with_extension("json");
    let _ = std::fs::write(sidecar_path, sidecar);

    info!("Screenshot saved: {}", filename);
}

/// Generate a timestamp string YYYYMMDD_HHMMSS.
#[cfg(not(target_arch = "wasm32"))]
fn chrono_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple UTC breakdown without chrono dependency
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_min = 60u64;

    let days = now / secs_per_day;
    let time_of_day = now % secs_per_day;
    let hours = time_of_day / secs_per_hour;
    let minutes = (time_of_day % secs_per_hour) / secs_per_min;
    let seconds = time_of_day % secs_per_min;

    // Days since epoch to date (simplified)
    let mut y = 1970i32;
    let mut remaining_days = days as i32;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days: [i32; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    let d = remaining_days + 1;

    format!("{y:04}{m:02}{d:02}_{hours:02}{minutes:02}{seconds:02}")
}

#[cfg(not(target_arch = "wasm32"))]
fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
