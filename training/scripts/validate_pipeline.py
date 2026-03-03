#!/usr/bin/env python3
"""Validate generated training data for the v2 pipeline.

Checks:
1. Lua syntax (basic parse validation)
2. API conformance (all ctx: calls match documented API)
3. Nil guard check (queries followed by nil/empty checks)
4. Budget audit (estimated usage < 50 points)
5. Think block check (<think> present, reasonable length)
6. Arena smoke test (optional: run scripts in 200-tick arena)

Usage:
  python training/scripts/validate_pipeline.py training/data/cc_v2_sft_raw.jsonl
  python training/scripts/validate_pipeline.py training/data/cc_v2_dpo_raw.jsonl --dpo
  python training/scripts/validate_pipeline.py training/data/cc_v2_sft_raw.jsonl --arena-test
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent

# Valid ctx: API methods
VALID_CTX_METHODS = {
    # Unit queries (cost 1)
    "my_units", "enemy_units", "idle_units", "wounded_units", "units_by_state",
    "count_units", "army_supply", "hp_pct",
    # Spatial queries (cost 2)
    "enemies_in_range", "nearest_enemy", "threats_to", "targets_for",
    "weakest_enemy_in_range", "strongest_enemy_in_range",
    "distance_squared_between", "distance_squared_to_nearest_enemy",
    "safe_positions", "position_at_range",
    # Pathfinding (cost 10)
    "can_reach", "path_length",
    # Buildings (cost 1)
    "my_buildings", "enemy_buildings",
    # Economy (free or cost 1)
    "resources", "get_resources", "nearest_deposit", "resource_deposits",
    # Terrain (cost 1)
    "terrain_at", "elevation_at", "cover_at", "is_passable",
    "movement_cost",
    # Game state (free)
    "tick", "map_size",
    # Commands (free)
    "move_units", "attack_units", "attack_move", "stop", "hold",
    "gather", "build", "train", "ability", "research", "rally",
    "cancel_queue", "cancel_research", "set_control_group",
}

# Valid ctx.behaviors: methods
VALID_BEHAVIOR_METHODS = {
    "assign_idle_workers", "attack_move_group", "focus_fire", "kite_squad",
    "retreat_wounded", "defend_area", "harass_economy", "scout_pattern",
    "focus_weakest", "use_ability", "split_squads", "protect_unit",
    "surround_target", "auto_produce", "balanced_production",
    "expand_economy", "coordinate_assault", "research_priority",
    "adaptive_defense",
}

# Budget costs for queries
QUERY_COSTS = {
    "my_units": 1, "enemy_units": 1, "idle_units": 1, "wounded_units": 1,
    "units_by_state": 1, "count_units": 1, "army_supply": 1, "hp_pct": 1,
    "my_buildings": 1, "enemy_buildings": 1,
    "resources": 0, "get_resources": 0, "nearest_deposit": 1,
    "resource_deposits": 1,
    "terrain_at": 1, "elevation_at": 1, "cover_at": 1, "is_passable": 1,
    "movement_cost": 1,
    "tick": 0, "map_size": 0,
    "enemies_in_range": 2, "nearest_enemy": 2, "threats_to": 2,
    "targets_for": 2, "weakest_enemy_in_range": 2, "strongest_enemy_in_range": 2,
    "distance_squared_between": 2, "distance_squared_to_nearest_enemy": 2,
    "safe_positions": 2, "position_at_range": 2,
    "can_reach": 10, "path_length": 10,
}

class ValidationResult:
    def __init__(self):
        self.total = 0
        self.valid = 0
        self.errors = []
        self.warnings = []
        self.category_counts = {}

    def add_error(self, idx: int, msg: str):
        self.errors.append(f"[{idx}] {msg}")

    def add_warning(self, idx: int, msg: str):
        self.warnings.append(f"[{idx}] {msg}")

    def summary(self) -> str:
        lines = [
            f"Total examples: {self.total}",
            f"Valid: {self.valid} ({self.valid/max(self.total,1)*100:.0f}%)",
            f"Errors: {len(self.errors)}",
            f"Warnings: {len(self.warnings)}",
        ]
        if self.category_counts:
            lines.append("Categories:")
            for cat, count in sorted(self.category_counts.items()):
                pct = count / max(self.total, 1) * 100
                flag = " ⚠ >30%" if pct > 30 else ""
                lines.append(f"  {cat}: {count} ({pct:.0f}%){flag}")
        return "\n".join(lines)


def extract_script(example: dict, is_dpo: bool = False) -> list[str]:
    """Extract Lua script(s) from an example."""
    scripts = []
    if is_dpo:
        # DPO format: prompt + chosen + rejected
        for key in ["chosen", "rejected"]:
            msgs = example.get(key, [])
            for msg in msgs:
                if msg.get("role") == "assistant":
                    scripts.append(extract_script_from_text(msg["content"]))
    else:
        # SFT format: messages
        msgs = example.get("messages", [])
        for msg in msgs:
            if msg.get("role") == "assistant":
                scripts.append(extract_script_from_text(msg["content"]))
    return [s for s in scripts if s]


def extract_script_from_text(text: str) -> str:
    """Extract Lua script from assistant response (after think block)."""
    # Remove think block
    think_match = re.search(r'<think>.*?</think>', text, re.DOTALL)
    if think_match:
        script = text[think_match.end():].strip()
    else:
        script = text.strip()

    # Remove code fences
    if script.startswith("```lua"):
        script = script[6:]
    elif script.startswith("```"):
        script = script[3:]
    if script.endswith("```"):
        script = script[:-3]

    return script.strip()


def check_lua_syntax(script: str) -> list[str]:
    """Basic Lua syntax validation (not a full parser)."""
    errors = []

    # Check balanced blocks
    openers = len(re.findall(r'\b(function|if|for|while|repeat)\b', script))
    closers = len(re.findall(r'\bend\b', script))
    untils = len(re.findall(r'\buntil\b', script))
    if openers > closers + untils:
        errors.append(f"Unbalanced blocks: {openers} openers, {closers} end + {untils} until")

    # Check for common mistakes
    if "goto " in script:
        errors.append("goto not supported in Luau sandbox")
    if "::label::" in script.lower():
        errors.append("goto labels not supported in Luau sandbox")
    if "_G." in script and "_G =" not in script:
        pass  # Reading _G is fine
    if re.search(r'_G\s*=\s*', script):
        errors.append("_G is read-only in Luau sandbox")

    return errors


def check_api_conformance(script: str) -> list[str]:
    """Check that all ctx: calls use valid API methods."""
    errors = []

    # Find ctx:method() calls
    for match in re.finditer(r'ctx:(\w+)', script):
        method = match.group(1)
        if method not in VALID_CTX_METHODS:
            errors.append(f"Unknown ctx method: ctx:{method}")

    # Find ctx.behaviors:method() calls
    for match in re.finditer(r'ctx\.behaviors:(\w+)', script):
        method = match.group(1)
        if method not in VALID_BEHAVIOR_METHODS:
            errors.append(f"Unknown behavior: ctx.behaviors:{method}")

    return errors


def check_nil_guards(script: str) -> list[str]:
    """Check that queries are followed by nil/empty guards."""
    warnings = []

    # Patterns: local x = ctx:query() should be followed by nil check
    query_patterns = [
        (r'local\s+(\w+)\s*=\s*ctx:my_units\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:enemy_units\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:idle_units\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:my_buildings\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:enemy_buildings\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:nearest_enemy\b', "{var}"),
        (r'local\s+(\w+)\s*=\s*ctx:nearest_deposit\b', "{var}"),
    ]

    for pattern, _ in query_patterns:
        for match in re.finditer(pattern, script):
            var_name = match.group(1)
            # Check if there's a nil guard within the next 5 lines
            remaining = script[match.end():]
            next_lines = "\n".join(remaining.split("\n")[:5])
            has_guard = (
                f"not {var_name}" in next_lines
                or f"if {var_name}" in next_lines
                or f"#{var_name}" in next_lines
                or f"{var_name} and" in next_lines
                or f"{var_name} then" in next_lines
            )
            if not has_guard:
                warnings.append(f"Missing nil guard for {var_name} = ctx:{pattern.split(':')[1].split('\\b')[0]}")

    return warnings


def estimate_budget(script: str) -> int:
    """Estimate budget usage of a script."""
    total = 0
    for match in re.finditer(r'ctx:(\w+)', script):
        method = match.group(1)
        cost = QUERY_COSTS.get(method, 0)
        total += cost
    return total


def check_think_block(text: str) -> list[str]:
    """Check think block presence and quality."""
    errors = []

    think_match = re.search(r'<think>(.*?)</think>', text, re.DOTALL)
    if not think_match:
        errors.append("Missing <think> block")
        return errors

    think_content = think_match.group(1).strip()
    lines = think_content.splitlines()

    if len(lines) < 1:
        errors.append("<think> block is empty")
    elif len(lines) > 10:
        errors.append(f"<think> block too long ({len(lines)} lines, max 10)")

    return errors


def validate_sft_example(idx: int, example: dict, result: ValidationResult):
    """Validate a single SFT training example."""
    prev_error_count = len(result.errors)
    msgs = example.get("messages", [])

    # Check message structure
    if len(msgs) < 3:
        result.add_error(idx, f"Too few messages: {len(msgs)} (need system+user+assistant)")
        return

    roles = [m.get("role") for m in msgs]
    if roles != ["system", "user", "assistant"]:
        result.add_error(idx, f"Wrong role sequence: {roles}")
        return

    assistant_text = msgs[2]["content"]

    # Check think block
    for err in check_think_block(assistant_text):
        result.add_error(idx, err)

    # Extract and validate script
    script = extract_script_from_text(assistant_text)
    if not script:
        result.add_error(idx, "No Lua script in assistant response")
        return

    # Lua syntax
    for err in check_lua_syntax(script):
        result.add_error(idx, f"Lua syntax: {err}")

    # API conformance
    for err in check_api_conformance(script):
        result.add_error(idx, f"API: {err}")

    # Nil guards
    for warn in check_nil_guards(script):
        result.add_warning(idx, f"Nil guard: {warn}")

    # Budget
    budget = estimate_budget(script)
    if budget > 50:
        result.add_warning(idx, f"High budget estimate: {budget} points")

    # Intent header
    if "-- Intent:" not in script and "--Intent:" not in script:
        result.add_warning(idx, "Missing -- Intent: header")

    # Only count as valid if no errors were added for this example
    if len(result.errors) == prev_error_count:
        result.valid += 1


def validate_dpo_example(idx: int, example: dict, result: ValidationResult):
    """Validate a single DPO training example."""
    for key in ["prompt", "chosen", "rejected"]:
        if key not in example:
            result.add_error(idx, f"Missing '{key}' in DPO example")
            return

    # Validate both chosen and rejected
    for variant in ["chosen", "rejected"]:
        msgs = example[variant]
        if not msgs:
            result.add_error(idx, f"Empty {variant} response")
            continue

        text = msgs[0].get("content", "")

        # Check think block
        for err in check_think_block(text):
            result.add_error(idx, f"{variant}: {err}")

        # Extract and validate script
        script = extract_script_from_text(text)
        if not script:
            result.add_error(idx, f"{variant}: No Lua script")
            continue

        for err in check_api_conformance(script):
            result.add_error(idx, f"{variant} API: {err}")

    result.valid += 1


def run_arena_smoke_test(scripts: list[str], max_tests: int = 20) -> list[str]:
    """Run scripts in 200-tick arena matches to check for runtime errors."""
    errors = []
    tested = 0

    for i, script in enumerate(scripts[:max_tests]):
        tested += 1
        try:
            result = subprocess.run(
                [
                    "cargo", "run", "-p", "cc_agent", "--bin", "arena",
                    "--features", "harness", "--",
                    "--seeds", "42",
                    "--max-ticks", "200",
                    "--p1-inline", script,
                ],
                capture_output=True,
                text=True,
                timeout=30,
                cwd=str(PROJECT_ROOT),
            )
            if result.returncode != 0:
                errors.append(f"Script {i}: arena returned {result.returncode}")
            if "script_errors" in result.stdout.lower() or "panic" in result.stderr.lower():
                errors.append(f"Script {i}: runtime error detected")
        except subprocess.TimeoutExpired:
            errors.append(f"Script {i}: timed out (>30s)")
        except Exception as e:
            errors.append(f"Script {i}: {e}")

        sys.stdout.write(f"\r  Arena test {tested}/{min(len(scripts), max_tests)}")
        sys.stdout.flush()

    print()
    return errors


def main():
    parser = argparse.ArgumentParser(
        description="Validate v2 training data"
    )
    parser.add_argument("input", type=Path, help="JSONL file to validate")
    parser.add_argument("--dpo", action="store_true",
                       help="Validate as DPO format instead of SFT")
    parser.add_argument("--arena-test", action="store_true",
                       help="Run arena smoke tests (slow)")
    parser.add_argument("--max-arena-tests", type=int, default=20,
                       help="Max scripts to arena test")
    parser.add_argument("--output", type=Path,
                       help="Write valid examples to this file")
    parser.add_argument("--verbose", "-v", action="store_true",
                       help="Show all errors")
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    result = ValidationResult()
    all_scripts = []

    print(f"=== Validating {args.input.name} ===")
    print(f"Mode: {'DPO' if args.dpo else 'SFT'}")

    valid_examples = []

    with open(args.input) as f:
        for i, line in enumerate(f):
            line = line.strip()
            if not line:
                continue

            result.total += 1
            prev_errors = len(result.errors)

            try:
                example = json.loads(line)
            except json.JSONDecodeError as e:
                result.add_error(i, f"Invalid JSON: {e}")
                continue

            if args.dpo:
                validate_dpo_example(i, example, result)
            else:
                validate_sft_example(i, example, result)

            # Collect scripts for arena testing
            scripts = extract_script(example, is_dpo=args.dpo)
            all_scripts.extend(scripts)

            # Track valid examples for output
            if len(result.errors) == prev_errors:
                valid_examples.append(example)

    print(f"\n{result.summary()}")

    if args.verbose and result.errors:
        print(f"\nErrors (first 20):")
        for err in result.errors[:20]:
            print(f"  {err}")

    if args.verbose and result.warnings:
        print(f"\nWarnings (first 20):")
        for warn in result.warnings[:20]:
            print(f"  {warn}")

    # Arena smoke test
    if args.arena_test and all_scripts:
        print(f"\n=== Arena Smoke Test ===")
        arena_errors = run_arena_smoke_test(all_scripts, args.max_arena_tests)
        if arena_errors:
            print(f"Arena errors: {len(arena_errors)}")
            for err in arena_errors[:10]:
                print(f"  {err}")
        else:
            print(f"All {min(len(all_scripts), args.max_arena_tests)} scripts passed arena test")

    # Write valid examples
    if args.output and valid_examples:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, "w") as f:
            for ex in valid_examples:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"\nValid examples written: {len(valid_examples)} → {args.output}")


if __name__ == "__main__":
    main()
