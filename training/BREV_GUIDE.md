# Brev Training Guide

> Fine-tuning Devstral Small 2 (24B) on [NVIDIA Brev](https://brev.nvidia.com) for Lua script generation with a $100 GPU budget.

## Strategy

**QLoRA iteration on Devstral Small 2 for Lua script generation, then a final full LoRA run.**

- QLoRA runs on **L40S 48GB** (~$1-1.50/hr) — cheap, fast, good for rapid iteration
- Final LoRA run on **A100 80GB** (~$2-2.50/hr) — full precision, best quality

## Budget Breakdown

| Phase | GPU | Est. $/hr | Hours | Cost |
|-------|-----|-----------|-------|------|
| First QLoRA run | L40S | ~$1.25 | ~1.5 | ~$4 |
| Iterate (3-5 runs) | L40S | ~$1.25 | ~6 | ~$15 |
| Final LoRA on A100 | A100 80GB | ~$2.50 | ~2 | ~$5 |
| Eval + debug buffer | L40S | ~$1.25 | ~5 | ~$6 |
| **Total planned** | | | | **~$30** |
| **Remaining buffer** | | | | **~$70** |

## Config → GPU Mapping

| Config File | GPU | VRAM Needed | Est. Time |
|-------------|-----|-------------|-----------|
| `devstral_24b_lua_qlora.yaml` | L40S 48GB | ~20GB | ~1.5 hrs |
| `devstral_24b_lora.yaml` | A100 80GB | ~55GB | ~2 hrs |

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
./scripts/brev_setup.sh configs/devstral_24b_lua_qlora.yaml
```

The setup script will:
- Verify GPU detection and VRAM
- Install Python dependencies
- Pre-download model weights (cached for reuse)
- Dry-run the config to verify parsing

### 3. Validate Training Data

Always validate before spending GPU hours:

```bash
python scripts/validate_lua_data.py data/gold_lua_examples.jsonl
```

### 4. Run Training

Start with a QLoRA run, then iterate on data/hyperparams:

```bash
# First QLoRA run (~$4, ~1.5 hrs on L40S)
python scripts/train_unsloth.py configs/devstral_24b_lua_qlora.yaml
```

Iterate 3-5 times, adjusting learning rate, data mix, or training steps between runs.

### 5. Evaluate

```bash
python scripts/eval_lua.py data/gold_lua_examples.jsonl \
    --model vllm::http://localhost:8080/v1 \
    --output results.json
```

Compare: Lua syntax correctness, ScriptContext API usage, multi-step behavior composition, latency.

### 6. Final LoRA Run

Once QLoRA iterations converge on good eval scores:

1. **Stop the L40S instance** (save money)
2. Launch a new **A100 80GB** instance
3. Run setup and final training:

```bash
./scripts/brev_setup.sh configs/devstral_24b_lora.yaml
python scripts/train_unsloth.py configs/devstral_24b_lora.yaml
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
- **Re-use model caches** — keep the instance alive between iteration runs (cheaper than re-downloading Devstral weights)
- **Dry-run first** — `python scripts/train_unsloth.py config.yaml --dry-run` catches config errors before GPU hours are spent
- **Monitor GPU** — `watch -n 1 nvidia-smi` in a separate terminal to catch OOM early

## Troubleshooting

### OOM on L40S with Devstral QLoRA
- Reduce `per_device_batch_size` from 4 to 2 in `devstral_24b_lua_qlora.yaml`
- Increase `gradient_accumulation_steps` from 8 to 16 to maintain effective batch size

### Slow download on Brev
- HuggingFace model downloads can be slow on some Brev instances
- Use `HF_HUB_ENABLE_HF_TRANSFER=1 pip install hf-transfer` for faster downloads

### Training loss not decreasing
- Check data quality with `validate_lua_data.py`
- Try lower learning rate (1.0e-4 instead of 2.0e-4)
- Verify chat template matches Devstral's mistral format
