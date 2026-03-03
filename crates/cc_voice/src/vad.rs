/// Silero VAD wrapper (currently unused — kept for future use).
///
/// Uses a Silero VAD v5 ONNX model to detect speech in 512-sample chunks.
/// The model is stateful (carries RNN h/c state across calls).
///
/// Not currently in the voice pipeline hot path — PTT press/release boundaries
/// are used instead. Will be re-enabled when ort dependency is wired back in.

/// Speech probability threshold — above this means "speech detected".
pub const DEFAULT_SPEECH_THRESHOLD: f32 = 0.5;

/// Number of samples per VAD chunk (must be 512 for Silero VAD at 16kHz).
pub const VAD_CHUNK_SAMPLES: usize = 512;

/// Number of context samples to prepend before each VAD chunk.
/// Fixes the 0.04 → 0.999 detection discontinuity by providing overlap.
const VAD_CONTEXT_SAMPLES: usize = 64;

/// Silero VAD v5 state dimensions: batch=1, hidden=128, num_layers=2.
const VAD_STATE_SIZE: usize = 2 * 1 * 128;

pub struct VadProcessor {
    /// Combined RNN state: [2, 1, 128] (Silero VAD v5 uses a single "state" tensor)
    state: Vec<f32>,
    /// Context buffer: last 64 samples from previous chunk, prepended to next chunk.
    /// Provides overlap to smooth detection probability transitions.
    context: Vec<f32>,
}

impl VadProcessor {
    /// Create a VAD processor stub.
    ///
    /// When ort is available (future), this will load the ONNX model.
    /// Currently returns a stub that always reports no speech.
    pub fn new(_model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            state: vec![0.0f32; VAD_STATE_SIZE],
            context: vec![0.0f32; VAD_CONTEXT_SAMPLES],
        })
    }

    /// Process a 512-sample audio chunk and return speech probability [0.0, 1.0].
    ///
    /// Prepends 64-sample context buffer before the chunk for smoother detection.
    /// Currently a stub — returns 0.0. Will use ort when re-enabled.
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        assert_eq!(
            audio_chunk.len(),
            VAD_CHUNK_SAMPLES,
            "VAD requires exactly {VAD_CHUNK_SAMPLES} samples per chunk"
        );

        // Save last VAD_CONTEXT_SAMPLES as context for next call
        let start = audio_chunk.len().saturating_sub(VAD_CONTEXT_SAMPLES);
        self.context.clear();
        self.context.extend_from_slice(&audio_chunk[start..]);

        // Stub: no model loaded, return 0.0
        Ok(0.0)
    }

    /// Reset the RNN state and context buffer (call between utterances).
    pub fn reset(&mut self) {
        self.state.fill(0.0);
        self.context.fill(0.0);
    }

    /// Get the current context buffer (for testing).
    #[cfg(test)]
    pub fn context(&self) -> &[f32] {
        &self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_state_shape() {
        let vad = VadProcessor::new("nonexistent.onnx").unwrap();
        assert_eq!(vad.state.len(), VAD_STATE_SIZE);
    }

    #[test]
    fn test_vad_reset() {
        let mut vad = VadProcessor::new("nonexistent.onnx").unwrap();
        vad.state.fill(1.0);
        vad.context.fill(1.0);
        vad.reset();
        assert!(vad.state.iter().all(|&v| v == 0.0));
        assert!(vad.context.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_vad_silence_returns_zero() {
        let mut vad = VadProcessor::new("nonexistent.onnx").unwrap();
        let silence = vec![0.0f32; VAD_CHUNK_SAMPLES];
        let prob = vad.process(&silence).unwrap();
        assert_eq!(prob, 0.0, "stub VAD should return 0.0 for any input");
    }

    #[test]
    fn test_vad_context_buffer_updated() {
        let mut vad = VadProcessor::new("nonexistent.onnx").unwrap();

        // Create a chunk with known values at the end
        let mut chunk = vec![0.0f32; VAD_CHUNK_SAMPLES];
        for i in 0..VAD_CONTEXT_SAMPLES {
            chunk[VAD_CHUNK_SAMPLES - VAD_CONTEXT_SAMPLES + i] = (i + 1) as f32;
        }

        vad.process(&chunk).unwrap();

        // Context should contain the last 64 samples
        let ctx = vad.context();
        assert_eq!(ctx.len(), VAD_CONTEXT_SAMPLES);
        for i in 0..VAD_CONTEXT_SAMPLES {
            assert_eq!(ctx[i], (i + 1) as f32, "context[{i}] mismatch");
        }
    }

    #[test]
    fn test_vad_context_reset_clears() {
        let mut vad = VadProcessor::new("nonexistent.onnx").unwrap();
        let chunk = vec![1.0f32; VAD_CHUNK_SAMPLES];
        vad.process(&chunk).unwrap();

        vad.reset();
        assert!(vad.context().iter().all(|&v| v == 0.0));
    }
}
