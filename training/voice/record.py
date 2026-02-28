#!/usr/bin/env python3
"""Record real voice samples for training data.

Usage:
    python record.py --word attack --count 20
    python record.py --word attack --count 20 --output-dir data/real
"""

import argparse
import wave
import struct
import time
from pathlib import Path

try:
    import sounddevice as sd
    HAS_SD = True
except ImportError:
    HAS_SD = False


def record_sample(duration=1.5, sr=16000):
    """Record a single audio sample from the microphone."""
    if not HAS_SD:
        raise RuntimeError("sounddevice not installed: pip install sounddevice")
    audio = sd.rec(int(duration * sr), samplerate=sr, channels=1, dtype="float32")
    sd.wait()
    return audio.flatten()


def trim_to_duration(audio, sr=16000, target_duration=1.0):
    """Center-crop or pad audio to target duration."""
    target_samples = int(sr * target_duration)
    if len(audio) > target_samples:
        start = (len(audio) - target_samples) // 2
        return audio[start : start + target_samples]
    elif len(audio) < target_samples:
        import numpy as np
        pad_total = target_samples - len(audio)
        pad_left = pad_total // 2
        pad_right = pad_total - pad_left
        return np.pad(audio, (pad_left, pad_right))
    return audio


def save_wav(audio, path, sr=16000):
    """Save float32 audio as 16-bit WAV."""
    samples = (audio * 32767).astype("int16")
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        wf.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def main():
    parser = argparse.ArgumentParser(description="Record voice samples for training")
    parser.add_argument("--word", required=True, help="Word to record")
    parser.add_argument("--count", type=int, default=20, help="Number of recordings")
    parser.add_argument("--output-dir", type=Path, default=Path("data/real"),
                        help="Output directory")
    parser.add_argument("--duration", type=float, default=1.5,
                        help="Recording duration in seconds (trimmed to 1.0s)")
    args = parser.parse_args()

    output_dir = args.output_dir / args.word
    output_dir.mkdir(parents=True, exist_ok=True)

    # Find starting index
    existing = list(output_dir.glob(f"{args.word}_real_*.wav"))
    start_idx = len(existing)

    print(f"Recording '{args.word}' -- {args.count} samples")
    print(f"Output: {output_dir}")
    print(f"Press Enter to record each sample, 'q' to quit\n")

    for i in range(args.count):
        idx = start_idx + i
        response = input(f"  [{i+1}/{args.count}] Press Enter to say '{args.word}'... ")
        if response.strip().lower() == "q":
            print("Stopped early.")
            break

        print("    Recording...", end="", flush=True)
        audio = record_sample(args.duration)
        audio = trim_to_duration(audio)

        filename = f"{args.word}_real_{idx:04d}.wav"
        save_wav(audio, output_dir / filename)
        print(f" saved: {filename}")

    print(f"\nDone! {output_dir}")


if __name__ == "__main__":
    main()
