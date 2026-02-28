#!/usr/bin/env python3
"""Audio augmentation for voice command training data."""

import numpy as np

try:
    from audiomentations import (
        Compose,
        AddGaussianNoise,
        PitchShift,
        TimeStretch,
        Gain,
        Shift,
    )
    HAS_AUDIOMENTATIONS = True
except ImportError:
    HAS_AUDIOMENTATIONS = False


def create_augmentation_pipeline(cfg):
    """Create audiomentations pipeline from config."""
    if not HAS_AUDIOMENTATIONS:
        print("WARNING: audiomentations not installed, using basic augmentation")
        return None

    aug_cfg = cfg["augmentation"]
    sr = cfg["audio"]["sample_rate"]

    return Compose([
        AddGaussianNoise(
            min_amplitude=0.001,
            max_amplitude=0.015,
            p=0.5,
        ),
        PitchShift(
            min_semitones=-aug_cfg["pitch_shift_semitones"],
            max_semitones=aug_cfg["pitch_shift_semitones"],
            p=0.5,
        ),
        TimeStretch(
            min_rate=aug_cfg["speed_min"],
            max_rate=aug_cfg["speed_max"],
            p=0.3,
        ),
        Gain(
            min_gain_db=-6,
            max_gain_db=6,
            p=0.3,
        ),
        Shift(
            min_shift=-0.1,
            max_shift=0.1,
            p=0.3,
        ),
    ])


def spec_augment(mel, num_freq_masks=2, freq_mask_width=4,
                 num_time_masks=2, time_mask_width=8):
    """Apply SpecAugment to a mel spectrogram.

    Args:
        mel: numpy array of shape [n_mels, n_frames]
    Returns:
        Augmented mel spectrogram
    """
    mel = mel.copy()
    n_mels, n_frames = mel.shape

    # Frequency masking
    for _ in range(num_freq_masks):
        f = np.random.randint(0, freq_mask_width + 1)
        f0 = np.random.randint(0, max(1, n_mels - f))
        mel[f0:f0 + f, :] = 0.0

    # Time masking
    for _ in range(num_time_masks):
        t = np.random.randint(0, time_mask_width + 1)
        t0 = np.random.randint(0, max(1, n_frames - t))
        mel[:, t0:t0 + t] = 0.0

    return mel


def augment_basic(audio, sr=16000):
    """Basic augmentation without audiomentations dependency.

    Adds Gaussian noise and random gain.
    """
    # Random gain
    gain = np.random.uniform(0.5, 1.5)
    audio = audio * gain

    # Gaussian noise
    if np.random.random() < 0.5:
        noise_level = np.random.uniform(0.001, 0.01)
        noise = np.random.randn(len(audio)).astype(np.float32) * noise_level
        audio = audio + noise

    # Random shift
    if np.random.random() < 0.3:
        shift = int(np.random.uniform(-0.1, 0.1) * len(audio))
        audio = np.roll(audio, shift)

    return np.clip(audio, -1.0, 1.0)


def augment_audio(audio, pipeline, sr=16000):
    """Apply augmentation pipeline to audio.

    Args:
        audio: numpy float32 array, shape [n_samples]
        pipeline: audiomentations Compose or None
        sr: sample rate
    Returns:
        Augmented audio array
    """
    if pipeline is not None:
        return pipeline(samples=audio, sample_rate=sr)
    return augment_basic(audio, sr)
