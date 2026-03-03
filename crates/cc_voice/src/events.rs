use bevy::prelude::*;

use cc_core::coords::GridPos;

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

/// Emitted when a voice command resolves to a target position.
/// The renderer spawns a visual sonar-ping at this location.
#[derive(Message, Debug, Clone)]
pub struct VoicePingRequest {
    pub target: GridPos,
    /// Terrain elevation at the target tile (for vertical offset).
    pub elevation: u8,
}
