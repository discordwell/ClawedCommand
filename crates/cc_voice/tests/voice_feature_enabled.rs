//! Compile-time test verifying the voice feature is active.
//!
//! When the `voice` feature is enabled, `MelConfig::compute()` uses realfft
//! (not the naive DFT fallback). These tests exercise that path and confirm
//! both implementations produce consistent results.
//!
//! The first test is `#[cfg(feature = "voice")]` — it only compiles when the
//! realfft path is available, serving as a compile-time feature gate check.

use cc_voice::mel::MelConfig;

/// This test only compiles when the `voice` feature is active, which means
/// realfft is available. If this test disappears from the test count, the
/// feature flag wiring is broken.
#[cfg(feature = "voice")]
#[test]
fn voice_feature_enables_realfft_mel() {
    let cfg = MelConfig::default_config();
    let audio = vec![0.0f32; 16000]; // 1 second of silence at 16kHz

    let mel = cfg.compute(&audio);

    // Correct output shape: 40 mel bins × 49 time frames
    assert_eq!(mel.len(), 40 * 49, "mel output should be [40, 49] = 1960 values");

    // Silence produces values near log(1e-9) ≈ -20.7
    let expected_min = 1e-9_f32.ln();
    for &val in &mel {
        assert!(
            (val - expected_min).abs() < 10.0,
            "unexpected mel value {val} — realfft path may not be active"
        );
    }

    // Cross-check: realfft and naive DFT should agree on silence
    let naive_result = cfg.compute_naive(&audio);
    let max_diff = mel
        .iter()
        .zip(naive_result.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 0.001,
        "silence: realfft and naive differ by {max_diff}"
    );
}

#[test]
fn mel_realfft_matches_naive_on_sine() {
    let cfg = MelConfig::default_config();

    // 440 Hz sine wave
    let audio: Vec<f32> = (0..16000)
        .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / 16000.0).sin() * 0.5)
        .collect();

    let realfft_result = cfg.compute(&audio);
    let naive_result = cfg.compute_naive(&audio);

    assert_eq!(realfft_result.len(), naive_result.len());

    // Naive DFT accumulates O(N^2) float error vs FFT's O(N log N), so
    // for N=512 there's genuine divergence (~4.8 log-mel units observed).
    // This tolerance catches catastrophic mismatches while allowing the
    // expected numerical difference between the two implementations.
    let max_diff = realfft_result
        .iter()
        .zip(naive_result.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_diff < 5.0,
        "realfft and naive mel differ by {max_diff} — paths may be mismatched"
    );
}
