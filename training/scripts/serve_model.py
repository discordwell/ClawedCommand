#!/usr/bin/env python3
"""Lightweight OpenAI-compatible server for fine-tuned Devstral Small 2.

Loads the extracted text model + LoRA adapter using transformers + peft,
serves via FastAPI with the /v1/chat/completions endpoint.

Run on Brev A100:
    python serve_model.py --port 8080

Game connects via:
    CLAWED_LLM_BACKEND=openai CLAWED_LLM_URL=http://<brev-ip>:8080 \
    CLAWED_LLM_MODEL=clawed-lua CLAWED_LLM_FINETUNED=1 cargo run -p cc_client
"""

import argparse
import json
import time
import uuid
from contextlib import asynccontextmanager

import torch
from fastapi import FastAPI
from fastapi.responses import JSONResponse
from pydantic import BaseModel

# Globals set during lifespan
model = None
tokenizer = None


def bypass_warmup():
    """Disable caching_allocator_warmup that causes OOM on transformers 5.2+."""
    try:
        import torch.utils._mu as _mu
        _mu.caching_allocator_warmup = lambda *a, **kw: None
    except (ImportError, AttributeError):
        pass


def patch_fp32_attention():
    """Monkey-patch eager attention to use fp32 (prevents NaN in bf16 SDPA).
    Casts QKV to float32 for the attention computation, then casts output back."""
    try:
        import transformers.models.ministral3.modeling_ministral3 as mod
        original_fn = mod.eager_attention_forward

        def patched(module, query, key, value, attention_mask, scaling, dropout=0.0, **kw):
            orig_dtype = query.dtype
            with torch.amp.autocast("cuda", enabled=False):
                result = original_fn(
                    module, query.float(), key.float(), value.float(),
                    attention_mask, scaling, dropout, **kw,
                )
            # Cast back to original dtype to avoid mat mul mismatches
            if isinstance(result, tuple):
                return (result[0].to(orig_dtype),) + result[1:]
            return result.to(orig_dtype)
        mod.eager_attention_forward = patched
        print("[OK] Patched fp32 attention")
    except (ImportError, AttributeError) as e:
        print(f"[WARN] Could not patch attention: {e}")


def load_model(base_path: str, adapter_path: str):
    """Load base model + LoRA adapter."""
    from transformers import AutoTokenizer, AutoModelForCausalLM
    from peft import PeftModel

    print(f"Loading base model from {base_path}...")
    tok = AutoTokenizer.from_pretrained(base_path)
    mdl = AutoModelForCausalLM.from_pretrained(
        base_path,
        torch_dtype=torch.bfloat16,
        device_map="auto",
        attn_implementation="eager",  # Use patched eager attention
    )
    print(f"Loading LoRA adapter from {adapter_path}...")
    mdl = PeftModel.from_pretrained(mdl, adapter_path)
    mdl.eval()
    print(f"[OK] Model loaded. Device: {mdl.device}")
    return mdl, tok


# --- Request/Response models ---

class ChatMessage(BaseModel):
    role: str
    content: str

class ChatRequest(BaseModel):
    model: str = "clawed-lua"
    messages: list[ChatMessage]
    temperature: float = 0.2
    max_tokens: int = 512
    top_p: float = 0.9
    stream: bool = False

# --- App ---

@asynccontextmanager
async def lifespan(app: FastAPI):
    global model, tokenizer
    bypass_warmup()
    patch_fp32_attention()
    model, tokenizer = load_model(app.state.base_path, app.state.adapter_path)
    yield

app = FastAPI(lifespan=lifespan)


@app.get("/v1/models")
async def list_models():
    return {"object": "list", "data": [{"id": "clawed-lua", "object": "model"}]}


@app.post("/v1/chat/completions")
async def chat_completions(req: ChatRequest):
    # Use the tokenizer's chat template (matches training format exactly)
    messages = [{"role": m.role, "content": m.content} for m in req.messages]
    prompt = tokenizer.apply_chat_template(
        messages, tokenize=False, add_generation_prompt=True,
    )
    inputs = tokenizer(prompt, return_tensors="pt").to(model.device)

    t0 = time.time()
    with torch.no_grad():
        output = model.generate(
            **inputs,
            max_new_tokens=req.max_tokens,
            temperature=max(req.temperature, 0.01),
            top_p=req.top_p,
            do_sample=req.temperature > 0,
            pad_token_id=tokenizer.pad_token_id or tokenizer.eos_token_id,
        )

    new_tokens = output[0][inputs["input_ids"].shape[1]:]
    content = tokenizer.decode(new_tokens, skip_special_tokens=True)
    elapsed = time.time() - t0

    return JSONResponse({
        "id": f"chatcmpl-{uuid.uuid4().hex[:8]}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": req.model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop",
        }],
        "usage": {
            "prompt_tokens": inputs["input_ids"].shape[1],
            "completion_tokens": len(new_tokens),
            "total_tokens": inputs["input_ids"].shape[1] + len(new_tokens),
        },
        "_elapsed_s": round(elapsed, 2),
    })


@app.get("/health")
async def health():
    return {"status": "ok", "model_loaded": model is not None}


if __name__ == "__main__":
    import uvicorn

    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", default="/home/ubuntu/.cache/text_model/devstral-small-2-24b")
    parser.add_argument("--adapter", default="/home/ubuntu/lua_training/outputs/devstral_24b_lua_v7/best")
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--host", default="0.0.0.0")
    args = parser.parse_args()

    app.state.base_path = args.base_model
    app.state.adapter_path = args.adapter
    uvicorn.run(app, host=args.host, port=args.port)
