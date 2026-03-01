#!/usr/bin/env python3
"""Generate synthetic Lua training examples by augmenting gold examples.

Takes 50 gold examples and uses Claude API to generate variations:
- Same strategy pattern, different unit types/counts
- Same intent, different phrasing
- Different game states (early/mid/late, resource-rich/poor)
- Edge cases (no units available, under attack, etc.)

Target: 50 gold → 500 validated examples.

Usage:
  # Generate 10 variations per gold example (50 * 10 = 500)
  python training/scripts/generate_lua_synthetic.py \
    training/data/gold_lua_examples.jsonl \
    --output training/data/synthetic_lua_examples.jsonl \
    --variations 10

  # Dry run (show prompts without calling API)
  python training/scripts/generate_lua_synthetic.py \
    training/data/gold_lua_examples.jsonl --dry-run

Environment:
  ANTHROPIC_API_KEY — required for Claude API calls
"""

import argparse
import json
import os
import random
import sys
import time
from pathlib import Path

try:
    import anthropic
except ImportError:
    anthropic = None

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent / "data"
SYSTEM_PROMPT_PATH = DATA_DIR / "system_prompt.txt"

# Variation strategies for augmenting gold examples
VARIATION_PROMPTS = [
    # Different unit types
    "Rewrite this script to use different unit types. Change the specific unit "
    "kinds mentioned (e.g., swap Hisser for Chonk, or Nuisance for Mouser). "
    "Keep the same strategic pattern but adapt the logic for the new unit types.",

    # Different phrasing
    "Keep the exact same Lua script output, but rewrite the user prompt to "
    "use completely different wording. Use casual, informal language as a "
    "player might actually type in-game. The script should be identical.",

    # Resource-constrained
    "Modify this script to handle a resource-constrained scenario. Add checks "
    "for low food/gpu_cores before taking actions. The script should gracefully "
    "handle not having enough resources.",

    # Early game variant
    "Adapt this script for an early game scenario where the player has fewer "
    "units and buildings. The script should work with minimal army/infrastructure.",

    # Late game variant
    "Adapt this script for a late game scenario with a large army, multiple "
    "production buildings, and high resources. Scale up the strategy.",

    # Edge case: no units
    "Add robust edge case handling to this script. What if there are no enemy "
    "units visible? No idle workers? No production buildings? Add early returns "
    "for each edge case.",

    # Defensive variant
    "Rewrite this script with a more defensive mindset. If the original attacks, "
    "make it defend. If it expands, make it turtle. Keep using the same API methods.",

    # Aggressive variant
    "Rewrite this script with a more aggressive mindset. Push forward, "
    "prioritize offense over defense. Use attack_move instead of move_units.",

    # Multi-step strategy
    "Expand this script into a multi-step strategy that first queries the game "
    "state to make decisions, then takes different actions based on what it finds. "
    "Add at least one conditional branch based on game state.",

    # Use behaviors
    "Rewrite this script to use ctx.behaviors where possible instead of raw "
    "commands. Replace manual loops with behavior helpers like assign_idle_workers, "
    "focus_fire, kite_squad, etc.",
]


AUGMENTATION_SYSTEM_PROMPT = """\
You are generating training data for a fine-tuned LLM that writes Lua scripts \
for a real-time strategy game. You will be given a gold example (user prompt + \
Lua script) and a variation instruction. Generate a new example following the \
variation instruction.

Rules:
1. Output ONLY valid JSON with two keys: "user" (the new prompt) and "script" (the new Lua code)
2. The Lua script MUST start with -- Intent: and -- Description: header comments
3. Only use ctx API methods from the reference (ctx:my_units, ctx:enemy_units, etc.)
4. The script must be syntactically valid Lua
5. Handle edge cases (empty tables, nil values)
6. Keep scripts concise — under 50 lines
7. Do NOT include markdown code fences or any text outside the JSON object

Valid unit kinds: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, \
Catnapper, FerretSapper, MechCommander
Valid building kinds: TheBox, CatTree, FishMarket, ServerRack, ScratchingPost, \
LitterBox, CatFlap, LaserPointer
Valid resource types: Food, GpuCores, Nfts
"""


def load_gold_examples(path: Path) -> list[dict]:
    """Load gold examples from JSONL."""
    examples = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))
    return examples


def load_system_prompt() -> str:
    """Load the ctx API system prompt."""
    return SYSTEM_PROMPT_PATH.read_text().strip()


def generate_variation(
    client,
    gold_user: str,
    gold_script: str,
    variation_instruction: str,
    ctx_api_reference: str = "",
    model: str = "claude-sonnet-4-20250514",
) -> dict | None:
    """Call Claude API to generate one variation of a gold example."""
    api_section = ""
    if ctx_api_reference:
        api_section = f"## ctx API Reference (use only these methods)\n\n{ctx_api_reference}\n\n"

    prompt = (
        f"{api_section}"
        f"## Gold Example\n\n"
        f"**User prompt:** {gold_user}\n\n"
        f"**Lua script:**\n```lua\n{gold_script}\n```\n\n"
        f"## Variation Instruction\n\n{variation_instruction}\n\n"
        f"Generate the varied example as JSON:"
    )

    try:
        response = client.messages.create(
            model=model,
            max_tokens=2000,
            system=AUGMENTATION_SYSTEM_PROMPT,
            messages=[{"role": "user", "content": prompt}],
            temperature=0.7,
        )

        content = response.content[0].text.strip()

        # Strip markdown code fences if present
        if content.startswith("```"):
            content = content.split("\n", 1)[1]
            if content.endswith("```"):
                content = content.rsplit("```", 1)[0]
            content = content.strip()

        result = json.loads(content)
        if "user" in result and "script" in result:
            return result
        else:
            print(f"  Warning: Missing user/script keys in response", file=sys.stderr)
            return None

    except json.JSONDecodeError as e:
        print(f"  Warning: Failed to parse JSON response: {e}", file=sys.stderr)
        return None
    except Exception as e:
        print(f"  Warning: API call failed: {e}", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(
        description="Generate synthetic Lua training examples from gold set"
    )
    parser.add_argument("input", type=Path, help="Gold examples JSONL")
    parser.add_argument(
        "--output", type=Path,
        default=DATA_DIR / "synthetic_lua_examples.jsonl",
        help="Output JSONL file",
    )
    parser.add_argument(
        "--variations", type=int, default=10,
        help="Number of variations per gold example (default: 10)",
    )
    parser.add_argument(
        "--model", type=str, default="claude-sonnet-4-20250514",
        help="Claude model to use for generation",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Show what would be generated without calling API",
    )
    parser.add_argument(
        "--delay", type=float, default=0.5,
        help="Delay between API calls in seconds (rate limiting)",
    )
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    gold_examples = load_gold_examples(args.input)
    system_prompt = load_system_prompt()

    print(f"Loaded {len(gold_examples)} gold examples")
    print(f"Target: {len(gold_examples) * args.variations} synthetic examples")
    print(f"Variations per example: {args.variations}")

    if args.dry_run:
        print("\n--- Dry Run ---")
        for i, ex in enumerate(gold_examples[:3]):
            user = ex["messages"][1]["content"]
            variation = random.choice(VARIATION_PROMPTS)
            print(f"\nExample {i+1}: \"{user}\"")
            print(f"Variation: {variation[:80]}...")
        print(f"\n... and {len(gold_examples) - 3} more examples")
        print(f"\nWould generate {len(gold_examples) * args.variations} examples")
        return

    if anthropic is None:
        print("Error: anthropic package not installed. Run: pip install anthropic",
              file=sys.stderr)
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY environment variable not set",
              file=sys.stderr)
        sys.exit(1)

    client = anthropic.Anthropic(api_key=api_key)

    synthetic_examples = []
    total_attempts = 0
    total_failures = 0

    for i, gold in enumerate(gold_examples):
        user_msg = gold["messages"][1]["content"]
        script = gold["messages"][2]["content"]

        print(f"\n[{i+1}/{len(gold_examples)}] \"{user_msg[:50]}...\"")

        # Select variation strategies (shuffle for diversity)
        variations = list(VARIATION_PROMPTS)
        random.shuffle(variations)
        variations = variations[:args.variations]

        for j, variation in enumerate(variations):
            total_attempts += 1
            result = generate_variation(
                client, user_msg, script, variation,
                ctx_api_reference=system_prompt, model=args.model,
            )

            if result:
                example = {
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": result["user"]},
                        {"role": "assistant", "content": result["script"]},
                    ]
                }
                synthetic_examples.append(example)
                sys.stdout.write(".")
                sys.stdout.flush()
            else:
                total_failures += 1
                sys.stdout.write("x")
                sys.stdout.flush()

            if args.delay > 0:
                time.sleep(args.delay)

        print()

    # Write output
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w") as f:
        for ex in synthetic_examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")

    print(f"\n{'='*60}")
    print(f"Synthetic generation complete")
    print(f"{'='*60}")
    print(f"Generated: {len(synthetic_examples)}/{total_attempts} ({total_failures} failures)")
    print(f"Output:    {args.output}")
    print(f"\nNext steps:")
    print(f"  1. Validate: python training/scripts/validate_lua_data.py {args.output}")
    print(f"  2. Evaluate: python training/scripts/eval_lua.py {args.output}")
    print(f"  3. Merge:    cat {args.input} {args.output} > training/data/combined_lua.jsonl")


if __name__ == "__main__":
    main()
