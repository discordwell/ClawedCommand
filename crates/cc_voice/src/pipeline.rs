/// Three-thread voice processing pipeline.
///
/// Architecture:
///   1. **OS audio thread** (cpal): mic → ring buffer (lock-free SPSC)
///   2. **Inference thread**: ring buffer → accumulate while PTT held → Whisper on release → channel
///   3. **Bevy main thread**: channel → VoiceCommandEvent
///
/// The inference thread runs in a tight loop:
///   - PTT not held → drain ring buffer, sleep
///   - PTT just pressed → clear accumulator, start accumulating
///   - PTT held → read samples from ring buffer into accumulator
///   - PTT released → run Whisper on accumulated audio, send text result
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

/// Resource holding the channel receiver for transcribed voice text.
#[cfg(feature = "voice")]
#[derive(Resource)]
pub struct VoiceTranscriptReceiver {
    pub rx: Receiver<String>,
}

/// Shared atomic flag: true when PTT is held down and we should listen.
#[derive(Resource, Clone)]
pub struct ListeningFlag {
    pub flag: Arc<AtomicBool>,
}

/// Startup system: loads Whisper model, starts audio capture, spawns inference thread.
#[cfg(feature = "voice")]
pub fn startup_voice_pipeline(mut commands: Commands, config: Res<VoiceConfig>) {
    log::info!("Initializing voice pipeline...");

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

    // PTT flag
    let listening = Arc::new(AtomicBool::new(false));
    let listening_clone = listening.clone();

    // Spawn inference thread
    std::thread::Builder::new()
        .name("voice-inference".into())
        .spawn(move || {
            inference_loop(consumer, transcriber, tx, listening_clone);
        })
        .expect("failed to spawn voice inference thread");

    log::info!("Voice pipeline ready (PTT: V key, Whisper tiny.en)");

    commands.insert_resource(VoiceTranscriptReceiver { rx });
    commands.insert_resource(ListeningFlag { flag: listening });
    commands.insert_resource(VoiceState { enabled: true });
}

/// Startup stub when voice feature is disabled.
#[cfg(not(feature = "voice"))]
pub fn startup_voice_pipeline(mut commands: Commands) {
    log::info!("Voice pipeline disabled (compiled without 'voice' feature)");
    commands.insert_resource(VoiceState { enabled: false });
}

/// The inference thread's main loop.
///
/// Accumulates audio while PTT is held, then transcribes on release.
#[cfg(feature = "voice")]
fn inference_loop(
    mut consumer: ringbuf::HeapCons<f32>,
    transcriber: crate::transcriber::WhisperTranscriber,
    tx: Sender<String>,
    is_listening: Arc<AtomicBool>,
) {
    let mut accumulator: Vec<f32> = Vec::with_capacity(48000); // up to 3s at 16kHz
    let mut was_listening = false;
    // Minimum samples for transcription (0.3 seconds) — shorter clips are noise
    let min_samples: usize = 4800;
    // Maximum samples (5 seconds) — truncate to avoid long Whisper runs
    let max_samples: usize = 80000;

    loop {
        let currently_listening = is_listening.load(Ordering::Relaxed);

        if currently_listening {
            if !was_listening {
                // PTT just pressed → clear accumulator
                accumulator.clear();
                log::debug!("PTT pressed — accumulating audio");
            }

            // Read all available samples from ring buffer into accumulator
            let mut count = 0;
            while let Some(sample) = consumer.try_pop() {
                if accumulator.len() < max_samples {
                    accumulator.push(sample);
                }
                count += 1;
            }

            if count > 0 {
                log::trace!("Accumulated {count} samples (total: {})", accumulator.len());
            }

            was_listening = true;
            std::thread::sleep(std::time::Duration::from_millis(10));
        } else if was_listening {
            // PTT just released → transcribe accumulated audio
            was_listening = false;

            // Drain any remaining samples that arrived between last read and release
            while let Some(sample) = consumer.try_pop() {
                if accumulator.len() < max_samples {
                    accumulator.push(sample);
                }
            }

            if accumulator.len() >= min_samples {
                let duration_ms = accumulator.len() * 1000 / 16000;
                log::info!(
                    "PTT released — transcribing {duration_ms}ms of audio ({} samples)",
                    accumulator.len()
                );

                match transcriber.transcribe(&accumulator) {
                    Ok(text) => {
                        if !text.is_empty() {
                            log::info!("Whisper transcription: \"{text}\"");
                            let _ = tx.send(text);
                        } else {
                            log::debug!("Whisper returned empty transcription — silence?");
                        }
                    }
                    Err(e) => {
                        log::error!("Whisper transcription error: {e}");
                    }
                }
            } else {
                log::debug!(
                    "PTT released — too short ({} samples < {min_samples}), skipping",
                    accumulator.len()
                );
            }

            accumulator.clear();
        } else {
            // Not listening — drain ring buffer to keep it fresh
            while consumer.try_pop().is_some() {}
            std::thread::sleep(std::time::Duration::from_millis(20));
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
