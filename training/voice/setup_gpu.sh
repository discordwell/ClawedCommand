#!/bin/bash
# Setup script for NVIDIA GPU training instance (brev.nvidia.com)
#
# Run: bash setup_gpu.sh
#
# Installs all dependencies for the voice keyword spotting distillation pipeline.

set -e

echo "=== ClawedCommand Voice Training Setup ==="
echo "GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'not detected')"

# System packages
echo ""
echo "--- Installing system dependencies ---"
sudo apt-get update -qq
sudo apt-get install -y -qq ffmpeg sox libsndfile1 libsox-dev wget unzip git

# Python virtual environment
echo ""
echo "--- Setting up Python environment ---"
VENV_DIR="${WORKSPACE:-$HOME}/venv"
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi
source "$VENV_DIR/bin/activate"

# Core ML with CUDA
echo ""
echo "--- Installing PyTorch + CUDA ---"
pip install -q --upgrade pip
pip install -q torch torchaudio --index-url https://download.pytorch.org/whl/cu124

# Audio and training deps
echo ""
echo "--- Installing training dependencies ---"
pip install -q numpy pyyaml soundfile tensorboard tqdm
pip install -q "audiomentations[extras]>=0.35.0"
pip install -q onnx onnxruntime-gpu matplotlib

# TTS engines
echo ""
echo "--- Installing TTS engines ---"
pip install -q piper-tts || echo "WARNING: piper-tts install failed (may need manual setup)"
pip install -q bark || echo "WARNING: bark install failed (may need manual setup)"

# Download Speech Commands v2
echo ""
echo "--- Downloading Google Speech Commands v2 ---"
DATA_DIR="${WORKSPACE:-$HOME}/data"
mkdir -p "$DATA_DIR"
SC_DIR="$DATA_DIR/speech_commands_v2"
if [ ! -d "$SC_DIR" ] || [ -z "$(ls -A "$SC_DIR" 2>/dev/null)" ]; then
    mkdir -p "$SC_DIR"
    wget -q -P "$SC_DIR" http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz
    tar xzf "$SC_DIR/speech_commands_v0.02.tar.gz" -C "$SC_DIR"
    rm -f "$SC_DIR/speech_commands_v0.02.tar.gz"
    echo "Downloaded $(ls -d "$SC_DIR"/*/ 2>/dev/null | wc -l) word directories"
else
    echo "Speech Commands already present at $SC_DIR"
fi

echo ""
echo "=== Setup complete ==="
echo "Activate with: source $VENV_DIR/bin/activate"
echo "Run pipeline with: bash run_pipeline.sh"
