//! Compile-time test verifying the voice feature is active.
//!
//! When the `voice` feature is enabled, whisper-rs is available and
//! the transcriber module can be constructed (though we don't load a
//! real model in CI — just verify the types compile).

/// This test only compiles when the `voice` feature is active, which means
/// whisper-rs is available. If this test disappears from the test count, the
/// feature flag wiring is broken.
#[cfg(feature = "voice")]
#[test]
fn voice_feature_enables_whisper() {
    // Verify that the whisper_rs types are available when feature is enabled.
    // We can't load the model in CI (no ggml-tiny.en.bin), but the type
    // existence proves the dependency is wired correctly.
    use whisper_rs::WhisperContextParameters;
    let _params = WhisperContextParameters::default();
}
