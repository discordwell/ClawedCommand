#!/usr/bin/env python3
"""
Train Devstral Small 2 (24B) — Raw PyTorch loop, no HF Trainer.
Bypasses accelerate/Trainer which add hooks that cause NaN gradients.

Key config:
- bf16 model + fp32 attention patch (prevents bf16 overflow)
- device_map={"": 0} (no accelerate hooks)
- Manual label masking (system+user → -100, assistant → keep)
- No gradient checkpointing
- AdamW optimizer with gradient clipping
"""

import os
import sys
import time
import json
import math
import torch
from torch import nn
from torch.utils.data import Dataset, DataLoader

# === Monkey-patch ministral3 attention to use fp32 ===
from transformers.models.ministral3 import modeling_ministral3

def fp32_eager_attention_forward(module, query, key, value, attention_mask, scaling, dropout=0.0, **kwargs):
    """Compute attention in fp32, immune to autocast."""
    from transformers.models.ministral3.modeling_ministral3 import repeat_kv
    key_states = repeat_kv(key, module.num_key_value_groups)
    value_states = repeat_kv(value, module.num_key_value_groups)
    # Disable autocast so fp32 computation actually sticks
    with torch.amp.autocast('cuda', enabled=False):
        q_fp32 = query.float()
        k_fp32 = key_states.float()
        v_fp32 = value_states.float()
        attn_weights = torch.matmul(q_fp32, k_fp32.transpose(2, 3)) * scaling
        if attention_mask is not None:
            attn_weights = attn_weights + attention_mask.float()
        attn_weights = nn.functional.softmax(attn_weights, dim=-1)
        attn_weights = nn.functional.dropout(attn_weights, p=dropout, training=module.training)
        attn_output = torch.matmul(attn_weights, v_fp32)
    attn_output = attn_output.to(query.dtype)
    attn_output = attn_output.transpose(1, 2).contiguous()
    return attn_output, attn_weights.to(query.dtype)

modeling_ministral3.eager_attention_forward = fp32_eager_attention_forward
# Also register in the global attention functions dict
import transformers.modeling_utils as _mu
ALL_ATTENTION_FUNCTIONS = _mu.ALL_ATTENTION_FUNCTIONS
ALL_ATTENTION_FUNCTIONS["eager"] = fp32_eager_attention_forward
_mu.caching_allocator_warmup = lambda *a, **kw: None  # Bypass OOM-causing warmup
print("Patched ministral3 attention to autocast-proof fp32")
# === End patch ===

from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import LoraConfig, get_peft_model, TaskType

MAX_LENGTH = 768  # Reduced from 1024 to fit in 80GB with eager fp32 attention


class LuaScriptDataset(Dataset):
    def __init__(self, jsonl_path, tokenizer, max_length=MAX_LENGTH):
        self.tokenizer = tokenizer
        self.max_length = max_length
        self.examples = []
        with open(jsonl_path) as f:
            for line in f:
                d = json.loads(line)
                self.examples.append(d["messages"])

    def __len__(self):
        return len(self.examples)

    def __getitem__(self, idx):
        # NOTE: Label masking assumes single-turn (system + user + assistant).
        # Multi-turn conversations would need per-turn boundary detection.
        messages = self.examples[idx]
        full_text = self.tokenizer.apply_chat_template(messages, tokenize=False)
        prompt_messages = [m for m in messages if m["role"] != "assistant"]
        prompt_text = self.tokenizer.apply_chat_template(
            prompt_messages, tokenize=False, add_generation_prompt=True
        )
        full_tokens = self.tokenizer(
            full_text, max_length=self.max_length, truncation=True,
            return_tensors="pt", padding="max_length",
        )
        prompt_tokens = self.tokenizer(
            prompt_text, max_length=self.max_length, truncation=True,
        )
        input_ids = full_tokens["input_ids"].squeeze()
        attention_mask = full_tokens["attention_mask"].squeeze()
        labels = input_ids.clone()
        prompt_len = len(prompt_tokens["input_ids"])
        labels[:prompt_len] = -100
        labels[attention_mask == 0] = -100
        return {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "labels": labels,
        }


def cosine_lr(step, total_steps, warmup_steps, max_lr):
    if step < warmup_steps:
        return max_lr * step / warmup_steps
    progress = (step - warmup_steps) / max(1, total_steps - warmup_steps)
    return max_lr * 0.5 * (1 + math.cos(math.pi * progress))


def main():
    model_path = os.path.expanduser("~/.cache/text_model/devstral-small-2-24b")
    data_dir = os.path.expanduser("~/lua_training/data")
    output_dir = os.path.expanduser("~/lua_training/outputs/devstral_24b_lua_v7")
    os.makedirs(output_dir, exist_ok=True)

    # Hyperparameters
    MAX_LR = 2e-4
    EPOCHS = 3
    GRAD_ACCUM = 16
    MAX_GRAD_NORM = 1.0
    WEIGHT_DECAY = 0.01
    WARMUP_RATIO = 0.1
    SAVE_EVERY = 25  # optimizer steps
    EVAL_EVERY = 25
    LOG_EVERY = 1

    print("--- Loading tokenizer ---")
    tokenizer = AutoTokenizer.from_pretrained(model_path, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token
    tokenizer.padding_side = "right"

    print("--- Loading model (bf16 + fp32 attention) ---")
    model = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=torch.bfloat16,
        device_map={"": 0},
        trust_remote_code=True,
        attn_implementation="eager",
    )
    model.config.use_cache = False
    print(f"VRAM after load: {torch.cuda.memory_allocated() / 1024**3:.1f}GB")

    print("--- LoRA ---")
    peft_config = LoraConfig(
        r=32, lora_alpha=64, lora_dropout=0.0,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                        "gate_proj", "up_proj", "down_proj"],
        bias="none", task_type=TaskType.CAUSAL_LM,
    )
    model = get_peft_model(model, peft_config)
    model.print_trainable_parameters()

    # --- Datasets ---
    print("\n--- Loading data ---")
    train_ds = LuaScriptDataset(
        os.path.join(data_dir, "combined_lua.jsonl"), tokenizer, max_length=MAX_LENGTH
    )
    eval_ds = LuaScriptDataset(
        os.path.join(data_dir, "gold_lua_examples.jsonl"), tokenizer, max_length=MAX_LENGTH
    )
    print(f"Train: {len(train_ds)}, Eval: {len(eval_ds)}")

    # Verify label masking
    for i in [0, 1, 10, 50]:
        if i >= len(train_ds):
            continue
        s = train_ds[i]
        nt = (s["labels"] != -100).sum().item()
        print(f"  Sample {i}: {nt} trainable tokens")
        assert nt > 10, f"Too few trainable tokens in sample {i}!"

    train_loader = DataLoader(train_ds, batch_size=1, shuffle=True,
                              generator=torch.Generator().manual_seed(42))
    eval_loader = DataLoader(eval_ds, batch_size=1, shuffle=False)

    # --- Optimizer ---
    trainable_params = [p for p in model.parameters() if p.requires_grad]
    print(f"\nTrainable parameters: {sum(p.numel() for p in trainable_params):,}")

    # Separate weight decay for biases (none in our case, but good practice)
    optimizer = torch.optim.AdamW(trainable_params, lr=MAX_LR, weight_decay=WEIGHT_DECAY)

    total_micro_steps = len(train_ds) * EPOCHS
    total_opt_steps = total_micro_steps // GRAD_ACCUM
    warmup_steps = int(WARMUP_RATIO * total_opt_steps)
    print(f"Total micro-steps: {total_micro_steps}")
    print(f"Total optimizer steps: {total_opt_steps}")
    print(f"Warmup steps: {warmup_steps}")

    # --- Pre-training gradient check ---
    print("\n--- Gradient sanity check ---")
    model.train()
    sample_batch = {k: v.unsqueeze(0).to("cuda:0") if v.dim() == 1 else v.to("cuda:0")
                    for k, v in train_ds[0].items()}
    out = model(**sample_batch)
    print(f"Sample loss: {out.loss.item():.4f}")
    out.loss.backward()

    nan_count = 0
    ok_count = 0
    for name, p in model.named_parameters():
        if p.requires_grad and p.grad is not None:
            if torch.isnan(p.grad).any():
                nan_count += 1
                print(f"  NaN grad: {name}")
            else:
                ok_count += 1

    print(f"Grads: {ok_count} OK, {nan_count} NaN")
    if nan_count > 0:
        print("FATAL: NaN in gradient check!")
        sys.exit(1)
    print("PASSED!")
    optimizer.zero_grad()
    torch.cuda.empty_cache()

    # --- Training loop ---
    print(f"\n{'='*60}")
    print(f"Starting training: {EPOCHS} epochs, {total_opt_steps} optimizer steps")
    print(f"{'='*60}")

    global_step = 0  # optimizer steps
    micro_step = 0   # individual forward passes
    accum_loss = 0.0
    best_eval_loss = float("inf")
    start_time = time.time()
    log_data = []

    model.train()
    for epoch in range(EPOCHS):
        for batch_idx, batch in enumerate(train_loader):
            # Move to GPU
            batch = {k: v.to("cuda:0") for k, v in batch.items()}

            # Forward
            outputs = model(**batch)
            loss = outputs.loss / GRAD_ACCUM
            accum_loss += outputs.loss.item()

            # Backward
            loss.backward()
            micro_step += 1

            # Optimizer step every GRAD_ACCUM micro-steps
            if micro_step % GRAD_ACCUM == 0:
                global_step += 1

                # Check for NaN gradients before clipping
                has_nan = False
                for name, p in model.named_parameters():
                    if p.requires_grad and p.grad is not None:
                        if torch.isnan(p.grad).any():
                            has_nan = True
                            print(f"  WARNING: NaN grad at step {global_step}: {name}")
                            break

                if has_nan:
                    print(f"  Skipping optimizer step {global_step} due to NaN gradients")
                    optimizer.zero_grad()
                    accum_loss = 0.0
                    continue

                # Gradient clipping
                grad_norm = torch.nn.utils.clip_grad_norm_(trainable_params, MAX_GRAD_NORM)

                # Update learning rate
                lr = cosine_lr(global_step, total_opt_steps, warmup_steps, MAX_LR)
                for pg in optimizer.param_groups:
                    pg["lr"] = lr

                # Step
                optimizer.step()
                optimizer.zero_grad()

                avg_loss = accum_loss / GRAD_ACCUM
                accum_loss = 0.0

                if global_step % LOG_EVERY == 0:
                    elapsed = time.time() - start_time
                    steps_per_sec = global_step / elapsed
                    eta = (total_opt_steps - global_step) / max(steps_per_sec, 1e-6)
                    vram_gb = torch.cuda.memory_allocated() / 1024**3
                    peak_gb = torch.cuda.max_memory_allocated() / 1024**3
                    print(f"step {global_step}/{total_opt_steps} | loss={avg_loss:.4f} | "
                          f"grad_norm={grad_norm:.4f} | lr={lr:.2e} | "
                          f"epoch={epoch + (batch_idx + 1) / len(train_loader):.2f} | "
                          f"VRAM={vram_gb:.1f}GB peak={peak_gb:.1f}GB | "
                          f"ETA={eta/60:.0f}min")
                    log_data.append({
                        "step": global_step, "loss": avg_loss,
                        "grad_norm": grad_norm.item() if not torch.isnan(grad_norm) else "nan",
                        "lr": lr, "epoch": epoch + (batch_idx + 1) / len(train_loader),
                    })

                # Eval
                if global_step % EVAL_EVERY == 0:
                    model.eval()
                    eval_losses = []
                    with torch.no_grad():
                        for eb in eval_loader:
                            eb = {k: v.to("cuda:0") for k, v in eb.items()}
                            eout = model(**eb)
                            eval_losses.append(eout.loss.item())
                    eval_loss = sum(eval_losses) / len(eval_losses)
                    print(f"  >> EVAL loss={eval_loss:.4f} (best={best_eval_loss:.4f})")
                    if eval_loss < best_eval_loss:
                        best_eval_loss = eval_loss
                        save_path = os.path.join(output_dir, "best")
                        model.save_pretrained(save_path)
                        tokenizer.save_pretrained(save_path)
                        print(f"  >> Saved best model to {save_path}")
                    model.train()

                # Save checkpoint
                if global_step % SAVE_EVERY == 0:
                    ckpt_path = os.path.join(output_dir, f"checkpoint-{global_step}")
                    model.save_pretrained(ckpt_path)
                    print(f"  >> Saved checkpoint to {ckpt_path}")

    # --- Final save ---
    elapsed = time.time() - start_time
    print(f"\n{'='*60}")
    print(f"Training complete! {elapsed/60:.1f} minutes")
    print(f"{'='*60}")

    final_path = os.path.join(output_dir, "final")
    model.save_pretrained(final_path)
    tokenizer.save_pretrained(final_path)
    print(f"Saved final adapter to {final_path}")

    # Save training log
    with open(os.path.join(output_dir, "train_log.json"), "w") as f:
        json.dump(log_data, f, indent=2)

    # Final eval
    model.eval()
    eval_losses = []
    with torch.no_grad():
        for eb in eval_loader:
            eb = {k: v.to("cuda:0") for k, v in eb.items()}
            eout = model(**eb)
            eval_losses.append(eout.loss.item())
    final_eval = sum(eval_losses) / len(eval_losses)
    print(f"Final eval loss: {final_eval:.4f}")
    print(f"Best eval loss: {best_eval_loss:.4f}")
    print(f"\n=== DONE ===")


if __name__ == "__main__":
    main()
