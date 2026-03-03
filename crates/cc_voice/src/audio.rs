/// Microphone audio capture using `cpal`.
///
/// Runs on an OS audio thread, writing 16kHz mono f32 samples into a lock-free
/// ring buffer. The inference thread reads from the consumer side.
///
/// The cpal `Stream` is `!Send + !Sync`, so we keep it alive on the thread
/// that creates it rather than storing it in a Bevy resource.
#[cfg(feature = "voice")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "voice")]
use ringbuf::{
    HeapRb,
    traits::{Producer, Split},
};
#[cfg(feature = "voice")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Capacity of the ring buffer in samples (2 seconds at 16kHz).
#[cfg(feature = "voice")]
const RING_BUFFER_CAPACITY: usize = 32000;

/// Start capturing audio from the default input device.
///
/// Returns a consumer for reading samples and an `is_active` flag.
/// The cpal Stream is kept alive by a dedicated thread (avoiding Send/Sync issues).
#[cfg(feature = "voice")]
pub fn start_capture()
-> Result<(ringbuf::HeapCons<f32>, Arc<AtomicBool>), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no input device available")?;

    log::info!("Audio input device: {:?}", device.name());

    // Use the device's default config, then resample to 16kHz mono.
    // Most devices (e.g. MacBook mic) only support 48kHz natively.
    let default_config = device.default_input_config()?;
    let native_rate = default_config.sample_rate().0;
    let native_channels = default_config.channels() as usize;

    let config = cpal::StreamConfig {
        channels: native_channels as u16,
        sample_rate: cpal::SampleRate(native_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    log::info!(
        "Audio capture: {}Hz {}ch → resample to 16kHz mono",
        native_rate,
        native_channels,
    );

    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (mut producer, consumer) = rb.split();

    let is_active = Arc::new(AtomicBool::new(true));
    let is_active_clone = is_active.clone();

    // Resampling state: average consecutive native-rate mono samples to produce
    // 16kHz output. This acts as a low-pass anti-aliasing filter (box filter)
    // rather than simple decimation which creates aliasing artifacts.
    let ratio = native_rate as f64 / 16000.0;
    let mut accum = 0.0f32;
    let mut accum_count = 0u32;
    let mut frac_pos = 0.0f64;

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !is_active_clone.load(Ordering::Relaxed) {
                return;
            }
            // Downmix to mono frame-by-frame, then average-and-decimate to 16kHz.
            // For 48kHz→16kHz (ratio=3), this averages every 3 mono samples into 1,
            // which low-pass filters the signal before decimation.
            let mut i = 0;
            while i + native_channels <= data.len() {
                // Downmix this frame to mono
                let mut sum = 0.0f32;
                for ch in 0..native_channels {
                    sum += data[i + ch];
                }
                let mono = sum / native_channels as f32;
                i += native_channels;

                // Accumulate for averaging
                accum += mono;
                accum_count += 1;
                frac_pos += 1.0;

                // Emit a 16kHz sample when we've accumulated enough native samples
                if frac_pos >= ratio {
                    let sample = accum / accum_count as f32;
                    let _ = producer.try_push(sample);
                    accum = 0.0;
                    accum_count = 0;
                    frac_pos -= ratio;
                }
            }
        },
        move |err| {
            log::error!("Audio capture error: {err}");
        },
        None,
    )?;

    stream.play()?;

    // Keep stream alive by leaking it — cpal::Stream is !Send so we can't
    // move it to another thread. Leaking is fine since the stream persists
    // for the lifetime of the process, and the audio callback runs on
    // cpal's internal OS thread regardless.
    Box::leak(Box::new(stream));

    Ok((consumer, is_active))
}
