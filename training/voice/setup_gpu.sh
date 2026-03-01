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
VENV_DIR="${WORKSPACE:-/workspace}/venv"
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
DATA_DIR="${WORKSPACE:-/workspace}/data"
mkdir -p "$DATA_DIR"
python3 -c "
import torchaudio
print('Downloading Speech Commands v2...')
torchaudio.datasets.SPEECHCOMMANDS('$DATA_DIR', url='speech_commands_v2', download=True)
print('Done')
" || echo "WARNING: Speech Commands download failed — will retry in pipeline"

echo ""
echo "=== Setup complete ==="
echo "Activate with: source $VENV_DIR/bin/activate"
echo "Run pipeline with: bash run_pipeline.sh"
