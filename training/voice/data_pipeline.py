#!/usr/bin/env python3
"""End-to-end data pipeline for voice keyword spotting training.

Orchestrates: download Speech Commands v2, generate GPU TTS, build unified dataset.

Usage:
    python data_pipeline.py --stage all --output-dir /workspace/data
    python data_pipeline.py --stage download --output-dir /workspace/data
    python data_pipeline.py --stage tts --output-dir /workspace/data
    python data_pipeline.py --stage unify --output-dir /workspace/data
"""

import argparse
import json
import os
import random
import shutil
from collections import defaultdict
from pathlib import Path

import numpy as np
import yaml


def load_config(config_path=None):
    if config_path is None:
        config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        return yaml.safe_load(f)


def load_vocabulary(cfg):
    """Flatten vocabulary categories into ordered list of words."""
    words = []
    for category in cfg["vocabulary"].values():
        if isinstance(category, list):
            for word in category:
                words.append(str(word))
    return words


# ---- Word mappings: Speech Commands v2 → game vocabulary ----

# Direct overlap: same word in both vocabularies
DIRECT_OVERLAP = {
    "stop": "stop",
    "yes": "yes",
    "no": "no",
    "one": "one",
    "two": "two",
    "three": "three",
    "tree": "tree",
}

# Semantic mapping: Speech Commands word → game vocabulary word
SEMANTIC_MAP = {
    "go": "move",
    "up": "north",
    "down": "south",
    "left": "west",
    "right": "east",
}

# All Speech Commands v2 classes
SPEECH_COMMANDS_CLASSES = [
    "backward", "bed", "bird", "cat", "dog", "down", "eight", "five",
    "follow", "forward", "four", "go", "happy", "house", "learn", "left",
    "marvin", "nine", "no", "off", "on", "one", "right", "seven",
    "sheila", "six", "stop", "three", "tree", "two", "up", "visual",
    "wow", "yes", "zero",
]


def stage_download(output_dir):
    """Download Google Speech Commands v2 dataset."""
    sc_dir = output_dir / "speech_commands_v2"

    if sc_dir.exists() and any(sc_dir.iterdir()):
        print(f"Speech Commands already exists at {sc_dir}, skipping download")
        return sc_dir

    print("Downloading Google Speech Commands v2...")
    try:
        import torchaudio
        torchaudio.datasets.SPEECHCOMMANDS(
            str(output_dir), url="speech_commands_v2", download=True
        )
        print("Download complete")
    except ImportError:
        print("torchaudio not available, trying manual download...")
        _download_speech_commands_manual(sc_dir)

    return sc_dir


def _download_speech_commands_manual(output_dir):
    """Manual download fallback using wget/curl."""
    import subprocess

    url = "http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz"
    tar_path = output_dir.parent / "speech_commands_v0.02.tar.gz"

    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Downloading {url}...")
    subprocess.run(["wget", "-q", "-O", str(tar_path), url], check=True)

    print("Extracting...")
    subprocess.run(["tar", "xzf", str(tar_path), "-C", str(output_dir)], check=True)
    tar_path.unlink()
    print("Done")


def stage_tts(output_dir, config_path=None):
    """Generate GPU TTS data for game-specific keywords."""
    tts_dir = output_dir / "tts_gpu"

    print("Running GPU TTS generation...")
    from generate_tts_gpu import load_vocabulary as load_vocab_tts, generate_piper, generate_bark

    cfg_path = config_path or (Path(__file__).parent / "config.yaml")
    words = load_vocab_tts(cfg_path)

    # Only generate TTS for words NOT in Speech Commands
    sc_words = set(DIRECT_OVERLAP.keys()) | set(SEMANTIC_MAP.keys())
    # Also include "follow" which is in both
    sc_words.add("follow")
    tts_words = [w for w in words if w not in sc_words]

    # Also generate TTS for ALL words (including overlap) to have synthetic diversity
    print(f"Generating TTS for {len(words)} words ({len(tts_words)} game-only + {len(words) - len(tts_words)} overlap)")

    print("\n--- Piper TTS ---")
    generate_piper(words, tts_dir)

    print("\n--- Bark TTS ---")
    generate_bark(words, tts_dir, samples_per_speaker=1)

    return tts_dir


def stage_unify(output_dir, config_path=None):
    """Build unified dataset from all sources with train/val/test splits.

    Sources:
    1. Google Speech Commands v2 (mapped words only)
    2. macOS TTS (from generate_tts.py, if present)
    3. GPU TTS (from generate_tts_gpu.py)
    4. Real recordings (from record.py, if present)
    """
    cfg = load_config(config_path)
    game_words = load_vocabulary(cfg)
    word_to_idx = {w: i for i, w in enumerate(game_words)}

    unified_dir = output_dir / "unified"
    unified_dir.mkdir(parents=True, exist_ok=True)

    # Collect all samples: list of (source_path, game_label)
    samples_by_label = defaultdict(list)

    # --- Source 1: Speech Commands v2 (mapped words) ---
    sc_dir = output_dir / "speech_commands_v2"
    if sc_dir.exists():
        all_maps = {**DIRECT_OVERLAP, **SEMANTIC_MAP}
        # Also check for "follow" which exists in both
        if "follow" in game_words:
            all_maps["follow"] = "follow"

        for sc_word, game_word in all_maps.items():
            if game_word not in word_to_idx:
                continue
            word_dir = sc_dir / sc_word
            if not word_dir.exists():
                # torchaudio saves under SpeechCommands/speech_commands_v0.02/
                alt_dir = sc_dir / "SpeechCommands" / "speech_commands_v0.02" / sc_word
                if alt_dir.exists():
                    word_dir = alt_dir
                else:
                    continue

            wavs = list(word_dir.glob("*.wav"))
            for wav in wavs:
                samples_by_label[game_word].append(wav)
            if wavs:
                print(f"  Speech Commands: {sc_word} → {game_word}: {len(wavs)} samples")

    # --- Source 2: macOS TTS (existing) ---
    mac_tts_dir = Path(__file__).parent / "data" / "tts"
    if mac_tts_dir.exists():
        for word in game_words:
            word_dir = mac_tts_dir / word
            if word_dir.exists():
                wavs = list(word_dir.glob("*.wav"))
                for wav in wavs:
                    samples_by_label[word].append(wav)

    # --- Source 3: GPU TTS ---
    gpu_tts_dir = output_dir / "tts_gpu"
    if gpu_tts_dir.exists():
        for word in game_words:
            word_dir = gpu_tts_dir / word
            if word_dir.exists():
                wavs = list(word_dir.glob("*.wav"))
                for wav in wavs:
                    samples_by_label[word].append(wav)

    # --- Source 4: Real recordings ---
    real_dir = Path(__file__).parent / "data" / "real"
    if real_dir.exists():
        for word in game_words:
            word_dir = real_dir / word
            if word_dir.exists():
                wavs = list(word_dir.glob("*.wav"))
                for wav in wavs:
                    samples_by_label[word].append(wav)

    # Report coverage
    print(f"\n=== Dataset Coverage ===")
    total_samples = 0
    missing_words = []
    for word in game_words:
        count = len(samples_by_label[word])
        total_samples += count
        if count == 0:
            missing_words.append(word)
    print(f"Total samples: {total_samples}")
    print(f"Words with data: {len(game_words) - len(missing_words)}/{len(game_words)}")
    if missing_words:
        print(f"Missing words: {missing_words}")

    # --- Create symlinks in unified directory structure ---
    # Split: 80% train, 10% val, 10% test (stratified by label)
    splits = {"train": [], "val": [], "test": []}

    rng = random.Random(42)

    for word in game_words:
        word_samples = samples_by_label[word]
        if not word_samples:
            continue

        rng.shuffle(word_samples)
        n = len(word_samples)
        n_test = max(1, int(n * 0.10))
        n_val = max(1, int(n * 0.10))
        n_train = n - n_test - n_val

        # Ensure at least 1 in each split if enough samples
        if n < 3:
            # Too few: put all in train
            splits["train"].extend([(p, word) for p in word_samples])
        else:
            splits["test"].extend([(p, word) for p in word_samples[:n_test]])
            splits["val"].extend([(p, word) for p in word_samples[n_test : n_test + n_val]])
            splits["train"].extend([(p, word) for p in word_samples[n_test + n_val :]])

    # Copy or symlink files into unified/{split}/{word}/
    manifest = {}
    for split_name, split_samples in splits.items():
        split_dir = unified_dir / split_name
        split_entries = []

        for src_path, word in split_samples:
            dst_dir = split_dir / word
            dst_dir.mkdir(parents=True, exist_ok=True)
            dst_path = dst_dir / src_path.name

            # Avoid name collisions
            if dst_path.exists():
                stem = src_path.stem
                suffix = src_path.suffix
                counter = 1
                while dst_path.exists():
                    dst_path = dst_dir / f"{stem}_{counter}{suffix}"
                    counter += 1

            # Symlink for efficiency (avoid copying GBs of audio)
            try:
                dst_path.symlink_to(src_path.resolve())
            except OSError:
                # Fallback to copy if symlinks not supported
                shutil.copy2(src_path, dst_path)

            split_entries.append({
                "path": str(dst_path.relative_to(unified_dir)),
                "label": word,
                "label_idx": word_to_idx[word],
            })

        manifest[split_name] = split_entries
        print(f"  {split_name}: {len(split_entries)} samples")

    # Write manifest
    manifest_path = unified_dir / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"\nManifest written to {manifest_path}")

    # Write labels.txt for reference
    labels_path = unified_dir / "labels.txt"
    with open(labels_path, "w") as f:
        for word in game_words:
            f.write(word + "\n")

    return unified_dir


def stage_speech_commands_pretrain(output_dir):
    """Prepare the full Speech Commands v2 dataset for teacher pretraining.

    This creates a directory structure matching VoiceCommandDataset expectations
    with all 35 Speech Commands classes.
    """
    sc_dir = output_dir / "speech_commands_v2"
    pretrain_dir = output_dir / "speech_commands_pretrain"

    if pretrain_dir.exists():
        print(f"Pretrain dataset already exists at {pretrain_dir}")
        return pretrain_dir

    pretrain_dir.mkdir(parents=True, exist_ok=True)

    # Find the actual data directory (torchaudio nests it)
    data_dir = sc_dir
    alt_dir = sc_dir / "SpeechCommands" / "speech_commands_v0.02"
    if alt_dir.exists():
        data_dir = alt_dir

    for sc_class in SPEECH_COMMANDS_CLASSES:
        src = data_dir / sc_class
        if src.exists():
            dst = pretrain_dir / sc_class
            if not dst.exists():
                dst.symlink_to(src.resolve())

    classes_found = sum(1 for c in SPEECH_COMMANDS_CLASSES if (pretrain_dir / c).exists())
    print(f"Pretrain dataset: {classes_found}/{len(SPEECH_COMMANDS_CLASSES)} classes linked")

    # Write pretrain labels
    labels_path = pretrain_dir / "labels.txt"
    with open(labels_path, "w") as f:
        for c in SPEECH_COMMANDS_CLASSES:
            f.write(c + "\n")

    return pretrain_dir


def main():
    parser = argparse.ArgumentParser(description="Voice data pipeline")
    parser.add_argument("--stage", type=str,
                        choices=["download", "tts", "unify", "pretrain", "all"],
                        default="all", help="Pipeline stage to run")
    parser.add_argument("--output-dir", type=Path, required=True,
                        help="Root output directory for all data")
    parser.add_argument("--config", type=str, default=None,
                        help="Config YAML path (default: config.yaml in script dir)")
    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)
    config_path = args.config

    if args.stage in ("download", "all"):
        print("\n===== Stage: Download Speech Commands v2 =====")
        stage_download(args.output_dir)

    if args.stage in ("pretrain", "all"):
        print("\n===== Stage: Prepare Speech Commands for pretraining =====")
        stage_speech_commands_pretrain(args.output_dir)

    if args.stage in ("tts", "all"):
        print("\n===== Stage: GPU TTS Generation =====")
        stage_tts(args.output_dir, config_path)

    if args.stage in ("unify", "all"):
        print("\n===== Stage: Build Unified Dataset =====")
        stage_unify(args.output_dir, config_path)

    print("\n===== Pipeline complete =====")


if __name__ == "__main__":
    main()
