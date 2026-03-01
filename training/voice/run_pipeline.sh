#!/bin/bash
# End-to-end voice keyword spotting distillation pipeline.
#
# Run on NVIDIA GPU instance after setup_gpu.sh.
# Total GPU time: ~5 hours (dominated by TTS generation)
#
# Usage:
#   bash run_pipeline.sh                    # Full pipeline
#   bash run_pipeline.sh --skip-tts         # Skip TTS if data exists
#   bash run_pipeline.sh --distill-only     # Only run distillation (data + teacher must exist)

set -e

WORKSPACE="${WORKSPACE:-$HOME}"
DATA_DIR="$WORKSPACE/data"
CKPT_DIR="$WORKSPACE/checkpoints"
EVAL_DIR="$WORKSPACE/eval_results"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FINAL_ONNX="$SCRIPT_DIR/../../assets/voice/keyword_classifier.onnx"

# Activate venv if it exists
if [ -f "$WORKSPACE/venv/bin/activate" ]; then
    source "$WORKSPACE/venv/bin/activate"
fi

export CUDA_VISIBLE_DEVICES=${CUDA_VISIBLE_DEVICES:-0}

cd "$SCRIPT_DIR"

SKIP_TTS=false
DISTILL_ONLY=false
for arg in "$@"; do
    case $arg in
        --skip-tts)     SKIP_TTS=true ;;
        --distill-only) DISTILL_ONLY=true ;;
    esac
done

mkdir -p "$CKPT_DIR" "$EVAL_DIR"

# ============================================================
# Step 1: Data Pipeline
# ============================================================
if [ "$DISTILL_ONLY" = false ]; then
    echo ""
    echo "=========================================="
    echo "  Step 1: Data Pipeline"
    echo "=========================================="

    # Download Speech Commands
    echo ""
    echo "--- 1a: Download Speech Commands v2 ---"
    python data_pipeline.py --stage download --output-dir "$DATA_DIR"

    # Prepare pretrain dataset
    echo ""
    echo "--- 1b: Prepare pretrain dataset ---"
    python data_pipeline.py --stage pretrain --output-dir "$DATA_DIR"

    # GPU TTS generation
    if [ "$SKIP_TTS" = false ]; then
        echo ""
        echo "--- 1c: GPU TTS Generation (this takes a while) ---"
        python data_pipeline.py --stage tts --output-dir "$DATA_DIR"
    else
        echo ""
        echo "--- 1c: Skipping TTS generation (--skip-tts) ---"
    fi

    # Build unified dataset
    echo ""
    echo "--- 1d: Build unified dataset ---"
    python data_pipeline.py --stage unify --output-dir "$DATA_DIR"

    # ============================================================
    # Step 2: Pretrain Teacher on Speech Commands
    # ============================================================
    echo ""
    echo "=========================================="
    echo "  Step 2: Pretrain Teacher (~30 min)"
    echo "=========================================="
    python train_teacher.py --phase pretrain \
        --data-dir "$DATA_DIR/speech_commands_pretrain" \
        --output-dir "$CKPT_DIR" \
        --device cuda

    # ============================================================
    # Step 3: Fine-tune Teacher on Game Vocabulary
    # ============================================================
    echo ""
    echo "=========================================="
    echo "  Step 3: Fine-tune Teacher (~45 min)"
    echo "=========================================="
    python train_teacher.py --phase finetune \
        --data-dir "$DATA_DIR/unified/train" \
        --val-dir "$DATA_DIR/unified/val" \
        --pretrained "$CKPT_DIR/teacher_pretrain_best.pt" \
        --output-dir "$CKPT_DIR" \
        --device cuda
fi

# ============================================================
# Step 4: Knowledge Distillation
# ============================================================
echo ""
echo "=========================================="
echo "  Step 4: Knowledge Distillation (~60 min)"
echo "=========================================="
python distill.py \
    --data-dir "$DATA_DIR/unified/train" \
    --val-dir "$DATA_DIR/unified/val" \
    --teacher "$CKPT_DIR/teacher_finetune_best.pt" \
    --output-dir "$CKPT_DIR" \
    --device cuda

# ============================================================
# Step 5: Post-Distillation Fine-Tune
# ============================================================
echo ""
echo "=========================================="
echo "  Step 5: Student Fine-Tune (~15 min)"
echo "=========================================="
python train.py \
    --data-dir "$DATA_DIR/unified/train" \
    --pretrained "$CKPT_DIR/distilled_student_best.pt" \
    --output "$FINAL_ONNX" \
    --epochs 30 \
    --device cuda

# ============================================================
# Step 6: Evaluation
# ============================================================
echo ""
echo "=========================================="
echo "  Step 6: Evaluation"
echo "=========================================="
python evaluate.py \
    --model "$FINAL_ONNX" \
    --test-dir "$DATA_DIR/unified/test" \
    --output-dir "$EVAL_DIR"

echo ""
echo "=========================================="
echo "  Pipeline Complete!"
echo "=========================================="
echo "  Model:   $FINAL_ONNX"
echo "  Results: $EVAL_DIR/eval_report.json"
echo "  Size:    $(du -h "$FINAL_ONNX" 2>/dev/null | cut -f1 || echo 'N/A')"
echo "=========================================="
