#!/usr/bin/env python3
"""GPU-accelerated TTS generation for game-specific keywords.

Uses Piper TTS (fast, many voices) and Bark (natural, diverse) to generate
diverse synthetic training data on NVIDIA GPUs. Replaces macOS `say` for
headless GPU training environments.

Usage:
    python generate_tts_gpu.py --config config.yaml --output-dir data/tts_gpu
    python generate_tts_gpu.py --config config.yaml --output-dir data/tts_gpu --engine piper
    python generate_tts_gpu.py --config config.yaml --output-dir data/tts_gpu --engine bark
"""

import argparse
import os
import struct
import wave
from pathlib import Path

import numpy as np
import yaml

# Piper voices — fast English TTS with varied speakers
PIPER_VOICES = [
    "en_US-lessac-medium",
    "en_US-libritts-high",
    "en_US-libritts_r-medium",
    "en_US-ryan-medium",
    "en_US-amy-medium",
    "en_US-arctic-medium",
    "en_US-hfc_female-medium",
    "en_US-hfc_male-medium",
    "en_US-joe-medium",
    "en_US-kusal-medium",
    "en_GB-alan-medium",
    "en_GB-alba-medium",
    "en_GB-aru-medium",
    "en_GB-cori-medium",
    "en_GB-jenny_dioco-medium",
    "en_GB-northern_english_male-medium",
    "en_GB-semaine-medium",
    "en_GB-southern_english_female-medium",
    "en_GB-vctk-medium",
    "en_AU-karen-medium",
]

# Bark speaker prompts — diverse accents and styles
BARK_SPEAKERS = [
    "v2/en_speaker_0",
    "v2/en_speaker_1",
    "v2/en_speaker_2",
    "v2/en_speaker_3",
    "v2/en_speaker_4",
    "v2/en_speaker_5",
    "v2/en_speaker_6",
    "v2/en_speaker_7",
    "v2/en_speaker_8",
    "v2/en_speaker_9",
]

PIPER_SPEED_VARIATIONS = [0.85, 1.0, 1.15]

TARGET_SR = 16000
TARGET_SAMPLES = 16000  # 1 second


def load_vocabulary(config_path):
    """Load the keyword vocabulary from config.yaml."""
    with open(config_path) as f:
        cfg = yaml.safe_load(f)

    words = []
    vocab = cfg["vocabulary"]
    for category in vocab.values():
        if isinstance(category, list):
            for word in category:
                words.append(str(word))
    return words


def save_wav(audio, path, sr=TARGET_SR):
    """Save float32 audio as 16-bit PCM WAV."""
    # Pad or center-crop to exactly 1 second
    if len(audio) < TARGET_SAMPLES:
        pad_total = TARGET_SAMPLES - len(audio)
        pad_left = pad_total // 2
        pad_right = pad_total - pad_left
        audio = np.pad(audio, (pad_left, pad_right))
    elif len(audio) > TARGET_SAMPLES:
        start = (len(audio) - TARGET_SAMPLES) // 2
        audio = audio[start : start + TARGET_SAMPLES]

    # Normalize and convert to int16
    audio = np.clip(audio, -1.0, 1.0)
    audio_int16 = (audio * 32767).astype(np.int16)

    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        wf.writeframes(audio_int16.tobytes())


def generate_piper(words, output_dir, voices=None, speeds=None):
    """Generate TTS samples using Piper TTS.

    Piper is fast and CPU-friendly but benefits from batch processing.
    """
    try:
        from piper import PiperVoice
        from piper.download import ensure_voice_exists, get_voices
    except ImportError:
        print("Piper TTS not installed. Install with: pip install piper-tts")
        print("Falling back to Piper CLI if available...")
        return _generate_piper_cli(words, output_dir, voices, speeds)

    voices = voices or PIPER_VOICES
    speeds = speeds or PIPER_SPEED_VARIATIONS
    output_dir = Path(output_dir)

    total = len(words) * len(voices) * len(speeds)
    generated = 0
    skipped = 0

    for voice_name in voices:
        try:
            voice = PiperVoice.load(voice_name)
        except Exception as e:
            print(f"  Skipping voice {voice_name}: {e}")
            continue

        for word in words:
            for speed in speeds:
                out_path = output_dir / word / f"{word}_piper_{voice_name}_{speed:.2f}x.wav"
                if out_path.exists():
                    skipped += 1
                    continue

                try:
                    # Piper synthesizes to raw audio
                    audio_bytes = b""
                    for chunk in voice.synthesize_stream_raw(
                        word, length_scale=1.0 / speed
                    ):
                        audio_bytes += chunk

                    # Convert to float32
                    audio = np.frombuffer(audio_bytes, dtype=np.int16).astype(np.float32) / 32768.0

                    # Resample if needed (Piper often outputs at 22050)
                    if voice.config.sample_rate != TARGET_SR:
                        audio = _resample(audio, voice.config.sample_rate, TARGET_SR)

                    save_wav(audio, out_path)
                    generated += 1
                except Exception as e:
                    print(f"  Error generating {word}/{voice_name}/{speed}: {e}")

        if generated % 100 == 0 and generated > 0:
            print(f"  Piper: {generated}/{total} generated, {skipped} skipped")

    print(f"Piper complete: {generated} generated, {skipped} skipped")


def _generate_piper_cli(words, output_dir, voices=None, speeds=None):
    """Fallback: use Piper CLI via subprocess."""
    import subprocess
    import shutil

    if not shutil.which("piper"):
        print("ERROR: Piper CLI not found. Skipping Piper TTS generation.")
        return

    voices = voices or PIPER_VOICES[:5]  # Use fewer voices for CLI
    speeds = speeds or PIPER_SPEED_VARIATIONS
    output_dir = Path(output_dir)

    generated = 0
    for voice_name in voices:
        for word in words:
            for speed in speeds:
                out_path = output_dir / word / f"{word}_piper_{voice_name}_{speed:.2f}x.wav"
                if out_path.exists():
                    continue

                out_path.parent.mkdir(parents=True, exist_ok=True)
                try:
                    result = subprocess.run(
                        [
                            "piper",
                            "--model", voice_name,
                            "--length-scale", str(1.0 / speed),
                            "--output_file", str(out_path),
                        ],
                        input=word,
                        capture_output=True,
                        text=True,
                        timeout=30,
                    )
                    if result.returncode == 0:
                        # Ensure correct format
                        _normalize_wav(out_path)
                        generated += 1
                except Exception as e:
                    print(f"  Piper CLI error for {word}: {e}")

    print(f"Piper CLI: {generated} generated")


def generate_bark(words, output_dir, speakers=None, samples_per_speaker=1):
    """Generate TTS samples using Bark (Suno).

    Bark produces very natural speech with variation but is GPU-intensive.
    """
    try:
        from bark import SAMPLE_RATE, generate_audio, preload_models
    except ImportError:
        print("Bark not installed. Install with: pip install bark")
        return

    speakers = speakers or BARK_SPEAKERS[:5]
    output_dir = Path(output_dir)

    print("Loading Bark models (this may take a moment)...")
    preload_models()

    total = len(words) * len(speakers) * samples_per_speaker
    generated = 0
    skipped = 0

    for speaker in speakers:
        for word in words:
            for idx in range(samples_per_speaker):
                out_path = output_dir / word / f"{word}_bark_{speaker.replace('/', '_')}_{idx}.wav"
                if out_path.exists():
                    skipped += 1
                    continue

                try:
                    audio = generate_audio(
                        word,
                        history_prompt=speaker,
                        text_temp=0.7,
                        waveform_temp=0.7,
                    )

                    # Bark outputs at 24kHz — resample to 16kHz
                    if SAMPLE_RATE != TARGET_SR:
                        audio = _resample(audio, SAMPLE_RATE, TARGET_SR)

                    save_wav(audio, out_path)
                    generated += 1
                except Exception as e:
                    print(f"  Bark error for {word}/{speaker}/{idx}: {e}")

            if generated % 50 == 0 and generated > 0:
                print(f"  Bark: {generated}/{total} generated, {skipped} skipped")

    print(f"Bark complete: {generated} generated, {skipped} skipped")


def _resample(audio, orig_sr, target_sr):
    """Simple linear interpolation resampling."""
    if orig_sr == target_sr:
        return audio
    ratio = target_sr / orig_sr
    target_len = int(len(audio) * ratio)
    indices = np.linspace(0, len(audio) - 1, target_len)
    resampled = np.interp(indices, np.arange(len(audio)), audio)
    return resampled.astype(np.float32)


def _normalize_wav(path):
    """Ensure a WAV file is 16kHz mono 16-bit. Re-save if needed."""
    try:
        with wave.open(str(path), "rb") as wf:
            sr = wf.getframerate()
            channels = wf.getnchannels()
            frames = wf.readframes(wf.getnframes())

        audio = np.frombuffer(frames, dtype=np.int16).astype(np.float32) / 32768.0

        if channels > 1:
            audio = audio.reshape(-1, channels).mean(axis=1)

        if sr != TARGET_SR:
            audio = _resample(audio, sr, TARGET_SR)

        save_wav(audio, Path(path))
    except Exception:
        pass


def main():
    parser = argparse.ArgumentParser(description="GPU TTS generation for game keywords")
    parser.add_argument("--config", type=str, default="config.yaml",
                        help="Config file with vocabulary")
    parser.add_argument("--output-dir", type=Path, required=True,
                        help="Output directory for generated WAVs")
    parser.add_argument("--engine", type=str, choices=["piper", "bark", "all"],
                        default="all", help="Which TTS engine to use")
    parser.add_argument("--bark-samples", type=int, default=1,
                        help="Number of Bark samples per speaker per word")
    args = parser.parse_args()

    config_path = Path(__file__).parent / args.config
    words = load_vocabulary(config_path)
    print(f"Vocabulary: {len(words)} words")

    if args.engine in ("piper", "all"):
        print("\n=== Piper TTS Generation ===")
        generate_piper(words, args.output_dir)

    if args.engine in ("bark", "all"):
        print("\n=== Bark TTS Generation ===")
        generate_bark(words, args.output_dir, samples_per_speaker=args.bark_samples)

    # Count total generated files
    total = sum(1 for _ in args.output_dir.rglob("*.wav"))
    print(f"\nTotal WAV files: {total}")
    print(f"Output directory: {args.output_dir}")


if __name__ == "__main__":
    main()
