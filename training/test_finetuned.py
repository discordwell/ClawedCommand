#!/usr/bin/env python3
"""
Test the fine-tuned LoRA adapter using mlx-lm.
Downloads the 4-bit MLX model and applies our LoRA adapter.
"""
import json
import time
import sys
import os

# Use mlx-lm for inference on Apple Silicon
from mlx_lm import load, generate
from mlx_lm.tuner.utils import apply_lora_layers

ADAPTER_DIR = os.path.join(os.path.dirname(__file__), "lora_checkpoints")
SYSTEM_PROMPT_FILE = os.path.join(os.path.dirname(__file__), "data", "system_prompt.txt")

# Read system prompt
with open(SYSTEM_PROMPT_FILE) as f:
    SYSTEM_PROMPT = f.read().strip()

TEST_PROMPTS = [
    # Standard (seen during training)
    ("standard", "Send idle workers to gather food"),
    ("standard", "Attack! Send all combat units at the enemy"),
    ("standard", "Make my Hissers kite away from melee enemies"),
    ("standard", "Focus fire the weakest enemy near my army"),
    ("standard", "Retreat! Fall back to base"),
    # Novel (never seen)
    ("novel", "Set up a flanking maneuver with Nuisances from the east while Chonks push center"),
    ("novel", "If I have more than 200 food, train a Catnapper, otherwise train Pawdlers"),
    ("novel", "Create a patrol route between my two Fish Markets"),
    ("novel", "Kite with Hissers but only if there are more than 3 melee enemies nearby"),
]

def format_chat(user_msg: str) -> str:
    """Format as Mistral chat template."""
    return f"<s>[SYSTEM_PROMPT]{SYSTEM_PROMPT}[/SYSTEM_PROMPT][INST]{user_msg}[/INST]"

def check_quality(response: str) -> dict:
    """Score response quality."""
    checks = {
        "intent_header": "-- Intent:" in response,
        "description_header": "-- Description:" in response,
        "uses_ctx": "ctx:" in response or "ctx." in response,
        "nil_guard": any(x in response for x in ["if ", "nil", "#", "== 0", "> 0"]),
        "valid_lua": not any(x in response for x in ["undefined", "null", "console.log", "const ", "let ", "var "]),
        "no_hallucination": not any(x in response for x in ["ctx:spawn", "ctx:delete", "ctx:kill", "ctx:heal", "ctx:teleport"]),
    }
    checks["score"] = sum(1 for v in checks.values() if v is True)
    checks["max"] = len(checks) - 2  # exclude score and max
    return checks

def main():
    base_model = sys.argv[1] if len(sys.argv) > 1 else "mlx-community/Devstral-Small-2-24B-Instruct-2512-4bit"
    use_adapter = "--no-adapter" not in sys.argv

    print(f"Loading model: {base_model}")
    print(f"Adapter: {'YES' if use_adapter else 'NO (baseline)'}")

    model, tokenizer = load(base_model)

    if use_adapter:
        print(f"Applying LoRA adapter from: {ADAPTER_DIR}")
        # mlx-lm LoRA adapter loading
        model = apply_lora_layers(model, ADAPTER_DIR)
        model.eval()

    results = []
    for category, prompt in TEST_PROMPTS:
        print(f"\n{'='*60}")
        print(f"[{category.upper()}] {prompt}")
        print(f"{'='*60}")

        formatted = format_chat(prompt)

        start = time.time()
        response = generate(
            model, tokenizer,
            prompt=formatted,
            max_tokens=1024,
            temp=0.2,
            top_p=0.9,
        )
        elapsed = time.time() - start

        # Print response
        lines = response.split("\n")
        for line in lines[:40]:
            print(f"  {line}")
        if len(lines) > 40:
            print(f"  ... ({len(lines) - 40} more lines)")

        quality = check_quality(response)
        print(f"\n  Time: {elapsed:.1f}s | Score: {quality['score']}/{quality['max']}")
        for name, val in quality.items():
            if name in ("score", "max"):
                continue
            print(f"    [{'PASS' if val else 'FAIL'}] {name}")

        results.append({
            "category": category,
            "prompt": prompt,
            "response": response,
            "elapsed": elapsed,
            "quality": quality,
        })

    # Summary
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")

    for cat in ["standard", "novel"]:
        group = [r for r in results if r["category"] == cat]
        if group:
            avg = sum(r["quality"]["score"] for r in group) / len(group)
            avg_t = sum(r["elapsed"] for r in group) / len(group)
            print(f"  {cat.title()}: {avg:.1f}/{group[0]['quality']['max']} avg score, {avg_t:.1f}s avg time")

    # Save
    outfile = "test_results_finetuned.json" if use_adapter else "test_results_base.json"
    with open(os.path.join(os.path.dirname(__file__), outfile), "w") as f:
        json.dump(results, f, indent=2, default=str)
    print(f"\nSaved to {outfile}")

if __name__ == "__main__":
    main()
