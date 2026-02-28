#!/usr/bin/env python3
"""Generate TTS training data using macOS 'say' command."""

import argparse
import subprocess
import wave
import struct
import yaml
from pathlib import Path


def load_config(config_path="config.yaml"):
    with open(Path(__file__).parent / config_path) as f:
        return yaml.safe_load(f)


def get_all_words(cfg):
    """Get flat list of all vocabulary words (excluding special)."""
    vocab = cfg["vocabulary"]
    words = []
    for category in ["commands", "directions", "meta", "units", "buildings"]:
        words.extend(vocab[category])
    return words


def generate_aiff(word, voice, rate, output_path):
    """Generate speech using macOS say command."""
    subprocess.run(
        ["say", "-v", voice, "-r", str(rate), "-o", str(output_path), word],
        check=True,
        capture_output=True,
    )


def aiff_to_wav_16k(aiff_path, wav_path, target_sr=16000, target_duration=1.0):
    """Convert AIFF to 16kHz mono WAV, pad/trim to target duration."""
    # Use afconvert (macOS) to convert to WAV 16kHz mono
    subprocess.run(
        [
            "afconvert",
            "-f", "WAVE",
            "-d", "LEI16@16000",
            "-c", "1",
            str(aiff_path),
            str(wav_path),
        ],
        check=True,
        capture_output=True,
    )

    # Pad or trim to exact duration
    target_samples = int(target_sr * target_duration)
    with wave.open(str(wav_path), "rb") as wf:
        n_frames = wf.getnframes()
        raw = wf.readframes(n_frames)
        samples = list(struct.unpack(f"<{n_frames}h", raw))

    if len(samples) > target_samples:
        # Center-crop
        start = (len(samples) - target_samples) // 2
        samples = samples[start : start + target_samples]
    elif len(samples) < target_samples:
        # Pad with silence (center the audio)
        pad_total = target_samples - len(samples)
        pad_left = pad_total // 2
        pad_right = pad_total - pad_left
        samples = [0] * pad_left + samples + [0] * pad_right

    with wave.open(str(wav_path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(target_sr)
        wf.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def main():
    parser = argparse.ArgumentParser(description="Generate TTS training data")
    parser.add_argument("--output-dir", type=Path, default=Path("data/tts"),
                        help="Output directory for WAV files")
    parser.add_argument("--config", type=str, default="config.yaml")
    parser.add_argument("--word", type=str, default=None,
                        help="Generate for a single word only")
    args = parser.parse_args()

    cfg = load_config(args.config)
    voices = cfg["tts"]["voices"]
    speeds = cfg["tts"]["speed_variations"]
    words = [args.word] if args.word else get_all_words(cfg)

    args.output_dir.mkdir(parents=True, exist_ok=True)
    tmp_dir = args.output_dir / "_tmp"
    tmp_dir.mkdir(exist_ok=True)

    total = len(words) * len(voices) * len(speeds)
    generated = 0

    for word in words:
        word_dir = args.output_dir / word
        word_dir.mkdir(exist_ok=True)

        for voice in voices:
            for speed in speeds:
                filename = f"{word}_{voice}_{speed}wpm.wav"
                wav_path = word_dir / filename

                if wav_path.exists():
                    generated += 1
                    continue

                aiff_path = tmp_dir / f"tmp_{word}_{voice}_{speed}.aiff"
                try:
                    generate_aiff(word, voice, speed, aiff_path)
                    aiff_to_wav_16k(aiff_path, wav_path)
                    generated += 1
                    print(f"[{generated}/{total}] {filename}")
                except subprocess.CalledProcessError as e:
                    print(f"SKIP {filename}: {e}")
                finally:
                    aiff_path.unlink(missing_ok=True)

    # Cleanup
    tmp_dir.rmdir()
    print(f"\nGenerated {generated}/{total} samples in {args.output_dir}")


if __name__ == "__main__":
    main()
