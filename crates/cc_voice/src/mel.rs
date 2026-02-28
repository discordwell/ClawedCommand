/// Mel spectrogram computation.
///
/// Matches the Python training pipeline (`dataset.py:compute_mel_spectrogram`)
/// exactly to avoid train/inference mismatch.
#[cfg(feature = "voice")]
use realfft::{RealFftPlanner, RealToComplex};
#[cfg(feature = "voice")]
use std::sync::Arc;

/// Precomputed mel spectrogram configuration and filter bank.
pub struct MelConfig {
    pub sample_rate: usize,
    pub n_fft: usize,
    pub hop_length: usize,
    pub n_mels: usize,
    pub num_frames: usize,
    /// Triangular mel filter bank: [n_mels][n_fft/2 + 1]
    mel_filters: Vec<Vec<f32>>,
    /// Hann window
    window: Vec<f32>,
    #[cfg(feature = "voice")]
    fft: Arc<dyn RealToComplex<f32>>,
}

impl MelConfig {
    pub fn new(
        sample_rate: usize,
        n_fft: usize,
        hop_length: usize,
        n_mels: usize,
        fmin: f32,
        fmax: f32,
        num_frames: usize,
    ) -> Self {
        let mel_filters = build_mel_filter_bank(sample_rate, n_fft, n_mels, fmin, fmax);
        let window = hann_window(n_fft);

        #[cfg(feature = "voice")]
        let fft = {
            let mut planner = RealFftPlanner::<f32>::new();
            planner.plan_fft_forward(n_fft)
        };

        Self {
            sample_rate,
            n_fft,
            hop_length,
            n_mels,
            num_frames,
            mel_filters,
            window,
            #[cfg(feature = "voice")]
            fft,
        }
    }

    /// Default config matching the Python training pipeline.
    pub fn default_config() -> Self {
        Self::new(16000, 512, 320, 40, 60.0, 7800.0, 49)
    }

    /// Compute log-mel spectrogram from audio samples.
    ///
    /// Input: `audio` — f32 samples at `self.sample_rate` Hz, exactly 1 second.
    /// Output: flat Vec<f32> of length `n_mels * num_frames` in row-major order.
    #[cfg(feature = "voice")]
    pub fn compute(&self, audio: &[f32]) -> Vec<f32> {
        let n_fft = self.n_fft;
        let hop = self.hop_length;
        let half = n_fft / 2;

        // Reflect-pad the audio (matching numpy's pad reflect)
        let pad = half;
        let padded_len = audio.len() + 2 * pad;
        let mut padded = vec![0.0f32; padded_len];
        // Left reflect pad
        for i in 0..pad {
            padded[pad - 1 - i] = audio[(i + 1).min(audio.len() - 1)];
        }
        // Center (original audio)
        padded[pad..pad + audio.len()].copy_from_slice(audio);
        // Right reflect pad
        for i in 0..pad {
            let src_idx = audio.len().saturating_sub(2 + i);
            padded[pad + audio.len() + i] = audio[src_idx];
        }

        let n_stft_frames = 1 + (padded_len - n_fft) / hop;

        // Compute power spectrogram
        let freq_bins = half + 1;
        let mut power_spec = vec![0.0f32; freq_bins * n_stft_frames];

        let mut scratch = self.fft.make_scratch_vec();

        for frame_idx in 0..n_stft_frames {
            let start = frame_idx * hop;

            // Apply window
            let mut input_buf = self.fft.make_input_vec();
            for j in 0..n_fft {
                input_buf[j] = padded[start + j] * self.window[j];
            }

            let mut output_buf = self.fft.make_output_vec();
            self.fft
                .process_with_scratch(&mut input_buf, &mut output_buf, &mut scratch)
                .expect("FFT failed");

            // Power spectrum: |X|^2
            for (bin, c) in output_buf.iter().enumerate() {
                let re = c.re;
                let im = c.im;
                power_spec[bin * n_stft_frames + frame_idx] = re * re + im * im;
            }
        }

        // Apply mel filter bank + log
        let mut mel = vec![0.0f32; self.n_mels * self.num_frames];

        for m in 0..self.n_mels {
            for t in 0..self.num_frames.min(n_stft_frames) {
                let mut sum = 0.0f32;
                for f in 0..freq_bins {
                    sum += self.mel_filters[m][f] * power_spec[f * n_stft_frames + t];
                }
                // log(x + eps) for numerical stability
                mel[m * self.num_frames + t] = (sum + 1e-9).ln();
            }
        }

        // If fewer STFT frames than num_frames, remaining values stay as 0.0
        // (matching Python np.pad behavior — log(1e-9) ≈ -20.7)
        // Actually we should fill with log(eps) for consistency
        if n_stft_frames < self.num_frames {
            let fill_val = 1e-9_f32.ln();
            for m in 0..self.n_mels {
                for t in n_stft_frames..self.num_frames {
                    mel[m * self.num_frames + t] = fill_val;
                }
            }
        }

        mel
    }

    /// Compute mel spectrogram without realfft (fallback for tests without voice feature).
    #[cfg(not(feature = "voice"))]
    pub fn compute(&self, audio: &[f32]) -> Vec<f32> {
        self.compute_naive(audio)
    }

    /// Naive DFT-based mel computation (for testing / non-voice builds).
    pub fn compute_naive(&self, audio: &[f32]) -> Vec<f32> {
        let n_fft = self.n_fft;
        let hop = self.hop_length;
        let half = n_fft / 2;
        let freq_bins = half + 1;

        // Reflect-pad
        let pad = half;
        let padded_len = audio.len() + 2 * pad;
        let mut padded = vec![0.0f32; padded_len];
        for i in 0..pad {
            padded[pad - 1 - i] = audio[(i + 1).min(audio.len() - 1)];
        }
        padded[pad..pad + audio.len()].copy_from_slice(audio);
        for i in 0..pad {
            let src_idx = audio.len().saturating_sub(2 + i);
            padded[pad + audio.len() + i] = audio[src_idx];
        }

        let n_stft_frames = 1 + (padded_len - n_fft) / hop;

        // Naive DFT for each frame
        let mut power_spec = vec![0.0f32; freq_bins * n_stft_frames];
        for frame_idx in 0..n_stft_frames {
            let start = frame_idx * hop;
            for bin in 0..freq_bins {
                let freq = std::f32::consts::TAU * (bin as f32) / (n_fft as f32);
                let mut re = 0.0f32;
                let mut im = 0.0f32;
                for j in 0..n_fft {
                    let sample = padded[start + j] * self.window[j];
                    let angle = freq * (j as f32);
                    re += sample * angle.cos();
                    im -= sample * angle.sin();
                }
                power_spec[bin * n_stft_frames + frame_idx] = re * re + im * im;
            }
        }

        // Mel filter + log
        let mut mel = vec![0.0f32; self.n_mels * self.num_frames];
        for m in 0..self.n_mels {
            for t in 0..self.num_frames.min(n_stft_frames) {
                let mut sum = 0.0f32;
                for f in 0..freq_bins {
                    sum += self.mel_filters[m][f] * power_spec[f * n_stft_frames + t];
                }
                mel[m * self.num_frames + t] = (sum + 1e-9).ln();
            }
        }
        if n_stft_frames < self.num_frames {
            let fill_val = 1e-9_f32.ln();
            for m in 0..self.n_mels {
                for t in n_stft_frames..self.num_frames {
                    mel[m * self.num_frames + t] = fill_val;
                }
            }
        }

        mel
    }
}

/// Build a Hann window of length n.
fn hann_window(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let cos_val = (std::f32::consts::TAU * i as f32 / n as f32).cos();
            0.5 * (1.0 - cos_val)
        })
        .collect()
}

/// Build triangular mel filter bank.
/// Matches Python `_mel_filter_bank` exactly.
fn build_mel_filter_bank(
    sr: usize,
    n_fft: usize,
    n_mels: usize,
    fmin: f32,
    fmax: f32,
) -> Vec<Vec<f32>> {
    let freq_bins = n_fft / 2 + 1;

    let hz_to_mel = |hz: f32| -> f32 { 2595.0 * (1.0 + hz / 700.0).log10() };
    let mel_to_hz = |mel: f32| -> f32 { 700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0) };

    let mel_min = hz_to_mel(fmin);
    let mel_max = hz_to_mel(fmax);

    // n_mels + 2 linearly spaced points in mel scale
    let mel_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&hz| ((n_fft as f32 + 1.0) * hz / sr as f32).floor() as usize)
        .collect();

    let mut filters = vec![vec![0.0f32; freq_bins]; n_mels];

    for i in 0..n_mels {
        let left = bin_points[i];
        let center = bin_points[i + 1];
        let right = bin_points[i + 2];

        // Rising slope
        if center > left {
            for j in left..center {
                filters[i][j] = (j - left) as f32 / (center - left) as f32;
            }
        }
        // Falling slope
        if right > center {
            for j in center..right {
                filters[i][j] = (right - j) as f32 / (right - center) as f32;
            }
        }
    }

    filters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_near_minimum() {
        let cfg = MelConfig::default_config();
        let silence = vec![0.0f32; 16000];
        let mel = cfg.compute_naive(&silence);

        assert_eq!(mel.len(), 40 * 49);

        // Silence should produce near-minimum values (close to log(eps))
        let min_expected = 1e-9_f32.ln(); // ≈ -20.7
        for &val in &mel {
            assert!(val < -15.0, "silence mel value {val} should be near log(eps)");
            assert!(
                (val - min_expected).abs() < 10.0,
                "silence mel value {val} too far from {min_expected}"
            );
        }
    }

    #[test]
    fn test_sine_wave_correct_bin() {
        let cfg = MelConfig::default_config();

        // Generate 1kHz sine wave at 16kHz sample rate
        let freq = 1000.0f32;
        let sr = 16000.0f32;
        let audio: Vec<f32> = (0..16000)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr).sin() * 0.5)
            .collect();

        let mel = cfg.compute_naive(&audio);
        assert_eq!(mel.len(), 40 * 49);

        // Find the mel bin with max energy at the middle time frame
        let mid_frame = 24;
        let mut max_bin = 0;
        let mut max_val = f32::NEG_INFINITY;
        for m in 0..40 {
            let val = mel[m * 49 + mid_frame];
            if val > max_val {
                max_val = val;
                max_bin = m;
            }
        }

        // 1kHz should land roughly in mel bins 10-20 (depends on fmin/fmax)
        assert!(
            max_bin >= 5 && max_bin <= 25,
            "1kHz sine peak at mel bin {max_bin}, expected ~10-20"
        );
        // Peak should be well above silence
        assert!(max_val > -10.0, "1kHz sine peak too quiet: {max_val}");
    }

    #[test]
    fn test_output_shape() {
        let cfg = MelConfig::default_config();
        let audio = vec![0.1f32; 16000];
        let mel = cfg.compute_naive(&audio);
        assert_eq!(mel.len(), 40 * 49, "mel output should be [40, 49] = 1960 values");
    }

    #[test]
    fn test_mel_filter_bank_shape() {
        let filters = build_mel_filter_bank(16000, 512, 40, 60.0, 7800.0);
        assert_eq!(filters.len(), 40);
        assert_eq!(filters[0].len(), 257); // n_fft/2 + 1
    }

    #[test]
    fn test_hann_window() {
        let w = hann_window(512);
        assert_eq!(w.len(), 512);
        // Endpoints should be near zero
        assert!(w[0].abs() < 1e-6);
        // Middle should be near 1.0
        assert!((w[256] - 1.0).abs() < 0.01);
    }
}
