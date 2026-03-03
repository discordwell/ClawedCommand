/// Whisper-based speech transcriber.
///
/// Replaces the TC-ResNet8 keyword classifier with whisper.cpp (via whisper-rs).
/// Uses the tiny.en model (~75MB) for fast on-device transcription of short
/// voice commands (1-3 seconds).
#[cfg(feature = "voice")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Wraps a Whisper context for transcribing short audio clips.
pub struct WhisperTranscriber {
    #[cfg(feature = "voice")]
    ctx: WhisperContext,
}

impl WhisperTranscriber {
    /// Load a GGML Whisper model from disk.
    ///
    /// `model_path`: path to a `ggml-*.bin` file (e.g. `ggml-tiny.en.bin`).
    #[cfg(feature = "voice")]
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load Whisper model from '{model_path}': {e}"))?;
        Ok(Self { ctx })
    }

    /// Stub constructor for non-voice builds.
    #[cfg(not(feature = "voice"))]
    pub fn new(_model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }

    /// Transcribe 16kHz mono f32 audio into lowercase text.
    ///
    /// Returns the trimmed, lowercased transcription. For very short or silent
    /// audio, may return an empty string.
    #[cfg(feature = "voice")]
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_single_segment(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // Suppress non-speech tokens for cleaner output
        params.set_suppress_blank(true);
        params.set_no_context(true);

        let mut state = self.ctx.create_state()
            .map_err(|e| format!("Failed to create Whisper state: {e}"))?;

        state.full(params, audio)
            .map_err(|e| format!("Whisper inference failed: {e}"))?;

        let n_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..n_segments {
            if let Some(segment) = state.get_segment(i) {
                // Discard segments that Whisper thinks are non-speech
                // (hallucinations like "thank you for watching", etc.)
                if segment.no_speech_probability() > 0.6 {
                    log::debug!(
                        "Discarding segment (no_speech_prob={:.2}): {:?}",
                        segment.no_speech_probability(),
                        segment.to_str_lossy()
                    );
                    continue;
                }
                if let Ok(segment_text) = segment.to_str_lossy() {
                    text.push_str(&segment_text);
                }
            }
        }

        Ok(text.trim().to_lowercase())
    }

    /// Stub transcribe for non-voice builds.
    #[cfg(not(feature = "voice"))]
    pub fn transcribe(&self, _audio: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_transcriber_returns_empty() {
        // Non-voice stub should construct and return empty string
        #[cfg(not(feature = "voice"))]
        {
            let t = WhisperTranscriber::new("nonexistent.bin").unwrap();
            let result = t.transcribe(&[0.0; 16000]).unwrap();
            assert!(result.is_empty());
        }
    }
}
