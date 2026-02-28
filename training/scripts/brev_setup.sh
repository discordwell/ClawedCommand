#!/usr/bin/env bash
# brev_setup.sh — One-shot setup for a fresh Brev GPU instance
#
# Usage:
#   chmod +x brev_setup.sh
#   ./brev_setup.sh [model_config]
#
# Examples:
#   ./brev_setup.sh                                  # Setup only, no model download
#   ./brev_setup.sh ../configs/xlam_8b_qlora.yaml    # Setup + cache xLAM weights
#   ./brev_setup.sh ../configs/qwen_32b_qlora.yaml   # Setup + cache Qwen weights
#
# Expected environment: Brev instance with NVIDIA GPU (L40S 48GB or A100 80GB)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRAINING_DIR="$(dirname "$SCRIPT_DIR")"

echo "========================================"
echo "  ClawedCommand Brev Training Setup"
echo "========================================"
echo ""

# --- Step 1: Check GPU ---
echo "[1/5] Checking GPU..."
if ! command -v nvidia-smi &>/dev/null; then
    echo "ERROR: nvidia-smi not found. Is this a GPU instance?"
    exit 1
fi

GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -1)
GPU_VRAM=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits | head -1)
CUDA_VERSION=$(nvidia-smi | grep "CUDA Version" | awk '{print $NF}')

echo "  GPU: $GPU_NAME"
echo "  VRAM: ${GPU_VRAM} MiB"
echo "  CUDA: $CUDA_VERSION"

if [ "$GPU_VRAM" -lt 40000 ]; then
    echo "WARNING: GPU has less than 40GB VRAM. 32B QLoRA may not fit."
    echo "         Use xlam_8b_qlora.yaml or devstral_24b_qlora.yaml instead."
fi
echo ""

# --- Step 2: Install Python dependencies ---
echo "[2/5] Installing Python dependencies..."
pip install -r "$TRAINING_DIR/requirements.txt" --quiet
echo "  Done."
echo ""

# --- Step 3: Verify imports ---
echo "[3/5] Verifying Python imports..."
python3 -c "
import unsloth; print(f'  unsloth: {unsloth.__version__}')
import trl; print(f'  trl: {trl.__version__}')
import datasets; print(f'  datasets: {datasets.__version__}')
import yaml; print(f'  pyyaml: OK')
import torch
print(f'  torch: {torch.__version__}')
print(f'  CUDA available: {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'  CUDA device: {torch.cuda.get_device_name(0)}')
    vram_gb = torch.cuda.get_device_properties(0).total_mem / 1e9
    print(f'  VRAM (torch): {vram_gb:.1f} GB')
"
echo ""

# --- Step 4: Download model weights (optional) ---
if [ -n "${1:-}" ]; then
    CONFIG_FILE="$1"
    echo "[4/5] Pre-downloading model weights from config: $CONFIG_FILE"

    # Extract model name from YAML config
    MODEL_NAME=$(python3 -c "
import yaml
with open('$CONFIG_FILE') as f:
    cfg = yaml.safe_load(f)
print(cfg['model']['name'])
")
    echo "  Model: $MODEL_NAME"
    echo "  Downloading to HuggingFace cache (reusable across runs)..."

    python3 -c "
from huggingface_hub import snapshot_download
snapshot_download('$MODEL_NAME', ignore_patterns=['*.gguf', '*.ggml'])
print('  Download complete.')
"
else
    echo "[4/5] Skipping model download (no config specified)"
    echo "  Run with a config file to pre-cache: ./brev_setup.sh ../configs/xlam_8b_qlora.yaml"
fi
echo ""

# --- Step 5: Smoke test ---
echo "[5/5] Running smoke test (dry-run on config)..."
if [ -n "${1:-}" ]; then
    python3 "$SCRIPT_DIR/train_unsloth.py" "$1" --dry-run
    echo "  Config parsed successfully."
else
    # Test with the smallest config if no arg provided
    SMOKE_CONFIG="$TRAINING_DIR/configs/xlam_8b_qlora.yaml"
    if [ -f "$SMOKE_CONFIG" ]; then
        python3 "$SCRIPT_DIR/train_unsloth.py" "$SMOKE_CONFIG" --dry-run
        echo "  Config parsed successfully."
    else
        echo "  No config found for smoke test. Skipping."
    fi
fi
echo ""

echo "========================================"
echo "  Setup complete!"
echo ""
echo "  Next steps:"
echo "    1. Validate training data:"
echo "       python scripts/validate_data.py data/cc_train_mistral.jsonl"
echo ""
echo "    2. Start training (cheapest first):"
echo "       python scripts/train_unsloth.py configs/xlam_8b_qlora.yaml"
echo ""
echo "    3. Monitor GPU usage:"
echo "       watch -n 1 nvidia-smi"
echo "========================================"
