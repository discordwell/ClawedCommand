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
    traits::{Producer, Split},
    HeapRb,
};
#[cfg(feature = "voice")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Capacity of the ring buffer in samples (2 seconds at 16kHz).
#[cfg(feature = "voice")]
const RING_BUFFER_CAPACITY: usize = 32000;

/// Start capturing audio from the default input device.
///
/// Returns a consumer for reading samples and an `is_active` flag.
/// The cpal Stream is kept alive by a dedicated thread (avoiding Send/Sync issues).
#[cfg(feature = "voice")]
pub fn start_capture() -> Result<
    (ringbuf::HeapCons<f32>, Arc<AtomicBool>),
    Box<dyn std::error::Error>,
> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("no input device available")?;

    log::info!("Audio input device: {:?}", device.name());

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (mut producer, consumer) = rb.split();

    let is_active = Arc::new(AtomicBool::new(true));
    let is_active_clone = is_active.clone();

    // Keep the stream alive on a dedicated thread (cpal::Stream is !Send on some platforms)
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if !is_active_clone.load(Ordering::Relaxed) {
                return;
            }
            for &sample in data {
                let _ = producer.try_push(sample);
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
