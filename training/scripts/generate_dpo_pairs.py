#!/usr/bin/env python3
"""Generate DPO preference pairs from arena evolution data.

Uses natural pairs from arena generations where one strategy beat another:
- Chosen: winning strategy with correct reasoning
- Rejected: losing strategy with flawed reasoning

Each pair includes <think> blocks explaining why the chosen approach works
and why the rejected approach fails.

Usage:
  python training/scripts/generate_dpo_pairs.py \
    --output training/data/cc_v2_dpo_raw.jsonl \
    --max-pairs 800

  # Dry run
  python training/scripts/generate_dpo_pairs.py --dry-run

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

# Natural DPO pairs from arena evolution
# Each pair has a winner strategy and a loser strategy with real win rates
ARENA_PAIRS = [
    {
        "name": "Focus fire: centroid closest vs per-unit targeting",
        "category": "targeting",
        "chosen": {
            "strategy": "centroid_closest_focus_fire",
            "gen": "P1_gen34",
            "win_rate": 0.80,
            "description": "All attackers target the closest enemy to the attacker centroid",
            "key_code": "Compute centroid of all attackers, find closest enemy within 12 tiles, ctx:attack_units(all_ids, target.id)",
        },
        "rejected": {
            "strategy": "per_unit_targeting",
            "gen": "23",
            "win_rate": 0.0,
            "description": "Each unit independently targets the weakest enemy in its range",
            "key_code": "For each unit, find weakest enemy in range, ctx:attack_units({u.id}, weak.id)",
            "flaw": "Scatters damage across multiple enemies. None die quickly.",
        },
        "prompt_variants": [
            "Handle combat — we have multiple attackers engaged.",
            "Manage the fight. Focus our damage.",
            "Enemy army spotted. Attack them effectively.",
        ],
    },
    {
        "name": "Army concentration vs army splitting",
        "category": "army_management",
        "chosen": {
            "strategy": "concentrated_force",
            "gen": "P1_gen34",
            "win_rate": 0.90,
            "description": "Keep army concentrated, all units fight together",
            "key_code": "All combat units in one group, focus fire as one army",
        },
        "rejected": {
            "strategy": "army_splitting_harassment",
            "gen": "P1_gen29",
            "win_rate": 0.0,
            "description": "Detach a Nuisance to harass enemy workers while main army fights",
            "key_code": "Split: send 1 raider to enemy base, rest fight. ctx.behaviors:harass_economy({raider.id})",
            "flaw": "One fewer unit in main army flips fights from wins to losses.",
        },
        "prompt_variants": [
            "Should I split my army to harass their economy?",
            "Send a raider to their workers while my main army fights?",
            "Can I multitask — harass and fight at the same time?",
        ],
    },
    {
        "name": "Pure combat vs combat + abilities",
        "category": "script_design",
        "chosen": {
            "strategy": "pure_combat_micro",
            "gen": "21",
            "win_rate": 0.50,
            "description": "Only combat commands (attack, move, stop). No abilities.",
            "key_code": "ctx:attack_units(), ctx:move_units() only. No ctx:ability() calls.",
        },
        "rejected": {
            "strategy": "combat_plus_abilities",
            "gen": "22",
            "win_rate": 0.0,
            "description": "Combat micro combined with ability activation scripts",
            "key_code": "ctx:attack_units() followed by ctx:ability(). Ability overrides attack.",
            "flaw": "ctx:ability() commands override ctx:attack_units() for the same unit. Units try to cast instead of attacking.",
        },
        "prompt_variants": [
            "Use abilities during combat to maximize damage.",
            "Activate unit abilities while fighting.",
            "Combat micro with ability support.",
        ],
    },
    {
        "name": "Timed push vs always kiting",
        "category": "late_game",
        "chosen": {
            "strategy": "timed_push",
            "gen": "P1_gen60",
            "win_rate": 0.90,
            "description": "After tick 4000, disable all retreat/kiting. Force all-in push.",
            "key_code": "if ctx:tick() >= 4000 then disable kiting, disable retreat, force attack_move to buildings",
        },
        "rejected": {
            "strategy": "always_kite",
            "gen": "P1_gen24",
            "win_rate": 0.75,
            "description": "Always kite regardless of game phase. Many stalemates.",
            "key_code": "Ranged always kite. No late-game aggression mode.",
            "flaw": "Kiting in late game prevents decisive engagement. Many timeouts instead of wins.",
        },
        "prompt_variants": [
            "It's tick 4500. Should I keep kiting or push?",
            "Late game — enemies are stalling. What to do?",
            "Running out of time. How to force a win?",
        ],
    },
    {
        "name": "Production building targeting vs HQ rush",
        "category": "pushing",
        "chosen": {
            "strategy": "production_building_targeting",
            "gen": "P1_gen30",
            "win_rate": 0.85,
            "description": "Target CatTree/ServerRack first, then HQ, then nearest building",
            "key_code": "Priority: prod buildings (CatTree, ServerRack) > HQ (TheBox) > nearest",
        },
        "rejected": {
            "strategy": "hq_rush",
            "gen": "21",
            "win_rate": 0.50,
            "description": "Rush directly to enemy HQ, ignoring other buildings",
            "key_code": "Find TheBox, attack_move all units to it directly",
            "flaw": "Enemy keeps training units from CatTree while you attack HQ. Gets overwhelmed by reinforcements.",
        },
        "prompt_variants": [
            "We broke through to their base. What do we attack first?",
            "Pushing into enemy base. Target priority?",
            "In their base now. Should I go straight for their HQ?",
        ],
    },
    {
        "name": "Closest targeting vs weakest targeting",
        "category": "targeting",
        "chosen": {
            "strategy": "closest_to_centroid",
            "gen": "P1_gen34",
            "win_rate": 0.80,
            "description": "Target closest enemy to attacker centroid",
            "key_code": "Centroid of attackers, find min distance enemy, all attack same target",
        },
        "rejected": {
            "strategy": "weakest_targeting",
            "gen": "P1_gen51",
            "win_rate": 0.10,
            "description": "Target the weakest enemy (lowest HP%) regardless of position",
            "key_code": "Find lowest hp_pct enemy, all units target it",
            "flaw": "Army chases distant wounded enemies, breaking formation and exposing flanks to healthy threats.",
        },
        "prompt_variants": [
            "Multiple enemies in range. Which one to focus?",
            "Target the weakest enemy to get quick kills?",
            "Focus fire — what targeting approach?",
        ],
    },
    {
        "name": "Inline formation vs separate formation script",
        "category": "script_design",
        "chosen": {
            "strategy": "inline_formation",
            "gen": "P1_gen53",
            "win_rate": 0.85,
            "description": "Formation logic embedded in combat_micro script",
            "key_code": "In combat script: idle tanks forward, idle ranged back, inline with focus fire",
        },
        "rejected": {
            "strategy": "separate_formation_script",
            "gen": "P1_gen52",
            "win_rate": 0.80,
            "description": "Formation as a separate shared script",
            "key_code": "Separate formation_orient.lua script in shared/ directory",
            "flaw": "Separate script commands conflict with combat script commands for same units. No measurable impact.",
        },
        "prompt_variants": [
            "Should I make a separate formation script?",
            "Tank/ranged positioning — separate file or inline?",
            "Best way to implement formation behavior?",
        ],
    },
    {
        "name": "Two features vs three features",
        "category": "script_design",
        "chosen": {
            "strategy": "two_feature_combo",
            "gen": "P1_gen63",
            "win_rate": 0.95,
            "description": "Timed push + inline formation only. Simple and effective.",
            "key_code": "focus_fire + timed_push + inline_formation. No HP kiting.",
        },
        "rejected": {
            "strategy": "three_feature_combo",
            "gen": "P1_gen64",
            "win_rate": 0.70,
            "description": "Timed push + inline formation + HP kiting. Features interfere.",
            "key_code": "focus_fire + timed_push + inline_formation + HP-threshold kite",
            "flaw": "HP kiting overrides timed push in late game. Ranged units retreat when they should commit. Less is more.",
        },
        "prompt_variants": [
            "Should I add HP-based kiting to my combat script?",
            "My combat script has push and formation. Should I add more features?",
            "How many tactical behaviors should one script have?",
        ],
    },
    {
        "name": "Conditional kiting vs unconditional kiting",
        "category": "kiting",
        "chosen": {
            "strategy": "conditional_kiting",
            "gen": "P1_gen26",
            "win_rate": 0.75,
            "description": "Kite only when outnumbered (my_count < enemy_count)",
            "key_code": "if outnumbered and ranged_attackers then kite",
        },
        "rejected": {
            "strategy": "unconditional_kiting",
            "gen": "P1_gen24",
            "win_rate": 0.75,
            "description": "All ranged units always kite when enemies near",
            "key_code": "if ranged and enemies_nearby then always flee",
            "flaw": "Causes stalemates when not outnumbered. Many games end in timeout. Conditional kiting has same effective rate but more decisive wins.",
        },
        "prompt_variants": [
            "Should ranged units always kite?",
            "When should I kite vs stand and fight?",
            "Kiting strategy for my Hissers.",
        ],
    },
    {
        "name": "DPS-weighted targeting vs closest targeting",
        "category": "targeting",
        "chosen": {
            "strategy": "closest_to_centroid",
            "gen": "P1_gen34",
            "win_rate": 0.80,
            "description": "Target closest enemy to attacker centroid",
            "key_code": "Centroid, min distance, all focus one",
        },
        "rejected": {
            "strategy": "dps_weighted_targeting",
            "gen": "P1_gen54",
            "win_rate": 0.50,
            "description": "Target enemy with highest damage/HP ratio",
            "key_code": "Score enemies by e.damage / e.hp, target highest",
            "flaw": "High-DPS enemies are often scattered. Army splits to chase priority targets. Breaks focus fire concentration.",
        },
        "prompt_variants": [
            "Target the most dangerous enemy first?",
            "Should I prioritize high-DPS targets?",
            "Targeting: closest enemy or highest threat?",
        ],
    },
]


DPO_GENERATION_SYSTEM = """\
You are generating DPO (Direct Preference Optimization) training data for a fine-tuned \
LLM that writes Lua scripts for ClawedCommand, an RTS game.

You will generate TWO responses to the same prompt:
1. CHOSEN: The correct approach with good reasoning
2. REJECTED: A plausible but flawed approach with incorrect reasoning

Both responses must include a <think> block followed by a Lua script.

The CHOSEN response should:
- Reference arena-validated patterns correctly
- Use proper tactical reasoning
- Generate correct, efficient Lua code

The REJECTED response should:
- Sound plausible and confident
- Contain a specific tactical error (described in the pair info)
- Generate Lua code that would actually work syntactically but lose games
- Show subtly flawed reasoning in the <think> block

Output format:
===CHOSEN===
<think>...</think>
[lua script]
===REJECTED===
<think>...</think>
[lua script]
"""


def build_dpo_prompt(pair: dict, prompt_variant: str, game_state_text: str) -> str:
    """Build prompt for Claude to generate a DPO pair."""
    return (
        f"## Pair: {pair['name']}\n\n"
        f"## Game State\n{game_state_text}\n\n"
        f"## Player Request\n\"{prompt_variant}\"\n\n"
        f"## Chosen Strategy: {pair['chosen']['strategy']}\n"
        f"Win rate: {pair['chosen']['win_rate']*100:.0f}% (Gen {pair['chosen']['gen']})\n"
        f"Description: {pair['chosen']['description']}\n"
        f"Key code pattern: {pair['chosen']['key_code']}\n\n"
        f"## Rejected Strategy: {pair['rejected']['strategy']}\n"
        f"Win rate: {pair['rejected']['win_rate']*100:.0f}% (Gen {pair['rejected']['gen']})\n"
        f"Description: {pair['rejected']['description']}\n"
        f"Key code pattern: {pair['rejected']['key_code']}\n"
        f"Why it fails: {pair['rejected']['flaw']}\n\n"
        f"Generate BOTH responses. The rejected one should sound plausible but contain the specific flaw described above.\n"
        f"Output:\n===CHOSEN===\n<think>...</think>\n[lua]\n===REJECTED===\n<think>...</think>\n[lua]"
    )


def generate_simple_game_state(pair: dict) -> str:
    """Generate a simple game state string for DPO pairs."""
    tick = random.randint(1500, 5000)

    # Build armies
    my_types = random.choice([
        {"Chonk": 3, "Hisser": 4, "Nuisance": 2},
        {"Chonk": 2, "Hisser": 5, "Yowler": 1, "Nuisance": 2},
        {"Chonk": 4, "Hisser": 3, "FlyingFox": 2},
        {"Chonk": 2, "Hisser": 3, "Mouser": 2, "Nuisance": 1},
    ])
    my_summary = " ".join(f"{v}x{k}" for k, v in my_types.items())
    my_total = sum(my_types.values())

    enemy_count = random.randint(max(3, my_total - 3), my_total + 4)
    enemy_types = random.choice([
        {"Chonk": 4, "Nuisance": 3, "Hisser": 2},
        {"Chonk": 3, "Hisser": 4, "Yowler": 1},
        {"Chonk": 5, "Nuisance": 4},
    ])
    enemy_summary = " ".join(f"{v}x{k}" for k, v in enemy_types.items())

    food = random.randint(50, 400)
    gpu = random.randint(0, 80)

    lines = [
        "GAME STATE:",
        f"Tick: {tick} | My: {my_summary} | "
        f"Enemy: {sum(enemy_types.values())} visible ({enemy_summary}) | "
        f"Food={food} GPU={gpu}",
    ]

    if random.random() < 0.5:
        lines.append("My Buildings: 1xTheBox 1xCatTree 1xLitterBox")

    if random.random() < 0.4:
        lines.append("Enemy Buildings: 1xTheBox 1xCatTree")

    return "\n".join(lines)


def parse_dpo_response(text: str) -> dict | None:
    """Parse Claude's DPO response into chosen + rejected."""
    parts = re.split(r'===CHOSEN===|===REJECTED===', text)
    if len(parts) < 3:
        return None

    chosen_text = parts[1].strip()
    rejected_text = parts[2].strip()

    chosen = parse_single_response(chosen_text)
    rejected = parse_single_response(rejected_text)

    if not chosen or not rejected:
        return None

    return {"chosen": chosen, "rejected": rejected}


def parse_single_response(text: str) -> dict | None:
    """Parse a single response (think + script)."""
    think_match = re.search(r'<think>(.*?)</think>', text, re.DOTALL)
    think_block = think_match.group(1).strip() if think_match else ""

    if think_match:
        script_text = text[think_match.end():].strip()
    else:
        script_text = text

    # Strip code fences
    if script_text.startswith("```lua"):
        script_text = script_text[6:]
    elif script_text.startswith("```"):
        script_text = script_text[3:]
    if script_text.endswith("```"):
        script_text = script_text[:-3]
    script_text = script_text.strip()

    if not script_text:
        return None

    return {"think": think_block, "script": script_text}


def build_dpo_example(system_prompt: str, game_state_text: str,
                      prompt_variant: str, chosen: dict, rejected: dict) -> dict:
    """Build a DPO training example."""
    user_content = f"{game_state_text}\n\nREQUEST: {prompt_variant}"

    return {
        "prompt": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_content},
        ],
        "chosen": [
            {"role": "assistant", "content": f"<think>\n{chosen['think']}\n</think>\n\n{chosen['script']}"},
        ],
        "rejected": [
            {"role": "assistant", "content": f"<think>\n{rejected['think']}\n</think>\n\n{rejected['script']}"},
        ],
    }


def main():
    parser = argparse.ArgumentParser(
        description="Generate DPO preference pairs from arena evolution"
    )
    parser.add_argument(
        "--output", type=Path,
        default=DATA_DIR / "cc_v2_dpo_raw.jsonl",
        help="Output JSONL file",
    )
    parser.add_argument(
        "--max-pairs", type=int, default=800,
        help="Maximum DPO pairs to generate",
    )
    parser.add_argument(
        "--model", type=str, default="claude-sonnet-4-20250514",
        help="Claude model to use",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Show what would be generated",
    )
    parser.add_argument(
        "--delay", type=float, default=0.3,
        help="Delay between API calls",
    )
    args = parser.parse_args()

    system_prompt = SYSTEM_PROMPT_V2_PATH.read_text().strip()

    # Build work queue: each arena pair × prompt variants × game state variants
    work_queue = []
    for pair in ARENA_PAIRS:
        for prompt_variant in pair["prompt_variants"]:
            # 5-8 game state variants per prompt
            n_variants = random.randint(5, 8)
            for _ in range(n_variants):
                work_queue.append({
                    "pair": pair,
                    "prompt_variant": prompt_variant,
                })

    random.shuffle(work_queue)
    if len(work_queue) > args.max_pairs:
        work_queue = work_queue[:args.max_pairs]

    print(f"=== DPO Pair Generation ===")
    print(f"Arena pairs: {len(ARENA_PAIRS)}")
    print(f"Work queue: {len(work_queue)} pairs")
    print(f"Output: {args.output}")

    if args.dry_run:
        print("\n--- Dry Run ---")
        for i, item in enumerate(work_queue[:5]):
            gs = generate_simple_game_state(item["pair"])
            print(f"\n[{i+1}] Pair: {item['pair']['name']}")
            print(f"    Chosen: {item['pair']['chosen']['strategy']} ({item['pair']['chosen']['win_rate']*100:.0f}%)")
            print(f"    Rejected: {item['pair']['rejected']['strategy']} ({item['pair']['rejected']['win_rate']*100:.0f}%)")
            print(f"    Prompt: \"{item['prompt_variant']}\"")
        print(f"\n... and {len(work_queue) - 5} more")
        est_cost = len(work_queue) * 5000 / 1_000_000 * 18
        print(f"\nEstimated cost: ~${est_cost:.0f}")
        return

    if anthropic is None:
        print("Error: anthropic not installed", file=sys.stderr)
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    client = anthropic.Anthropic(api_key=api_key)

    generated = 0
    failures = 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    outfile = open(args.output, "w")

    try:
        for i, item in enumerate(work_queue):
            gs_text = generate_simple_game_state(item["pair"])
            prompt = build_dpo_prompt(item["pair"], item["prompt_variant"], gs_text)

            try:
                response = client.messages.create(
                    model=args.model,
                    max_tokens=6000,
                    system=DPO_GENERATION_SYSTEM,
                    messages=[{"role": "user", "content": prompt}],
                    temperature=0.7,
                )

                if not response.content:
                    failures += 1
                    sys.stdout.write("x")
                    continue
                text = response.content[0].text.strip()
                parsed = parse_dpo_response(text)

                if parsed:
                    example = build_dpo_example(
                        system_prompt, gs_text, item["prompt_variant"],
                        parsed["chosen"], parsed["rejected"],
                    )
                    outfile.write(json.dumps(example, ensure_ascii=False) + "\n")
                    generated += 1
                    sys.stdout.write(".")
                else:
                    failures += 1
                    sys.stdout.write("x")

            except Exception as e:
                failures += 1
                sys.stdout.write("E")
                print(f"\n  Error: {e}", file=sys.stderr)

            sys.stdout.flush()

            if (i + 1) % 50 == 0:
                outfile.flush()
                print(f"\n  [{i+1}/{len(work_queue)}] generated={generated} failures={failures}")

            if args.delay > 0:
                time.sleep(args.delay)

    except KeyboardInterrupt:
        print("\n\nInterrupted! Saving...")
    finally:
        outfile.close()

    print(f"\n\n{'='*60}")
    print(f"DPO Pair Generation Complete")
    print(f"{'='*60}")
    print(f"Generated: {generated} pairs ({failures} failures)")
    print(f"Output: {args.output}")


if __name__ == "__main__":
    main()
