#!/usr/bin/env python3
"""Merge LoRA adapter into base Devstral Small 2 and prepare for serving.

Run on Brev A100 (cc-train-minimal):
    python training/scripts/merge_and_export.py \
        --adapter ~/lua_training/outputs/devstral_24b_lua_v7/best \
        --output ~/lua_training/merged/clawed-lua-24b

After merging:
    # Start vLLM for immediate testing:
    python -m vllm.entrypoints.openai.api_server \
        --model ~/lua_training/merged/clawed-lua-24b \
        --port 8080 --dtype bfloat16 --max-model-len 2048

    # Or convert to GGUF for Ollama (see --gguf-instructions flag):
    python training/scripts/merge_and_export.py --gguf-instructions
"""

import argparse
import sys
from pathlib import Path


def bypass_warmup():
    """Disable caching_allocator_warmup that causes OOM on transformers 5.2+."""
    try:
        import torch.utils._mu as _mu
        _mu.caching_allocator_warmup = lambda *a, **kw: None
        print("[OK] Bypassed caching_allocator_warmup")
    except (ImportError, AttributeError):
        pass


def patch_fp32_attention():
    """Monkey-patch eager attention to use fp32 (prevents NaN in bf16 SDPA)."""
    import torch

    try:
        import transformers.models.ministral3.modeling_ministral3 as mod
        original_fn = mod.eager_attention_forward

        def patched_eager_attention_forward(
            module, query, key, value, attention_mask, scaling, dropout=0.0, **kwargs
        ):
            with torch.amp.autocast("cuda", enabled=False):
                return original_fn(
                    module,
                    query.float(),
                    key.float(),
                    value.float(),
                    attention_mask,
                    scaling,
                    dropout,
                    **kwargs,
                )

        mod.eager_attention_forward = patched_eager_attention_forward
        print("[OK] Patched fp32 attention")
    except (ImportError, AttributeError) as e:
        print(f"[WARN] Could not patch attention: {e}")


def print_gguf_instructions():
    """Print instructions for GGUF conversion + Ollama import."""
    print("""
=== GGUF Conversion Instructions ===

1. Clone llama.cpp (if not already):
   git clone https://github.com/ggerganov/llama.cpp
   cd llama.cpp && make -j

2. Convert merged model to GGUF:
   python convert_hf_to_gguf.py ~/lua_training/merged/clawed-lua-24b \\
       --outfile clawed-lua-24b-f16.gguf --outtype f16

3. Quantize to Q4_K_M (~14GB, good quality/speed tradeoff):
   ./llama-quantize clawed-lua-24b-f16.gguf clawed-lua-24b-Q4_K_M.gguf Q4_K_M

4. Import into Ollama:
   # Copy the Modelfile from training/scripts/Modelfile.clawed-lua
   # Update the FROM line to point to your GGUF path
   ollama create clawed-lua -f Modelfile.clawed-lua

5. Test:
   ollama run clawed-lua "send idle workers to gather food"
""")


def main():
    parser = argparse.ArgumentParser(description="Merge LoRA adapter into base model")
    parser.add_argument(
        "--base-model",
        default="mistralai/Devstral-Small-2-24B-Instruct-2512",
        help="Base model ID or path",
    )
    parser.add_argument(
        "--adapter",
        default="~/lua_training/outputs/devstral_24b_lua_v7/best",
        help="Path to LoRA adapter directory",
    )
    parser.add_argument(
        "--output",
        default="~/lua_training/merged/clawed-lua-24b",
        help="Output directory for merged model",
    )
    parser.add_argument(
        "--gguf-instructions",
        action="store_true",
        help="Just print GGUF conversion instructions and exit",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        default=True,
        help="Run a quick inference check after merging",
    )
    args = parser.parse_args()

    if args.gguf_instructions:
        print_gguf_instructions()
        return

    adapter_path = Path(args.adapter).expanduser()
    output_path = Path(args.output).expanduser()

    if not adapter_path.exists():
        print(f"[ERROR] Adapter not found: {adapter_path}")
        sys.exit(1)

    # Step 0: Bypass OOM + patch attention
    bypass_warmup()
    patch_fp32_attention()

    import torch
    from transformers import AutoTokenizer, AutoModel
    from peft import PeftModel

    # Step 1: Load base model (fp8 auto-dequantizes to bf16 via composite model)
    print(f"[1/4] Loading base model: {args.base_model}")
    model = AutoModel.from_pretrained(
        args.base_model,
        torch_dtype=torch.bfloat16,
        device_map="auto",
        trust_remote_code=True,
    )
    tokenizer = AutoTokenizer.from_pretrained(args.base_model)

    # Step 2: Load LoRA adapter
    print(f"[2/4] Loading LoRA adapter: {adapter_path}")
    # The adapter targets the text model inside the composite
    text_model = model.language_model if hasattr(model, "language_model") else model
    text_model = PeftModel.from_pretrained(text_model, str(adapter_path))

    # Step 3: Merge and unload
    print("[3/4] Merging adapter into base model...")
    text_model = text_model.merge_and_unload()

    # Replace the text model back into composite if needed
    if hasattr(model, "language_model"):
        model.language_model = text_model

    # Step 4: Save merged model
    print(f"[4/4] Saving merged model to: {output_path}")
    output_path.mkdir(parents=True, exist_ok=True)
    model.save_pretrained(str(output_path))
    tokenizer.save_pretrained(str(output_path))

    print(f"[OK] Merged model saved to {output_path}")

    # Verify with a quick inference
    if args.verify:
        print("\n--- Quick inference verification ---")
        prompt = "[INST] send idle workers to gather food [/INST]"
        inputs = tokenizer(prompt, return_tensors="pt").to(model.device)
        with torch.no_grad():
            output = model.generate(
                **inputs,
                max_new_tokens=200,
                temperature=0.2,
                do_sample=True,
            )
        result = tokenizer.decode(output[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True)
        print(f"Prompt: {prompt}")
        print(f"Output:\n{result}")

        # Sanity check
        if "ctx:" in result or "local" in result:
            print("\n[OK] Output looks like valid Lua!")
        else:
            print("\n[WARN] Output may not be valid Lua. Check manually.")

    print("\n--- Next steps ---")
    print(f"  vLLM:   python -m vllm.entrypoints.openai.api_server --model {output_path} --port 8080 --dtype bfloat16")
    print(f"  GGUF:   python training/scripts/merge_and_export.py --gguf-instructions")


if __name__ == "__main__":
    main()
