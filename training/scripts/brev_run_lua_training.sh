#!/bin/bash
# One-shot script to run Lua QLoRA fine-tuning on a Brev L40S instance.
# Paste this entire script into `brev shell clawedcommand-voice-training`
# or any Brev instance with an L40S 48GB GPU.
#
# Estimated: ~1.5 hrs, ~$4

set -euo pipefail

echo "=== ClawedCommand Lua QLoRA Training ==="
echo "Started: $(date)"

# 1. Check GPU
echo ""
echo "--- GPU Check ---"
nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
GPU_MEM=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits | head -1 | tr -d ' ')
echo "GPU VRAM: ${GPU_MEM}MB"
if [ "$GPU_MEM" -lt 40000 ]; then
    echo "ERROR: Need at least 40GB VRAM for QLoRA. Got ${GPU_MEM}MB."
    exit 1
fi

# 2. Clone repo (or pull if exists)
echo ""
echo "--- Repo Setup ---"
cd ~
if [ -d "ClawedCommand" ]; then
    echo "Repo exists, pulling latest..."
    cd ClawedCommand
    git pull
else
    echo "Cloning repo..."
    git clone https://github.com/discordwell/ClawedCommand.git
    cd ClawedCommand
fi

# 3. Install Python deps
echo ""
echo "--- Python Dependencies ---"
pip install --quiet --upgrade pip
pip install --quiet \
    unsloth \
    "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git" \
    trl \
    transformers \
    datasets \
    accelerate \
    bitsandbytes \
    peft \
    torch \
    sentencepiece \
    protobuf

# 4. Validate training data
echo ""
echo "--- Validating Training Data ---"
cd training
python scripts/validate_lua_data.py data/combined_lua.jsonl
echo ""
python scripts/eval_lua.py data/combined_lua.jsonl 2>&1 | head -15

# 5. Check if train_unsloth.py exists, if not create a minimal one
if [ ! -f "scripts/train_unsloth.py" ]; then
    echo ""
    echo "--- Creating train_unsloth.py ---"
    cat > scripts/train_unsloth.py << 'TRAINEOF'
#!/usr/bin/env python3
"""Train Devstral Small 2 with QLoRA using Unsloth + TRL SFTTrainer."""

import argparse
import json
import yaml
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="QLoRA training with Unsloth")
    parser.add_argument("config", type=Path, help="YAML config file")
    parser.add_argument("--dry-run", action="store_true", help="Verify config without training")
    args = parser.parse_args()

    with open(args.config) as f:
        cfg = yaml.safe_load(f)

    print(f"Config: {args.config}")
    print(f"Model: {cfg['model']['name']}")
    print(f"Train file: {cfg['data']['train_file']}")
    print(f"Output dir: {cfg['training']['output_dir']}")

    # Resolve data paths relative to config dir
    config_dir = args.config.parent
    train_path = config_dir / cfg["data"]["train_file"]
    eval_path = config_dir / cfg["data"]["eval_file"]

    if not train_path.exists():
        raise FileNotFoundError(f"Training data not found: {train_path}")

    train_lines = sum(1 for _ in open(train_path))
    print(f"Training examples: {train_lines}")

    if args.dry_run:
        print("\n--- Dry run complete, config is valid ---")
        return

    # Import heavy deps only when actually training
    from unsloth import FastLanguageModel
    from trl import SFTTrainer, SFTConfig
    from datasets import Dataset
    import torch

    # Load model with QLoRA
    print("\n--- Loading model ---")
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=cfg["model"]["name"],
        max_seq_length=cfg["model"]["max_seq_len"],
        load_in_4bit=cfg["model"].get("load_in_4bit", True),
        dtype=getattr(torch, cfg["model"].get("dtype", "bfloat16")),
    )

    # Apply LoRA
    print("--- Applying LoRA ---")
    lora_cfg = cfg["lora"]
    model = FastLanguageModel.get_peft_model(
        model,
        r=lora_cfg["rank"],
        lora_alpha=lora_cfg["alpha"],
        lora_dropout=lora_cfg["dropout"],
        target_modules=lora_cfg["target_modules"],
        use_gradient_checkpointing="unsloth",
    )

    # Load training data
    print("--- Loading training data ---")
    train_data = []
    with open(train_path) as f:
        for line in f:
            if line.strip():
                example = json.loads(line)
                messages = example["messages"]
                # Format as chat using tokenizer
                text = tokenizer.apply_chat_template(
                    messages, tokenize=False, add_generation_prompt=False
                )
                train_data.append({"text": text})

    eval_data = []
    if eval_path.exists():
        with open(eval_path) as f:
            for line in f:
                if line.strip():
                    example = json.loads(line)
                    messages = example["messages"]
                    text = tokenizer.apply_chat_template(
                        messages, tokenize=False, add_generation_prompt=False
                    )
                    eval_data.append({"text": text})

    train_dataset = Dataset.from_list(train_data)
    eval_dataset = Dataset.from_list(eval_data) if eval_data else None

    print(f"Train examples: {len(train_dataset)}")
    if eval_dataset:
        print(f"Eval examples: {len(eval_dataset)}")

    # Training config
    tcfg = cfg["training"]
    output_dir = tcfg["output_dir"]

    training_args = SFTConfig(
        output_dir=output_dir,
        per_device_train_batch_size=tcfg["per_device_batch_size"],
        gradient_accumulation_steps=tcfg["gradient_accumulation_steps"],
        num_train_epochs=tcfg["num_epochs"],
        learning_rate=tcfg["learning_rate"],
        warmup_ratio=tcfg["warmup_ratio"],
        weight_decay=tcfg["weight_decay"],
        lr_scheduler_type=tcfg["lr_scheduler_type"],
        bf16=tcfg.get("bf16", True),
        seed=tcfg.get("seed", 42),
        logging_steps=tcfg.get("logging_steps", 10),
        eval_steps=tcfg.get("eval_steps", 50) if eval_dataset else None,
        eval_strategy="steps" if eval_dataset else "no",
        save_steps=tcfg.get("save_steps", 100),
        max_grad_norm=tcfg.get("max_grad_norm", 1.0),
        optim=tcfg.get("optimizer", "adamw_8bit"),
        max_seq_length=cfg["model"]["max_seq_len"],
        dataset_text_field="text",
        report_to="none",
    )

    # Train
    print("\n--- Starting training ---")
    trainer = SFTTrainer(
        model=model,
        args=training_args,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        processing_class=tokenizer,
    )

    trainer.train()

    # Save
    print("\n--- Saving ---")
    adapter_dir = f"{output_dir}/final_adapter"
    merged_dir = f"{output_dir}/merged"

    model.save_pretrained(adapter_dir)
    tokenizer.save_pretrained(adapter_dir)
    print(f"Adapter saved to {adapter_dir}")

    # Save merged model
    print("Merging adapter into base model...")
    merged_model = model.merge_and_unload()
    merged_model.save_pretrained(merged_dir)
    tokenizer.save_pretrained(merged_dir)
    print(f"Merged model saved to {merged_dir}")

    print("\n=== Training complete ===")
    print(f"Adapter: {adapter_dir}")
    print(f"Merged:  {merged_dir}")


if __name__ == "__main__":
    main()
TRAINEOF
    chmod +x scripts/train_unsloth.py
fi

# 6. Dry run
echo ""
echo "--- Dry Run ---"
python scripts/train_unsloth.py configs/devstral_24b_lua_qlora.yaml --dry-run

# 7. Train
echo ""
echo "=== Starting QLoRA Training ==="
echo "This will take ~1.5 hours on L40S 48GB"
echo ""
python scripts/train_unsloth.py configs/devstral_24b_lua_qlora.yaml 2>&1 | tee training_log.txt

echo ""
echo "=== Training Complete ==="
echo "Finished: $(date)"
echo ""
echo "Adapter: outputs/devstral_24b_lua_qlora_v1/final_adapter/"
echo "Merged:  outputs/devstral_24b_lua_qlora_v1/merged/"
echo ""
echo "Next: tar -czf adapter.tar.gz outputs/devstral_24b_lua_qlora_v1/final_adapter/"
