#!/usr/bin/env python3
"""TC-ResNet14-Wide teacher model for knowledge distillation.

Scaled-up version of TC-ResNet8 with 5 temporal blocks, wider channels,
and dropout regularization. Same input/output format as student.
"""

import torch
import torch.nn as nn

from model import TemporalBlock


class TCResNet14Wide(nn.Module):
    """TC-ResNet14-Wide teacher model for keyword spotting.

    Input: [batch, 1, n_mels, n_frames] mel spectrogram
    Output: [batch, num_classes] logits

    ~5M params with default channels [64, 96, 128, 192, 256].
    """

    def __init__(self, n_mels=40, num_classes=118, channels=None, dropout=0.1):
        super().__init__()
        if channels is None:
            channels = [64, 96, 128, 192, 256]

        # Collapse frequency axis: [B, 1, n_mels, T] -> [B, C0, 1, T]
        self.conv0 = nn.Conv2d(1, channels[0], kernel_size=(n_mels, 1))
        self.bn0 = nn.BatchNorm2d(channels[0])
        self.relu = nn.ReLU(inplace=True)
        self.drop = nn.Dropout(dropout)

        # 5 temporal residual blocks (reuses TemporalBlock from model.py)
        self.blocks = nn.ModuleList()
        for i in range(len(channels) - 1):
            self.blocks.append(TemporalBlock(channels[i], channels[i + 1]))

        self.pool = nn.AdaptiveAvgPool1d(1)
        self.fc = nn.Linear(channels[-1], num_classes)

    def forward(self, x):
        # x: [B, 1, 40, 49]
        x = self.relu(self.bn0(self.conv0(x)))  # [B, C0, 1, 49]
        x = x.squeeze(2)  # [B, C0, 49]
        for block in self.blocks:
            x = self.drop(block(x))
        x = self.pool(x).squeeze(2)  # [B, C_last]
        return self.fc(x)

    def get_features(self, x):
        """Return pre-FC pooled features for feature hint distillation."""
        x = self.relu(self.bn0(self.conv0(x)))
        x = x.squeeze(2)
        for block in self.blocks:
            x = self.drop(block(x))
        return self.pool(x).squeeze(2)  # [B, C_last]

    def replace_fc(self, num_classes):
        """Replace the final FC layer (for transfer learning)."""
        in_features = self.fc.in_features
        self.fc = nn.Linear(in_features, num_classes)

    def freeze_early_layers(self, num_blocks_to_freeze=2):
        """Freeze conv0 and the first N temporal blocks."""
        for param in self.conv0.parameters():
            param.requires_grad = False
        for param in self.bn0.parameters():
            param.requires_grad = False
        for i, block in enumerate(self.blocks):
            if i < num_blocks_to_freeze:
                for param in block.parameters():
                    param.requires_grad = False

    def unfreeze_all(self):
        """Unfreeze all layers."""
        for param in self.parameters():
            param.requires_grad = True


def get_student_features_fn():
    """Return a function that extracts pre-FC features from TCResNet8.

    Usage:
        from model import TCResNet8
        student = TCResNet8()
        features = get_student_features(student, mel_batch)
    """

    def get_features(model, x):
        x = model.relu(model.bn0(model.conv0(x)))
        x = x.squeeze(2)
        x = model.block1(x)
        x = model.block2(x)
        x = model.block3(x)
        return model.pool(x).squeeze(2)  # [B, 96]

    return get_features


if __name__ == "__main__":
    # Quick sanity check
    model = TCResNet14Wide(n_mels=40, num_classes=118)
    param_count = sum(p.numel() for p in model.parameters())
    print(f"Teacher params: {param_count:,}")

    dummy = torch.randn(2, 1, 40, 49)
    logits = model(dummy)
    features = model.get_features(dummy)
    print(f"Logits shape: {logits.shape}")    # [2, 118]
    print(f"Features shape: {features.shape}")  # [2, 256]
