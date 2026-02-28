#!/usr/bin/env python3
"""PyTorch Dataset for voice command keyword spotting."""

import wave
import struct
from pathlib import Path

import numpy as np
import torch
from torch.utils.data import Dataset
import yaml

from augment import create_augmentation_pipeline, augment_audio, spec_augment


def load_config(config_path="config.yaml"):
    with open(Path(__file__).parent / config_path) as f:
        return yaml.safe_load(f)


def load_labels(config_path="config.yaml"):
    """Load ordered label list from config."""
    cfg = load_config(config_path)
    vocab = cfg["vocabulary"]
    labels = []
    for category in ["commands", "directions", "meta", "units", "buildings", "special"]:
        labels.extend(vocab[category])
    return labels


def load_wav(path, target_sr=16000, target_samples=16000):
    """Load WAV file as float32 numpy array, pad/trim to target length."""
    with wave.open(str(path), "rb") as wf:
        assert wf.getframerate() == target_sr, (
            f"Expected {target_sr}Hz, got {wf.getframerate()}Hz: {path}"
        )
        n_frames = wf.getnframes()
        raw = wf.readframes(n_frames)
        samples = np.array(
            struct.unpack(f"<{n_frames}h", raw), dtype=np.float32
        )

    # Normalize to [-1, 1]
    samples /= 32768.0

    # Pad or trim
    if len(samples) > target_samples:
        start = (len(samples) - target_samples) // 2
        samples = samples[start : start + target_samples]
    elif len(samples) < target_samples:
        pad_total = target_samples - len(samples)
        pad_left = pad_total // 2
        pad_right = pad_total - pad_left
        samples = np.pad(samples, (pad_left, pad_right))

    return samples


def compute_mel_spectrogram(audio, sr=16000, n_fft=512, hop_length=320,
                            n_mels=40, fmin=60, fmax=7800, num_frames=49):
    """Compute mel spectrogram from audio waveform.

    Uses numpy-only implementation to match the Rust pipeline exactly.

    Args:
        audio: float32 numpy array, shape [n_samples]
    Returns:
        mel spectrogram, shape [n_mels, num_frames]
    """
    # STFT
    # Pad audio to ensure we get enough frames
    pad_length = n_fft // 2
    audio_padded = np.pad(audio, (pad_length, pad_length), mode="reflect")

    # Window
    window = np.hanning(n_fft + 1)[:-1].astype(np.float32)

    # Compute frames
    n_samples = len(audio_padded)
    n_stft_frames = 1 + (n_samples - n_fft) // hop_length

    # Pre-allocate
    power_spec = np.zeros((n_fft // 2 + 1, n_stft_frames), dtype=np.float32)

    for i in range(n_stft_frames):
        start = i * hop_length
        frame = audio_padded[start : start + n_fft] * window
        spectrum = np.fft.rfft(frame)
        power_spec[:, i] = np.abs(spectrum) ** 2

    # Mel filter bank
    mel_filters = _mel_filter_bank(sr, n_fft, n_mels, fmin, fmax)

    # Apply mel filters
    mel = mel_filters @ power_spec  # [n_mels, n_stft_frames]

    # Log mel (add small epsilon for numerical stability)
    mel = np.log(mel + 1e-9)

    # Trim or pad to exact frame count
    if mel.shape[1] > num_frames:
        mel = mel[:, :num_frames]
    elif mel.shape[1] < num_frames:
        # Pad with log(eps) to represent silence (matches Rust pipeline)
        mel = np.pad(mel, ((0, 0), (0, num_frames - mel.shape[1])),
                     constant_values=np.log(1e-9))

    return mel


def _mel_filter_bank(sr, n_fft, n_mels, fmin, fmax):
    """Create mel-scale triangular filter bank."""
    # Mel scale conversion
    def hz_to_mel(hz):
        return 2595.0 * np.log10(1.0 + hz / 700.0)

    def mel_to_hz(mel):
        return 700.0 * (10.0 ** (mel / 2595.0) - 1.0)

    mel_min = hz_to_mel(fmin)
    mel_max = hz_to_mel(fmax)
    mel_points = np.linspace(mel_min, mel_max, n_mels + 2)
    hz_points = mel_to_hz(mel_points)

    # FFT bin indices
    bin_points = np.floor((n_fft + 1) * hz_points / sr).astype(int)

    filters = np.zeros((n_mels, n_fft // 2 + 1), dtype=np.float32)

    for i in range(n_mels):
        left = bin_points[i]
        center = bin_points[i + 1]
        right = bin_points[i + 2]

        # Rising slope
        for j in range(left, center):
            if center > left:
                filters[i, j] = (j - left) / (center - left)

        # Falling slope
        for j in range(center, right):
            if right > center:
                filters[i, j] = (right - j) / (right - center)

    return filters


class VoiceCommandDataset(Dataset):
    """Dataset for voice command keyword spotting.

    Expects directory structure:
        data_dir/
            attack/
                attack_Daniel_130wpm.wav
                ...
            retreat/
                ...
            _silence/   (optional, for silence class)
    """

    def __init__(self, data_dir, labels=None, config_path="config.yaml",
                 augment=True, split="train"):
        self.cfg = load_config(config_path)
        self.data_dir = Path(data_dir)
        self.labels = labels or load_labels(config_path)
        self.label_to_idx = {l: i for i, l in enumerate(self.labels)}
        self.augment = augment

        audio_cfg = self.cfg["audio"]
        self.sr = audio_cfg["sample_rate"]
        self.target_samples = int(self.sr * audio_cfg["duration_sec"])
        self.n_fft = audio_cfg["n_fft"]
        self.hop_length = audio_cfg["hop_length"]
        self.n_mels = audio_cfg["n_mels"]
        self.fmin = audio_cfg["fmin"]
        self.fmax = audio_cfg["fmax"]
        self.num_frames = audio_cfg["num_frames"]

        # Create augmentation pipeline
        self.aug_pipeline = create_augmentation_pipeline(self.cfg) if augment else None

        # Scan for samples
        self.samples = []
        for word_dir in sorted(self.data_dir.iterdir()):
            if not word_dir.is_dir():
                continue
            word = word_dir.name.lstrip("_")  # Handle _silence
            if word not in self.label_to_idx:
                continue
            label_idx = self.label_to_idx[word]
            for wav_file in sorted(word_dir.glob("*.wav")):
                self.samples.append((wav_file, label_idx))

        print(f"[{split}] Loaded {len(self.samples)} samples "
              f"across {len(set(s[1] for s in self.samples))} classes")

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, idx):
        wav_path, label_idx = self.samples[idx]

        # Load audio
        audio = load_wav(wav_path, self.sr, self.target_samples)

        # Augment audio (waveform-level)
        if self.augment and self.aug_pipeline is not None:
            audio = augment_audio(audio, self.aug_pipeline, self.sr)
        elif self.augment:
            from augment import augment_basic
            audio = augment_basic(audio, self.sr)

        # Compute mel spectrogram
        mel = compute_mel_spectrogram(
            audio, self.sr, self.n_fft, self.hop_length,
            self.n_mels, self.fmin, self.fmax, self.num_frames,
        )

        # SpecAugment (on mel, training only)
        if self.augment:
            mel = spec_augment(mel)

        # To tensor: [1, n_mels, num_frames]
        mel_tensor = torch.from_numpy(mel).unsqueeze(0)

        return mel_tensor, label_idx
