/// TC-ResNet8 keyword classifier wrapper.
///
/// Loads an ONNX model and classifies mel spectrograms into keyword classes.
/// Stateless — each call is independent.
#[cfg(feature = "voice")]
use ort::session::Session;

/// Result of keyword classification.
#[derive(Debug, Clone)]
pub struct ClassifyResult {
    /// Predicted label string.
    pub label: String,
    /// Index into the labels list.
    pub label_idx: usize,
    /// Confidence (softmax probability) for the predicted class.
    pub confidence: f32,
}

pub struct KeywordClassifier {
    #[cfg(feature = "voice")]
    session: Session,
    labels: Vec<String>,
}

impl KeywordClassifier {
    /// Load the keyword classifier model and labels.
    ///
    /// `model_path`: path to `keyword_classifier.onnx`
    /// `labels_path`: path to `labels.txt` (one label per line)
    #[cfg(feature = "voice")]
    pub fn new(
        model_path: &str,
        labels_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let session = Session::builder()?.commit_from_file(model_path)?;
        let labels = Self::load_labels(labels_path)?;

        Ok(Self { session, labels })
    }

    /// Stub constructor for non-voice builds.
    #[cfg(not(feature = "voice"))]
    pub fn new(
        _model_path: &str,
        labels_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let labels = Self::load_labels(labels_path)?;
        Ok(Self { labels })
    }

    fn load_labels(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let labels: Vec<String> = content.lines().map(|l| l.trim().to_string()).collect();
        if labels.is_empty() {
            return Err("labels file is empty".into());
        }
        Ok(labels)
    }

    /// Classify a mel spectrogram.
    ///
    /// `mel`: flat Vec<f32> of length `n_mels * num_frames` (40 * 49 = 1960).
    /// Returns the top label and its confidence.
    #[cfg(feature = "voice")]
    pub fn classify(&mut self, mel: &[f32]) -> Result<ClassifyResult, Box<dyn std::error::Error>> {
        use ort::value::Value;

        let n_mels = 40;
        let num_frames = 49;
        assert_eq!(
            mel.len(),
            n_mels * num_frames,
            "Expected mel length {}, got {}",
            n_mels * num_frames,
            mel.len()
        );

        // Reshape to [1, 1, 40, 49]
        let input_tensor =
            Value::from_array(([1_usize, 1, n_mels, num_frames], mel.to_vec()))?;

        let outputs = self.session.run(ort::inputs!["mel_spectrogram" => input_tensor])?;

        let (_, logits) = outputs["logits"].try_extract_tensor::<f32>()?;

        // Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = logits.iter().map(|&x| (x - max_logit).exp()).sum();
        let probs: Vec<f32> = logits.iter().map(|&x| (x - max_logit).exp() / exp_sum).collect();

        // Find argmax
        let (label_idx, &confidence) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a): &(usize, &f32), (_, b): &(usize, &f32)| a.partial_cmp(b).unwrap())
            .unwrap();

        let label = self
            .labels
            .get(label_idx)
            .cloned()
            .unwrap_or_else(|| format!("class_{label_idx}"));

        Ok(ClassifyResult {
            label,
            label_idx,
            confidence,
        })
    }

    /// Stub classify for non-voice builds.
    #[cfg(not(feature = "voice"))]
    pub fn classify(&mut self, mel: &[f32]) -> Result<ClassifyResult, Box<dyn std::error::Error>> {
        let _ = mel;
        Ok(ClassifyResult {
            label: "unknown".to_string(),
            label_idx: 0,
            confidence: 0.0,
        })
    }

    pub fn num_classes(&self) -> usize {
        self.labels.len()
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_labels() {
        // Create a temp labels file
        let dir = std::env::temp_dir().join("cc_voice_test_labels");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("labels.txt");
        std::fs::write(
            &path,
            "attack\nretreat\nmove\nbuild\nstop\nunknown\nsilence\n",
        )
        .unwrap();

        let labels = KeywordClassifier::load_labels(path.to_str().unwrap()).unwrap();
        assert_eq!(labels.len(), 7);
        assert_eq!(labels[0], "attack");
        assert_eq!(labels[4], "stop");
        assert_eq!(labels[6], "silence");

        std::fs::remove_dir_all(dir).ok();
    }
}
