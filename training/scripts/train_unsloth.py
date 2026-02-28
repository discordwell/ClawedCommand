#!/usr/bin/env python3
"""Fine-tune models with Unsloth + TRL SFTTrainer.

Supports: Qwen2.5-Coder-32B, Devstral Small 2 (24B), xLAM-2-8B.
Reads config from a YAML file (see training/configs/).

Usage:
    python train_unsloth.py ../configs/qwen_32b_lora.yaml
    python train_unsloth.py ../configs/devstral_24b_lora.yaml
    python train_unsloth.py ../configs/xlam_8b_lora.yaml
"""

import argparse
import sys
from pathlib import Path

import yaml


def load_config(path: Path) -> dict:
    with open(path) as f:
        return yaml.safe_load(f)


def main():
    parser = argparse.ArgumentParser(description="Unsloth LoRA fine-tuning")
    parser.add_argument("config", type=Path, help="YAML config file")
    parser.add_argument(
        "--dry-run", action="store_true", help="Print config and exit"
    )
    args = parser.parse_args()

    cfg = load_config(args.config)

    if args.dry_run:
        print(yaml.dump(cfg, default_flow_style=False))
        return

    # Late imports — these are heavy and slow to load
    from unsloth import FastLanguageModel
    from trl import SFTTrainer, SFTConfig
    from datasets import load_dataset

    model_cfg = cfg["model"]
    lora_cfg = cfg["lora"]
    train_cfg = cfg["training"]
    data_cfg = cfg["data"]

    print(f"Loading model: {model_cfg['name']}")
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=model_cfg["name"],
        max_seq_length=model_cfg["max_seq_len"],
        load_in_4bit=model_cfg.get("load_in_4bit", False),
        dtype=None,  # Auto-detect
    )

    print(f"Applying LoRA: rank={lora_cfg['rank']}, alpha={lora_cfg['alpha']}")
    model = FastLanguageModel.get_peft_model(
        model,
        r=lora_cfg["rank"],
        lora_alpha=lora_cfg["alpha"],
        lora_dropout=lora_cfg["dropout"],
        target_modules=lora_cfg["target_modules"],
        bias="none",
        use_gradient_checkpointing="unsloth",  # Unsloth optimized
        random_state=train_cfg.get("seed", 42),
    )

    # Load datasets
    train_path = str(
        (args.config.parent / data_cfg["train_file"]).resolve()
    )
    eval_path = str(
        (args.config.parent / data_cfg["eval_file"]).resolve()
    )

    print(f"Loading train data: {train_path}")
    print(f"Loading eval data: {eval_path}")

    train_dataset = load_dataset("json", data_files=train_path, split="train")
    eval_dataset = load_dataset("json", data_files=eval_path, split="train")

    def format_example(example):
        """Apply chat template to convert messages+tools into a formatted string."""
        messages = example["messages"]
        # For models with native tool support, the tokenizer handles tool formatting
        try:
            text = tokenizer.apply_chat_template(
                messages,
                tools=example.get("tools"),
                tokenize=False,
                add_generation_prompt=False,
            )
        except Exception:
            # Fallback: some tokenizers don't support tools arg
            text = tokenizer.apply_chat_template(
                messages,
                tokenize=False,
                add_generation_prompt=False,
            )
        return {"text": text}

    train_dataset = train_dataset.map(format_example)
    eval_dataset = eval_dataset.map(format_example)

    # Training config
    sft_config = SFTConfig(
        output_dir=train_cfg["output_dir"],
        num_train_epochs=train_cfg["num_epochs"],
        per_device_train_batch_size=train_cfg["per_device_batch_size"],
        gradient_accumulation_steps=train_cfg["gradient_accumulation_steps"],
        learning_rate=train_cfg["learning_rate"],
        warmup_ratio=train_cfg["warmup_ratio"],
        weight_decay=train_cfg["weight_decay"],
        optim=train_cfg.get("optimizer", "adamw_torch"),
        lr_scheduler_type=train_cfg["lr_scheduler_type"],
        bf16=train_cfg.get("bf16", True),
        logging_steps=train_cfg.get("logging_steps", 10),
        eval_strategy="steps",
        eval_steps=train_cfg.get("eval_steps", 50),
        save_steps=train_cfg.get("save_steps", 100),
        save_total_limit=3,
        max_grad_norm=train_cfg.get("max_grad_norm", 1.0),
        seed=train_cfg.get("seed", 42),
        max_seq_length=model_cfg["max_seq_len"],
        dataset_text_field="text",
        packing=False,  # Don't pack — our examples vary in length
    )

    trainer = SFTTrainer(
        model=model,
        args=sft_config,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        processing_class=tokenizer,
    )

    print("Starting training...")
    trainer.train()

    # Save adapter
    adapter_dir = Path(train_cfg["output_dir"]) / "final_adapter"
    print(f"Saving LoRA adapter to {adapter_dir}")
    model.save_pretrained(str(adapter_dir))
    tokenizer.save_pretrained(str(adapter_dir))

    # Save merged model (for direct inference)
    merged_dir = Path(train_cfg["output_dir"]) / "merged"
    print(f"Saving merged model to {merged_dir}")
    model.save_pretrained_merged(
        str(merged_dir), tokenizer, save_method="merged_16bit"
    )

    print("Training complete.")

    # Print eval metrics
    metrics = trainer.evaluate()
    print("\nEval metrics:")
    for k, v in metrics.items():
        print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
