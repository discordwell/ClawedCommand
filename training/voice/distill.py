#!/usr/bin/env python3
"""Knowledge distillation: transfer teacher knowledge into TC-ResNet8 student.

Three-component loss:
  L = alpha * L_KD + (1 - alpha) * L_CE + beta * L_feature

  L_KD:      KL divergence between softened teacher/student distributions (scaled by T^2)
  L_CE:      Standard cross-entropy against hard labels
  L_feature: MSE between student pooled features and projected teacher features

Usage:
    python distill.py \
        --data-dir /workspace/data/unified/train \
        --val-dir /workspace/data/unified/val \
        --teacher /workspace/checkpoints/teacher_finetune_best.pt \
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

from model import TCResNet8, export_onnx
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


def get_temperature(epoch, total_epochs, cfg):
    """Get temperature for current epoch (optionally annealed)."""
    dist_cfg = cfg["distillation"]
    if dist_cfg.get("anneal_temperature", False):
        t_start = dist_cfg["temp_start"]
        t_end = dist_cfg["temp_end"]
        progress = epoch / max(1, total_epochs - 1)
        return t_start + (t_end - t_start) * progress
    return dist_cfg["temperature"]


def _register_feature_hook(model, pool_layer):
    """Register a forward hook on the pool layer to capture features.

    Returns a dict that will contain 'features' after each forward pass.
    This avoids running the network twice to get both logits and features.
    """
    captured = {}

    def hook_fn(module, input, output):
        captured["features"] = output.squeeze(2)  # [B, C]

    handle = pool_layer.register_forward_hook(hook_fn)
    return captured, handle


def distill_epoch(student, teacher, feature_proj,
                  loader, optimizer, device, T, alpha, beta,
                  student_hook, teacher_hook):
    """Run one distillation training epoch."""
    student.train()
    feature_proj.train()
    teacher.eval()

    total_loss = 0.0
    total_kd = 0.0
    total_ce = 0.0
    total_feat = 0.0
    correct = 0
    total = 0

    for mel, labels in loader:
        mel = mel.to(device)
        labels = labels.to(device)

        # Teacher forward (frozen) — hook captures features
        with torch.no_grad():
            teacher_logits = teacher(mel)
        teacher_features = teacher_hook["features"]

        # Student forward — hook captures features (single pass for both logits and features)
        student_logits = student(mel)
        student_features = student_hook["features"]

        # L_KD: KL divergence on softened distributions
        teacher_soft = F.softmax(teacher_logits / T, dim=-1)
        student_log_soft = F.log_softmax(student_logits / T, dim=-1)
        L_kd = F.kl_div(student_log_soft, teacher_soft, reduction="batchmean") * (T * T)

        # L_CE: hard label cross-entropy
        L_ce = F.cross_entropy(student_logits, labels)

        # L_feature: MSE between projected teacher features and student features
        teacher_proj = feature_proj(teacher_features.detach())
        L_feat = F.mse_loss(student_features, teacher_proj)

        # Combined loss
        loss = alpha * L_kd + (1 - alpha) * L_ce + beta * L_feat

        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        total_loss += loss.item() * mel.size(0)
        total_kd += L_kd.item() * mel.size(0)
        total_ce += L_ce.item() * mel.size(0)
        total_feat += L_feat.item() * mel.size(0)
        preds = student_logits.argmax(dim=1)
        correct += (preds == labels).sum().item()
        total += mel.size(0)

    n = max(total, 1)
    return {
        "loss": total_loss / n,
        "kd": total_kd / n,
        "ce": total_ce / n,
        "feat": total_feat / n,
        "acc": correct / n,
    }


def evaluate(model, loader, device):
    """Evaluate student model accuracy."""
    model.eval()
    correct = 0
    total = 0
    total_loss = 0.0

    with torch.no_grad():
        for mel, labels in loader:
            mel = mel.to(device)
            labels = labels.to(device)

            logits = model(mel)
            loss = F.cross_entropy(logits, labels)

            total_loss += loss.item() * mel.size(0)
            preds = logits.argmax(dim=1)
            correct += (preds == labels).sum().item()
            total += mel.size(0)

    n = max(total, 1)
    return total_loss / n, correct / n


def main():
    parser = argparse.ArgumentParser(description="Knowledge distillation")
    parser.add_argument("--data-dir", type=Path, required=True,
                        help="Training data directory")
    parser.add_argument("--val-dir", type=Path, default=None,
                        help="Validation data directory")
    parser.add_argument("--teacher", type=Path, required=True,
                        help="Teacher checkpoint path")
    parser.add_argument("--output-dir", type=Path, default=Path("checkpoints"),
                        help="Output directory for checkpoints")
    parser.add_argument("--output-onnx", type=Path, default=None,
                        help="Also export final ONNX model to this path")
    parser.add_argument("--device", type=str, default=None)
    parser.add_argument("--epochs", type=int, default=None,
                        help="Override epoch count from config")
    args = parser.parse_args()

    cfg = load_config()
    base_cfg = load_base_config()
    dist_cfg = cfg["distillation"]
    train_cfg = cfg["training"]
    teacher_cfg = cfg["teacher"]
    student_cfg = cfg["student"]

    set_seed(train_cfg["seed"])
    device = get_device(args.device)
    print(f"Device: {device}")

    # Load game labels
    labels = load_labels("config.yaml")
    num_classes = len(labels)
    print(f"Classes: {num_classes}")

    # --- Load teacher ---
    checkpoint = torch.load(args.teacher, map_location="cpu", weights_only=True)
    teacher = TCResNet14Wide(
        n_mels=base_cfg["audio"]["n_mels"],
        num_classes=checkpoint["num_classes"],
        channels=checkpoint["channels"],
        dropout=0.0,  # No dropout during inference
    )
    teacher.load_state_dict(checkpoint["model_state_dict"])
    teacher = teacher.to(device)
    teacher.eval()
    for param in teacher.parameters():
        param.requires_grad = False

    teacher_params = sum(p.numel() for p in teacher.parameters())
    print(f"Teacher: {teacher_params:,} params (frozen)")
    print(f"Teacher val acc during training: {checkpoint.get('best_val_acc', 'N/A')}")

    # --- Create student ---
    student = TCResNet8(
        n_mels=base_cfg["audio"]["n_mels"],
        num_classes=num_classes,
        channels=student_cfg["channels"],
    ).to(device)

    student_params = sum(p.numel() for p in student.parameters())
    print(f"Student: {student_params:,} params")

    # Validate teacher/student class count match
    if checkpoint["num_classes"] != num_classes:
        raise ValueError(
            f"Teacher has {checkpoint['num_classes']} classes but game vocabulary "
            f"has {num_classes}. Did you pass a pretrain checkpoint instead of finetune?"
        )

    # Feature hint projector: teacher_dim → student_dim
    feature_proj = nn.Linear(
        teacher_cfg["feature_dim"],
        student_cfg["feature_dim"],
    ).to(device)

    # Register forward hooks to capture pool features in a single forward pass
    # (avoids running the network twice for logits + features)
    student_hook, student_hook_handle = _register_feature_hook(student, student.pool)
    teacher_hook, teacher_hook_handle = _register_feature_hook(teacher, teacher.pool)

    # --- Dataset ---
    dataset = VoiceCommandDataset(
        args.data_dir, labels=labels, config_path="config.yaml",
        augment=True, split="all",
    )

    if args.val_dir and args.val_dir.exists():
        val_dataset = VoiceCommandDataset(
            args.val_dir, labels=labels, config_path="config.yaml",
            augment=False, split="all",
        )
        train_dataset = dataset
    else:
        val_size = int(len(dataset) * 0.15)
        train_size = len(dataset) - val_size
        train_dataset, val_subset = random_split(
            dataset, [train_size, val_size],
            generator=torch.Generator().manual_seed(train_cfg["seed"]),
        )
        val_dataset = NoAugDataset(val_subset, dataset)

    train_loader = DataLoader(
        train_dataset, batch_size=train_cfg["batch_size"],
        shuffle=True, num_workers=0, pin_memory=True,
    )
    val_loader = DataLoader(
        val_dataset, batch_size=train_cfg["batch_size"],
        shuffle=False, num_workers=0, pin_memory=True,
    )

    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    # --- Optimizer (student + projector) ---
    epochs = args.epochs or train_cfg["epochs"]
    warmup = train_cfg["warmup_epochs"]
    alpha = dist_cfg["alpha"]
    beta = dist_cfg["beta"]

    optimizer = torch.optim.AdamW(
        list(student.parameters()) + list(feature_proj.parameters()),
        lr=train_cfg["learning_rate"],
        weight_decay=train_cfg["weight_decay"],
    )

    def lr_lambda(epoch):
        if epoch < warmup:
            return max(epoch, 1) / warmup
        progress = (epoch - warmup) / max(epochs - warmup, 1)
        return 0.5 * (1.0 + np.cos(np.pi * progress))

    scheduler = torch.optim.lr_scheduler.LambdaLR(optimizer, lr_lambda)

    # --- Distillation loop ---
    best_val_acc = 0.0
    best_state = None
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"\nDistillation: {epochs} epochs, alpha={alpha}, beta={beta}")
    print(f"Temperature: {'annealing' if dist_cfg.get('anneal_temperature') else dist_cfg['temperature']}")

    for epoch in range(epochs):
        T = get_temperature(epoch, epochs, cfg)

        metrics = distill_epoch(
            student, teacher, feature_proj,
            train_loader, optimizer, device, T, alpha, beta,
            student_hook, teacher_hook,
        )
        val_loss, val_acc = evaluate(student, val_loader, device)
        scheduler.step()

        if (epoch + 1) % 10 == 0 or epoch == 0:
            lr = optimizer.param_groups[0]["lr"]
            print(f"Epoch {epoch+1:3d}/{epochs} | "
                  f"Loss: {metrics['loss']:.4f} (KD={metrics['kd']:.3f} "
                  f"CE={metrics['ce']:.3f} Feat={metrics['feat']:.4f}) | "
                  f"Train Acc: {metrics['acc']:.3f} | "
                  f"Val: {val_loss:.4f} / {val_acc:.3f} | "
                  f"T={T:.2f} LR={lr:.6f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            best_state = {k: v.cpu().clone() for k, v in student.state_dict().items()}

    # Clean up forward hooks
    student_hook_handle.remove()
    teacher_hook_handle.remove()

    print(f"\nBest distilled student val accuracy: {best_val_acc:.3f}")

    # Save best student checkpoint
    save_path = output_dir / "distilled_student_best.pt"
    torch.save({
        "model_state_dict": best_state,
        "num_classes": num_classes,
        "channels": student_cfg["channels"],
        "best_val_acc": best_val_acc,
        "phase": "distilled",
    }, save_path)
    print(f"Saved checkpoint to {save_path}")

    # Optionally export ONNX
    if args.output_onnx:
        student.load_state_dict(best_state)
        student = student.cpu()
        args.output_onnx.parent.mkdir(parents=True, exist_ok=True)
        export_onnx(
            student, str(args.output_onnx),
            num_classes=num_classes,
            n_mels=base_cfg["audio"]["n_mels"],
            n_frames=base_cfg["audio"]["num_frames"],
        )
        import os
        size_kb = os.path.getsize(args.output_onnx) / 1024
        print(f"ONNX model: {args.output_onnx} ({size_kb:.1f} KB)")


if __name__ == "__main__":
    main()
