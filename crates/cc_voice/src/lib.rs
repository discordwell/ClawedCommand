pub mod audio;
pub mod events;
pub mod intent;
pub mod pipeline;
pub mod transcriber;
pub mod vad;

use bevy::prelude::*;

/// Configuration resource for the voice pipeline.
#[derive(Resource)]
pub struct VoiceConfig {
    /// Path to Silero VAD ONNX model.
    pub vad_model_path: String,
    /// Path to Whisper GGML model file.
    pub whisper_model_path: String,
    /// Key to toggle mute/unmute (starts unmuted = listening).
    pub toggle_key: KeyCode,
    /// VAD probability threshold for speech onset.
    pub vad_onset_threshold: f32,
    /// VAD probability threshold for speech offset (lower = hysteresis).
    pub vad_offset_threshold: f32,
    /// Milliseconds of silence after speech offset before triggering transcription.
    pub silence_duration_ms: u32,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            vad_model_path: "assets/voice/silero_vad.onnx".into(),
            whisper_model_path: "assets/voice/ggml-tiny.en.bin".into(),
            toggle_key: KeyCode::KeyV,
            vad_onset_threshold: 0.5,
            vad_offset_threshold: 0.3,
            silence_duration_ms: 500,
        }
    }
}

/// Runtime state for the voice pipeline.
#[derive(Resource)]
pub struct VoiceState {
    /// Whether the voice pipeline is active (models loaded, mic available).
    pub enabled: bool,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// Bevy plugin for voice command recognition.
///
/// Registers:
/// - `VoiceConfig` resource (insert before adding plugin to override defaults)
/// - `VoiceCommandEvent` and `VoiceStateChanged` events
/// - Startup system to load models and spawn inference thread
/// - Update systems for mute toggle, polling results, and intent mapping
pub struct VoicePlugin;

impl Plugin for VoicePlugin {
    fn build(&self, app: &mut App) {
        // Insert default config if not already present
        if !app.world().contains_resource::<VoiceConfig>() {
            app.insert_resource(VoiceConfig::default());
        }
        app.insert_resource(VoiceState::default());

        app.add_message::<events::VoiceCommandEvent>();
        app.add_message::<events::VoiceStateChanged>();
        app.add_message::<events::VoicePingRequest>();

        app.add_systems(Startup, pipeline::startup_voice_pipeline);
        app.add_systems(
            Update,
            (
                pipeline::handle_voice_toggle,
                pipeline::poll_voice_results,
                intent::voice_intent_system,
            )
                .chain(),
        );
    }
}
