# Brev Training Guide

> Fine-tuning ClawedCommand AI agents on [NVIDIA Brev](https://brev.nvidia.com) with a $100 GPU budget.

## Strategy

**QLoRA for fast iteration, then one final full LoRA run on the winner.**

- QLoRA runs on **L40S 48GB** (~$1-1.50/hr) — cheap, fast, good enough for model comparison
- Final LoRA run on **A100 80GB** (~$2-2.50/hr) — full precision, best quality on the winning model

## Budget Breakdown

| Phase | GPU | Est. $/hr | Hours | Cost |
|-------|-----|-----------|-------|------|
| Codestral API baseline (x2 runs) | — | — | — | $10 (Mistral credits) |
| xLAM-2-8B QLoRA (x2 runs) | L40S | ~$1.25 | ~1.5 | ~$4 |
| Devstral-24B QLoRA (x2 runs) | L40S | ~$1.25 | ~3 | ~$8 |
| Qwen-32B QLoRA (x2 runs) | L40S | ~$1.25 | ~5 | ~$12 |
| Eval runs (all models) | L40S | ~$1.25 | ~3 | ~$8 |
| **Final LoRA on winner** | **A100 80GB** | ~$2.50 | ~3 | ~$8 |
| Setup / debug buffer | L40S | ~$1.25 | ~8 | ~$10 |
| **Total planned** | | | | **~$50** |
| **Remaining buffer** | | | | **~$50** |

## Config → GPU Mapping

| Config File | GPU | VRAM Needed | Est. Time |
|-------------|-----|-------------|-----------|
| `xlam_8b_qlora.yaml` | L40S 48GB | ~12GB | ~45 min |
| `devstral_24b_qlora.yaml` | L40S 48GB | ~20GB | ~1.5 hrs |
| `qwen_32b_qlora.yaml` | L40S 48GB | ~28GB | ~2.5 hrs |
| `xlam_8b_lora.yaml` | A100 80GB | ~20GB | ~1 hr |
| `devstral_24b_lora.yaml` | A100 80GB | ~55GB | ~2 hrs |
| `qwen_32b_lora.yaml` | A100 80GB | ~70GB | ~3 hrs |

## Step-by-Step

### 1. Create a Brev Instance

1. Go to [brev.nvidia.com](https://brev.nvidia.com)
2. Launch a new instance:
   - **GPU**: L40S 48GB (for QLoRA runs) or A100 80GB (for final LoRA)
   - **Image**: Use a pre-built Unsloth launchable if available (`brevdev/unsloth-notebook-adaptor`), or any PyTorch + CUDA 12.4 image
   - **Disk**: 100GB minimum (model weights are large)
3. SSH into the instance

### 2. Clone and Setup

```bash
git clone <your-repo-url>  # e.g. git@github.com:org/ClawedCommand.git
cd ClawedCommand/training

# Run the one-shot setup script
chmod +x scripts/brev_setup.sh
./scripts/brev_setup.sh configs/xlam_8b_qlora.yaml
```

The setup script will:
- Verify GPU detection and VRAM
- Install Python dependencies
- Pre-download model weights (cached for reuse)
- Dry-run the config to verify parsing

### 3. Validate Training Data

Always validate before spending GPU hours:

```bash
python scripts/validate_data.py data/cc_train_mistral.jsonl
python scripts/validate_data.py data/cc_train_qwen.jsonl
python scripts/validate_data.py data/cc_train_xlam.jsonl
```

### 4. Run Training (Budget-Aware Order)

Train cheapest-first to catch issues early:

```bash
# Step 1: Codestral API baseline ($10 Mistral credits, no GPU)
export MISTRAL_API_KEY=your_key
python scripts/train_mistral_api.py data/cc_train_mistral.jsonl data/cc_eval_mistral.jsonl \
    --model codestral-latest --steps 300

# Step 2: xLAM-2-8B QLoRA (~$2, ~45 min on L40S)
python scripts/train_unsloth.py configs/xlam_8b_qlora.yaml

# Step 3: Devstral-24B QLoRA (~$4, ~1.5 hrs on L40S)
python scripts/train_unsloth.py configs/devstral_24b_qlora.yaml

# Step 4: Qwen-32B QLoRA (~$6, ~2.5 hrs on L40S)
python scripts/train_unsloth.py configs/qwen_32b_qlora.yaml
```

### 5. Evaluate All Models

```bash
# Run eval harness on all adapters + API baselines
python scripts/evaluate.py data/cc_eval_mistral.jsonl \
    --model vllm::http://localhost:8080/v1 \
    --model mistral::codestral-latest \
    --model mistral::ft:codestral-latest:xxx:20260228 \
    --output results.json
```

Compare: tool call accuracy, instruction following, multi-step completion, latency.

### 6. Final LoRA Run on Winner

Once you've picked the best model from QLoRA eval:

1. **Stop the L40S instance** (save money)
2. Launch a new **A100 80GB** instance
3. Run setup and final training:

```bash
# Example: if Qwen won
./scripts/brev_setup.sh configs/qwen_32b_lora.yaml
python scripts/train_unsloth.py configs/qwen_32b_lora.yaml
```

### 7. Export and Deploy

The trained adapter will be in `outputs/<model>/final_adapter/`. The merged full model is in `outputs/<model>/merged/`.

Download the adapter before terminating the instance:

```bash
# Compress and download
tar -czf adapter.tar.gz outputs/<model>/final_adapter/
# Use scp, rsync, or Brev's file transfer to pull it locally
```

## Budget Tracking Tips

- **Check Brev billing** after each run — actual costs may differ from estimates
- **Terminate instances** immediately after training completes — don't leave them running
- **Re-use model caches** — if you need multiple runs on the same model, keep the instance alive between runs (cheaper than re-downloading)
- **Start small** — xLAM at ~$2/run is your cheapest feedback loop
- **Dry-run first** — `python scripts/train_unsloth.py config.yaml --dry-run` catches config errors before GPU hours are spent
- **Monitor GPU** — `watch -n 1 nvidia-smi` in a separate terminal to catch OOM early

## Troubleshooting

### OOM on L40S with Qwen 32B QLoRA
- Reduce `per_device_batch_size` from 2 to 1 in `qwen_32b_qlora.yaml`
- Increase `gradient_accumulation_steps` from 16 to 32 to maintain effective batch size

### Slow download on Brev
- HuggingFace model downloads can be slow on some Brev instances
- Use `HF_HUB_ENABLE_HF_TRANSFER=1 pip install hf-transfer` for faster downloads

### Training loss not decreasing
- Check data quality with `validate_data.py`
- Try lower learning rate (1.0e-4 instead of 2.0e-4)
- Verify chat template matches model format (mistral/qwen/xlam)
