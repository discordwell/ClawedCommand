use bevy::prelude::*;

/// Emitted when the voice pipeline produces a transcription after VAD detects
/// the end of a speech segment (silence timeout or max duration).
#[derive(Message, Debug, Clone)]
pub struct VoiceCommandEvent {
    /// Full transcribed text (e.g. "all attack", "hisser stop", "move north").
    pub text: String,
}

/// Emitted when the voice listening state changes (mute/unmute toggle).
#[derive(Message, Debug, Clone)]
pub struct VoiceStateChanged {
    /// True when unmuted (actively listening via VAD), false when muted.
    pub listening: bool,
}
