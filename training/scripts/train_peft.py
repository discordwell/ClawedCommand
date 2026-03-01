#!/usr/bin/env python3
"""Train Devstral Small 2 with LoRA using PEFT + TRL SFTTrainer.

Devstral Small 2 ships as fp8 on HuggingFace. We load with dtype=bfloat16
which casts the fp8 weights to bf16, then apply LoRA on top.
L40S 48GB can handle bf16 24B model (~48GB) with gradient checkpointing.
"""

import argparse
import json
import os
import yaml
from pathlib import Path


def patch_model_config(model_name):
    """Remove vLLM-specific quantization_config and fix tokenizer class."""
    from huggingface_hub import snapshot_download, hf_hub_download
    import json as _json

    cache_dir = os.path.expanduser(
        "~/.cache/patched_models/" + model_name.replace("/", "_")
    )

    # Check if already patched
    config_path_cached = os.path.join(cache_dir, "config.json")
    if os.path.exists(config_path_cached):
        with open(config_path_cached) as f:
            config = _json.load(f)
        if "quantization_config" not in config:
            print(f"Using pre-patched model from {cache_dir}")
            return cache_dir

    config_path = hf_hub_download(model_name, "config.json")
    with open(config_path) as f:
        config = _json.load(f)

    if "quantization_config" in config:
        qmethod = config["quantization_config"].get("quant_method", "?")
        print(f"Stripping quantization_config ({qmethod})")
        del config["quantization_config"]

        os.makedirs(cache_dir, exist_ok=True)
        print("Downloading model weights...")
        local_dir = snapshot_download(model_name, local_dir=cache_dir)

        with open(os.path.join(local_dir, "config.json"), "w") as f:
            _json.dump(config, f, indent=2)

        # Patch tokenizer_config.json if needed
        tok_cfg_path = os.path.join(local_dir, "tokenizer_config.json")
        if os.path.exists(tok_cfg_path):
            with open(tok_cfg_path) as f:
                tok_cfg = _json.load(f)
            changed = False
            if tok_cfg.get("tokenizer_class") == "TokenizersBackend":
                tok_cfg["tokenizer_class"] = "PreTrainedTokenizerFast"
                changed = True
            if isinstance(tok_cfg.get("extra_special_tokens"), list):
                tok_cfg["extra_special_tokens"] = {
                    t: t for t in tok_cfg["extra_special_tokens"]
                }
                changed = True
            if changed:
                with open(tok_cfg_path, "w") as f:
                    _json.dump(tok_cfg, f, indent=2)
                print("Patched tokenizer_config.json")

        return local_dir
    return model_name


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("config", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    with open(args.config) as f:
        cfg = yaml.safe_load(f)

    config_dir = args.config.parent
    train_path = config_dir / cfg["data"]["train_file"]
    eval_path = config_dir / cfg["data"]["eval_file"]

    model_name = cfg["model"]["name"]
    print(f"Model: {model_name}")
    print(f"Train: {train_path} ({sum(1 for _ in open(train_path))} examples)")
    output_dir = cfg["training"]["output_dir"]
    print(f"Output: {output_dir}")

    if args.dry_run:
        print("Dry run OK")
        return

    import torch
    from transformers import AutoModelForCausalLM, AutoTokenizer, AutoConfig
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from trl import SFTTrainer, SFTConfig
    from datasets import Dataset

    # Patch and load model
    model_path = patch_model_config(model_name)

    print("\n--- Loading tokenizer ---")
    tokenizer = AutoTokenizer.from_pretrained(model_path, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    # Devstral Small 2 uses Mistral3ForConditionalGeneration architecture.
    # Weights are bf16 on disk. Use BitsAndBytes 4-bit (QLoRA) to fit on L40S.
    print("--- Loading model (4-bit QLoRA) ---")
    from transformers import Mistral3ForConditionalGeneration, BitsAndBytesConfig
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.bfloat16,
        bnb_4bit_use_double_quant=True,
    )
    model = Mistral3ForConditionalGeneration.from_pretrained(
        model_path,
        quantization_config=bnb_config,
        device_map="auto",
        trust_remote_code=True,
        attn_implementation="sdpa",
    )
    model.gradient_checkpointing_enable()

    allocated = torch.cuda.memory_allocated() / 1024**3
    print(f"VRAM after load: {allocated:.1f}GB")

    # Apply LoRA — target language_model layers inside the composite model
    print("--- Applying LoRA ---")
    lora_cfg = cfg["lora"]
    peft_config = LoraConfig(
        r=lora_cfg["rank"],
        lora_alpha=lora_cfg["alpha"],
        lora_dropout=lora_cfg["dropout"],
        target_modules=lora_cfg["target_modules"],
        bias="none",
        task_type=None,  # Raw PeftModel — composite isn't a standard CausalLM
    )
    from peft import PeftModel
    model = get_peft_model(model, peft_config)
    model.print_trainable_parameters()

    # Load data
    print("--- Loading data ---")

    def load_jsonl_as_chat(path):
        data = []
        with open(path) as f:
            for line in f:
                if line.strip():
                    ex = json.loads(line)
                    text = tokenizer.apply_chat_template(
                        ex["messages"], tokenize=False, add_generation_prompt=False
                    )
                    data.append({"text": text})
        return Dataset.from_list(data)

    train_dataset = load_jsonl_as_chat(train_path)
    eval_dataset = load_jsonl_as_chat(eval_path) if eval_path.exists() else None
    eval_count = len(eval_dataset) if eval_dataset else 0
    print(f"Train: {len(train_dataset)}, Eval: {eval_count}")

    # Training
    tcfg = cfg["training"]
    batch_size = 1
    grad_accum = 32

    training_args = SFTConfig(
        output_dir=output_dir,
        per_device_train_batch_size=batch_size,
        gradient_accumulation_steps=grad_accum,
        num_train_epochs=tcfg["num_epochs"],
        learning_rate=tcfg["learning_rate"],
        warmup_ratio=tcfg["warmup_ratio"],
        weight_decay=tcfg["weight_decay"],
        lr_scheduler_type=tcfg["lr_scheduler_type"],
        bf16=True,
        seed=tcfg.get("seed", 42),
        logging_steps=tcfg.get("logging_steps", 10),
        eval_steps=tcfg.get("eval_steps", 50) if eval_dataset else None,
        eval_strategy="steps" if eval_dataset else "no",
        save_steps=tcfg.get("save_steps", 100),
        max_grad_norm=tcfg.get("max_grad_norm", 1.0),
        optim=tcfg.get("optimizer", "adamw_8bit"),
        max_length=cfg["model"]["max_seq_len"],
        dataset_text_field="text",
        report_to="none",
        gradient_checkpointing=True,
    )

    eff_batch = batch_size * grad_accum
    print(f"\nEffective batch: {eff_batch}")
    print("--- Training ---")
    trainer = SFTTrainer(
        model=model,
        args=training_args,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        processing_class=tokenizer,
    )
    trainer.train()

    # Save
    print("\n--- Saving adapter ---")
    adapter_dir = f"{output_dir}/final_adapter"
    model.save_pretrained(adapter_dir)
    tokenizer.save_pretrained(adapter_dir)
    print(f"Saved to {adapter_dir}")
    print("\n=== Done ===")


if __name__ == "__main__":
    main()
