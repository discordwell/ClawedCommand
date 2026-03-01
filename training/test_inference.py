#!/usr/bin/env python3
"""
Test inference quality of the fine-tuned Devstral Small 2 LoRA adapter.
Compares base model vs fine-tuned model on Lua generation prompts.
Requires: Ollama running with 'devstral-small-2' and 'devstral-lua' models.
"""
import json
import subprocess
import sys
import time

# Test prompts: mix of standard (from training) and novel (unseen)
TEST_PROMPTS = [
    # Standard prompts (seen during training)
    ("standard", "Send idle workers to gather food"),
    ("standard", "Attack! Send all combat units at the enemy"),
    ("standard", "Make my Hissers kite away from melee enemies"),
    ("standard", "Focus fire the weakest enemy near my army"),
    ("standard", "Retreat! Fall back to base"),
    # Novel prompts (never seen during training)
    ("novel", "Set up a flanking maneuver with Nuisances from the east while Chonks push center"),
    ("novel", "If I have more than 200 food, train a Catnapper, otherwise train Pawdlers"),
    ("novel", "Create a patrol route between my two Fish Markets"),
    ("novel", "Kite with Hissers but only if there are more than 3 melee enemies nearby"),
    ("novel", "Emergency macro: build 2 Litter Boxes, train 5 Chonks, and rally to map center"),
]

# Quality checks for generated Lua
QUALITY_CHECKS = {
    "has_intent_header": lambda s: "-- Intent:" in s,
    "has_description_header": lambda s: "-- Description:" in s,
    "uses_ctx_api": lambda s: "ctx:" in s or "ctx." in s,
    "has_nil_check": lambda s: "if " in s and ("nil" in s or "#" in s or "== 0" in s or "> 0" in s),
    "valid_lua_syntax": lambda s: not any(bad in s for bad in ["undefined", "null", "console.log", "function()", "const ", "let ", "var "]),
    "no_hallucinated_api": lambda s: not any(bad in s for bad in ["ctx:spawn", "ctx:delete", "ctx:kill", "ctx:heal", "ctx:teleport", "ctx:upgrade_unit"]),
}

def ollama_generate(model: str, prompt: str, system: str = "") -> tuple[str, float]:
    """Generate text using Ollama API via CLI."""
    cmd = ["/opt/homebrew/opt/ollama/bin/ollama", "run", model]

    full_prompt = prompt
    if system:
        # Ollama run doesn't have a --system flag easily, use the API instead
        pass

    start = time.time()
    result = subprocess.run(
        cmd,
        input=full_prompt,
        capture_output=True,
        text=True,
        timeout=120,
    )
    elapsed = time.time() - start
    return result.stdout.strip(), elapsed

def ollama_api_generate(model: str, prompt: str, system: str = "") -> tuple[str, float]:
    """Generate using Ollama HTTP API for better control."""
    import urllib.request

    payload = {
        "model": model,
        "prompt": prompt,
        "system": system,
        "stream": False,
        "options": {
            "temperature": 0.2,
            "top_p": 0.9,
            "num_predict": 2048,
        }
    }

    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        "http://localhost:11434/api/generate",
        data=data,
        headers={"Content-Type": "application/json"},
    )

    start = time.time()
    with urllib.request.urlopen(req, timeout=120) as resp:
        result = json.loads(resp.read().decode())
    elapsed = time.time() - start

    return result.get("response", ""), elapsed

def score_response(response: str) -> dict:
    """Score a response against quality checks."""
    scores = {}
    for name, check_fn in QUALITY_CHECKS.items():
        try:
            scores[name] = check_fn(response)
        except Exception:
            scores[name] = False
    scores["total"] = sum(1 for v in scores.values() if v is True)
    scores["max"] = len(QUALITY_CHECKS)
    return scores

def run_test(model: str, use_system_prompt: bool = True):
    """Run all test prompts against a model."""
    system = ""
    if use_system_prompt:
        try:
            with open("data/system_prompt.txt") as f:
                system = f.read()
        except FileNotFoundError:
            print("Warning: system_prompt.txt not found, running without system prompt")

    results = []
    for category, prompt in TEST_PROMPTS:
        print(f"\n{'='*60}")
        print(f"[{category.upper()}] {prompt}")
        print(f"{'='*60}")

        try:
            response, elapsed = ollama_api_generate(model, prompt, system)
        except Exception as e:
            print(f"  ERROR: {e}")
            results.append({"category": category, "prompt": prompt, "error": str(e)})
            continue

        scores = score_response(response)

        # Print response (truncated)
        lines = response.split("\n")
        for line in lines[:30]:
            print(f"  {line}")
        if len(lines) > 30:
            print(f"  ... ({len(lines) - 30} more lines)")

        print(f"\n  Time: {elapsed:.1f}s")
        print(f"  Score: {scores['total']}/{scores['max']}")
        for name, passed in scores.items():
            if name in ("total", "max"):
                continue
            status = "PASS" if passed else "FAIL"
            print(f"    [{status}] {name}")

        results.append({
            "category": category,
            "prompt": prompt,
            "response": response,
            "elapsed": elapsed,
            "scores": scores,
        })

    return results

def print_summary(model: str, results: list):
    """Print summary statistics."""
    print(f"\n{'='*60}")
    print(f"SUMMARY: {model}")
    print(f"{'='*60}")

    standard = [r for r in results if r.get("category") == "standard" and "error" not in r]
    novel = [r for r in results if r.get("category") == "novel" and "error" not in r]
    errors = [r for r in results if "error" in r]

    for label, group in [("Standard", standard), ("Novel", novel), ("All", standard + novel)]:
        if not group:
            continue
        avg_score = sum(r["scores"]["total"] for r in group) / len(group)
        max_score = group[0]["scores"]["max"] if group else 0
        avg_time = sum(r["elapsed"] for r in group) / len(group)
        print(f"  {label}: {avg_score:.1f}/{max_score} avg score, {avg_time:.1f}s avg time ({len(group)} prompts)")

    if errors:
        print(f"  Errors: {len(errors)}")

def main():
    models_to_test = sys.argv[1:] if len(sys.argv) > 1 else ["devstral-lua"]

    all_results = {}
    for model in models_to_test:
        print(f"\n{'#'*60}")
        print(f"# Testing model: {model}")
        print(f"{'#'*60}")

        # Use system prompt for base model, skip for fine-tuned (it's baked in)
        use_system = model == "devstral-small-2"
        results = run_test(model, use_system_prompt=use_system)
        all_results[model] = results
        print_summary(model, results)

    # Save results
    output_file = "test_results.json"
    with open(output_file, "w") as f:
        json.dump(all_results, f, indent=2, default=str)
    print(f"\nResults saved to {output_file}")

if __name__ == "__main__":
    main()
