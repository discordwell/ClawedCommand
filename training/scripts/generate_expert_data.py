#!/usr/bin/env python3
"""Generate expert SFT training data using Claude API + arena knowledge.

For each scenario template × prompt variant × game state variant:
- Constructs a game state snapshot
- Calls Claude API to generate <think> block + Lua script
- Validates output format
- Writes to JSONL

Usage:
  python training/scripts/generate_expert_data.py \
    --output training/data/cc_v2_sft_raw.jsonl \
    --max-examples 2000

  # Dry run (no API calls)
  python training/scripts/generate_expert_data.py --dry-run

  # Resume from partial output
  python training/scripts/generate_expert_data.py \
    --output training/data/cc_v2_sft_raw.jsonl --resume

Environment:
  ANTHROPIC_API_KEY — required for Claude API calls
"""

import argparse
import json
import os
import random
import re
import sys
import time
from pathlib import Path

try:
    import anthropic
except ImportError:
    anthropic = None

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
DATA_DIR = PROJECT_ROOT / "training" / "data"
SYSTEM_PROMPT_V2_PATH = DATA_DIR / "system_prompt_v2.txt"
ARENA_KNOWLEDGE_PATH = DATA_DIR / "arena_knowledge.json"
SCENARIO_TEMPLATES_PATH = DATA_DIR / "scenario_templates.json"
CHAMPION_MODULES_PATH = DATA_DIR / "champion_modules.json"

# Unit stats for generating realistic game states
UNIT_STATS = {
    "Pawdler": {"hp": 30, "speed": 0.1, "damage": 5, "range": 1.0, "attack_type": "Melee"},
    "Nuisance": {"hp": 45, "speed": 0.15, "damage": 8, "range": 1.5, "attack_type": "Melee"},
    "Chonk": {"hp": 150, "speed": 0.06, "damage": 12, "range": 1.5, "attack_type": "Melee"},
    "FlyingFox": {"hp": 40, "speed": 0.14, "damage": 10, "range": 4.0, "attack_type": "Ranged"},
    "Hisser": {"hp": 70, "speed": 0.1, "damage": 14, "range": 5.0, "attack_type": "Ranged"},
    "Yowler": {"hp": 50, "speed": 0.08, "damage": 6, "range": 4.0, "attack_type": "Ranged"},
    "Mouser": {"hp": 55, "speed": 0.13, "damage": 18, "range": 1.5, "attack_type": "Melee"},
    "Catnapper": {"hp": 80, "speed": 0.05, "damage": 30, "range": 7.0, "attack_type": "Ranged"},
    "FerretSapper": {"hp": 60, "speed": 0.09, "damage": 25, "range": 2.0, "attack_type": "Melee"},
    "MechCommander": {"hp": 200, "speed": 0.07, "damage": 20, "range": 3.0, "attack_type": "Melee"},
}

COMBAT_UNITS = [k for k in UNIT_STATS if k != "Pawdler"]
RANGED_UNITS = [k for k, v in UNIT_STATS.items() if v["attack_type"] == "Ranged"]
MELEE_UNITS = [k for k, v in UNIT_STATS.items() if v["attack_type"] == "Melee" and k != "Pawdler"]

BUILDING_KINDS = ["TheBox", "CatTree", "FishMarket", "ServerRack",
                  "ScratchingPost", "LitterBox", "CatFlap", "LaserPointer"]


def load_system_prompt() -> str:
    return SYSTEM_PROMPT_V2_PATH.read_text().strip()


def load_arena_knowledge() -> dict:
    with open(ARENA_KNOWLEDGE_PATH) as f:
        return json.load(f)


def load_scenarios() -> dict:
    with open(SCENARIO_TEMPLATES_PATH) as f:
        return json.load(f)


def load_modules() -> dict:
    with open(CHAMPION_MODULES_PATH) as f:
        return json.load(f)


def generate_unit_id() -> int:
    """Generate a realistic entity ID."""
    return random.randint(100, 99999)


def generate_army(count_range: list, unit_types: dict | None = None,
                  all_workers: bool = False, wounded: bool = False) -> list[dict]:
    """Generate a realistic army composition."""
    if all_workers:
        count = random.randint(max(1, count_range[0]), count_range[1])
        units = []
        for _ in range(count):
            stats = UNIT_STATS["Pawdler"]
            units.append({
                "id": generate_unit_id(),
                "kind": "Pawdler",
                "x": random.randint(5, 58),
                "y": random.randint(5, 58),
                "hp": stats["hp"],
                "hp_max": stats["hp"],
                "speed": stats["speed"],
                "damage": stats["damage"],
                "range": stats["range"],
                "attack_type": stats["attack_type"],
                "attacking": False,
                "moving": False,
                "idle": True,
                "gathering": random.random() < 0.5,
            })
        return units

    if unit_types:
        units = []
        for kind, count_r in unit_types.items():
            count = random.randint(count_r[0], count_r[1])
            stats = UNIT_STATS.get(kind, UNIT_STATS["Nuisance"])
            for _ in range(count):
                hp = stats["hp"]
                if wounded and random.random() < 0.4:
                    hp = int(hp * random.uniform(0.1, 0.5))
                units.append({
                    "id": generate_unit_id(),
                    "kind": kind,
                    "x": random.randint(5, 58),
                    "y": random.randint(5, 58),
                    "hp": hp,
                    "hp_max": stats["hp"],
                    "speed": stats["speed"],
                    "damage": stats["damage"],
                    "range": stats["range"],
                    "attack_type": stats["attack_type"],
                    "attacking": random.random() < 0.3,
                    "moving": False,
                    "idle": random.random() < 0.4,
                    "gathering": False,
                })
        return units

    count = random.randint(count_range[0], count_range[1])
    if count == 0:
        return []

    # Generate realistic composition
    units = []
    # Always include some workers
    n_workers = random.randint(1, min(3, count))
    n_combat = count - n_workers

    for _ in range(n_workers):
        stats = UNIT_STATS["Pawdler"]
        units.append({
            "id": generate_unit_id(),
            "kind": "Pawdler",
            "x": random.randint(5, 58),
            "y": random.randint(5, 58),
            "hp": stats["hp"],
            "hp_max": stats["hp"],
            "speed": stats["speed"],
            "damage": stats["damage"],
            "range": stats["range"],
            "attack_type": stats["attack_type"],
            "attacking": False,
            "moving": False,
            "idle": random.random() < 0.3,
            "gathering": random.random() < 0.5,
        })

    for _ in range(n_combat):
        kind = random.choice(COMBAT_UNITS)
        stats = UNIT_STATS[kind]
        hp = stats["hp"]
        if wounded and random.random() < 0.4:
            hp = int(hp * random.uniform(0.1, 0.5))
        in_combat = random.random() < 0.3
        units.append({
            "id": generate_unit_id(),
            "kind": kind,
            "x": random.randint(5, 58),
            "y": random.randint(5, 58),
            "hp": hp,
            "hp_max": stats["hp"],
            "speed": stats["speed"],
            "damage": stats["damage"],
            "range": stats["range"],
            "attack_type": stats["attack_type"],
            "attacking": in_combat,
            "moving": not in_combat and random.random() < 0.2,
            "idle": not in_combat and random.random() < 0.5,
            "gathering": False,
        })

    return units


def generate_enemy_army(count_range: list) -> list[dict]:
    """Generate enemy army (simplified — less detail visible)."""
    count = random.randint(count_range[0], count_range[1])
    units = []
    for _ in range(count):
        kind = random.choice(COMBAT_UNITS)
        stats = UNIT_STATS[kind]
        units.append({
            "id": generate_unit_id(),
            "kind": kind,
            "x": random.randint(5, 58),
            "y": random.randint(5, 58),
            "hp": stats["hp"],
            "hp_max": stats["hp"],
            "attack_type": stats["attack_type"],
        })
    return units


def format_game_state(tick: int, my_units: list, enemy_units: list,
                      resources: dict, my_buildings: list | None = None,
                      enemy_buildings: list | None = None) -> str:
    """Format game state into the human-readable format for user messages."""
    # Summarize my units
    my_counts = {}
    for u in my_units:
        my_counts[u["kind"]] = my_counts.get(u["kind"], 0) + 1
    my_summary = " ".join(f"{v}x{k}" for k, v in sorted(my_counts.items()))

    # Summarize enemy units
    enemy_count = len(enemy_units)
    enemy_counts = {}
    for u in enemy_units:
        enemy_counts[u["kind"]] = enemy_counts.get(u["kind"], 0) + 1
    if enemy_units:
        enemy_summary = f"{enemy_count} visible ({' '.join(f'{v}x{k}' for k, v in sorted(enemy_counts.items()))})"
    else:
        enemy_summary = "none visible"

    # Resources
    food = resources.get("food", 0)
    gpu = resources.get("gpu_cores", 0)
    supply = resources.get("supply", len(my_units))
    supply_cap = resources.get("supply_cap", 20)

    lines = [
        "GAME STATE:",
        f"Tick: {tick} | My: {my_summary} | Enemy: {enemy_summary} | "
        f"Food={food} GPU={gpu} Supply={supply}/{supply_cap}",
    ]

    # Add building info if relevant
    if my_buildings:
        bldg_summary = " ".join(
            f"{sum(1 for b in my_buildings if b['kind']==k)}x{k}"
            for k in sorted(set(b["kind"] for b in my_buildings))
        )
        lines.append(f"My Buildings: {bldg_summary}")

    if enemy_buildings:
        ebldg_summary = " ".join(
            f"{sum(1 for b in enemy_buildings if b['kind']==k)}x{k}"
            for k in sorted(set(b["kind"] for b in enemy_buildings))
        )
        lines.append(f"Enemy Buildings: {ebldg_summary}")

    # Wounded info
    wounded = [u for u in my_units if u["hp"] < u.get("hp_max", u["hp"]) * 0.5]
    if wounded:
        lines.append(f"Wounded: {len(wounded)} units below 50% HP")

    return "\n".join(lines)


def generate_game_state_for_scenario(scenario: dict) -> dict:
    """Generate a randomized game state matching a scenario template."""
    template = scenario["game_state_template"]

    tick = random.randint(*template.get("tick_range", [1000, 4000]))

    # Resources
    res_template = template.get("resources", {"food": [100, 300], "gpu_cores": [10, 50]})
    food = random.randint(*res_template.get("food", [100, 300]))
    gpu = random.randint(*res_template.get("gpu_cores", [0, 50]))
    supply_near_cap = template.get("supply_near_cap", False)

    # My units
    all_workers = template.get("my_units_all_workers", False)
    wounded = template.get("my_units_wounded", False)
    unit_types = template.get("my_unit_types")
    my_units = generate_army(
        template.get("my_units_range", [5, 10]),
        unit_types=unit_types,
        all_workers=all_workers,
        wounded=wounded,
    )

    supply = sum(1 for u in my_units if u["kind"] != "Pawdler") * 2 + \
             sum(1 for u in my_units if u["kind"] == "Pawdler")
    supply_cap = supply + (random.randint(0, 2) if supply_near_cap else random.randint(4, 10))

    # Enemy units
    enemy_units = generate_enemy_army(template.get("enemy_units_range", [5, 10]))

    # Buildings
    my_buildings = []
    if template.get("buildings_present", False):
        my_buildings.append({"kind": "TheBox", "x": random.randint(3, 10),
                            "y": random.randint(3, 10), "id": generate_unit_id(),
                            "hp": 500, "hp_max": 500})
        if template.get("has_production", False) or random.random() < 0.7:
            my_buildings.append({"kind": "CatTree", "x": random.randint(5, 15),
                                "y": random.randint(5, 15), "id": generate_unit_id(),
                                "hp": 300, "hp_max": 300})
        if random.random() < 0.4:
            my_buildings.append({"kind": "LitterBox", "x": random.randint(3, 12),
                                "y": random.randint(3, 12), "id": generate_unit_id(),
                                "hp": 200, "hp_max": 200})

    enemy_buildings = []
    if template.get("enemy_buildings_present", False) or random.random() < 0.5:
        enemy_buildings.append({"kind": "TheBox", "x": random.randint(50, 60),
                               "y": random.randint(50, 60), "id": generate_unit_id(),
                               "hp": 500, "hp_max": 500})
        if random.random() < 0.6:
            enemy_buildings.append({"kind": "CatTree", "x": random.randint(48, 58),
                                   "y": random.randint(48, 58), "id": generate_unit_id(),
                                   "hp": 300, "hp_max": 300})

    resources = {
        "food": food, "gpu_cores": gpu,
        "supply": supply, "supply_cap": supply_cap,
    }

    return {
        "tick": tick,
        "my_units": my_units,
        "enemy_units": enemy_units,
        "resources": resources,
        "my_buildings": my_buildings,
        "enemy_buildings": enemy_buildings,
    }


def build_generation_prompt(scenario: dict, game_state: dict,
                           prompt_variant: str, arena_knowledge: dict) -> str:
    """Build the full prompt for Claude to generate a training example."""
    gs = format_game_state(
        game_state["tick"],
        game_state["my_units"],
        game_state["enemy_units"],
        game_state["resources"],
        game_state.get("my_buildings"),
        game_state.get("enemy_buildings"),
    )

    # Build context about expected behaviors
    expected = scenario.get("expected_behaviors", [])
    avoid = scenario.get("anti_patterns_to_avoid", [])

    context_lines = []
    if expected:
        context_lines.append(f"Expected behaviors: {', '.join(expected)}")
    if avoid:
        context_lines.append(f"Anti-patterns to avoid: {', '.join(avoid)}")

    context = "\n".join(context_lines)

    return (
        f"Generate a training example for this scenario.\n\n"
        f"## Scenario: {scenario['name']}\n"
        f"{scenario['description']}\n\n"
        f"## {context}\n\n"
        f"## Game State\n{gs}\n\n"
        f"## Player Request\n\"{prompt_variant}\"\n\n"
        f"## Instructions\n"
        f"Generate the assistant response as it should appear in the training data.\n"
        f"The response MUST include:\n"
        f"1. A <think> block (2-5 lines) analyzing the situation, referencing arena-validated patterns\n"
        f"2. A complete Lua script with -- Intent: and -- Description: headers\n"
        f"3. Proper nil/empty checks for all ctx: queries\n"
        f"4. Budget-efficient queries (estimated budget usage < 50/500)\n\n"
        f"Output ONLY the assistant response (think block + script). No JSON wrapper, no explanation."
    )


GENERATION_SYSTEM_PROMPT = """\
You are generating expert training data for a fine-tuned LLM that writes Lua scripts \
for ClawedCommand, a real-time strategy game. You generate the ASSISTANT side of training \
examples: a <think> reasoning block followed by a complete Lua script.

Rules:
1. The <think> block must reference arena-validated patterns (focus fire, timed push, etc.)
2. The Lua script must start with -- Intent: and -- Description: comment headers
3. Use ONLY documented ctx: API methods
4. Handle nil/empty edge cases for ALL queries
5. Keep budget usage under 50 points (queries cost 1-10, commands are free)
6. For combat scripts: ALWAYS use centroid-closest focus fire, NEVER per-unit targeting
7. For complex scripts: combine at most 2 complementary behaviors
8. Scripts should be 20-200 lines (simple tasks shorter, combat micro longer)

Output format: <think>...</think> followed by the Lua script (no code fences, no JSON wrapper).
"""


def parse_response(text: str) -> dict | None:
    """Parse Claude's response into think block + script."""
    text = text.strip()

    # Extract think block
    think_match = re.search(r'<think>(.*?)</think>', text, re.DOTALL)
    think_block = think_match.group(1).strip() if think_match else ""

    # Extract script (everything after think block, strip code fences)
    if think_match:
        script_text = text[think_match.end():].strip()
    else:
        script_text = text

    # Remove markdown code fences
    if script_text.startswith("```lua"):
        script_text = script_text[6:]
    elif script_text.startswith("```"):
        script_text = script_text[3:]
    if script_text.endswith("```"):
        script_text = script_text[:-3]
    script_text = script_text.strip()

    if not script_text:
        return None

    # Basic validation
    if "ctx:" not in script_text and "ctx.behaviors:" not in script_text:
        return None

    return {
        "think": think_block,
        "script": script_text,
    }


def build_training_example(system_prompt: str, game_state_text: str,
                           prompt_variant: str, think_block: str,
                           script: str) -> dict:
    """Assemble a complete training example in JSONL format."""
    user_content = f"{game_state_text}\n\nREQUEST: {prompt_variant}"
    assistant_content = f"<think>\n{think_block}\n</think>\n\n{script}"

    return {
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content},
            {"role": "assistant", "content": assistant_content},
        ]
    }


def count_existing(path: Path) -> int:
    """Count existing examples in a JSONL file."""
    if not path.exists():
        return 0
    count = 0
    with open(path) as f:
        for line in f:
            if line.strip():
                count += 1
    return count


def main():
    parser = argparse.ArgumentParser(
        description="Generate expert SFT training data using Claude API"
    )
    parser.add_argument(
        "--output", type=Path,
        default=DATA_DIR / "cc_v2_sft_raw.jsonl",
        help="Output JSONL file",
    )
    parser.add_argument(
        "--max-examples", type=int, default=2000,
        help="Maximum examples to generate (default: 2000)",
    )
    parser.add_argument(
        "--model", type=str, default="claude-sonnet-4-20250514",
        help="Claude model to use",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Show what would be generated without calling API",
    )
    parser.add_argument(
        "--resume", action="store_true",
        help="Resume from existing partial output",
    )
    parser.add_argument(
        "--delay", type=float, default=0.3,
        help="Delay between API calls in seconds",
    )
    parser.add_argument(
        "--batch-size", type=int, default=50,
        help="Save checkpoint every N examples",
    )
    args = parser.parse_args()

    # Load all data
    system_prompt = load_system_prompt()
    arena_knowledge = load_arena_knowledge()
    scenarios = load_scenarios()
    modules = load_modules()

    # Flatten scenarios into a work queue
    work_queue = []
    for category_name, category in scenarios["categories"].items():
        for scenario in category["scenarios"]:
            for prompt_variant in scenario["prompt_variants"]:
                # Generate 3-5 game state variants per prompt
                n_variants = random.randint(3, 5)
                for _ in range(n_variants):
                    work_queue.append({
                        "category": category_name,
                        "scenario": scenario,
                        "prompt_variant": prompt_variant,
                    })

    random.seed(42)
    random.shuffle(work_queue)

    # Limit to max examples
    if len(work_queue) > args.max_examples:
        work_queue = work_queue[:args.max_examples]

    print(f"=== Expert SFT Data Generation ===")
    print(f"Total scenarios: {sum(len(c['scenarios']) for c in scenarios['categories'].values())}")
    print(f"Work queue: {len(work_queue)} examples")
    print(f"Output: {args.output}")
    print(f"Model: {args.model}")

    if args.dry_run:
        print("\n--- Dry Run ---")
        # Show a few example prompts
        for i, item in enumerate(work_queue[:5]):
            gs = generate_game_state_for_scenario(item["scenario"])
            gs_text = format_game_state(
                gs["tick"], gs["my_units"], gs["enemy_units"],
                gs["resources"], gs.get("my_buildings"), gs.get("enemy_buildings"),
            )
            print(f"\n[{i+1}] Category: {item['category']}")
            print(f"    Scenario: {item['scenario']['name']}")
            print(f"    Prompt: \"{item['prompt_variant']}\"")
            print(f"    Game State:\n      {gs_text}")

        print(f"\n... and {len(work_queue) - 5} more")
        print(f"\nEstimated cost: ~{len(work_queue) * 3300 / 1_000_000 * 18:.0f}$ "
              f"({len(work_queue)} examples × ~3.3K tokens × $18/MTok)")
        return

    # Check API key
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

    # Resume handling
    existing_count = 0
    if args.resume:
        existing_count = count_existing(args.output)
        if existing_count > 0:
            print(f"Resuming: {existing_count} existing examples, "
                  f"generating {len(work_queue) - existing_count} more")
            work_queue = work_queue[existing_count:]

    # Generate
    generated = 0
    failures = 0
    mode = "a" if args.resume and existing_count > 0 else "w"

    args.output.parent.mkdir(parents=True, exist_ok=True)
    outfile = open(args.output, mode)

    category_counts = {}

    try:
        for i, item in enumerate(work_queue):
            gs = generate_game_state_for_scenario(item["scenario"])
            gs_text = format_game_state(
                gs["tick"], gs["my_units"], gs["enemy_units"],
                gs["resources"], gs.get("my_buildings"), gs.get("enemy_buildings"),
            )
            prompt = build_generation_prompt(
                item["scenario"], gs, item["prompt_variant"], arena_knowledge,
            )

            try:
                response = client.messages.create(
                    model=args.model,
                    max_tokens=4000,
                    system=GENERATION_SYSTEM_PROMPT,
                    messages=[{"role": "user", "content": prompt}],
                    temperature=0.7,
                )

                if not response.content:
                    failures += 1
                    sys.stdout.write("x")
                    continue
                text = response.content[0].text.strip()
                parsed = parse_response(text)

                if parsed:
                    example = build_training_example(
                        system_prompt, gs_text, item["prompt_variant"],
                        parsed["think"], parsed["script"],
                    )
                    outfile.write(json.dumps(example, ensure_ascii=False) + "\n")
                    generated += 1
                    category_counts[item["category"]] = \
                        category_counts.get(item["category"], 0) + 1
                    sys.stdout.write(".")
                else:
                    failures += 1
                    sys.stdout.write("x")

            except Exception as e:
                failures += 1
                sys.stdout.write("E")
                print(f"\n  Error: {e}", file=sys.stderr)

            sys.stdout.flush()

            # Checkpoint
            if (generated + failures) % args.batch_size == 0:
                outfile.flush()
                total = generated + failures + existing_count
                print(f"\n  [{total}/{len(work_queue) + existing_count}] "
                      f"generated={generated} failures={failures}")

            if args.delay > 0:
                time.sleep(args.delay)

    except KeyboardInterrupt:
        print("\n\nInterrupted! Saving progress...")
    finally:
        outfile.close()

    total = generated + existing_count
    print(f"\n\n{'='*60}")
    print(f"Expert SFT Generation Complete")
    print(f"{'='*60}")
    print(f"Generated: {generated} new ({failures} failures)")
    print(f"Total: {total} examples")
    print(f"Output: {args.output}")
    print(f"\nCategory distribution:")
    for cat, count in sorted(category_counts.items()):
        pct = count / max(generated, 1) * 100
        print(f"  {cat}: {count} ({pct:.0f}%)")
    print(f"\nNext steps:")
    print(f"  1. Validate: python training/scripts/validate_pipeline.py {args.output}")
    print(f"  2. Assemble: python training/scripts/assemble_dataset.py")


if __name__ == "__main__":
    main()
