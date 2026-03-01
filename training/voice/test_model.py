#!/usr/bin/env python3
"""Tests for the TC-ResNet8 model and dataset pipeline."""

import sys
import tempfile
from pathlib import Path

import numpy as np
import torch

# Add training/voice to path
sys.path.insert(0, str(Path(__file__).parent))

from model import TCResNet8, export_onnx
from dataset import compute_mel_spectrogram, _mel_filter_bank


def test_model_output_shape():
    """Random input produces correct output shape."""
    model = TCResNet8(n_mels=40, num_classes=119, channels=[32, 48, 64, 96])
    model.eval()
    x = torch.randn(4, 1, 40, 49)  # batch of 4
    with torch.no_grad():
        out = model(x)
    assert out.shape == (4, 119), f"Expected (4, 119), got {out.shape}"
    print("PASS: model output shape")


def test_model_single_sample():
    """Single sample produces valid logits."""
    model = TCResNet8(n_mels=40, num_classes=119)
    model.eval()
    x = torch.randn(1, 1, 40, 49)
    with torch.no_grad():
        out = model(x)
    assert out.shape == (1, 119)
    # Logits should be finite
    assert torch.isfinite(out).all(), "Output contains non-finite values"
    print("PASS: single sample logits")


def test_onnx_export_roundtrip():
    """ONNX export produces file that matches PyTorch output."""
    model = TCResNet8(n_mels=40, num_classes=119, channels=[32, 48, 64, 96])
    model.eval()

    x = torch.randn(1, 1, 40, 49)

    with torch.no_grad():
        pytorch_out = model(x).numpy()

    with tempfile.NamedTemporaryFile(suffix=".onnx", delete=False) as f:
        onnx_path = f.name

    export_onnx(model, onnx_path, num_classes=119, n_mels=40, n_frames=49)

    # Verify file exists and is non-empty
    assert Path(onnx_path).exists()
    size = Path(onnx_path).stat().st_size
    assert size > 0, f"ONNX file is empty"
    assert size < 2000 * 1024, f"ONNX file too large: {size / 1024:.1f}KB (expected <2MB)"

    try:
        import onnxruntime as ort

        sess = ort.InferenceSession(onnx_path)
        onnx_out = sess.run(None, {"mel_spectrogram": x.numpy()})[0]

        # Should match within floating point tolerance
        np.testing.assert_allclose(pytorch_out, onnx_out, rtol=1e-4, atol=1e-5)
        print(f"PASS: ONNX roundtrip matches (model size: {size / 1024:.1f}KB)")
    except ImportError:
        print(f"PASS: ONNX export (size: {size / 1024:.1f}KB, onnxruntime not installed for roundtrip)")

    Path(onnx_path).unlink()


def test_mel_spectrogram_shape():
    """Mel spectrogram has correct shape [40, 49]."""
    audio = np.random.randn(16000).astype(np.float32) * 0.1
    mel = compute_mel_spectrogram(audio, sr=16000, n_fft=512, hop_length=320,
                                  n_mels=40, fmin=60, fmax=7800, num_frames=49)
    assert mel.shape == (40, 49), f"Expected (40, 49), got {mel.shape}"
    print("PASS: mel spectrogram shape")


def test_mel_silence():
    """Silence produces near-minimum mel values."""
    audio = np.zeros(16000, dtype=np.float32)
    mel = compute_mel_spectrogram(audio, sr=16000, n_fft=512, hop_length=320,
                                  n_mels=40, fmin=60, fmax=7800, num_frames=49)
    # log(eps) ≈ -20.7
    assert mel.max() < -15.0, f"Silence mel max too high: {mel.max()}"
    print("PASS: mel silence values")


def test_mel_sine_wave():
    """1kHz sine wave produces energy in expected mel bins."""
    t = np.arange(16000, dtype=np.float32) / 16000.0
    audio = (np.sin(2 * np.pi * 1000 * t) * 0.5).astype(np.float32)
    mel = compute_mel_spectrogram(audio, sr=16000, n_fft=512, hop_length=320,
                                  n_mels=40, fmin=60, fmax=7800, num_frames=49)

    # Find peak mel bin at middle time frame
    mid_frame = 24
    peak_bin = np.argmax(mel[:, mid_frame])
    assert 5 <= peak_bin <= 25, f"1kHz sine peak at mel bin {peak_bin}, expected 5-25"
    print(f"PASS: mel sine wave peak at bin {peak_bin}")


def test_mel_filter_bank():
    """Mel filter bank has correct shape and properties."""
    filters = _mel_filter_bank(16000, 512, 40, 60, 7800)
    assert filters.shape == (40, 257), f"Expected (40, 257), got {filters.shape}"
    # All values should be non-negative
    assert (filters >= 0).all(), "Filter bank has negative values"
    # Each filter should have at least some non-zero values
    for i in range(40):
        assert filters[i].sum() > 0, f"Filter {i} is all zeros"
    print("PASS: mel filter bank")


def test_model_param_count():
    """Model parameter count is within expected range for k=2."""
    model = TCResNet8(n_mels=40, num_classes=119, channels=[32, 48, 64, 96])
    param_count = sum(p.numel() for p in model.parameters())
    assert 200_000 < param_count < 600_000, (
        f"Param count {param_count:,} outside expected range 200K-600K"
    )
    print(f"PASS: model has {param_count:,} parameters")


if __name__ == "__main__":
    test_model_output_shape()
    test_model_single_sample()
    test_mel_spectrogram_shape()
    test_mel_silence()
    test_mel_sine_wave()
    test_mel_filter_bank()
    test_model_param_count()
    test_onnx_export_roundtrip()
    print("\nAll tests passed!")
