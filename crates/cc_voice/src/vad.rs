//! Silero VAD wrapper — detects speech in 512-sample chunks using ONNX inference.
//!
//! Uses Silero VAD v5 (`silero_vad.onnx`, ~2.2MB). The model is stateful,
//! carrying RNN h/c state across calls for continuous speech detection.
//!
//! When compiled without the `voice` feature, falls back to a stub returning 0.0.

/// Speech probability threshold — above this means "speech detected".
pub const DEFAULT_SPEECH_THRESHOLD: f32 = 0.5;

/// Number of samples per VAD chunk (must be 512 for Silero VAD at 16kHz).
pub const VAD_CHUNK_SAMPLES: usize = 512;

/// Number of context samples to prepend before each VAD chunk.
/// Fixes the 0.04 → 0.999 detection discontinuity by providing overlap.
const VAD_CONTEXT_SAMPLES: usize = 64;

/// Silero VAD v5 state dimensions: batch=1, hidden=128, num_layers=2.
const VAD_STATE_SIZE: usize = 2 * 128;

/// Total input size: context (64) + chunk (512) = 576 samples.
#[cfg(feature = "voice")]
const VAD_INPUT_SIZE: usize = VAD_CONTEXT_SAMPLES + VAD_CHUNK_SAMPLES;

pub struct VadProcessor {
    /// Combined RNN state: [2, 1, 128] (Silero VAD v5 uses a single "state" tensor)
    state: Vec<f32>,
    /// Context buffer: last 64 samples from previous chunk, prepended to next chunk.
    /// Provides overlap to smooth detection probability transitions.
    context: Vec<f32>,
    /// ONNX runtime session for Silero VAD inference. None when running as stub.
    #[cfg(feature = "voice")]
    session: Option<ort::session::Session>,
}

impl VadProcessor {
    /// Create a VAD processor and load the ONNX model.
    #[cfg(feature = "voice")]
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let session = ort::session::Session::builder()?
            .with_intra_threads(1)?
            .commit_from_file(model_path)?;

        log::info!("Silero VAD model loaded from '{model_path}'");

        Ok(Self {
            state: vec![0.0f32; VAD_STATE_SIZE],
            context: vec![0.0f32; VAD_CONTEXT_SAMPLES],
            session: Some(session),
        })
    }

    /// Create a VAD processor stub (no model loaded, always returns 0.0).
    #[cfg(not(feature = "voice"))]
    pub fn new(_model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            state: vec![0.0f32; VAD_STATE_SIZE],
            context: vec![0.0f32; VAD_CONTEXT_SAMPLES],
        })
    }

    /// Create a stub VAD processor for testing (no ONNX model, returns 0.0).
    #[cfg(all(test, feature = "voice"))]
    fn new_stub() -> Self {
        Self {
            state: vec![0.0f32; VAD_STATE_SIZE],
            context: vec![0.0f32; VAD_CONTEXT_SAMPLES],
            session: None,
        }
    }

    /// Update the rolling context buffer with the tail of the given chunk.
    fn update_context(&mut self, audio_chunk: &[f32]) {
        let start = audio_chunk.len().saturating_sub(VAD_CONTEXT_SAMPLES);
        self.context.clear();
        self.context.extend_from_slice(&audio_chunk[start..]);
    }

    /// Process a 512-sample audio chunk and return speech probability [0.0, 1.0].
    ///
    /// Prepends 64-sample context buffer before the chunk for smoother detection.
    /// Runs ONNX inference with stateful RNN (h/c state carried across calls).
    #[cfg(feature = "voice")]
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        assert_eq!(
            audio_chunk.len(),
            VAD_CHUNK_SAMPLES,
            "VAD requires exactly {VAD_CHUNK_SAMPLES} samples per chunk"
        );

        let Some(session) = &mut self.session else {
            // Stub path: no model loaded, update context and return 0.0
            self.update_context(audio_chunk);
            return Ok(0.0);
        };

        use ort::value::TensorRef;

        // Build input: [1, 576] = context(64) + chunk(512)
        let mut input = Vec::with_capacity(VAD_INPUT_SIZE);
        input.extend_from_slice(&self.context);
        input.extend_from_slice(audio_chunk);

        let input_ref = TensorRef::from_array_view(([1usize, VAD_INPUT_SIZE], &input[..]))?;
        let state_ref = TensorRef::from_array_view(([2usize, 1, 128], &self.state[..]))?;
        let sr_data = [16000i64];
        let sr_ref = TensorRef::from_array_view(([1usize], &sr_data[..]))?;

        let outputs = session.run(ort::inputs![
            "input" => input_ref,
            "state" => state_ref,
            "sr" => sr_ref,
        ])?;

        // Extract speech probability and new RNN state from outputs.
        // Clone values before dropping `outputs` (which borrows `session`/`self`).
        let (_shape, prob_data) = outputs["output"].try_extract_tensor::<f32>()?;
        let prob = prob_data[0];
        let (_shape, new_state_data) = outputs["stateN"].try_extract_tensor::<f32>()?;
        let new_state = new_state_data.to_vec();
        drop(outputs);

        self.state = new_state;
        self.update_context(audio_chunk);

        Ok(prob)
    }

    /// Stub process — returns 0.0 when voice feature is disabled.
    #[cfg(not(feature = "voice"))]
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        assert_eq!(
            audio_chunk.len(),
            VAD_CHUNK_SAMPLES,
            "VAD requires exactly {VAD_CHUNK_SAMPLES} samples per chunk"
        );

        self.update_context(audio_chunk);

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

    fn make_vad() -> VadProcessor {
        #[cfg(feature = "voice")]
        {
            VadProcessor::new_stub()
        }
        #[cfg(not(feature = "voice"))]
        {
            VadProcessor::new("nonexistent.onnx").unwrap()
        }
    }

    #[test]
    fn test_vad_state_shape() {
        let vad = make_vad();
        assert_eq!(vad.state.len(), VAD_STATE_SIZE);
    }

    #[test]
    fn test_vad_reset() {
        let mut vad = make_vad();
        vad.state.fill(1.0);
        vad.context.fill(1.0);
        vad.reset();
        assert!(vad.state.iter().all(|&v| v == 0.0));
        assert!(vad.context.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_vad_silence_returns_zero() {
        let mut vad = make_vad();
        let silence = vec![0.0f32; VAD_CHUNK_SAMPLES];
        let prob = vad.process(&silence).unwrap();
        assert_eq!(prob, 0.0, "stub VAD should return 0.0 for any input");
    }

    #[test]
    fn test_vad_context_buffer_updated() {
        let mut vad = make_vad();

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
        let mut vad = make_vad();
        let chunk = vec![1.0f32; VAD_CHUNK_SAMPLES];
        vad.process(&chunk).unwrap();

        vad.reset();
        assert!(vad.context().iter().all(|&v| v == 0.0));
    }
}
