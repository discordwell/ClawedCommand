#!/usr/bin/env python3
"""
Properly extract Devstral Small 2 — with lm_head fix.
Weights are fp8 → let transformers auto-dequantize via composite model.
Then manually handle lm_head.weight which is UNEXPECTED in composite.
"""
import os, json, glob, torch
from torch import nn

# fp32 attention patch
from transformers.models.ministral3 import modeling_ministral3
def fp32_eager_attn(module, query, key, value, attention_mask, scaling, dropout=0.0, **kwargs):
    from transformers.models.ministral3.modeling_ministral3 import repeat_kv
    key_states = repeat_kv(key, module.num_key_value_groups)
    value_states = repeat_kv(value, module.num_key_value_groups)
    with torch.amp.autocast('cuda', enabled=False):
        q = query.float(); k = key_states.float(); v = value_states.float()
        w = torch.matmul(q, k.transpose(2, 3)) * scaling
        if attention_mask is not None: w = w + attention_mask.float()
        w = nn.functional.softmax(w, dim=-1)
        o = torch.matmul(w, v)
    o = o.to(query.dtype).transpose(1, 2).contiguous()
    return o, w.to(query.dtype)
modeling_ministral3.eager_attention_forward = fp32_eager_attn
from transformers.modeling_utils import ALL_ATTENTION_FUNCTIONS
ALL_ATTENTION_FUNCTIONS["eager"] = fp32_eager_attn

from transformers import AutoModel, AutoModelForCausalLM, AutoTokenizer
import transformers.modeling_utils as _mu
_mu.caching_allocator_warmup = lambda *a, **kw: None  # Disable OOM-causing warmup
from safetensors.torch import save_file, load_file

MODEL_ID = "mistralai/Devstral-Small-2-24B-Instruct-2512"
OUTPUT_DIR = os.path.expanduser("~/.cache/text_model/devstral-small-2-24b")

# Step 1: Load composite model (auto-dequantizes fp8→bf16)
print("=== Step 1: Loading composite (fp8→bf16 dequantization) ===")
composite = AutoModel.from_pretrained(
    MODEL_ID, torch_dtype=torch.bfloat16, device_map={"": 0},
    trust_remote_code=True, attn_implementation="eager",
)
print(f"VRAM: {torch.cuda.memory_allocated() / 1024**3:.1f}GB")

# Step 2: Extract text model weights
print("\n=== Step 2: Extracting text model state dict ===")
text_model = composite.language_model
text_state = text_model.state_dict()
print(f"Text model: {len(text_state)} tensors")

# Build CausalLM state dict with model. prefix
causal_state = {}
for key, tensor in text_state.items():
    if not key.startswith('model.'):
        causal_state[f'model.{key}'] = tensor.cpu()
    else:
        causal_state[key] = tensor.cpu()

# Step 3: Handle lm_head.weight
# The composite model reports language_model.lm_head.weight as UNEXPECTED.
# We need to find it in the original safetensors.
# First check: is lm_head tied to embed_tokens?
embed_weight = causal_state.get('model.embed_tokens.weight')
print(f"embed_tokens shape: {embed_weight.shape if embed_weight is not None else 'NOT FOUND'}")

# Try to find lm_head in the HF-cached safetensors
hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
snap_dirs = glob.glob(os.path.join(hf_cache, "models--mistralai*", "snapshots", "*"))
lm_head_found = False

if snap_dirs:
    snap_dir = snap_dirs[0]
    for sf_path in sorted(glob.glob(os.path.join(snap_dir, "*.safetensors"))):
        tensors = load_file(sf_path)
        if "language_model.lm_head.weight" in tensors:
            lm_head_raw = tensors["language_model.lm_head.weight"]
            print(f"Found lm_head in {os.path.basename(sf_path)}: {lm_head_raw.shape} {lm_head_raw.dtype}")

            # Check if it's fp8 or bf16
            if lm_head_raw.dtype in (torch.float8_e4m3fn, torch.float8_e5m2):
                print("lm_head is fp8 — need scale factor")
                # Look for its scale
                scale_key = "language_model.lm_head.weight_scale_inv"
                if scale_key in tensors:
                    scale = tensors[scale_key]
                    lm_head_bf16 = lm_head_raw.float() * scale.float()
                    lm_head_bf16 = lm_head_bf16.to(torch.bfloat16)
                    print(f"Dequantized lm_head: {lm_head_bf16.shape} {lm_head_bf16.dtype}")
                    causal_state['lm_head.weight'] = lm_head_bf16.cpu()
                    lm_head_found = True
                else:
                    # Check other safetensors files for the scale
                    for sf2 in sorted(glob.glob(os.path.join(snap_dir, "*.safetensors"))):
                        t2 = load_file(sf2)
                        if scale_key in t2:
                            scale = t2[scale_key]
                            lm_head_bf16 = lm_head_raw.float() * scale.float()
                            lm_head_bf16 = lm_head_bf16.to(torch.bfloat16)
                            causal_state['lm_head.weight'] = lm_head_bf16.cpu()
                            lm_head_found = True
                            print(f"Found scale in {os.path.basename(sf2)}, dequantized lm_head")
                            break
            else:
                # Already bf16 or fp32
                causal_state['lm_head.weight'] = lm_head_raw.to(torch.bfloat16).cpu()
                lm_head_found = True
                print(f"lm_head already in {lm_head_raw.dtype}, using directly")
            break
        del tensors

if not lm_head_found:
    # Fallback: tie lm_head to embed_tokens
    print("WARNING: lm_head not found in safetensors, tying to embed_tokens")
    causal_state['lm_head.weight'] = embed_weight.clone()
    lm_head_found = True

print(f"\nFinal state dict: {len(causal_state)} tensors")
print(f"lm_head: {causal_state['lm_head.weight'].shape}")

# Free GPU memory
del composite, text_model
torch.cuda.empty_cache()
print(f"GPU freed: {torch.cuda.memory_allocated() / 1024**3:.1f}GB")

# Step 4: Save
print(f"\n=== Step 4: Saving to {OUTPUT_DIR} ===")
os.makedirs(OUTPUT_DIR, exist_ok=True)

# Remove old shards
for old_f in glob.glob(os.path.join(OUTPUT_DIR, "model-*.safetensors")):
    os.remove(old_f)

# Config
text_config_path = os.path.join(OUTPUT_DIR, "config.json")
# Load config from the model we just used
from transformers import AutoConfig
config = AutoConfig.from_pretrained(MODEL_ID, trust_remote_code=True)
text_config = config.text_config.to_dict() if hasattr(config, 'text_config') else config.to_dict()
text_config["architectures"] = ["Ministral3ForCausalLM"]
text_config["model_type"] = "ministral3"
text_config.pop("quantization_config", None)
text_config.pop("auto_map", None)
with open(text_config_path, "w") as f:
    json.dump(text_config, f, indent=2)

# Save weights
chunk_size = 5_000_000_000
current_chunk = {}
current_size = 0
chunk_idx = 0
chunks = []
for key in sorted(causal_state.keys()):
    tensor = causal_state[key]
    sz = tensor.numel() * tensor.element_size()
    if current_size + sz > chunk_size and current_chunk:
        path = os.path.join(OUTPUT_DIR, f"model-{chunk_idx:05d}-of-PLACEHOLDER.safetensors")
        save_file(current_chunk, path)
        chunks.append((chunk_idx, path, list(current_chunk.keys())))
        chunk_idx += 1
        current_chunk = {}
        current_size = 0
    current_chunk[key] = tensor
    current_size += sz
if current_chunk:
    path = os.path.join(OUTPUT_DIR, f"model-{chunk_idx:05d}-of-PLACEHOLDER.safetensors")
    save_file(current_chunk, path)
    chunks.append((chunk_idx, path, list(current_chunk.keys())))
    chunk_idx += 1

num_chunks = chunk_idx
index = {"metadata": {"total_size": sum(t.numel() * t.element_size() for t in causal_state.values())}, "weight_map": {}}
for idx, old_path, keys in chunks:
    new_name = f"model-{idx:05d}-of-{num_chunks:05d}.safetensors"
    new_path = os.path.join(OUTPUT_DIR, new_name)
    os.rename(old_path, new_path)
    for k in keys:
        index["weight_map"][k] = new_name
with open(os.path.join(OUTPUT_DIR, "model.safetensors.index.json"), "w") as f:
    json.dump(index, f, indent=2)
print(f"Saved {num_chunks} shards")

# Tokenizer
tokenizer = AutoTokenizer.from_pretrained(MODEL_ID, trust_remote_code=True)
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token
tokenizer.save_pretrained(OUTPUT_DIR)
tok_config_path = os.path.join(OUTPUT_DIR, "tokenizer_config.json")
with open(tok_config_path) as f:
    tc = json.load(f)
if not tc.get("chat_template"):
    tc["chat_template"] = (
        "{{bos_token}}"
        "{% for message in messages %}"
        "{% if message['role'] == 'system' %}"
        "[SYSTEM_PROMPT]{{ message['content'] }}[/SYSTEM_PROMPT]"
        "{% elif message['role'] == 'user' %}"
        "[INST]{{ message['content'] }}[/INST]"
        "{% elif message['role'] == 'assistant' %}"
        "{{ message['content'] }}{{eos_token}}"
        "{% endif %}"
        "{% endfor %}"
    )
    with open(tok_config_path, "w") as f:
        json.dump(tc, f, indent=2)

# Step 5: Verify
print("\n=== Step 5: Verification ===")
model = AutoModelForCausalLM.from_pretrained(
    OUTPUT_DIR, torch_dtype=torch.bfloat16, device_map={"": 0},
    trust_remote_code=True, attn_implementation="eager",
)
model.eval()
tokenizer = AutoTokenizer.from_pretrained(OUTPUT_DIR, trust_remote_code=True)
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token

# Code completion test
test = "def fibonacci(n):\n    if n <= 1:\n        return n\n    return"
inputs = tokenizer(test, return_tensors="pt").to("cuda:0")
with torch.no_grad():
    out = model(**inputs, labels=inputs["input_ids"])
print(f"Loss on code: {out.loss.item():.4f}")

gen = model.generate(inputs["input_ids"], max_new_tokens=30, do_sample=False)
print(f"Code completion: {tokenizer.decode(gen[0], skip_special_tokens=True)}")

# Chat test
messages = [
    {"role": "system", "content": "You are a helpful coding assistant."},
    {"role": "user", "content": "Write hello world in Lua."},
]
fmt = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
inp2 = tokenizer(fmt, return_tensors="pt").to("cuda:0")
gen2 = model.generate(inp2["input_ids"], max_new_tokens=100, do_sample=True, temperature=0.3)
chat = tokenizer.decode(gen2[0][inp2["input_ids"].shape[1]:], skip_special_tokens=True)
print(f"\nChat response:\n{chat[:400]}")

print("\n=== ALL DONE ===")
