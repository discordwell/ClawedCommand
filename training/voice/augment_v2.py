#!/usr/bin/env python3
"""Extended audio augmentation for voice command training.

Adds game-specific augmentations on top of the base augment.py pipeline:
- Game noise mixing (combat, ambient, music at random SNR)
- Room impulse response simulation
- Codec degradation (simulate compressed audio artifacts)
"""

import io
import os
from pathlib import Path

import numpy as np

from augment import (
    create_augmentation_pipeline,
    augment_audio,
    spec_augment,
)

# Noise bank directory for game audio clips
NOISE_BANK_DIR = Path(__file__).parent / "noise_bank"


def load_noise_bank(noise_dir=None):
    """Load all WAV files from the noise bank directory.

    Returns list of float32 numpy arrays, each normalized to [-1, 1].
    """
    noise_dir = noise_dir or NOISE_BANK_DIR
    if not noise_dir.exists():
        return []

    noises = []
    for wav_path in sorted(noise_dir.glob("*.wav")):
        try:
            audio = _load_wav_simple(wav_path)
            if audio is not None and len(audio) > 0:
                noises.append(audio)
        except Exception:
            pass
    return noises


def _load_wav_simple(path):
    """Load a WAV file as float32 numpy array. Minimal dependency."""
    import wave

    with wave.open(str(path), "rb") as wf:
        if wf.getnchannels() > 1:
            return None  # Skip stereo
        sr = wf.getframerate()
        frames = wf.readframes(wf.getnframes())
        audio = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0
        return audio


def mix_game_noise(audio, noise_clips, snr_min=0.0, snr_max=20.0, p=0.5):
    """Mix a random game noise clip at a random SNR.

    Args:
        audio: float32 array [n_samples]
        noise_clips: list of float32 arrays (noise bank)
        snr_min: minimum signal-to-noise ratio in dB
        snr_max: maximum signal-to-noise ratio in dB
        p: probability of applying
    """
    if np.random.random() > p or not noise_clips:
        return audio

    noise = noise_clips[np.random.randint(len(noise_clips))]

    # Match length: loop or trim noise to match audio
    if len(noise) < len(audio):
        repeats = (len(audio) // len(noise)) + 1
        noise = np.tile(noise, repeats)
    # Random offset into noise
    max_start = len(noise) - len(audio)
    start = np.random.randint(0, max(1, max_start))
    noise = noise[start : start + len(audio)]

    # Compute SNR-scaled noise
    snr_db = np.random.uniform(snr_min, snr_max)
    signal_power = np.mean(audio ** 2) + 1e-10
    noise_power = np.mean(noise ** 2) + 1e-10
    scale = np.sqrt(signal_power / (noise_power * (10 ** (snr_db / 10))))

    mixed = audio + noise * scale
    return np.clip(mixed, -1.0, 1.0).astype(np.float32)


def simulate_room_impulse(audio, sr=16000, p=0.3):
    """Simulate room acoustics with a simple synthetic impulse response.

    Creates a basic exponentially decaying reverb tail. Not physically accurate
    but adds enough variation for training robustness.
    """
    if np.random.random() > p:
        return audio

    # Random room size (small to medium)
    rt60 = np.random.uniform(0.1, 0.5)  # Reverberation time in seconds
    decay_samples = int(rt60 * sr)

    # Exponential decay impulse response
    ir = np.random.randn(decay_samples).astype(np.float32)
    decay = np.exp(-3.0 * np.arange(decay_samples) / decay_samples)
    ir *= decay
    ir[0] = 1.0  # Direct sound

    # Normalize IR
    ir = ir / (np.sum(np.abs(ir)) + 1e-10)

    # Apply convolution
    reverbed = np.convolve(audio, ir, mode="full")[: len(audio)]

    # Mix dry/wet
    wet = np.random.uniform(0.1, 0.4)
    mixed = (1 - wet) * audio + wet * reverbed
    return np.clip(mixed, -1.0, 1.0).astype(np.float32)


def simulate_codec_degradation(audio, sr=16000, p=0.2):
    """Simulate audio compression artifacts.

    Applies quantization noise to approximate low-bitrate codec effects.
    """
    if np.random.random() > p:
        return audio

    # Random quantization level (fewer bits = more degradation)
    bits = np.random.randint(6, 14)  # 6-bit (heavy) to 14-bit (mild)
    levels = 2 ** bits
    quantized = np.round(audio * levels) / levels
    return quantized.astype(np.float32)


def create_augmentation_pipeline_v2(cfg, noise_clips=None):
    """Create the extended augmentation pipeline.

    Returns a callable that applies all augmentations (base + extended).

    Args:
        cfg: config dict from config.yaml
        noise_clips: optional pre-loaded noise bank (list of arrays)
    """
    base_pipeline = create_augmentation_pipeline(cfg)
    sr = cfg["audio"]["sample_rate"]

    if noise_clips is None:
        noise_clips = load_noise_bank()

    def augment(audio):
        # Stage 1: Base augmentations (noise, pitch, stretch, gain, shift)
        audio = augment_audio(audio, base_pipeline, sr)

        # Stage 2: Game noise mixing
        audio = mix_game_noise(audio, noise_clips, snr_min=0.0, snr_max=20.0, p=0.5)

        # Stage 3: Room impulse response
        audio = simulate_room_impulse(audio, sr, p=0.3)

        # Stage 4: Codec degradation
        audio = simulate_codec_degradation(audio, sr, p=0.2)

        return audio

    return augment


def augment_audio_v2(audio, pipeline_fn, sr=16000):
    """Apply extended augmentation pipeline to audio.

    Args:
        audio: numpy float32 array, shape [n_samples]
        pipeline_fn: callable from create_augmentation_pipeline_v2, or None
        sr: sample rate
    Returns:
        Augmented audio array
    """
    if pipeline_fn is not None:
        return pipeline_fn(audio)
    # Fallback: no augmentation
    return audio
