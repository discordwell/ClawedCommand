#!/usr/bin/env python3
"""TC-ResNet8 keyword spotting model."""

import torch
import torch.nn as nn


class TemporalBlock(nn.Module):
    """Temporal residual block with two 1D convolutions."""

    def __init__(self, in_channels, out_channels, kernel_size=9, padding=4):
        super().__init__()
        self.conv1 = nn.Conv1d(in_channels, out_channels, kernel_size, padding=padding)
        self.bn1 = nn.BatchNorm1d(out_channels)
        self.conv2 = nn.Conv1d(out_channels, out_channels, kernel_size, padding=padding)
        self.bn2 = nn.BatchNorm1d(out_channels)
        self.relu = nn.ReLU(inplace=True)

        self.skip = None
        if in_channels != out_channels:
            self.skip = nn.Sequential(
                nn.Conv1d(in_channels, out_channels, 1),
                nn.BatchNorm1d(out_channels),
            )

    def forward(self, x):
        identity = x
        out = self.relu(self.bn1(self.conv1(x)))
        out = self.bn2(self.conv2(out))
        if self.skip is not None:
            identity = self.skip(identity)
        out += identity
        return self.relu(out)


class TCResNet8(nn.Module):
    """TC-ResNet8 for keyword spotting.

    Input: [batch, 1, n_mels, n_frames] mel spectrogram
    Output: [batch, num_classes] logits
    """

    def __init__(self, n_mels=40, num_classes=119, channels=None):
        super().__init__()
        if channels is None:
            channels = [32, 48, 64, 96]  # k=2 for 119-class vocab

        # Collapse frequency axis: [B, 1, n_mels, T] -> [B, C0, 1, T]
        self.conv0 = nn.Conv2d(1, channels[0], kernel_size=(n_mels, 1))
        self.bn0 = nn.BatchNorm2d(channels[0])
        self.relu = nn.ReLU(inplace=True)

        # 3 temporal residual blocks (1D along time axis)
        self.block1 = TemporalBlock(channels[0], channels[1])
        self.block2 = TemporalBlock(channels[1], channels[2])
        self.block3 = TemporalBlock(channels[2], channels[3])

        self.pool = nn.AdaptiveAvgPool1d(1)
        self.fc = nn.Linear(channels[3], num_classes)

    def forward(self, x):
        # x: [B, 1, 40, 49]
        x = self.relu(self.bn0(self.conv0(x)))  # [B, C0, 1, 49]
        x = x.squeeze(2)  # [B, C0, 49]
        x = self.block1(x)
        x = self.block2(x)
        x = self.block3(x)
        x = self.pool(x).squeeze(2)  # [B, C3]
        return self.fc(x)


def export_onnx(model, path, num_classes=119, n_mels=40, n_frames=49):
    """Export model to ONNX format."""
    model.eval()
    dummy = torch.randn(1, 1, n_mels, n_frames)
    torch.onnx.export(
        model,
        dummy,
        path,
        opset_version=17,
        input_names=["mel_spectrogram"],
        output_names=["logits"],
        dynamic_axes={
            "mel_spectrogram": {0: "batch"},
            "logits": {0: "batch"},
        },
    )
    print(f"Exported ONNX model to {path}")
