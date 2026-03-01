#!/usr/bin/env python3
"""
Devstral Small 2 LoRA fine-tuning — v5b: OOM fix
Changes from v5: gradient_checkpointing=True, r=32 (from 64), lora_alpha=64
Still uses completion-only masking + all linear layers + base Trainer
"""
import os, sys, torch
from torch import nn
from dataclasses import dataclass
from typing import Any

# === AUTOCAST-PROOF FP32 Attention Patch ===
def fp32_eager_attention_forward(module, query, key, value, attention_mask, scaling, dropout=0.0, **kwargs):
    from transformers.models.ministral3.modeling_ministral3 import repeat_kv
    key_states = repeat_kv(key, module.num_key_value_groups)
    value_states = repeat_kv(value, module.num_key_value_groups)
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

from transformers.models.ministral3 import modeling_ministral3
modeling_ministral3.eager_attention_forward = fp32_eager_attention_forward
from transformers.modeling_utils import ALL_ATTENTION_FUNCTIONS
ALL_ATTENTION_FUNCTIONS["eager"] = fp32_eager_attention_forward
print("Patched: autocast-proof fp32 attention")

from transformers import AutoTokenizer, AutoModelForCausalLM, Trainer, TrainingArguments
from peft import LoraConfig, get_peft_model, TaskType
from datasets import load_dataset

VARIANT = os.environ.get("VARIANT", "full")
MODEL_DIR = os.path.expanduser("~/.cache/text_model/devstral-small-2-24b")
DATA_DIR = os.path.expanduser("~/lua_training/data")
OUTPUT_DIR = os.path.expanduser(f"~/lua_training/output_v5b_{VARIANT}")
MAX_LENGTH = 4096
RESPONSE_TOKEN_ID = 4  # [/INST]

if VARIANT == "minimal":
    TRAIN_FILE = os.path.join(DATA_DIR, "combined_lua_minimal.jsonl")
    EVAL_FILE = os.path.join(DATA_DIR, "gold_lua_minimal.jsonl")
else:
    TRAIN_FILE = os.path.join(DATA_DIR, "combined_lua.jsonl")
    EVAL_FILE = os.path.join(DATA_DIR, "gold_lua_examples.jsonl")

print(f"=== Variant: {VARIANT} (v5b: OOM fix — grad ckpt + r=32) ===")

tokenizer = AutoTokenizer.from_pretrained(MODEL_DIR)
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token
PAD_ID = tokenizer.pad_token_id

def tokenize_and_mask(example):
    text = tokenizer.apply_chat_template(
        example["messages"], tokenize=False, add_generation_prompt=False
    )
    enc = tokenizer(text, truncation=True, max_length=MAX_LENGTH)
    input_ids = enc["input_ids"]
    attention_mask = enc["attention_mask"]
    labels = list(input_ids)
    last_inst = -1
    for i, tid in enumerate(input_ids):
        if tid == RESPONSE_TOKEN_ID:
            last_inst = i
    if last_inst >= 0:
        for i in range(last_inst + 1):
            labels[i] = -100
    for i, tid in enumerate(input_ids):
        if tid == PAD_ID:
            labels[i] = -100
    return {"input_ids": input_ids, "attention_mask": attention_mask, "labels": labels}

@dataclass
class PaddingCollator:
    pad_token_id: int
    max_length: int = MAX_LENGTH
    def __call__(self, features):
        max_len = min(max(len(f["input_ids"]) for f in features), self.max_length)
        input_ids, attention_mask, labels = [], [], []
        for f in features:
            pad_len = max_len - len(f["input_ids"])
            input_ids.append(f["input_ids"] + [self.pad_token_id] * pad_len)
            attention_mask.append(f["attention_mask"] + [0] * pad_len)
            labels.append(f["labels"] + [-100] * pad_len)
        return {
            "input_ids": torch.tensor(input_ids),
            "attention_mask": torch.tensor(attention_mask),
            "labels": torch.tensor(labels),
        }

print("Loading model...")
model = AutoModelForCausalLM.from_pretrained(
    MODEL_DIR, torch_dtype=torch.bfloat16, device_map="auto",
    attn_implementation="eager",
)
model.config.use_cache = False
# Enable gradient checkpointing to save VRAM
model.gradient_checkpointing_enable()
print(f"VRAM after model load: {torch.cuda.memory_allocated()/1e9:.1f} GB")

# v5b: r=32 (from 64), still all linear layers
lora_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    r=32,
    lora_alpha=64,
    lora_dropout=0.05,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                     "gate_proj", "up_proj", "down_proj"],
    bias="none",
)
model = get_peft_model(model, lora_config)
model.print_trainable_parameters()

train_ds = load_dataset("json", data_files=TRAIN_FILE, split="train")
eval_ds = load_dataset("json", data_files=EVAL_FILE, split="train")
print(f"Raw: Train={len(train_ds)}, Eval={len(eval_ds)}")

train_ds = train_ds.map(tokenize_and_mask, remove_columns=train_ds.column_names)
eval_ds = eval_ds.map(tokenize_and_mask, remove_columns=eval_ds.column_names)

sample = train_ds[0]
n_total = len(sample["input_ids"])
n_masked = sum(1 for l in sample["labels"] if l == -100)
print(f"Sample: {n_total} total, {n_total - n_masked} train tokens ({100*(n_total-n_masked)/n_total:.1f}%)")

train_token_counts = [sum(1 for l in ex["labels"] if l != -100) for ex in train_ds]
print(f"Assistant tokens/example: min={min(train_token_counts)}, max={max(train_token_counts)}, avg={sum(train_token_counts)/len(train_token_counts):.0f}")

collator = PaddingCollator(pad_token_id=PAD_ID)

# Gradient sanity check
print("\n--- Gradient check ---")
model.train()
batch = collator([train_ds[0]])
batch = {k: v.to(model.device) for k, v in batch.items()}
with torch.amp.autocast('cuda', dtype=torch.bfloat16):
    out = model(**batch)
    loss = out.loss
print(f"Loss: {loss.item():.4f}")
loss.backward()
ok = nan = 0
for n, p in model.named_parameters():
    if p.grad is not None:
        if torch.isnan(p.grad).any(): nan += 1
        else: ok += 1
print(f"Gradients: {ok} OK, {nan} NaN")
if nan > 0:
    print("FATAL: NaN. Aborting.")
    sys.exit(1)
model.zero_grad()
print("PASSED\n")

print(f"VRAM: {torch.cuda.memory_allocated()/1e9:.1f} GB")
print("--- Starting training ---")

training_args = TrainingArguments(
    output_dir=OUTPUT_DIR,
    num_train_epochs=5,
    per_device_train_batch_size=1,
    gradient_accumulation_steps=4,
    learning_rate=1e-3,
    lr_scheduler_type="cosine",
    warmup_ratio=0.03,
    bf16=True,
    logging_steps=5,
    eval_strategy="steps",
    eval_steps=50,
    save_strategy="steps",
    save_steps=100,
    save_total_limit=3,
    report_to="none",
    gradient_checkpointing=True,
    gradient_checkpointing_kwargs={"use_reentrant": False},
    optim="adamw_torch",
    max_grad_norm=1.0,
    seed=42,
    dataloader_pin_memory=False,
    weight_decay=0.01,
)

trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=train_ds,
    eval_dataset=eval_ds,
    data_collator=collator,
)

result = trainer.train()
print(f"\n=== Training complete ===")
print(f"Train loss: {result.training_loss:.4f}")
print(f"Runtime: {result.metrics['train_runtime']:.0f}s")

trainer.save_model(os.path.join(OUTPUT_DIR, "final"))
tokenizer.save_pretrained(os.path.join(OUTPUT_DIR, "final"))

eval_result = trainer.evaluate()
print(f"Eval loss: {eval_result['eval_loss']:.4f}")

# Generate samples
print("\n--- Sample generation ---")
model.eval()
prompts = [
    "Send all military units to attack the enemy base",
    "Build 3 fish markets near the pond",
    "Have my ranged units kite the enemy melee",
]
for prompt in prompts:
    test_msgs = [
        {"role": "system", "content": "You write Lua scripts for an RTS game."},
        {"role": "user", "content": prompt},
    ]
    test_text = tokenizer.apply_chat_template(test_msgs, tokenize=False, add_generation_prompt=True)
    test_inp = tokenizer(test_text, return_tensors="pt").to(model.device)
    with torch.no_grad():
        gen = model.generate(**test_inp, max_new_tokens=300, temperature=0.2, do_sample=True)
    response = tokenizer.decode(gen[0][test_inp["input_ids"].shape[1]:], skip_special_tokens=True)
    print(f"\nPrompt: {prompt!r}")
    print(f"Response:\n{response}\n---")

print(f"\n=== DONE: {VARIANT} (v5b) ===")
