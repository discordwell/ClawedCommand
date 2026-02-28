/// Silero VAD wrapper.
///
/// Uses a Silero VAD v5 ONNX model to detect speech in 512-sample chunks.
/// The model is stateful (carries RNN h/c state across calls).
#[cfg(feature = "voice")]
use ort::session::Session;

/// Speech probability threshold — above this means "speech detected".
pub const DEFAULT_SPEECH_THRESHOLD: f32 = 0.5;

/// Number of samples per VAD chunk (must be 512 for Silero VAD at 16kHz).
pub const VAD_CHUNK_SAMPLES: usize = 512;

pub struct VadProcessor {
    #[cfg(feature = "voice")]
    session: Session,
    /// RNN hidden state: [2, 1, 64]
    h_state: Vec<f32>,
    /// RNN cell state: [2, 1, 64]
    c_state: Vec<f32>,
    #[cfg(feature = "voice")]
    sample_rate: i64,
}

impl VadProcessor {
    /// Load Silero VAD model from ONNX file.
    #[cfg(feature = "voice")]
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let session = Session::builder()?.commit_from_file(model_path)?;

        Ok(Self {
            session,
            h_state: vec![0.0f32; 2 * 1 * 64],
            c_state: vec![0.0f32; 2 * 1 * 64],
            sample_rate: 16000,
        })
    }

    /// Create a stub processor for testing without a model file.
    #[cfg(not(feature = "voice"))]
    pub fn new(_model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            h_state: vec![0.0f32; 2 * 1 * 64],
            c_state: vec![0.0f32; 2 * 1 * 64],
        })
    }

    /// Process a 512-sample audio chunk and return speech probability [0.0, 1.0].
    #[cfg(feature = "voice")]
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        assert_eq!(
            audio_chunk.len(),
            VAD_CHUNK_SAMPLES,
            "VAD requires exactly {VAD_CHUNK_SAMPLES} samples per chunk"
        );

        use ort::value::Value;

        let input_tensor =
            Value::from_array(([1, VAD_CHUNK_SAMPLES], audio_chunk.to_vec()))?;
        let sr_tensor = Value::from_array(([0usize; 0], vec![self.sample_rate]))?;
        let h_tensor = Value::from_array(([2, 1, 64], self.h_state.clone()))?;
        let c_tensor = Value::from_array(([2, 1, 64], self.c_state.clone()))?;

        let outputs = self.session.run(ort::inputs![
            "input" => input_tensor,
            "sr" => sr_tensor,
            "h" => h_tensor,
            "c" => c_tensor,
        ])?;

        // Output 0: speech probability [1, 1]
        let (_, prob_data) = outputs["output"].try_extract_tensor::<f32>()?;
        let prob = prob_data[0];

        // Output 1, 2: updated h, c states
        let (_, h_data) = outputs["hn"].try_extract_tensor::<f32>()?;
        self.h_state.copy_from_slice(h_data);

        let (_, c_data) = outputs["cn"].try_extract_tensor::<f32>()?;
        self.c_state.copy_from_slice(c_data);

        Ok(prob)
    }

    /// Stub process for non-voice builds (always returns 0.0).
    #[cfg(not(feature = "voice"))]
    pub fn process(&mut self, audio_chunk: &[f32]) -> Result<f32, Box<dyn std::error::Error>> {
        let _ = audio_chunk;
        Ok(0.0)
    }

    /// Reset the RNN state (call between utterances).
    pub fn reset(&mut self) {
        self.h_state.fill(0.0);
        self.c_state.fill(0.0);
    }

    /// Create a stub processor for testing (no ONNX model needed).
    #[cfg(test)]
    fn test_stub() -> Self {
        Self {
            #[cfg(feature = "voice")]
            session: panic!("test_stub should only be used in no-voice-feature tests"),
            h_state: vec![0.0f32; 2 * 1 * 64],
            c_state: vec![0.0f32; 2 * 1 * 64],
            #[cfg(feature = "voice")]
            sample_rate: 16000,
        }
    }
}

#[cfg(test)]
#[cfg(not(feature = "voice"))]
mod tests {
    use super::*;

    #[test]
    fn test_vad_state_shape() {
        let vad = VadProcessor::test_stub();
        assert_eq!(vad.h_state.len(), 128);
        assert_eq!(vad.c_state.len(), 128);
    }

    #[test]
    fn test_vad_reset() {
        let mut vad = VadProcessor::test_stub();
        vad.h_state.fill(1.0);
        vad.c_state.fill(1.0);
        vad.reset();
        assert!(vad.h_state.iter().all(|&v| v == 0.0));
        assert!(vad.c_state.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_vad_silence_returns_zero() {
        let mut vad = VadProcessor::test_stub();
        let silence = vec![0.0f32; VAD_CHUNK_SAMPLES];
        let prob = vad.process(&silence).unwrap();
        assert_eq!(prob, 0.0, "stub VAD should return 0.0 for any input");
    }
}
