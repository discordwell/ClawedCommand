#!/usr/bin/env python3
"""Extract game state snapshots from arena matches and generate situational training examples.

Runs arena matches with --snapshot-interval to capture game state at regular intervals,
then uses Claude API to generate situationally appropriate scripts + reasoning.

Usage:
  # Step 1: Run matches and capture snapshots
  python training/scripts/extract_snapshots.py --capture \
    --seeds 1,2,3,4,5 --snapshot-interval 500 \
    --p1-scripts training/arena/gen_063/player_1/

  # Step 2: Generate training examples from snapshots
  python training/scripts/extract_snapshots.py --generate \
    --snapshot-dir training/data/snapshots/ \
    --output training/data/cc_v2_snapshot_examples.jsonl

  # Both steps
  python training/scripts/extract_snapshots.py --capture --generate \
    --seeds 1,2,3,4,5,6,7,8,9,10

Environment:
  ANTHROPIC_API_KEY — required for --generate step
"""

import argparse
import json
import os
import random
import re
import subprocess
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
DEFAULT_SNAPSHOT_DIR = DATA_DIR / "snapshots"
DEFAULT_P1_SCRIPTS = PROJECT_ROOT / "training" / "arena" / "gen_063" / "player_1"


def run_arena_with_snapshots(seeds: list[int], snapshot_interval: int,
                             output_dir: Path, p1_scripts: Path | None = None,
                             max_ticks: int = 6000) -> int:
    """Run arena matches with snapshot capture enabled."""
    output_dir.mkdir(parents=True, exist_ok=True)

    seeds_str = ",".join(str(s) for s in seeds)
    cmd = [
        "cargo", "run", "-p", "cc_agent", "--bin", "arena",
        "--features", "harness", "--release", "--",
        "--seeds", seeds_str,
        "--snapshot-interval", str(snapshot_interval),
        "--max-ticks", str(max_ticks),
        "--output", str(output_dir),
    ]

    if p1_scripts:
        cmd.extend(["--p1-scripts", str(p1_scripts)])

    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=False, cwd=str(PROJECT_ROOT))

    if result.returncode != 0:
        print(f"Warning: arena exited with code {result.returncode}", file=sys.stderr)

    # Count snapshots
    snap_dir = output_dir / "snapshots"
    if snap_dir.exists():
        count = len(list(snap_dir.glob("*.json")))
        print(f"Captured {count} snapshots → {snap_dir}")
        return count
    return 0


def load_snapshots(snapshot_dir: Path) -> list[dict]:
    """Load all snapshot JSON files."""
    snapshots = []
    for path in sorted(snapshot_dir.glob("*.json")):
        with open(path) as f:
            snap = json.load(f)
            snap["_path"] = str(path)
            snapshots.append(snap)
    return snapshots


def format_snapshot_as_game_state(snap: dict) -> str:
    """Convert a cc_sim GameStateSnapshot into the training data game state format."""
    tick = snap.get("tick", 0)

    # Player data
    players = snap.get("players", [])
    p0_food = 0
    p0_gpu = 0
    p0_supply = 0
    p0_supply_cap = 0
    for p in players:
        if p.get("player_id") == 0:
            p0_food = p.get("food", 0)
            p0_gpu = p.get("gpu_cores", 0)
            p0_supply = p.get("supply", 0)
            p0_supply_cap = p.get("supply_cap", 0)

    # Unit counts by player
    units = snap.get("units", [])
    my_counts = {}
    enemy_counts = {}
    my_wounded = 0
    for u in units:
        kind = u.get("kind", "Unknown")
        owner = u.get("owner", 0)
        if owner == 0:
            my_counts[kind] = my_counts.get(kind, 0) + 1
            hp = u.get("health_current", 0)
            hp_max = u.get("health_max", 1)
            if hp < hp_max * 0.5:
                my_wounded += 1
        else:
            enemy_counts[kind] = enemy_counts.get(kind, 0) + 1

    my_summary = " ".join(f"{v}x{k}" for k, v in sorted(my_counts.items())) or "none"
    enemy_total = sum(enemy_counts.values())
    if enemy_counts:
        enemy_summary = f"{enemy_total} visible ({' '.join(f'{v}x{k}' for k, v in sorted(enemy_counts.items()))})"
    else:
        enemy_summary = "none visible"

    # Buildings
    buildings = snap.get("buildings", [])
    my_buildings = {}
    enemy_buildings = {}
    for b in buildings:
        kind = b.get("kind", "Unknown")
        owner = b.get("owner", 0)
        if owner == 0:
            my_buildings[kind] = my_buildings.get(kind, 0) + 1
        else:
            enemy_buildings[kind] = enemy_buildings.get(kind, 0) + 1

    lines = [
        "GAME STATE:",
        f"Tick: {tick} | My: {my_summary} | Enemy: {enemy_summary} | "
        f"Food={p0_food} GPU={p0_gpu} Supply={p0_supply}/{p0_supply_cap}",
    ]

    if my_buildings:
        bldg = " ".join(f"{v}x{k}" for k, v in sorted(my_buildings.items()))
        lines.append(f"My Buildings: {bldg}")

    if enemy_buildings:
        ebldg = " ".join(f"{v}x{k}" for k, v in sorted(enemy_buildings.items()))
        lines.append(f"Enemy Buildings: {ebldg}")

    if my_wounded > 0:
        lines.append(f"Wounded: {my_wounded} units below 50% HP")

    # Combat stats
    melee = snap.get("melee_attack_count", 0)
    ranged = snap.get("ranged_attack_count", 0)
    if melee > 0 or ranged > 0:
        lines.append(f"Combat: {melee} melee + {ranged} ranged attacks total")

    return "\n".join(lines)


def classify_snapshot(snap: dict) -> str:
    """Classify a snapshot into a game phase for prompt selection."""
    tick = snap.get("tick", 0)

    units = snap.get("units", [])
    my_combat = sum(1 for u in units if u.get("owner") == 0
                    and u.get("kind") not in ("Pawdler", "Scrounger", "Delver", "Ponderer"))
    enemy_combat = sum(1 for u in units if u.get("owner") != 0
                       and u.get("kind") not in ("Pawdler", "Scrounger", "Delver", "Ponderer"))

    melee = snap.get("melee_attack_count", 0)
    ranged = snap.get("ranged_attack_count", 0)
    in_combat = melee > 0 or ranged > 0

    if tick < 500:
        return "early_game"
    elif tick >= 4000:
        return "late_game"
    elif my_combat < enemy_combat - 2:
        return "outnumbered"
    elif my_combat > enemy_combat + 2:
        return "advantage"
    elif in_combat:
        return "mid_combat"
    else:
        return "mid_game"


PHASE_PROMPTS = {
    "early_game": [
        "Set up early economy and start building.",
        "Get workers gathering and queue first units.",
        "Early game setup — economy first.",
    ],
    "mid_game": [
        "Manage the mid-game. Build army and prepare to fight.",
        "Balance economy and army production.",
        "Get ready for combat — build a balanced force.",
    ],
    "mid_combat": [
        "We're in a fight. Handle combat micro.",
        "Combat happening — focus fire and manage the army.",
        "Fight's on. Micromanage our units.",
    ],
    "outnumbered": [
        "We're outnumbered. Fight smart.",
        "They have more units. Kite and focus fire.",
        "Outnumbered — preserve army with smart micro.",
    ],
    "advantage": [
        "We have the advantage. Push and attack.",
        "Army lead — go destroy their buildings.",
        "We outnumber them. Press the attack.",
    ],
    "late_game": [
        "Late game — force a decisive fight.",
        "Time's running out. All-in push.",
        "Late game push. No more retreating.",
    ],
}

SNAPSHOT_GENERATION_SYSTEM = """\
You are generating training data for a fine-tuned LLM that writes Lua scripts \
for ClawedCommand, an RTS game. You are given a real game state snapshot from an \
actual arena match. Generate an appropriate response.

Rules:
1. Include a <think> block (2-5 lines) analyzing the SPECIFIC game state numbers
2. Reference arena-validated patterns (focus fire, timed push, formation, etc.)
3. Generate a complete Lua script appropriate for the current game phase
4. Use ONLY documented ctx: API methods
5. Handle nil/empty edge cases
6. Budget usage < 50 points

Output: <think>...</think> followed by the Lua script (no code fences, no JSON wrapper).
"""


def generate_snapshot_example(client, system_prompt: str, model: str,
                              snap: dict, prompt: str) -> dict | None:
    """Generate a training example from a snapshot."""
    gs_text = format_snapshot_as_game_state(snap)
    phase = classify_snapshot(snap)

    gen_prompt = (
        f"## Game Phase: {phase}\n\n"
        f"## Actual Game State (from arena match)\n{gs_text}\n\n"
        f"## Player Request\n\"{prompt}\"\n\n"
        f"Generate the assistant response (think block + Lua script) for this specific game state.\n"
        f"The script should be appropriate for the current situation based on the actual unit counts, "
        f"tick number, and resource levels shown above."
    )

    try:
        response = client.messages.create(
            model=model,
            max_tokens=4000,
            system=SNAPSHOT_GENERATION_SYSTEM,
            messages=[{"role": "user", "content": gen_prompt}],
            temperature=0.7,
        )

        text = response.content[0].text.strip()

        # Parse think + script
        think_match = re.search(r'<think>(.*?)</think>', text, re.DOTALL)
        think_block = think_match.group(1).strip() if think_match else ""

        script_text = text[think_match.end():].strip() if think_match else text
        if script_text.startswith("```lua"):
            script_text = script_text[6:]
        elif script_text.startswith("```"):
            script_text = script_text[3:]
        if script_text.endswith("```"):
            script_text = script_text[:-3]
        script_text = script_text.strip()

        if not script_text or "ctx:" not in script_text:
            return None

        user_content = f"{gs_text}\n\nREQUEST: {prompt}"
        assistant_content = f"<think>\n{think_block}\n</think>\n\n{script_text}"

        return {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_content},
                {"role": "assistant", "content": assistant_content},
            ],
            "metadata": {
                "source": "snapshot",
                "phase": phase,
                "tick": snap.get("tick", 0),
            },
        }

    except Exception as e:
        print(f"\n  Error: {e}", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(
        description="Extract snapshots and generate situational training examples"
    )
    parser.add_argument("--capture", action="store_true",
                       help="Run arena matches to capture snapshots")
    parser.add_argument("--generate", action="store_true",
                       help="Generate training examples from snapshots")
    parser.add_argument("--seeds", type=str, default="1,2,3,4,5,6,7,8,9,10",
                       help="Comma-separated seeds for arena matches")
    parser.add_argument("--snapshot-interval", type=int, default=500,
                       help="Ticks between snapshots (default: 500)")
    parser.add_argument("--snapshot-dir", type=Path, default=DEFAULT_SNAPSHOT_DIR,
                       help="Directory for snapshot JSON files")
    parser.add_argument("--p1-scripts", type=Path, default=DEFAULT_P1_SCRIPTS,
                       help="P1 script directory for arena matches")
    parser.add_argument("--output", type=Path,
                       default=DATA_DIR / "cc_v2_snapshot_examples.jsonl",
                       help="Output JSONL file for training examples")
    parser.add_argument("--max-examples", type=int, default=500,
                       help="Maximum examples to generate")
    parser.add_argument("--model", type=str, default="claude-sonnet-4-20250514",
                       help="Claude model for generation")
    parser.add_argument("--dry-run", action="store_true",
                       help="Show what would happen")
    parser.add_argument("--delay", type=float, default=0.3,
                       help="Delay between API calls")
    args = parser.parse_args()

    # Step 1: Capture snapshots
    if args.capture:
        seeds = [int(s) for s in args.seeds.split(",") if s.strip()]
        print(f"=== Capturing Snapshots ===")
        print(f"Seeds: {seeds}")
        print(f"Interval: every {args.snapshot_interval} ticks")

        if args.dry_run:
            est = len(seeds) * (6000 // args.snapshot_interval)
            print(f"Would capture ~{est} snapshots from {len(seeds)} matches")
        else:
            # Run in the parent of snapshot-dir so snapshots go to snapshot_dir/snapshots/
            output_base = args.snapshot_dir.parent if args.snapshot_dir.name == "snapshots" else args.snapshot_dir
            count = run_arena_with_snapshots(
                seeds, args.snapshot_interval, output_base,
                args.p1_scripts,
            )
            print(f"Captured {count} snapshots")

    # Step 2: Generate training examples
    if args.generate:
        print(f"\n=== Generating Snapshot Examples ===")

        snap_dir = args.snapshot_dir
        if not snap_dir.exists():
            # Try the nested path
            alt = args.snapshot_dir.parent / "snapshots"
            if alt.exists():
                snap_dir = alt
            else:
                print(f"Error: snapshot directory not found: {snap_dir}", file=sys.stderr)
                sys.exit(1)

        snapshots = load_snapshots(snap_dir)
        print(f"Loaded {len(snapshots)} snapshots")

        if not snapshots:
            print("No snapshots to process.")
            return

        # Build work queue: each snapshot × 1-2 prompts
        work_queue = []
        for snap in snapshots:
            phase = classify_snapshot(snap)
            prompts = PHASE_PROMPTS.get(phase, PHASE_PROMPTS["mid_game"])
            # Pick 1-2 random prompts per snapshot
            selected = random.sample(prompts, min(2, len(prompts)))
            for prompt in selected:
                work_queue.append({"snap": snap, "prompt": prompt, "phase": phase})

        random.shuffle(work_queue)
        if len(work_queue) > args.max_examples:
            work_queue = work_queue[:args.max_examples]

        print(f"Work queue: {len(work_queue)} examples")

        if args.dry_run:
            for i, item in enumerate(work_queue[:5]):
                gs = format_snapshot_as_game_state(item["snap"])
                print(f"\n[{i+1}] Phase: {item['phase']}")
                print(f"    Prompt: \"{item['prompt']}\"")
                print(f"    State: {gs.splitlines()[1][:80]}...")
            print(f"\n... and {len(work_queue) - 5} more")
            return

        if anthropic is None:
            print("Error: anthropic not installed", file=sys.stderr)
            sys.exit(1)

        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
            sys.exit(1)

        client = anthropic.Anthropic(api_key=api_key)
        system_prompt = SYSTEM_PROMPT_V2_PATH.read_text().strip()

        generated = 0
        failures = 0

        args.output.parent.mkdir(parents=True, exist_ok=True)
        outfile = open(args.output, "w")

        try:
            for i, item in enumerate(work_queue):
                result = generate_snapshot_example(
                    client, system_prompt, args.model,
                    item["snap"], item["prompt"],
                )

                if result:
                    outfile.write(json.dumps(result, ensure_ascii=False) + "\n")
                    generated += 1
                    sys.stdout.write(".")
                else:
                    failures += 1
                    sys.stdout.write("x")
                sys.stdout.flush()

                if (i + 1) % 50 == 0:
                    outfile.flush()
                    print(f"\n  [{i+1}/{len(work_queue)}] generated={generated} failures={failures}")

                if args.delay > 0:
                    time.sleep(args.delay)

        except KeyboardInterrupt:
            print("\n\nInterrupted!")
        finally:
            outfile.close()

        print(f"\n\n{'='*60}")
        print(f"Snapshot Example Generation Complete")
        print(f"{'='*60}")
        print(f"Generated: {generated} ({failures} failures)")
        print(f"Output: {args.output}")

    if not args.capture and not args.generate:
        print("Specify --capture, --generate, or both.")
        parser.print_help()


if __name__ == "__main__":
    main()
