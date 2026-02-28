use bevy::prelude::*;

/// Emitted when the voice pipeline recognizes a keyword.
#[derive(Message, Debug, Clone)]
pub struct VoiceCommandEvent {
    /// The recognized keyword (e.g. "attack", "stop", "pawdler").
    pub keyword: String,
    /// Classifier confidence in [0.0, 1.0].
    pub confidence: f32,
}

/// Emitted when the voice listening state changes.
#[derive(Message, Debug, Clone)]
pub struct VoiceStateChanged {
    pub listening: bool,
}
