#!/usr/bin/env python3
"""Train TC-ResNet8 keyword spotting model.

Usage:
    python train.py --data-dir data/tts --output keyword_classifier.onnx
    python train.py --data-dir data/tts --real-dir data/real --epochs 150
"""

import argparse
import random
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, random_split
import yaml

from model import TCResNet8, export_onnx
from dataset import VoiceCommandDataset, load_labels


def load_config(config_path="config.yaml"):
    with open(Path(__file__).parent / config_path) as f:
        return yaml.safe_load(f)


def set_seed(seed):
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(seed)


def train_epoch(model, loader, criterion, optimizer, device):
    model.train()
    total_loss = 0.0
    correct = 0
    total = 0

    for mel, labels in loader:
        mel = mel.to(device)
        labels = labels.to(device)

        optimizer.zero_grad()
        logits = model(mel)
        loss = criterion(logits, labels)
        loss.backward()
        optimizer.step()

        total_loss += loss.item() * mel.size(0)
        preds = logits.argmax(dim=1)
        correct += (preds == labels).sum().item()
        total += mel.size(0)

    return total_loss / total, correct / total


def evaluate(model, loader, criterion, device):
    model.eval()
    total_loss = 0.0
    correct = 0
    total = 0

    with torch.no_grad():
        for mel, labels in loader:
            mel = mel.to(device)
            labels = labels.to(device)

            logits = model(mel)
            loss = criterion(logits, labels)

            total_loss += loss.item() * mel.size(0)
            preds = logits.argmax(dim=1)
            correct += (preds == labels).sum().item()
            total += mel.size(0)

    return total_loss / total, correct / total


def main():
    parser = argparse.ArgumentParser(description="Train TC-ResNet8")
    parser.add_argument("--data-dir", type=Path, required=True,
                        help="Directory with word subdirectories of WAV files")
    parser.add_argument("--real-dir", type=Path, default=None,
                        help="Additional real recordings directory")
    parser.add_argument("--output", type=Path,
                        default=Path("../../assets/voice/keyword_classifier.onnx"),
                        help="Output ONNX model path")
    parser.add_argument("--config", type=str, default="config.yaml")
    parser.add_argument("--epochs", type=int, default=None)
    parser.add_argument("--device", type=str, default=None)
    args = parser.parse_args()

    cfg = load_config(args.config)
    train_cfg = cfg["training"]
    audio_cfg = cfg["audio"]

    seed = train_cfg["seed"]
    set_seed(seed)

    epochs = args.epochs or train_cfg["epochs"]

    if args.device:
        device = torch.device(args.device)
    elif torch.cuda.is_available():
        device = torch.device("cuda")
    elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        device = torch.device("mps")
    else:
        device = torch.device("cpu")

    print(f"Device: {device}")

    # Load labels
    labels = load_labels(args.config)
    num_classes = len(labels)
    print(f"Classes: {num_classes} -- {labels}")

    # Create dataset
    dataset = VoiceCommandDataset(
        args.data_dir, labels=labels, config_path=args.config,
        augment=True, split="all",
    )

    # Train/val split
    val_size = int(len(dataset) * train_cfg["val_split"])
    train_size = len(dataset) - val_size
    train_dataset, val_dataset = random_split(
        dataset, [train_size, val_size],
        generator=torch.Generator().manual_seed(seed),
    )

    # Disable augmentation for validation
    # (We use the same dataset object, so we create a wrapper)
    class NoAugWrapper(torch.utils.data.Dataset):
        def __init__(self, subset, original_dataset):
            self.subset = subset
            self.original = original_dataset

        def __len__(self):
            return len(self.subset)

        def __getitem__(self, idx):
            real_idx = self.subset.indices[idx]
            wav_path, label_idx = self.original.samples[real_idx]
            from dataset import load_wav, compute_mel_spectrogram
            audio = load_wav(wav_path, self.original.sr, self.original.target_samples)
            mel = compute_mel_spectrogram(
                audio, self.original.sr, self.original.n_fft,
                self.original.hop_length, self.original.n_mels,
                self.original.fmin, self.original.fmax, self.original.num_frames,
            )
            mel_tensor = torch.from_numpy(mel).unsqueeze(0)
            return mel_tensor, label_idx

    val_no_aug = NoAugWrapper(val_dataset, dataset)

    train_loader = DataLoader(
        train_dataset, batch_size=train_cfg["batch_size"],
        shuffle=True, num_workers=4, pin_memory=True,
    )
    val_loader = DataLoader(
        val_no_aug, batch_size=train_cfg["batch_size"],
        shuffle=False, num_workers=4, pin_memory=True,
    )

    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    # Create model
    channels = cfg["model"]["channels"]
    model = TCResNet8(
        n_mels=audio_cfg["n_mels"],
        num_classes=num_classes,
        channels=channels,
    ).to(device)

    param_count = sum(p.numel() for p in model.parameters())
    print(f"Model params: {param_count:,}")

    # Training setup
    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=train_cfg["learning_rate"],
        weight_decay=train_cfg["weight_decay"],
    )

    # Cosine annealing with warmup
    warmup_epochs = train_cfg["warmup_epochs"]

    def lr_lambda(epoch):
        if epoch < warmup_epochs:
            return epoch / warmup_epochs
        progress = (epoch - warmup_epochs) / (epochs - warmup_epochs)
        return 0.5 * (1.0 + np.cos(np.pi * progress))

    scheduler = torch.optim.lr_scheduler.LambdaLR(optimizer, lr_lambda)

    # Training loop
    best_val_acc = 0.0
    best_model_state = None

    for epoch in range(epochs):
        train_loss, train_acc = train_epoch(model, train_loader, criterion, optimizer, device)
        val_loss, val_acc = evaluate(model, val_loader, criterion, device)
        scheduler.step()

        lr = optimizer.param_groups[0]["lr"]

        if (epoch + 1) % 5 == 0 or epoch == 0:
            print(f"Epoch {epoch+1:3d}/{epochs} | "
                  f"Train Loss: {train_loss:.4f} Acc: {train_acc:.3f} | "
                  f"Val Loss: {val_loss:.4f} Acc: {val_acc:.3f} | "
                  f"LR: {lr:.6f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            best_model_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}

    print(f"\nBest val accuracy: {best_val_acc:.3f}")

    # Load best model and export
    model.load_state_dict(best_model_state)
    model = model.cpu()

    args.output.parent.mkdir(parents=True, exist_ok=True)
    export_onnx(
        model, str(args.output),
        num_classes=num_classes,
        n_mels=audio_cfg["n_mels"],
        n_frames=audio_cfg["num_frames"],
    )

    print(f"Model saved to {args.output}")

    # Report model size
    import os
    size_kb = os.path.getsize(args.output) / 1024
    print(f"Model size: {size_kb:.1f} KB")


if __name__ == "__main__":
    main()
