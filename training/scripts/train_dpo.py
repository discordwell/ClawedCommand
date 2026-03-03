#!/usr/bin/env python3
"""DPO (Direct Preference Optimization) training for Devstral Small 2.

Starts from an SFT checkpoint and trains using preference pairs where:
- Chosen: arena-validated strategies (high win rate)
- Rejected: anti-patterns (low win rate)

Usage:
  python training/scripts/train_dpo.py --config training/configs/devstral_24b_dpo.yaml

  # Override config values
  python training/scripts/train_dpo.py --config training/configs/devstral_24b_dpo.yaml \
    --lr 1e-5 --epochs 2

Environment:
  CUDA must be available. Tested on L40S 48GB and A100 80GB.
"""

import argparse
import json
import os
import sys
from pathlib import Path

import yaml


def load_config(path: Path) -> dict:
    """Load YAML config file."""
    with open(path) as f:
        return yaml.safe_load(f)


def load_dpo_dataset(path: Path) -> list[dict]:
    """Load DPO dataset from JSONL.

    Expected format:
    {
        "prompt": [{"role": "system", ...}, {"role": "user", ...}],
        "chosen": [{"role": "assistant", "content": "..."}],
        "rejected": [{"role": "assistant", "content": "..."}]
    }
    """
    examples = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))
    return examples


def format_for_trl(examples: list[dict], tokenizer) -> dict:
    """Convert our DPO format to TRL DPOTrainer format.

    TRL expects:
    - prompt: str (the conversation up to the assistant turn)
    - chosen: str (the preferred response)
    - rejected: str (the dispreferred response)
    """
    prompts = []
    chosen_list = []
    rejected_list = []

    for ex in examples:
        # Build prompt from messages
        prompt_msgs = ex.get("prompt", [])
        prompt_text = tokenizer.apply_chat_template(
            prompt_msgs, tokenize=False, add_generation_prompt=True
        )

        # Chosen and rejected are just the assistant text
        chosen_msgs = ex.get("chosen", [])
        rejected_msgs = ex.get("rejected", [])

        chosen_text = chosen_msgs[0]["content"] if chosen_msgs else ""
        rejected_text = rejected_msgs[0]["content"] if rejected_msgs else ""

        if chosen_text and rejected_text:
            prompts.append(prompt_text)
            chosen_list.append(chosen_text)
            rejected_list.append(rejected_text)

    return {
        "prompt": prompts,
        "chosen": chosen_list,
        "rejected": rejected_list,
    }


def main():
    parser = argparse.ArgumentParser(description="DPO training for Devstral Small 2")
    parser.add_argument("--config", type=Path, required=True, help="YAML config file")
    parser.add_argument("--lr", type=float, help="Override learning rate")
    parser.add_argument("--epochs", type=int, help="Override num epochs")
    parser.add_argument("--beta", type=float, help="Override DPO beta")
    parser.add_argument("--dry-run", action="store_true", help="Show config without training")
    args = parser.parse_args()

    config = load_config(args.config)

    # Apply overrides
    if args.lr:
        config["training"]["learning_rate"] = args.lr
    if args.epochs:
        config["training"]["num_epochs"] = args.epochs
    if args.beta:
        config["dpo"]["beta"] = args.beta

    print("=== DPO Training Configuration ===")
    print(f"Model: {config['model']['name']}")
    print(f"DPO beta: {config['dpo']['beta']}")
    print(f"LR: {config['training']['learning_rate']}")
    print(f"Epochs: {config['training']['num_epochs']}")
    print(f"Batch: {config['training']['per_device_batch_size']} × "
          f"{config['training']['gradient_accumulation_steps']} = "
          f"{config['training']['per_device_batch_size'] * config['training']['gradient_accumulation_steps']}")
    print(f"Train: {config['data']['train_file']}")
    print(f"Eval: {config['data']['eval_file']}")
    print(f"Output: {config['training']['output_dir']}")

    if args.dry_run:
        print("\n--- Dry run, not training ---")
        return

    # Check CUDA
    try:
        import torch
        if not torch.cuda.is_available():
            print("Error: CUDA not available", file=sys.stderr)
            sys.exit(1)
        print(f"\nGPU: {torch.cuda.get_device_name(0)}")
        print(f"VRAM: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")
    except ImportError:
        print("Error: PyTorch not installed", file=sys.stderr)
        sys.exit(1)

    # Import training libraries
    try:
        from transformers import AutoTokenizer, AutoModelForCausalLM, BitsAndBytesConfig
        from peft import LoraConfig, get_peft_model, PeftModel
        from trl import DPOTrainer, DPOConfig
        from datasets import Dataset
    except ImportError as e:
        print(f"Error: Missing dependency: {e}", file=sys.stderr)
        print("Install: pip install transformers peft trl datasets bitsandbytes",
              file=sys.stderr)
        sys.exit(1)

    # Load tokenizer
    model_name = config["model"]["name"]
    base_model = config.get("ref_model", {}).get("name", "mistralai/Devstral-Small-2-24B-Instruct-2512")

    print(f"\nLoading tokenizer from {base_model}...")
    tokenizer = AutoTokenizer.from_pretrained(base_model)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    # Load datasets
    config_dir = args.config.parent
    train_path = config_dir / config["data"]["train_file"]
    eval_path = config_dir / config["data"]["eval_file"]

    print(f"Loading training data from {train_path}...")
    train_raw = load_dpo_dataset(train_path)
    eval_raw = load_dpo_dataset(eval_path)
    print(f"  Train: {len(train_raw)} pairs")
    print(f"  Eval: {len(eval_raw)} pairs")

    # Format for TRL
    train_data = format_for_trl(train_raw, tokenizer)
    eval_data = format_for_trl(eval_raw, tokenizer)

    train_dataset = Dataset.from_dict(train_data)
    eval_dataset = Dataset.from_dict(eval_data)

    # Load model with quantization
    print(f"\nLoading model from {model_name}...")
    bnb_config = None
    if config["model"].get("load_in_4bit", False):
        bnb_config = BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_quant_type="nf4",
            bnb_4bit_compute_dtype=torch.bfloat16,
            bnb_4bit_use_double_quant=True,
        )

    # Check if model_name is a LoRA adapter or a full model
    model_path = Path(model_name)
    is_adapter = (model_path / "adapter_config.json").exists() if model_path.exists() else False

    if is_adapter:
        print(f"  Detected LoRA adapter, loading base model first from {base_model}...")
        model = AutoModelForCausalLM.from_pretrained(
            base_model,
            quantization_config=bnb_config,
            torch_dtype=torch.bfloat16,
            device_map="auto",
        )
        model = PeftModel.from_pretrained(model, model_name)
        # Merge SFT adapter so we can apply a fresh DPO LoRA on top
        model = model.merge_and_unload()
    else:
        model = AutoModelForCausalLM.from_pretrained(
            model_name,
            quantization_config=bnb_config,
            torch_dtype=torch.bfloat16,
            device_map="auto",
        )

    # Apply fresh LoRA for DPO training
    lora_config = LoraConfig(
        r=config["lora"]["rank"],
        lora_alpha=config["lora"]["alpha"],
        lora_dropout=config["lora"]["dropout"],
        target_modules=config["lora"]["target_modules"],
        bias="none",
        task_type="CAUSAL_LM",
    )

    model = get_peft_model(model, lora_config)
    trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
    total = sum(p.numel() for p in model.parameters())
    print(f"Trainable: {trainable:,} / {total:,} ({trainable/total*100:.2f}%)")

    # Load reference model (for DPO KL divergence)
    print(f"\nLoading reference model from {base_model}...")
    ref_model = AutoModelForCausalLM.from_pretrained(
        base_model,
        quantization_config=bnb_config,
        torch_dtype=torch.bfloat16,
        device_map="auto",
    )

    # DPO training arguments
    dpo_cfg = config["dpo"]
    train_cfg = config["training"]

    training_args = DPOConfig(
        optim=train_cfg.get("optimizer", "adamw_torch"),
        output_dir=train_cfg["output_dir"],
        num_train_epochs=train_cfg["num_epochs"],
        per_device_train_batch_size=train_cfg["per_device_batch_size"],
        gradient_accumulation_steps=train_cfg["gradient_accumulation_steps"],
        learning_rate=train_cfg["learning_rate"],
        warmup_ratio=train_cfg["warmup_ratio"],
        weight_decay=train_cfg["weight_decay"],
        lr_scheduler_type=train_cfg["lr_scheduler_type"],
        bf16=train_cfg["bf16"],
        seed=train_cfg["seed"],
        logging_steps=train_cfg["logging_steps"],
        eval_strategy="steps",
        eval_steps=train_cfg["eval_steps"],
        save_steps=train_cfg["save_steps"],
        max_grad_norm=train_cfg["max_grad_norm"],
        gradient_checkpointing=train_cfg.get("gradient_checkpointing", True),
        beta=dpo_cfg["beta"],
        loss_type=dpo_cfg.get("loss_type", "sigmoid"),
        label_smoothing=dpo_cfg.get("label_smoothing", 0.0),
        max_prompt_length=dpo_cfg.get("max_prompt_length", 2048),
        max_length=dpo_cfg.get("max_length", 8192),
        remove_unused_columns=False,
    )

    # Create trainer
    print("\nInitializing DPO trainer...")
    trainer = DPOTrainer(
        model=model,
        ref_model=ref_model,
        args=training_args,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        processing_class=tokenizer,
    )

    # Train
    print("\n=== Starting DPO Training ===")
    trainer.train()

    # Save
    print("\nSaving model...")
    trainer.save_model()
    tokenizer.save_pretrained(train_cfg["output_dir"])

    print(f"\n{'='*60}")
    print(f"DPO Training Complete")
    print(f"{'='*60}")
    print(f"Output: {train_cfg['output_dir']}")
    print(f"\nNext steps:")
    print(f"  1. Evaluate on held-out prompts")
    print(f"  2. Run arena matches with the fine-tuned model")
    print(f"  3. Compare win rates vs SFT-only checkpoint")


if __name__ == "__main__":
    main()
