use bevy::prelude::*;

/// Emitted when the voice pipeline produces a transcription from PTT release.
#[derive(Message, Debug, Clone)]
pub struct VoiceCommandEvent {
    /// Full transcribed text (e.g. "all attack", "hisser stop", "move north").
    pub text: String,
}

/// Emitted when the voice listening state changes.
#[derive(Message, Debug, Clone)]
pub struct VoiceStateChanged {
    pub listening: bool,
}
