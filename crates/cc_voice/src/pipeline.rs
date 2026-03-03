/// Three-thread voice processing pipeline with VAD-driven speech detection.
///
/// Architecture:
///   1. **OS audio thread** (cpal): mic → ring buffer (lock-free SPSC)
///   2. **Inference thread**: ring buffer → VAD detects speech → accumulate → silence timeout → Whisper → channel
///   3. **Bevy main thread**: channel → VoiceCommandEvent
///
/// The inference thread runs a VAD state machine:
///   - Idle: read 512-sample chunks, run VAD. Speech onset → Speaking.
///   - Speaking: accumulate chunks. Speech offset → Trailing. Max 5s → force transcribe.
///   - Trailing: continue accumulating. Speech resumes → Speaking. 500ms silence → transcribe.
#[cfg(feature = "voice")]
use crossbeam_channel::{Receiver, Sender};
#[cfg(feature = "voice")]
use ringbuf::traits::Consumer;

use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(feature = "voice")]
use crate::events::VoiceCommandEvent;
use crate::events::VoiceStateChanged;
use crate::vad::VAD_CHUNK_SAMPLES;
use crate::{VoiceConfig, VoiceState};

/// Resource holding the channel receiver for transcribed voice text.
#[cfg(feature = "voice")]
#[derive(Resource)]
pub struct VoiceTranscriptReceiver {
    pub rx: Receiver<String>,
}

/// Shared atomic flag: true when muted (voice listening suppressed).
/// Default: false (starts listening/unmuted).
#[derive(Resource, Clone)]
pub struct MuteFlag {
    pub flag: Arc<AtomicBool>,
}

// ── VAD State Machine ──────────────────────────────────────────────────────

/// VAD-driven speech detection state machine.
///
/// States: Idle → Speaking → Trailing → (emit audio) → Idle
///
/// Designed to be unit-testable: feed VAD probabilities and audio chunks,
/// get back completed speech segments.
#[derive(Debug, PartialEq, Eq)]
enum VadState {
    Idle,
    Speaking,
    Trailing,
}

pub(crate) struct VadStateMachine {
    state: VadState,
    /// Accumulated speech audio samples.
    accumulator: Vec<f32>,
    /// Rolling pre-speech buffer (1 VAD chunk = 512 samples).
    pre_speech_buf: Vec<f32>,
    /// Count of consecutive low-probability chunks in Trailing state.
    silence_chunks: u32,

    // Thresholds
    onset_threshold: f32,
    offset_threshold: f32,
    /// Number of silent chunks before triggering transcription.
    silence_chunk_limit: u32,
    /// Minimum samples for valid speech (shorter = noise).
    min_samples: usize,
    /// Maximum samples before forced transcription.
    max_samples: usize,
}

impl VadStateMachine {
    /// Create a new VAD state machine with the given thresholds.
    ///
    /// - `onset_threshold`: VAD probability to start speech (default 0.5)
    /// - `offset_threshold`: VAD probability to consider silence (default 0.3)
    /// - `silence_duration_ms`: milliseconds of silence to end speech (default 500)
    pub fn new(onset_threshold: f32, offset_threshold: f32, silence_duration_ms: u32) -> Self {
        // 16kHz / 512 samples = 31.25 chunks/sec → silence_ms / 32 ≈ chunk count
        let silence_chunk_limit = (silence_duration_ms as f32 / 32.0).ceil() as u32;

        Self {
            state: VadState::Idle,
            accumulator: Vec::with_capacity(48000),
            pre_speech_buf: Vec::with_capacity(VAD_CHUNK_SAMPLES),
            silence_chunks: 0,
            onset_threshold,
            offset_threshold,
            silence_chunk_limit,
            min_samples: 4800,  // 300ms at 16kHz
            max_samples: 80000, // 5s at 16kHz
        }
    }

    /// Feed a VAD probability and audio chunk. Returns accumulated speech audio
    /// when a complete utterance is detected (speech ended by silence timeout
    /// or max duration reached).
    pub fn feed(&mut self, prob: f32, chunk: &[f32]) -> Option<Vec<f32>> {
        match self.state {
            VadState::Idle => {
                if prob >= self.onset_threshold {
                    // Speech onset — prepend pre-speech buffer for word onset preservation
                    self.accumulator.clear();
                    self.accumulator.extend_from_slice(&self.pre_speech_buf);
                    self.accumulator.extend_from_slice(chunk);
                    self.state = VadState::Speaking;
                    log::debug!("VAD: speech onset (prob={prob:.2})");
                } else {
                    // Update rolling pre-speech buffer
                    self.pre_speech_buf.clear();
                    self.pre_speech_buf.extend_from_slice(chunk);
                }
                None
            }
            VadState::Speaking => {
                self.accumulator.extend_from_slice(chunk);

                // Max duration reached — force transcription
                if self.accumulator.len() >= self.max_samples {
                    log::info!("VAD: max speech duration reached, forcing transcription");
                    return self.emit();
                }

                if prob < self.offset_threshold {
                    // Speech may be ending — enter trailing silence
                    self.silence_chunks = 1;
                    self.state = VadState::Trailing;
                    log::trace!("VAD: speech offset candidate (prob={prob:.2})");
                }
                None
            }
            VadState::Trailing => {
                self.accumulator.extend_from_slice(chunk);

                // Max duration check
                if self.accumulator.len() >= self.max_samples {
                    log::info!("VAD: max speech duration reached during trailing");
                    return self.emit();
                }

                if prob >= self.onset_threshold {
                    // Speech resumed
                    self.silence_chunks = 0;
                    self.state = VadState::Speaking;
                    log::trace!("VAD: speech resumed (prob={prob:.2})");
                    None
                } else {
                    self.silence_chunks += 1;
                    if self.silence_chunks >= self.silence_chunk_limit {
                        // Silence timeout — end of utterance
                        log::debug!(
                            "VAD: silence timeout ({} chunks), ending utterance",
                            self.silence_chunks
                        );
                        // Trim trailing silence (remove the silent chunks)
                        let trim_samples = self.silence_chunks as usize * VAD_CHUNK_SAMPLES;
                        let keep = self.accumulator.len().saturating_sub(trim_samples);
                        self.accumulator.truncate(keep);
                        self.emit()
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Extract accumulated audio and reset state. Returns None if too short.
    fn emit(&mut self) -> Option<Vec<f32>> {
        let audio = std::mem::take(&mut self.accumulator);
        self.state = VadState::Idle;
        self.silence_chunks = 0;
        self.pre_speech_buf.clear();

        if audio.len() >= self.min_samples {
            let duration_ms = audio.len() * 1000 / 16000;
            log::info!(
                "VAD: emitting {duration_ms}ms speech ({} samples)",
                audio.len()
            );
            Some(audio)
        } else {
            log::debug!(
                "VAD: discarding too-short speech ({} samples < {})",
                audio.len(),
                self.min_samples
            );
            None
        }
    }

    /// Reset all state (call when muting).
    pub fn reset(&mut self) {
        self.accumulator.clear();
        self.pre_speech_buf.clear();
        self.silence_chunks = 0;
        self.state = VadState::Idle;
    }
}

// ── Startup & Systems ──────────────────────────────────────────────────────

/// Startup system: loads Whisper + VAD models, starts audio capture, spawns inference thread.
#[cfg(feature = "voice")]
pub fn startup_voice_pipeline(mut commands: Commands, config: Res<VoiceConfig>) {
    log::info!("Initializing voice pipeline (passive VAD mode)...");

    let whisper_path = &config.whisper_model_path;

    let transcriber = match crate::transcriber::WhisperTranscriber::new(whisper_path) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to load Whisper model from '{whisper_path}': {e}");
            log::warn!("Voice pipeline disabled — Whisper model not found");
            commands.insert_resource(VoiceState { enabled: false });
            return;
        }
    };

    // Load VAD model
    let vad_path = &config.vad_model_path;
    let vad = match crate::vad::VadProcessor::new(vad_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to load VAD model from '{vad_path}': {e}");
            log::warn!("Voice pipeline disabled — VAD model not found");
            commands.insert_resource(VoiceState { enabled: false });
            return;
        }
    };

    // Start audio capture (stream stays alive on its own keepalive thread)
    let (consumer, _audio_active) = match crate::audio::start_capture() {
        Ok(pair) => pair,
        Err(e) => {
            log::error!("Failed to start audio capture: {e}");
            log::warn!("Voice pipeline disabled — no microphone");
            commands.insert_resource(VoiceState { enabled: false });
            return;
        }
    };

    // Channel for inference → Bevy
    let (tx, rx) = crossbeam_channel::bounded::<String>(16);

    // Mute flag — false = listening (default)
    let mute = Arc::new(AtomicBool::new(false));
    let mute_clone = mute.clone();

    let state_machine = VadStateMachine::new(
        config.vad_onset_threshold,
        config.vad_offset_threshold,
        config.silence_duration_ms,
    );

    // Spawn inference thread
    std::thread::Builder::new()
        .name("voice-inference".into())
        .spawn(move || {
            inference_loop(consumer, vad, transcriber, tx, mute_clone, state_machine);
        })
        .expect("failed to spawn voice inference thread");

    log::info!("Voice pipeline ready (passive VAD, V key = mute toggle)");

    commands.insert_resource(VoiceTranscriptReceiver { rx });
    commands.insert_resource(MuteFlag { flag: mute });
    commands.insert_resource(VoiceState { enabled: true });
}

/// Startup stub when voice feature is disabled.
#[cfg(not(feature = "voice"))]
pub fn startup_voice_pipeline(mut commands: Commands) {
    log::info!("Voice pipeline disabled (compiled without 'voice' feature)");
    commands.insert_resource(VoiceState { enabled: false });
}

/// The inference thread's main loop — VAD-driven speech detection.
///
/// Reads 512-sample chunks from the ring buffer, runs VAD, and uses the
/// VadStateMachine to detect speech boundaries. Transcribes with Whisper
/// when speech ends.
#[cfg(feature = "voice")]
fn inference_loop(
    mut consumer: ringbuf::HeapCons<f32>,
    mut vad: crate::vad::VadProcessor,
    transcriber: crate::transcriber::WhisperTranscriber,
    tx: Sender<String>,
    is_muted: Arc<AtomicBool>,
    mut state_machine: VadStateMachine,
) {
    let mut chunk_buf = vec![0.0f32; VAD_CHUNK_SAMPLES];
    // Persist fill position across iterations to avoid losing partial reads
    let mut filled = 0usize;

    loop {
        if is_muted.load(Ordering::Relaxed) {
            // Muted — drain ring buffer, discard in-progress speech, sleep
            while consumer.try_pop().is_some() {}
            state_machine.reset();
            vad.reset();
            filled = 0;
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }

        // Continue filling chunk_buf from where we left off
        while filled < VAD_CHUNK_SAMPLES {
            match consumer.try_pop() {
                Some(sample) => {
                    chunk_buf[filled] = sample;
                    filled += 1;
                }
                None => break,
            }
        }

        if filled < VAD_CHUNK_SAMPLES {
            // Not enough samples yet — sleep briefly and retry (partial data preserved)
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }

        // Full chunk ready — reset fill counter for next chunk
        filled = 0;

        // Run VAD on the chunk
        let prob = match vad.process(&chunk_buf) {
            Ok(p) => p,
            Err(e) => {
                log::error!("VAD inference error: {e}");
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        };

        // Feed to state machine
        if let Some(speech_audio) = state_machine.feed(prob, &chunk_buf) {
            // Speech segment complete — transcribe with Whisper
            let duration_ms = speech_audio.len() * 1000 / 16000;
            log::info!(
                "Transcribing {duration_ms}ms of speech ({} samples)",
                speech_audio.len()
            );

            match transcriber.transcribe(&speech_audio) {
                Ok(text) => {
                    if !text.is_empty() {
                        log::info!("Whisper transcription: \"{text}\"");
                        let _ = tx.send(text);
                    } else {
                        log::debug!("Whisper returned empty transcription — noise?");
                    }
                }
                Err(e) => {
                    log::error!("Whisper transcription error: {e}");
                }
            }

            // Reset VAD RNN state between utterances for clean detection
            vad.reset();
        }
    }
}

/// Bevy system: polls the voice transcript channel and emits VoiceCommandEvents.
#[cfg(feature = "voice")]
pub fn poll_voice_results(
    receiver: Option<Res<VoiceTranscriptReceiver>>,
    mut voice_events: MessageWriter<VoiceCommandEvent>,
) {
    let Some(receiver) = receiver else { return };
    // Drain all pending transcriptions
    while let Ok(text) = receiver.rx.try_recv() {
        voice_events.write(VoiceCommandEvent { text });
    }
}

/// Stub poller for non-voice builds.
#[cfg(not(feature = "voice"))]
pub fn poll_voice_results() {}

/// Bevy system: V key toggles mute/unmute for voice listening.
pub fn handle_voice_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mute: Option<Res<MuteFlag>>,
    config: Res<VoiceConfig>,
    mut state_events: MessageWriter<VoiceStateChanged>,
) {
    let Some(mute) = mute else { return };

    if keyboard.just_pressed(config.toggle_key) {
        let was_muted = mute.flag.load(Ordering::Relaxed);
        let now_muted = !was_muted;
        mute.flag.store(now_muted, Ordering::Relaxed);
        state_events.write(VoiceStateChanged {
            listening: !now_muted,
        });
        if now_muted {
            log::info!("Voice muted (V toggle)");
        } else {
            log::info!("Voice unmuted (V toggle)");
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sm() -> VadStateMachine {
        // onset=0.5, offset=0.3, silence=500ms (~16 chunks)
        VadStateMachine::new(0.5, 0.3, 500)
    }

    fn chunk(val: f32) -> Vec<f32> {
        vec![val; VAD_CHUNK_SAMPLES]
    }

    #[test]
    fn idle_stays_idle_on_silence() {
        let mut sm = make_sm();
        let result = sm.feed(0.1, &chunk(0.0));
        assert!(result.is_none());
        assert_eq!(sm.state, VadState::Idle);
    }

    #[test]
    fn onset_transitions_to_speaking() {
        let mut sm = make_sm();
        let result = sm.feed(0.6, &chunk(0.5));
        assert!(result.is_none());
        assert_eq!(sm.state, VadState::Speaking);
        // Accumulator should contain pre-speech buf + chunk
        assert_eq!(sm.accumulator.len(), VAD_CHUNK_SAMPLES); // pre-speech buf was empty
    }

    #[test]
    fn onset_prepends_pre_speech_buffer() {
        let mut sm = make_sm();
        // First, feed a silent chunk to populate pre-speech buffer
        sm.feed(0.1, &chunk(0.1));
        assert_eq!(sm.pre_speech_buf.len(), VAD_CHUNK_SAMPLES);

        // Now trigger onset — should prepend pre-speech buf
        sm.feed(0.6, &chunk(0.5));
        assert_eq!(sm.state, VadState::Speaking);
        assert_eq!(sm.accumulator.len(), VAD_CHUNK_SAMPLES * 2);
    }

    #[test]
    fn speaking_to_trailing_on_low_prob() {
        let mut sm = make_sm();
        sm.feed(0.6, &chunk(0.5)); // → Speaking
        sm.feed(0.2, &chunk(0.0)); // → Trailing
        assert_eq!(sm.state, VadState::Trailing);
    }

    #[test]
    fn trailing_back_to_speaking_on_high_prob() {
        let mut sm = make_sm();
        sm.feed(0.6, &chunk(0.5)); // → Speaking
        sm.feed(0.2, &chunk(0.0)); // → Trailing
        sm.feed(0.6, &chunk(0.5)); // → Speaking
        assert_eq!(sm.state, VadState::Speaking);
        assert_eq!(sm.silence_chunks, 0);
    }

    #[test]
    fn silence_timeout_triggers_emission() {
        let mut sm = make_sm();

        // Need enough speech to pass min_samples (4800 = ~9.4 chunks)
        sm.feed(0.6, &chunk(0.5)); // Speaking
        for _ in 0..10 {
            sm.feed(0.6, &chunk(0.5)); // Keep speaking
        }

        // Now go silent for silence_chunk_limit chunks
        let limit = sm.silence_chunk_limit;
        for i in 0..limit {
            let result = sm.feed(0.1, &chunk(0.0));
            if i == limit - 1 {
                // Last silent chunk should trigger emission
                assert!(result.is_some(), "should emit on silence timeout");
                let audio = result.unwrap();
                // Audio should have trailing silence trimmed
                assert!(audio.len() >= sm.min_samples);
            } else {
                assert!(result.is_none());
            }
        }
        assert_eq!(sm.state, VadState::Idle);
    }

    #[test]
    fn too_short_speech_discarded() {
        let mut sm = make_sm();

        // Only 1 chunk of speech (~32ms) — below min_samples (300ms = 4800 samples)
        sm.feed(0.6, &chunk(0.5)); // Speaking, 512 samples

        // Immediately go silent
        let limit = sm.silence_chunk_limit;
        let mut emitted = false;
        for _ in 0..limit {
            if sm.feed(0.1, &chunk(0.0)).is_some() {
                emitted = true;
            }
        }
        assert!(!emitted, "too-short speech should be discarded");
        assert_eq!(sm.state, VadState::Idle);
    }

    #[test]
    fn max_speech_forces_transcription() {
        let mut sm = make_sm();
        sm.feed(0.6, &chunk(0.5)); // Speaking

        // Feed until max_samples exceeded
        let chunks_to_max = sm.max_samples / VAD_CHUNK_SAMPLES + 1;
        let mut emitted = false;
        for _ in 0..chunks_to_max {
            if sm.feed(0.6, &chunk(0.5)).is_some() {
                emitted = true;
                break;
            }
        }
        assert!(emitted, "should force transcription at max_samples");
        assert_eq!(sm.state, VadState::Idle);
    }

    #[test]
    fn reset_returns_to_idle() {
        let mut sm = make_sm();
        sm.feed(0.6, &chunk(0.5)); // Speaking
        sm.feed(0.6, &chunk(0.5)); // More speaking
        assert_eq!(sm.state, VadState::Speaking);

        sm.reset();
        assert_eq!(sm.state, VadState::Idle);
        assert!(sm.accumulator.is_empty());
        assert!(sm.pre_speech_buf.is_empty());
        assert_eq!(sm.silence_chunks, 0);
    }
}
