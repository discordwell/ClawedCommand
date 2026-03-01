#!/usr/bin/env python3
"""Comprehensive evaluation for keyword spotting models.

Evaluates on multiple test sets, reports per-category accuracy, confusion matrix,
model size, and inference latency. Compares baseline vs distilled student.

Usage:
    python evaluate.py \
        --model ../../assets/voice/keyword_classifier.onnx \
        --test-dir /workspace/data/unified/test \
        --output-dir /workspace/eval_results

    # Compare baseline vs distilled
    python evaluate.py \
        --model /workspace/checkpoints/distilled.onnx \
        --baseline /workspace/checkpoints/baseline.onnx \
        --test-dir /workspace/data/unified/test \
        --output-dir /workspace/eval_results
"""

import argparse
import json
import os
import time
from collections import defaultdict
from pathlib import Path

import numpy as np
import yaml

# Categories for per-group accuracy reporting (matches config.yaml structure)
CATEGORY_MAP = {
    "agents": [
        "attack", "retreat", "move", "defend", "hold", "patrol", "gather",
        "scout", "build", "train", "stop", "follow", "guard", "heal",
        "flank", "charge", "siege", "rally",
    ],
    "catgpt_units": [
        "pawdler", "pawds", "nuisance", "chonk", "fox", "hisser", "yowler",
        "mouser", "catnapper", "napper", "sapper", "mech",
    ],
    "enemy_units": [],  # Filled dynamically from all faction unit lists
    "selectors": [
        "all", "screen", "selected", "group", "army", "workers", "idle",
        "one", "two", "three",
    ],
    "directions": ["north", "south", "east", "west"],
    "buildings": ["barracks", "refinery", "tower", "box", "tree", "market", "rack", "post"],
    "meta": ["base", "cancel", "help", "undo", "yes", "no"],
    "conjunctions": ["and", "with", "except", "not"],
    "special": ["unknown", "silence"],
}


def load_labels(config_path=None):
    """Load label list from config.yaml."""
    if config_path is None:
        config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        cfg = yaml.safe_load(f)

    labels = []
    for category in cfg["vocabulary"].values():
        if isinstance(category, list):
            for word in category:
                labels.append(str(word))
    return labels


def build_category_map(labels):
    """Build word → category mapping."""
    # All enemy faction units (everything not in other categories)
    known = set()
    for cat_words in CATEGORY_MAP.values():
        known.update(cat_words)

    word_to_cat = {}
    for cat, words in CATEGORY_MAP.items():
        for w in words:
            word_to_cat[w] = cat

    # Everything not categorized is an enemy unit
    for label in labels:
        if label not in word_to_cat:
            word_to_cat[label] = "enemy_units"

    return word_to_cat


def load_test_samples(test_dir, labels):
    """Load test samples from directory structure.

    Expects: test_dir/{label}/*.wav
    Returns: list of (wav_path, label_idx)
    """
    from dataset import load_wav, compute_mel_spectrogram

    label_to_idx = {l: i for i, l in enumerate(labels)}
    samples = []

    test_dir = Path(test_dir)
    for label in labels:
        label_dir = test_dir / label
        if not label_dir.exists():
            continue
        for wav_path in sorted(label_dir.glob("*.wav")):
            if label in label_to_idx:
                samples.append((wav_path, label_to_idx[label]))

    return samples


def compute_mel_from_wav(wav_path, cfg):
    """Compute mel spectrogram from a WAV file."""
    from dataset import load_wav, compute_mel_spectrogram

    audio_cfg = cfg["audio"]
    sr = audio_cfg["sample_rate"]
    target_samples = int(audio_cfg["duration_sec"] * sr)

    audio = load_wav(wav_path, sr, target_samples)
    mel = compute_mel_spectrogram(
        audio, sr, audio_cfg["n_fft"], audio_cfg["hop_length"],
        audio_cfg["n_mels"], audio_cfg["fmin"], audio_cfg["fmax"],
        audio_cfg["num_frames"],
    )
    return mel


def run_onnx_inference(onnx_path, mel):
    """Run ONNX inference on a mel spectrogram. Returns logits."""
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path))
    input_name = sess.get_inputs()[0].name
    mel_input = mel[np.newaxis, np.newaxis, :, :].astype(np.float32)  # [1, 1, 40, 49]
    logits = sess.run(None, {input_name: mel_input})[0]
    return logits[0]  # [118]


def evaluate_model(onnx_path, test_dir, labels, cfg):
    """Run full evaluation of an ONNX model on a test set.

    Returns dict with accuracy metrics and predictions.
    """
    import onnxruntime as ort

    samples = load_test_samples(test_dir, labels)
    if not samples:
        return {"error": f"No test samples found in {test_dir}"}

    sess = ort.InferenceSession(str(onnx_path))
    input_name = sess.get_inputs()[0].name

    correct = 0
    top3_correct = 0
    total = 0
    per_class_correct = defaultdict(int)
    per_class_total = defaultdict(int)
    predictions = []
    confidences = []

    for wav_path, true_idx in samples:
        mel = compute_mel_from_wav(wav_path, cfg)
        mel_input = mel[np.newaxis, np.newaxis, :, :].astype(np.float32)
        logits = sess.run(None, {input_name: mel_input})[0][0]

        logits_shifted = logits - np.max(logits)
        probs = np.exp(logits_shifted) / np.sum(np.exp(logits_shifted))
        pred_idx = int(np.argmax(logits))
        top3 = np.argsort(logits)[-3:][::-1]
        confidence = float(probs[pred_idx])

        is_correct = pred_idx == true_idx
        is_top3 = true_idx in top3

        correct += is_correct
        top3_correct += is_top3
        total += 1
        per_class_correct[true_idx] += is_correct
        per_class_total[true_idx] += 1
        predictions.append((true_idx, pred_idx))
        confidences.append(confidence)

    # Per-class accuracy
    per_class_acc = {}
    for idx in range(len(labels)):
        t = per_class_total.get(idx, 0)
        c = per_class_correct.get(idx, 0)
        if t > 0:
            per_class_acc[labels[idx]] = {"correct": c, "total": t, "accuracy": c / t}

    # Per-category accuracy
    word_to_cat = build_category_map(labels)
    cat_correct = defaultdict(int)
    cat_total = defaultdict(int)
    for true_idx, pred_idx in predictions:
        label = labels[true_idx]
        cat = word_to_cat.get(label, "unknown")
        cat_total[cat] += 1
        cat_correct[cat] += (true_idx == pred_idx)

    per_category_acc = {}
    for cat in sorted(cat_total.keys()):
        t = cat_total[cat]
        c = cat_correct[cat]
        per_category_acc[cat] = {"correct": c, "total": t, "accuracy": c / t}

    # Confusion matrix (as sparse dict for storage)
    confusion = defaultdict(lambda: defaultdict(int))
    for true_idx, pred_idx in predictions:
        confusion[labels[true_idx]][labels[pred_idx]] += 1

    return {
        "top1_accuracy": correct / max(total, 1),
        "top3_accuracy": top3_correct / max(total, 1),
        "total_samples": total,
        "mean_confidence": float(np.mean(confidences)) if confidences else 0,
        "per_class": per_class_acc,
        "per_category": per_category_acc,
        "confusion": dict(confusion),
    }


def benchmark_latency(onnx_path, n_iters=100):
    """Benchmark ONNX inference latency."""
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path))
    input_name = sess.get_inputs()[0].name
    dummy = np.random.randn(1, 1, 40, 49).astype(np.float32)

    # Warmup
    for _ in range(10):
        sess.run(None, {input_name: dummy})

    # Benchmark
    times = []
    for _ in range(n_iters):
        start = time.perf_counter()
        sess.run(None, {input_name: dummy})
        elapsed = time.perf_counter() - start
        times.append(elapsed * 1000)  # ms

    return {
        "mean_ms": float(np.mean(times)),
        "p50_ms": float(np.percentile(times, 50)),
        "p95_ms": float(np.percentile(times, 95)),
        "p99_ms": float(np.percentile(times, 99)),
        "min_ms": float(np.min(times)),
        "max_ms": float(np.max(times)),
    }


def verify_onnx_format(onnx_path, num_classes=118):
    """Verify ONNX model has correct input/output format."""
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path))
    inp = sess.get_inputs()[0]
    out = sess.get_outputs()[0]

    checks = {
        "input_name": inp.name == "mel_spectrogram",
        "output_name": out.name == "logits",
        "input_shape_mels": inp.shape[2] == 40,
        "input_shape_frames": inp.shape[3] == 49,
        "output_classes": out.shape[1] == num_classes,
    }

    # Run inference
    dummy = np.random.randn(1, 1, 40, 49).astype(np.float32)
    logits = sess.run(None, {inp.name: dummy})[0]
    checks["output_shape"] = logits.shape == (1, num_classes)
    checks["finite_output"] = bool(np.isfinite(logits).all())

    # Model size
    size_bytes = os.path.getsize(onnx_path)
    checks["under_1mb"] = size_bytes < 1_000_000

    return checks, size_bytes


def print_report(results, model_name, labels):
    """Print a formatted evaluation report."""
    print(f"\n{'='*60}")
    print(f"  {model_name}")
    print(f"{'='*60}")

    if "error" in results:
        print(f"  ERROR: {results['error']}")
        return

    print(f"  Top-1 Accuracy: {results['top1_accuracy']:.4f} ({results['top1_accuracy']*100:.1f}%)")
    print(f"  Top-3 Accuracy: {results['top3_accuracy']:.4f} ({results['top3_accuracy']*100:.1f}%)")
    print(f"  Total Samples:  {results['total_samples']}")
    print(f"  Mean Confidence: {results['mean_confidence']:.3f}")

    print(f"\n  Per-Category Accuracy:")
    for cat, data in sorted(results.get("per_category", {}).items()):
        bar = "#" * int(data["accuracy"] * 20)
        print(f"    {cat:20s} {data['accuracy']:.3f} ({data['correct']}/{data['total']}) {bar}")

    # Worst-performing classes
    per_class = results.get("per_class", {})
    if per_class:
        sorted_classes = sorted(per_class.items(), key=lambda x: x[1]["accuracy"])
        worst = sorted_classes[:10]
        print(f"\n  Worst 10 Classes:")
        for label, data in worst:
            print(f"    {label:20s} {data['accuracy']:.3f} ({data['correct']}/{data['total']})")


def save_confusion_matrix(confusion, labels, output_path):
    """Save confusion matrix as a heatmap image."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not available, skipping confusion matrix plot")
        return

    n = len(labels)
    matrix = np.zeros((n, n))
    for true_label, preds in confusion.items():
        if true_label in labels:
            i = labels.index(true_label)
            for pred_label, count in preds.items():
                if pred_label in labels:
                    j = labels.index(pred_label)
                    matrix[i, j] = count

    # Normalize rows
    row_sums = matrix.sum(axis=1, keepdims=True)
    row_sums = np.where(row_sums == 0, 1, row_sums)
    matrix_norm = matrix / row_sums

    fig, ax = plt.subplots(figsize=(20, 20))
    im = ax.imshow(matrix_norm, cmap="Blues", vmin=0, vmax=1)
    ax.set_xlabel("Predicted")
    ax.set_ylabel("True")
    ax.set_title("Confusion Matrix (normalized)")
    plt.colorbar(im, ax=ax)
    plt.tight_layout()
    plt.savefig(output_path, dpi=100)
    plt.close()
    print(f"  Confusion matrix saved to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Evaluate keyword spotting model")
    parser.add_argument("--model", type=Path, required=True,
                        help="ONNX model to evaluate")
    parser.add_argument("--baseline", type=Path, default=None,
                        help="Baseline ONNX model for comparison")
    parser.add_argument("--test-dir", type=Path, required=True,
                        help="Test data directory")
    parser.add_argument("--speech-commands-dir", type=Path, default=None,
                        help="Speech Commands test directory (optional)")
    parser.add_argument("--output-dir", type=Path, default=Path("eval_results"),
                        help="Output directory for reports")
    parser.add_argument("--config", type=str, default="config.yaml")
    args = parser.parse_args()

    config_path = Path(__file__).parent / args.config
    with open(config_path) as f:
        cfg = yaml.safe_load(f)

    labels = load_labels(config_path)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Labels: {len(labels)}")

    # --- Verify ONNX format ---
    print("\n--- ONNX Format Verification ---")
    checks, size_bytes = verify_onnx_format(args.model, len(labels))
    for check, passed in checks.items():
        status = "PASS" if passed else "FAIL"
        print(f"  [{status}] {check}")
    print(f"  Model size: {size_bytes / 1024:.1f} KB")

    # --- Latency benchmark ---
    print("\n--- Latency Benchmark ---")
    latency = benchmark_latency(args.model)
    print(f"  Mean: {latency['mean_ms']:.2f} ms")
    print(f"  P50:  {latency['p50_ms']:.2f} ms")
    print(f"  P95:  {latency['p95_ms']:.2f} ms")
    print(f"  P99:  {latency['p99_ms']:.2f} ms")

    # --- Evaluate on synthetic test set ---
    print("\n--- Synthetic Test Set ---")
    results = evaluate_model(args.model, args.test_dir, labels, cfg)
    print_report(results, "Distilled Student", labels)

    if results.get("confusion"):
        save_confusion_matrix(
            results["confusion"], labels,
            output_dir / "confusion_matrix.png",
        )

    # --- Evaluate baseline if provided ---
    baseline_results = None
    if args.baseline and args.baseline.exists():
        print("\n--- Baseline Comparison ---")
        baseline_results = evaluate_model(args.baseline, args.test_dir, labels, cfg)
        print_report(baseline_results, "Baseline Student", labels)

        # Comparison
        if "error" not in results and "error" not in baseline_results:
            delta = results["top1_accuracy"] - baseline_results["top1_accuracy"]
            print(f"\n  Improvement: {delta:+.4f} ({delta*100:+.1f}%)")

    # --- Evaluate on Speech Commands if provided ---
    sc_results = None
    if args.speech_commands_dir and args.speech_commands_dir.exists():
        print("\n--- Speech Commands Test Set ---")
        sc_results = evaluate_model(args.model, args.speech_commands_dir, labels, cfg)
        print_report(sc_results, "Speech Commands (real speech)", labels)

    # --- Save full report ---
    report = {
        "model": str(args.model),
        "model_size_kb": size_bytes / 1024,
        "onnx_checks": checks,
        "latency": latency,
        "synthetic_test": results,
    }
    if baseline_results:
        report["baseline"] = baseline_results
    if sc_results:
        report["speech_commands_test"] = sc_results

    report_path = output_dir / "eval_report.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"\nFull report saved to {report_path}")


if __name__ == "__main__":
    main()
