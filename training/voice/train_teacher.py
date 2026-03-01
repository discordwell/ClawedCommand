#!/usr/bin/env python3
"""Two-phase teacher model training for knowledge distillation.

Phase 1 (pretrain): Train TC-ResNet14-Wide on Google Speech Commands v2 (35 classes)
                    to learn general speech representations from real human audio.

Phase 2 (finetune): Fine-tune on the unified game vocabulary (118 classes) with
                    label smoothing and mixup regularization.

Usage:
    # Phase 1: Pretrain on Speech Commands
    python train_teacher.py --phase pretrain \
        --data-dir /workspace/data/speech_commands_pretrain \
        --output-dir /workspace/checkpoints \
        --device cuda

    # Phase 2: Fine-tune on game vocabulary
    python train_teacher.py --phase finetune \
        --data-dir /workspace/data/unified/train \
        --val-dir /workspace/data/unified/val \
        --pretrained /workspace/checkpoints/teacher_pretrain_best.pt \
        --output-dir /workspace/checkpoints \
        --device cuda
"""

import argparse
import random
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, random_split
import yaml

from teacher_model import TCResNet14Wide
from dataset import VoiceCommandDataset, NoAugDataset, load_labels


def load_config(path="distill_config.yaml"):
    with open(Path(__file__).parent / path) as f:
        return yaml.safe_load(f)


def load_base_config(path="config.yaml"):
    with open(Path(__file__).parent / path) as f:
        return yaml.safe_load(f)


def set_seed(seed):
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(seed)


def get_device(device_str=None):
    if device_str:
        return torch.device(device_str)
    if torch.cuda.is_available():
        return torch.device("cuda")
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


def mixup_data(x, y, alpha=0.2):
    """Apply mixup augmentation to a batch.

    Generates mixed inputs and pairs of targets for mixup loss computation.
    """
    if alpha <= 0:
        return x, y, y, 1.0

    lam = np.random.beta(alpha, alpha)
    batch_size = x.size(0)
    index = torch.randperm(batch_size, device=x.device)

    mixed_x = lam * x + (1 - lam) * x[index]
    y_a, y_b = y, y[index]
    return mixed_x, y_a, y_b, lam


def mixup_criterion(criterion, pred, y_a, y_b, lam):
    """Compute mixup loss."""
    return lam * criterion(pred, y_a) + (1 - lam) * criterion(pred, y_b)


def train_epoch(model, loader, criterion, optimizer, device, mixup_alpha=0.0):
    model.train()
    total_loss = 0.0
    correct = 0
    total = 0

    for mel, labels in loader:
        mel = mel.to(device)
        labels = labels.to(device)

        optimizer.zero_grad()

        if mixup_alpha > 0:
            mel, labels_a, labels_b, lam = mixup_data(mel, labels, mixup_alpha)
            logits = model(mel)
            loss = mixup_criterion(criterion, logits, labels_a, labels_b, lam)
            preds = logits.argmax(dim=1)
            correct += (lam * (preds == labels_a).float() +
                        (1 - lam) * (preds == labels_b).float()).sum().item()
        else:
            logits = model(mel)
            loss = criterion(logits, labels)
            preds = logits.argmax(dim=1)
            correct += (preds == labels).sum().item()

        loss.backward()
        optimizer.step()

        total_loss += loss.item() * mel.size(0)
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


def phase_pretrain(args):
    """Phase 1: Pretrain teacher on Speech Commands v2 (35 classes)."""
    cfg = load_config()
    base_cfg = load_base_config()
    phase_cfg = cfg["teacher_pretrain"]
    teacher_cfg = cfg["teacher"]

    set_seed(cfg["training"]["seed"])
    device = get_device(args.device)
    print(f"Device: {device}")

    # Load Speech Commands labels
    labels_path = args.data_dir / "labels.txt"
    if labels_path.exists():
        with open(labels_path) as f:
            labels = [line.strip() for line in f if line.strip()]
    else:
        from data_pipeline import SPEECH_COMMANDS_CLASSES
        labels = SPEECH_COMMANDS_CLASSES

    num_classes = len(labels)
    print(f"Pretrain classes: {num_classes}")

    # Create dataset
    dataset = VoiceCommandDataset(
        args.data_dir, labels=labels, config_path="config.yaml",
        augment=True, split="all",
    )
    print(f"Dataset size: {len(dataset)}")

    # Train/val split
    val_size = int(len(dataset) * 0.15)
    train_size = len(dataset) - val_size
    train_dataset, val_dataset = random_split(
        dataset, [train_size, val_size],
        generator=torch.Generator().manual_seed(cfg["training"]["seed"]),
    )

    val_no_aug = NoAugDataset(val_dataset, dataset)

    train_loader = DataLoader(
        train_dataset, batch_size=phase_cfg["batch_size"],
        shuffle=True, num_workers=4, pin_memory=True,
    )
    val_loader = DataLoader(
        val_no_aug, batch_size=phase_cfg["batch_size"],
        shuffle=False, num_workers=4, pin_memory=True,
    )

    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    # Create teacher model
    model = TCResNet14Wide(
        n_mels=base_cfg["audio"]["n_mels"],
        num_classes=num_classes,
        channels=teacher_cfg["channels"],
        dropout=teacher_cfg["dropout"],
    ).to(device)

    param_count = sum(p.numel() for p in model.parameters())
    print(f"Teacher params: {param_count:,}")

    # Training setup
    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=phase_cfg["learning_rate"],
        weight_decay=phase_cfg["weight_decay"],
    )

    epochs = phase_cfg["epochs"]
    warmup = phase_cfg["warmup_epochs"]

    def lr_lambda(epoch):
        if epoch < warmup:
            return max(epoch, 1) / warmup
        progress = (epoch - warmup) / max(epochs - warmup, 1)
        return 0.5 * (1.0 + np.cos(np.pi * progress))

    scheduler = torch.optim.lr_scheduler.LambdaLR(optimizer, lr_lambda)

    # Training loop
    best_val_acc = 0.0
    best_state = None
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    for epoch in range(epochs):
        train_loss, train_acc = train_epoch(model, train_loader, criterion, optimizer, device)
        val_loss, val_acc = evaluate(model, val_loader, criterion, device)
        scheduler.step()

        if (epoch + 1) % 5 == 0 or epoch == 0:
            lr = optimizer.param_groups[0]["lr"]
            print(f"Epoch {epoch+1:3d}/{epochs} | "
                  f"Train: {train_loss:.4f} / {train_acc:.3f} | "
                  f"Val: {val_loss:.4f} / {val_acc:.3f} | "
                  f"LR: {lr:.6f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            best_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}

    print(f"\nBest pretrain val accuracy: {best_val_acc:.3f}")

    # Save best model
    save_path = output_dir / "teacher_pretrain_best.pt"
    torch.save({
        "model_state_dict": best_state,
        "num_classes": num_classes,
        "channels": teacher_cfg["channels"],
        "best_val_acc": best_val_acc,
        "phase": "pretrain",
    }, save_path)
    print(f"Saved to {save_path}")


def phase_finetune(args):
    """Phase 2: Fine-tune teacher on game vocabulary (118 classes)."""
    cfg = load_config()
    base_cfg = load_base_config()
    phase_cfg = cfg["teacher_finetune"]
    teacher_cfg = cfg["teacher"]

    set_seed(cfg["training"]["seed"])
    device = get_device(args.device)
    print(f"Device: {device}")

    # Load game labels
    game_labels = load_labels("config.yaml")
    num_classes = len(game_labels)
    print(f"Fine-tune classes: {num_classes}")

    # Create dataset
    dataset = VoiceCommandDataset(
        args.data_dir, labels=game_labels, config_path="config.yaml",
        augment=True, split="all",
    )

    # Use separate val dir if provided, else split
    if args.val_dir and args.val_dir.exists():
        val_dataset = VoiceCommandDataset(
            args.val_dir, labels=game_labels, config_path="config.yaml",
            augment=False, split="all",
        )
        train_dataset = dataset
    else:
        val_size = int(len(dataset) * 0.15)
        train_size = len(dataset) - val_size
        train_dataset, val_subset = random_split(
            dataset, [train_size, val_size],
            generator=torch.Generator().manual_seed(cfg["training"]["seed"]),
        )
        val_dataset = NoAugDataset(val_subset, dataset)

    train_loader = DataLoader(
        train_dataset, batch_size=phase_cfg["batch_size"],
        shuffle=True, num_workers=4, pin_memory=True,
    )
    val_loader = DataLoader(
        val_dataset, batch_size=phase_cfg["batch_size"],
        shuffle=False, num_workers=4, pin_memory=True,
    )

    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    # Load pretrained teacher and replace FC head
    model = TCResNet14Wide(
        n_mels=base_cfg["audio"]["n_mels"],
        num_classes=35,  # Pretrain classes (will be replaced)
        channels=teacher_cfg["channels"],
        dropout=teacher_cfg["dropout"],
    )

    if args.pretrained:
        checkpoint = torch.load(args.pretrained, map_location="cpu", weights_only=True)
        model.load_state_dict(checkpoint["model_state_dict"])
        print(f"Loaded pretrained teacher from {args.pretrained} "
              f"(pretrain acc: {checkpoint.get('best_val_acc', 'N/A')})")

    # Replace FC for 118 classes
    model.replace_fc(num_classes)
    model = model.to(device)

    param_count = sum(p.numel() for p in model.parameters())
    print(f"Teacher params: {param_count:,}")

    # Freeze early layers initially
    freeze_epochs = phase_cfg["freeze_epochs"]
    freeze_blocks = phase_cfg["freeze_blocks"]
    model.freeze_early_layers(freeze_blocks)
    trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
    print(f"Frozen for {freeze_epochs} epochs. Trainable: {trainable:,}")

    # Training setup
    criterion = nn.CrossEntropyLoss(label_smoothing=phase_cfg["label_smoothing"])
    epochs = phase_cfg["epochs"]
    warmup = phase_cfg["warmup_epochs"]
    mixup_alpha = phase_cfg["mixup_alpha"]

    # Create optimizer with ALL params (frozen ones will have requires_grad=False,
    # AdamW skips them). This avoids needing to recreate optimizer after unfreeze.
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=phase_cfg["learning_rate"],
        weight_decay=phase_cfg["weight_decay"],
    )

    def lr_lambda(epoch):
        if epoch < warmup:
            return max(epoch, 1) / warmup
        progress = (epoch - warmup) / max(epochs - warmup, 1)
        return 0.5 * (1.0 + np.cos(np.pi * progress))

    scheduler = torch.optim.lr_scheduler.LambdaLR(optimizer, lr_lambda)

    # Training loop
    best_val_acc = 0.0
    best_state = None
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    for epoch in range(epochs):
        # Unfreeze after freeze_epochs
        if epoch == freeze_epochs:
            model.unfreeze_all()
            trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
            print(f"\nUnfreezing all layers. Trainable: {trainable:,}")

        train_loss, train_acc = train_epoch(
            model, train_loader, criterion, optimizer, device,
            mixup_alpha=mixup_alpha,
        )
        val_loss, val_acc = evaluate(model, val_loader, criterion, device)
        scheduler.step()

        if (epoch + 1) % 5 == 0 or epoch == 0:
            lr = optimizer.param_groups[0]["lr"]
            print(f"Epoch {epoch+1:3d}/{epochs} | "
                  f"Train: {train_loss:.4f} / {train_acc:.3f} | "
                  f"Val: {val_loss:.4f} / {val_acc:.3f} | "
                  f"LR: {lr:.6f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            best_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}

    print(f"\nBest finetune val accuracy: {best_val_acc:.3f}")

    # Save best model
    save_path = output_dir / "teacher_finetune_best.pt"
    torch.save({
        "model_state_dict": best_state,
        "num_classes": num_classes,
        "channels": teacher_cfg["channels"],
        "best_val_acc": best_val_acc,
        "phase": "finetune",
    }, save_path)
    print(f"Saved to {save_path}")


def main():
    parser = argparse.ArgumentParser(description="Teacher model training")
    parser.add_argument("--phase", type=str, required=True,
                        choices=["pretrain", "finetune"],
                        help="Training phase")
    parser.add_argument("--data-dir", type=Path, required=True,
                        help="Training data directory")
    parser.add_argument("--val-dir", type=Path, default=None,
                        help="Validation data directory (finetune phase)")
    parser.add_argument("--pretrained", type=Path, default=None,
                        help="Pretrained checkpoint to load (finetune phase)")
    parser.add_argument("--output-dir", type=Path, default=Path("checkpoints"),
                        help="Output directory for checkpoints")
    parser.add_argument("--device", type=str, default=None)
    args = parser.parse_args()

    if args.phase == "pretrain":
        phase_pretrain(args)
    elif args.phase == "finetune":
        phase_finetune(args)


if __name__ == "__main__":
    main()
