#!/usr/bin/env python3
"""Generate synthetic training data from gold examples using Claude API.

Takes gold examples as templates and generates 500-1000 variations by:
- Varying player instructions (different phrasing, unit counts, positions)
- Varying game states (resource levels, map positions, enemy compositions)
- Generating diverse scenarios (early/mid/late game, offensive/defensive)

Quality filtering is applied automatically via validate_data.py checks.

Usage:
    export ANTHROPIC_API_KEY=your_key
    python generate_synthetic.py ../data/gold_50.jsonl --count 500 --output ../data/synthetic.jsonl
"""

import argparse
import json
import os
import random
import re
import sys
import time
from pathlib import Path

import anthropic

VALID_TOOLS = {
    "get_units", "move_units", "attack_units", "build", "train_unit",
    "get_visible_enemies", "get_resources", "get_buildings", "get_map_info",
    "set_rally_point", "patrol", "gather_resource", "execute_strategy",
}

TOOL_CALL_ID_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"

VARIATION_PROMPT = """You are generating training data for a fine-tuned model that controls an RTS game via tool calls.

Given this gold training example, generate a NEW variation that:
1. Uses a DIFFERENT player instruction (rephrase, change specifics like unit counts, positions, target types)
2. Uses a DIFFERENT game state (vary resource amounts 50-2000, unit counts 1-30, positions 0-100, enemy types)
3. Keeps the same general PATTERN (same tool call sequence structure)
4. Uses realistic, varied game scenarios (early game economy, mid game expansion, late game push, defense, harassment)

CRITICAL FORMAT RULES:
- Tool call IDs must be exactly 9 random alphanumeric characters
- Arguments must be STRINGIFIED JSON (a string, not an object)
- Message ordering: system → user → assistant (tool_calls) → tool (result) → ...
- Last message must be assistant with content (final summary text)
- Tool names must be from: {tools}

Return ONLY the JSON object (one line, no markdown fences). The object must have "messages" and "tools" keys.

GOLD EXAMPLE:
{example}

Generate one new variation:"""


def generate_tool_call_id() -> str:
    return "".join(random.choices(TOOL_CALL_ID_CHARS, k=9))


def quick_validate(example: dict) -> list[str]:
    """Quick validation checks. Returns list of errors (empty = valid)."""
    errors = []

    if "messages" not in example:
        errors.append("Missing 'messages'")
        return errors
    if "tools" not in example:
        errors.append("Missing 'tools'")

    messages = example["messages"]
    if not messages:
        errors.append("Empty messages")
        return errors

    # Check last message is assistant with content
    last = messages[-1]
    if last.get("role") != "assistant":
        errors.append(f"Last message role is '{last.get('role')}', expected 'assistant'")
    elif not last.get("content"):
        errors.append("Last assistant message has no content")

    # Check tool calls
    for msg in messages:
        if msg.get("role") == "assistant" and "tool_calls" in msg:
            for tc in msg["tool_calls"]:
                fn = tc.get("function", {})
                name = fn.get("name", "")
                if name not in VALID_TOOLS:
                    errors.append(f"Invalid tool: '{name}'")
                args = fn.get("arguments", "")
                if not isinstance(args, str):
                    errors.append(f"Arguments not stringified for '{name}'")
                else:
                    try:
                        json.loads(args)
                    except json.JSONDecodeError:
                        errors.append(f"Invalid JSON in arguments for '{name}'")
                tc_id = tc.get("id", "")
                if not re.match(r"^[a-zA-Z0-9]{9}$", tc_id):
                    errors.append(f"Invalid tool call ID: '{tc_id}'")

    return errors


def generate_variation(
    client: anthropic.Anthropic, gold_example: dict, max_retries: int = 3
) -> dict | None:
    """Generate one synthetic variation from a gold example."""
    prompt = VARIATION_PROMPT.format(
        tools=", ".join(sorted(VALID_TOOLS)),
        example=json.dumps(gold_example, indent=2),
    )

    for attempt in range(max_retries):
        try:
            response = client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=4096,
                messages=[{"role": "user", "content": prompt}],
                temperature=0.8,
            )

            text = response.content[0].text.strip()
            # Strip markdown fences if present
            if text.startswith("```"):
                text = re.sub(r"^```\w*\n?", "", text)
                text = re.sub(r"\n?```$", "", text)
            text = text.strip()

            variation = json.loads(text)
            errors = quick_validate(variation)
            if errors:
                print(f"    Validation failed (attempt {attempt+1}): {errors[0]}")
                continue

            return variation

        except (json.JSONDecodeError, KeyError, IndexError) as e:
            print(f"    Parse error (attempt {attempt+1}): {e}")
            continue
        except anthropic.RateLimitError:
            print("    Rate limited, waiting 10s...")
            time.sleep(10)
            continue

    return None


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic training data")
    parser.add_argument("gold_file", type=Path, help="Gold examples JSONL")
    parser.add_argument("--output", type=Path, required=True, help="Output JSONL")
    parser.add_argument(
        "--count", type=int, default=500, help="Target number of synthetic examples"
    )
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument(
        "--dry-run", action="store_true", help="Generate 3 examples and print"
    )
    args = parser.parse_args()

    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    if not args.gold_file.exists():
        print(f"Error: {args.gold_file} not found", file=sys.stderr)
        sys.exit(1)

    random.seed(args.seed)
    client = anthropic.Anthropic(api_key=api_key)

    # Load gold examples
    gold_examples = []
    with open(args.gold_file) as f:
        for line in f:
            line = line.strip()
            if line:
                gold_examples.append(json.loads(line))

    print(f"Loaded {len(gold_examples)} gold examples")

    if args.dry_run:
        args.count = 3

    target = args.count
    generated = []
    failures = 0

    print(f"Generating {target} synthetic examples...")

    while len(generated) < target:
        # Pick a random gold example as template
        template = random.choice(gold_examples)
        idx = len(generated) + 1

        print(f"  [{idx}/{target}] Generating variation...")
        variation = generate_variation(client, template)

        if variation:
            generated.append(variation)
            if args.dry_run:
                print(json.dumps(variation, indent=2))
                print()
        else:
            failures += 1
            if failures > target * 0.5:
                print(f"Too many failures ({failures}), stopping.")
                break

    print(f"\nGenerated: {len(generated)}, Failed: {failures}")

    if not args.dry_run:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, "w") as f:
            for ex in generated:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"Wrote {len(generated)} examples to {args.output}")


if __name__ == "__main__":
    main()
