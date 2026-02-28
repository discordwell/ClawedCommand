/// Three-thread voice processing pipeline.
///
/// Architecture:
///   1. **OS audio thread** (cpal): mic → ring buffer (lock-free SPSC)
///   2. **Inference thread**: ring buffer → VAD → mel → classifier → channel
///   3. **Bevy main thread**: channel → VoiceCommandEvent
///
/// The inference thread runs in a tight loop:
///   - Check `is_listening` (PTT flag)
///   - Read 512 samples from ring buffer
///   - Run Silero VAD; if speech > threshold, accumulate
///   - At 16000 samples (1s), compute mel and classify
///   - Send result over crossbeam channel
#[cfg(feature = "voice")]
use crossbeam_channel::{Receiver, Sender};
#[cfg(feature = "voice")]
use ringbuf::traits::Consumer;

use bevy::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[cfg(feature = "voice")]
use crate::events::VoiceCommandEvent;
use crate::events::VoiceStateChanged;
use crate::{VoiceConfig, VoiceState};
#[cfg(feature = "voice")]
use crate::classifier::ClassifyResult;
#[cfg(feature = "voice")]
use crate::mel::MelConfig;

/// Resource holding the channel receiver for classified voice results.
#[cfg(feature = "voice")]
#[derive(Resource)]
pub struct VoiceResultReceiver {
    pub rx: Receiver<ClassifyResult>,
}

/// Shared atomic flag: true when PTT is held down and we should listen.
#[derive(Resource, Clone)]
pub struct ListeningFlag {
    pub flag: Arc<AtomicBool>,
}

/// Startup system: loads models, starts audio capture, spawns inference thread.
#[cfg(feature = "voice")]
pub fn startup_voice_pipeline(mut commands: Commands, config: Res<VoiceConfig>) {
    log::info!("Initializing voice pipeline...");

    // Load models
    let vad_path = &config.vad_model_path;
    let classifier_path = &config.classifier_model_path;
    let labels_path = &config.labels_path;

    let vad = match crate::vad::VadProcessor::new(vad_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to load VAD model from '{vad_path}': {e}");
            log::warn!("Voice pipeline disabled — VAD model not found");
            commands.insert_resource(VoiceState { enabled: false });
            return;
        }
    };

    let classifier = match crate::classifier::KeywordClassifier::new(classifier_path, labels_path)
    {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "Failed to load classifier from '{classifier_path}' / '{labels_path}': {e}"
            );
            log::warn!("Voice pipeline disabled — classifier model not found");
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
    let (tx, rx) = crossbeam_channel::bounded::<ClassifyResult>(16);

    // PTT flag
    let listening = Arc::new(AtomicBool::new(false));
    let listening_clone = listening.clone();
    let confidence_threshold = config.confidence_threshold;

    // Spawn inference thread
    std::thread::Builder::new()
        .name("voice-inference".into())
        .spawn(move || {
            inference_loop(consumer, vad, classifier, tx, listening_clone, confidence_threshold);
        })
        .expect("failed to spawn voice inference thread");

    log::info!("Voice pipeline ready (PTT: V key)");

    commands.insert_resource(VoiceResultReceiver { rx });
    commands.insert_resource(ListeningFlag { flag: listening });
    commands.insert_resource(VoiceState { enabled: true });
}

/// Startup stub when voice feature is disabled.
#[cfg(not(feature = "voice"))]
pub fn startup_voice_pipeline(mut commands: Commands) {
    log::info!("Voice pipeline disabled (compiled without 'voice' feature)");
    commands.insert_resource(VoiceState { enabled: false });
}

/// Pad/crop accumulated audio to target length, compute mel, classify, and send result.
///
/// Handles short utterances by zero-padding to `target_samples`.
#[cfg(feature = "voice")]
fn classify_and_send(
    accumulator: &[f32],
    target_samples: usize,
    mel_config: &MelConfig,
    classifier: &mut crate::classifier::KeywordClassifier,
    confidence_threshold: f32,
    tx: &Sender<ClassifyResult>,
) {
    // Pad or center-crop to exactly target_samples
    let audio: Vec<f32> = if accumulator.len() > target_samples {
        let start = (accumulator.len() - target_samples) / 2;
        accumulator[start..start + target_samples].to_vec()
    } else if accumulator.len() < target_samples {
        // Zero-pad short utterances (centered)
        let mut padded = vec![0.0f32; target_samples];
        let offset = (target_samples - accumulator.len()) / 2;
        padded[offset..offset + accumulator.len()].copy_from_slice(accumulator);
        padded
    } else {
        accumulator.to_vec()
    };

    let mel = mel_config.compute(&audio);
    match classifier.classify(&mel) {
        Ok(result) => {
            if result.confidence >= confidence_threshold
                && result.label != "unknown"
                && result.label != "silence"
            {
                log::debug!(
                    "Classified: '{}' ({:.2}%)",
                    result.label,
                    result.confidence * 100.0
                );
                let _ = tx.send(result);
            }
        }
        Err(e) => {
            log::error!("Classifier error: {e}");
        }
    }
}

/// The inference thread's main loop.
#[cfg(feature = "voice")]
fn inference_loop(
    mut consumer: ringbuf::HeapCons<f32>,
    mut vad: crate::vad::VadProcessor,
    mut classifier: crate::classifier::KeywordClassifier,
    tx: Sender<ClassifyResult>,
    is_listening: Arc<AtomicBool>,
    confidence_threshold: f32,
) {
    let mel_config = MelConfig::default_config();
    let target_samples: usize = 16000; // 1 second
    let mut accumulator: Vec<f32> = Vec::with_capacity(target_samples);
    let mut speech_active = false;
    let mut chunk_buf = [0.0f32; crate::vad::VAD_CHUNK_SAMPLES];
    // Track partial reads across iterations to avoid dropping samples
    let mut chunk_offset: usize = 0;
    // Minimum samples for classification (half a second) — shorter utterances are padded
    let min_speech_samples: usize = target_samples / 2;

    loop {
        // If not listening (PTT not held), sleep briefly and drain buffer
        if !is_listening.load(Ordering::Relaxed) {
            if speech_active {
                // PTT released mid-speech — reset
                accumulator.clear();
                speech_active = false;
                vad.reset();
            }
            chunk_offset = 0;
            // Drain any buffered audio to keep ring buffer fresh
            while consumer.try_pop().is_some() {}
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue;
        }

        // Try to read a VAD-sized chunk (512 samples), continuing from partial reads
        while chunk_offset < crate::vad::VAD_CHUNK_SAMPLES {
            match consumer.try_pop() {
                Some(sample) => {
                    chunk_buf[chunk_offset] = sample;
                    chunk_offset += 1;
                }
                None => break,
            }
        }

        if chunk_offset < crate::vad::VAD_CHUNK_SAMPLES {
            // Not enough samples yet — wait a bit (partial read preserved for next iteration)
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }

        // Full chunk ready — reset offset for next chunk
        chunk_offset = 0;

        // Run VAD
        let speech_prob = match vad.process(&chunk_buf) {
            Ok(p) => p,
            Err(e) => {
                log::error!("VAD error: {e}");
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        };

        if speech_prob > crate::vad::DEFAULT_SPEECH_THRESHOLD {
            if !speech_active {
                speech_active = true;
                accumulator.clear();
            }
            accumulator.extend_from_slice(&chunk_buf);
        } else if speech_active {
            // Speech ended — add trailing audio then classify immediately
            accumulator.extend_from_slice(&chunk_buf);

            // Classify if we have enough audio (at least half a second)
            if accumulator.len() >= min_speech_samples {
                classify_and_send(
                    &accumulator, target_samples, &mel_config,
                    &mut classifier, confidence_threshold, &tx,
                );
            }

            accumulator.clear();
            speech_active = false;
            vad.reset();
            continue;
        }

        // When we have enough audio during ongoing speech, classify
        if speech_active && accumulator.len() >= target_samples {
            classify_and_send(
                &accumulator, target_samples, &mel_config,
                &mut classifier, confidence_threshold, &tx,
            );

            // Reset for next utterance
            accumulator.clear();
            speech_active = false;
            vad.reset();
        }

        // Safety limit: if accumulator grows too large without classification, reset
        if accumulator.len() > target_samples * 3 {
            log::warn!("Voice accumulator overflow — resetting");
            accumulator.clear();
            speech_active = false;
            vad.reset();
        }
    }
}

/// Bevy system: polls the voice result channel and emits VoiceCommandEvents.
#[cfg(feature = "voice")]
pub fn poll_voice_results(
    receiver: Option<Res<VoiceResultReceiver>>,
    mut voice_events: MessageWriter<VoiceCommandEvent>,
) {
    let Some(receiver) = receiver else { return };
    // Drain all pending results
    while let Ok(result) = receiver.rx.try_recv() {
        voice_events.write(VoiceCommandEvent {
            keyword: result.label,
            confidence: result.confidence,
        });
    }
}

/// Stub poller for non-voice builds.
#[cfg(not(feature = "voice"))]
pub fn poll_voice_results() {}

/// Bevy system: V key toggles push-to-talk.
pub fn handle_ptt_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    listening: Option<Res<ListeningFlag>>,
    config: Res<VoiceConfig>,
    mut state_events: MessageWriter<VoiceStateChanged>,
) {
    let Some(listening) = listening else { return };

    let ptt_key = config.ptt_key;

    if keyboard.just_pressed(ptt_key) {
        listening.flag.store(true, Ordering::Relaxed);
        state_events.write(VoiceStateChanged { listening: true });
        log::debug!("PTT pressed — listening");
    }
    if keyboard.just_released(ptt_key) {
        listening.flag.store(false, Ordering::Relaxed);
        state_events.write(VoiceStateChanged { listening: false });
        log::debug!("PTT released — stopped");
    }
}
