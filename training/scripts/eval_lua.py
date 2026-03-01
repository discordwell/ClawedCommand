#!/usr/bin/env python3
"""Evaluate Lua scripts generated for ClawedCommand.

Checks each script on a rubric:
1. Syntax validity — does it parse as valid Lua?
2. API correctness — does it only call valid ctx methods?
3. Command coverage — does it issue at least one game command?
4. Header comments — does it have Intent/Description headers?
5. No runtime errors — does it execute against a mock game state?

Usage:
  python training/scripts/eval_lua.py training/data/gold_lua_examples.jsonl
  python training/scripts/eval_lua.py --script path/to/script.lua
  python training/scripts/eval_lua.py training/data/gold_lua_examples.jsonl --output results.json

Requires: lupa (pip install lupa) for Lua syntax checking, or falls back to
          subprocess with luajit/lua if available.
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field, asdict
from pathlib import Path

from lua_api_surface import (
    VALID_CTX_METHODS, VALID_BEHAVIOR_METHODS, COMMAND_METHODS,
    CTX_METHOD_PATTERN, BEHAVIOR_METHOD_PATTERN,
    INTENT_PATTERN, DESCRIPTION_PATTERN,
    check_lua_block_balance,
)


# ---------------------------------------------------------------------------
# Score dataclass
# ---------------------------------------------------------------------------

@dataclass
class ScriptScore:
    """Evaluation score for a single Lua script."""
    example_index: int = 0
    user_prompt: str = ""
    syntax_valid: bool = False
    syntax_error: str = ""
    has_intent_header: bool = False
    has_description_header: bool = False
    ctx_methods_used: list = field(default_factory=list)
    behavior_methods_used: list = field(default_factory=list)
    invalid_ctx_methods: list = field(default_factory=list)
    invalid_behavior_methods: list = field(default_factory=list)
    issues_commands: bool = False
    command_methods_used: list = field(default_factory=list)
    runtime_ok: bool = False
    runtime_error: str = ""

    @property
    def api_correct(self) -> bool:
        return len(self.invalid_ctx_methods) == 0 and len(self.invalid_behavior_methods) == 0

    @property
    def total_score(self) -> float:
        """Score from 0.0 to 1.0 across all rubric dimensions."""
        points = 0.0
        total = 5.0

        if self.syntax_valid:
            points += 1.0
        if self.has_intent_header and self.has_description_header:
            points += 1.0
        elif self.has_intent_header or self.has_description_header:
            points += 0.5
        if self.api_correct:
            points += 1.0
        if self.issues_commands:
            points += 1.0
        if self.runtime_ok:
            points += 1.0

        return points / total

    def summary(self) -> str:
        status = "PASS" if self.total_score >= 0.8 else "WARN" if self.total_score >= 0.6 else "FAIL"
        parts = [f"[{status}] {self.total_score:.0%}"]
        if not self.syntax_valid:
            parts.append(f"syntax:{self.syntax_error[:40]}")
        if self.invalid_ctx_methods:
            parts.append(f"bad_ctx:{self.invalid_ctx_methods}")
        if self.invalid_behavior_methods:
            parts.append(f"bad_bhv:{self.invalid_behavior_methods}")
        if not self.issues_commands:
            parts.append("no_commands")
        if not self.has_intent_header:
            parts.append("no_intent")
        return " | ".join(parts)


# ---------------------------------------------------------------------------
# Lua syntax checking
# ---------------------------------------------------------------------------

def check_lua_syntax(script: str) -> tuple[bool, str]:
    """Check if script is valid Lua syntax using luac/luajit/lupa."""
    # Try luajit first (fast, supports Luau-like syntax)
    for cmd in ["luajit", "luac", "lua"]:
        try:
            with tempfile.NamedTemporaryFile(mode="w", suffix=".lua", delete=False) as f:
                f.write(script)
                f.flush()
                result = subprocess.run(
                    [cmd, "-p", f.name] if cmd != "luajit" else [cmd, "-bl", f.name],
                    capture_output=True, text=True, timeout=5,
                )
                Path(f.name).unlink(missing_ok=True)
                if result.returncode == 0:
                    return True, ""
                else:
                    return False, result.stderr.strip()
        except (FileNotFoundError, subprocess.TimeoutExpired):
            Path(f.name).unlink(missing_ok=True)
            continue

    # Fall back to Python-based syntax heuristics
    return check_lua_syntax_heuristic(script)


def check_lua_syntax_heuristic(script: str) -> tuple[bool, str]:
    """Basic heuristic Lua syntax validation when no Lua binary is available."""
    # Check balanced blocks using shared heuristic
    openers, closers = check_lua_block_balance(script)
    if openers != closers:
        return False, f"Unbalanced blocks: {openers} openers vs {closers} 'end' closures"

    # Check for obviously invalid syntax
    if re.search(r"[^=!<>]==[^=]", script):
        pass  # == is valid comparison in Lua

    # Check unclosed strings
    single_quotes = script.count("'") - script.count("\\'")
    double_quotes = script.count('"') - script.count('\\"')
    if single_quotes % 2 != 0:
        return False, "Unclosed single-quoted string"
    if double_quotes % 2 != 0:
        return False, "Unclosed double-quoted string"

    return True, ""


# ---------------------------------------------------------------------------
# API correctness checking
# ---------------------------------------------------------------------------

def check_api_usage(script: str) -> tuple[list, list, list, list]:
    """Extract and validate ctx API method calls.

    Returns (valid_ctx, invalid_ctx, valid_behavior, invalid_behavior).
    """
    ctx_calls = CTX_METHOD_PATTERN.findall(script)
    behavior_calls = BEHAVIOR_METHOD_PATTERN.findall(script)

    valid_ctx = [m for m in ctx_calls if m in VALID_CTX_METHODS]
    invalid_ctx = [m for m in ctx_calls if m not in VALID_CTX_METHODS]

    valid_behavior = [m for m in behavior_calls if m in VALID_BEHAVIOR_METHODS]
    invalid_behavior = [m for m in behavior_calls if m not in VALID_BEHAVIOR_METHODS]

    return valid_ctx, invalid_ctx, valid_behavior, invalid_behavior


def check_commands(script: str) -> tuple[bool, list]:
    """Check if the script issues any game commands."""
    ctx_calls = CTX_METHOD_PATTERN.findall(script)
    behavior_calls = BEHAVIOR_METHOD_PATTERN.findall(script)

    command_calls = [m for m in ctx_calls if m in COMMAND_METHODS]
    # All behaviors issue commands
    all_commands = command_calls + behavior_calls

    return len(all_commands) > 0, list(set(all_commands))


def check_headers(script: str) -> tuple[bool, bool]:
    """Check for Intent and Description header comments."""
    has_intent = bool(INTENT_PATTERN.search(script))
    has_description = bool(DESCRIPTION_PATTERN.search(script))
    return has_intent, has_description


# ---------------------------------------------------------------------------
# Runtime check (mock execution)
# ---------------------------------------------------------------------------

def check_runtime(script: str) -> tuple[bool, str]:
    """Try to execute the script in a mock Lua environment.

    Creates a stub ctx object that records calls without real game state.
    Requires luajit or lua binary.
    """
    mock_preamble = """
-- Mock ctx object for validation
local mock_calls = {}
local function record(name, ...)
    table.insert(mock_calls, name)
end

local mock_unit = {id=1, kind="Pawdler", x=10, y=10, hp=100, hp_max=100,
    speed=2, damage=5, range=1, attack_speed=1, attack_type="Melee",
    moving=false, attacking=false, idle=true, gathering=false, owner=0}

local mock_building = {id=50, kind="TheBox", x=5, y=5, hp=500, hp_max=500,
    under_construction=false, construction_progress=1.0, producing=false, owner=0}

local mock_deposit = {id=100, x=15, y=15, remaining=500, resource_type="Food", kind="Food"}

local mock_resources = {food=500, gpu_cores=100, nfts=0, supply=5, supply_cap=20}

local behaviors = {}
local bmt = {__index = function(t, k)
    return function(self, ...) record("behaviors:"..k, ...) return 1 end
end}
setmetatable(behaviors, bmt)

ctx = {}
local cmt = {__index = function(t, k)
    if k == "behaviors" then return behaviors end
    return function(self, ...)
        record(k, ...)
        if k == "my_units" or k == "enemy_units" or k == "idle_units"
            or k == "wounded_units" or k == "units_by_state"
            or k == "enemies_in_range" then
            return {mock_unit}
        elseif k == "my_buildings" or k == "enemy_buildings" then
            return {mock_building}
        elseif k == "resource_deposits" then
            return {mock_deposit}
        elseif k == "get_resources" or k == "resources" then
            return mock_resources
        elseif k == "nearest_enemy" or k == "weakest_enemy_in_range"
            or k == "strongest_enemy_in_range" then
            return mock_unit
        elseif k == "nearest_deposit" then
            return mock_deposit
        elseif k == "count_units" or k == "army_supply" then
            return 5
        elseif k == "tick" then
            return 100
        elseif k == "map_size" then
            return 64, 64
        elseif k == "hp_pct" then
            return 0.8
        elseif k == "terrain_at" then
            return "Grass"
        elseif k == "elevation_at" then
            return 1
        elseif k == "cover_at" then
            return "None"
        elseif k == "is_passable" then
            return true
        elseif k == "can_reach" then
            return true
        elseif k == "path_length" then
            return 10
        elseif k == "position_at_range" then
            return 8, 8
        elseif k == "safe_positions" then
            return {{x=3, y=3}, {x=4, y=4}}
        elseif k == "threats_to" or k == "targets_for" then
            return {mock_unit}
        elseif k == "distance_squared_between" or k == "distance_squared_to_nearest_enemy" then
            return 25.0
        end
    end
end}
setmetatable(ctx, cmt)

-- Run the actual script
"""
    full_script = mock_preamble + "\n" + script

    for cmd in ["luajit", "lua"]:
        try:
            with tempfile.NamedTemporaryFile(mode="w", suffix=".lua", delete=False) as f:
                f.write(full_script)
                f.flush()
                result = subprocess.run(
                    [cmd, f.name],
                    capture_output=True, text=True, timeout=5,
                )
                Path(f.name).unlink(missing_ok=True)
                if result.returncode == 0:
                    return True, ""
                else:
                    return False, result.stderr.strip()
        except (FileNotFoundError, subprocess.TimeoutExpired) as e:
            Path(f.name).unlink(missing_ok=True)
            if isinstance(e, subprocess.TimeoutExpired):
                return False, "Execution timed out (possible infinite loop)"
            continue

    # No Lua binary available — skip runtime check
    return True, "(skipped — no lua binary)"


# ---------------------------------------------------------------------------
# Evaluation pipeline
# ---------------------------------------------------------------------------

def evaluate_script(script: str, index: int = 0, user_prompt: str = "") -> ScriptScore:
    """Evaluate a single Lua script on all rubric dimensions."""
    score = ScriptScore(example_index=index, user_prompt=user_prompt)

    # 1. Syntax
    score.syntax_valid, score.syntax_error = check_lua_syntax(script)

    # 2. Headers
    score.has_intent_header, score.has_description_header = check_headers(script)

    # 3. API correctness
    valid_ctx, invalid_ctx, valid_bhv, invalid_bhv = check_api_usage(script)
    score.ctx_methods_used = list(set(valid_ctx))
    score.behavior_methods_used = list(set(valid_bhv))
    score.invalid_ctx_methods = list(set(invalid_ctx))
    score.invalid_behavior_methods = list(set(invalid_bhv))

    # 4. Command coverage
    score.issues_commands, score.command_methods_used = check_commands(script)

    # 5. Runtime
    if score.syntax_valid:
        score.runtime_ok, score.runtime_error = check_runtime(script)
    else:
        score.runtime_ok = False
        score.runtime_error = "Skipped (syntax invalid)"

    return score


def evaluate_jsonl(path: Path) -> list[ScriptScore]:
    """Evaluate all examples in a JSONL file."""
    scores = []
    with open(path) as f:
        for i, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
            except json.JSONDecodeError as e:
                score = ScriptScore(example_index=i)
                score.syntax_error = f"Invalid JSON: {e}"
                scores.append(score)
                continue

            messages = data.get("messages", [])
            if len(messages) < 3:
                score = ScriptScore(example_index=i)
                score.syntax_error = "Too few messages"
                scores.append(score)
                continue

            user_msg = messages[1].get("content", "")
            assistant_msg = messages[2].get("content", "")

            score = evaluate_script(assistant_msg, index=i, user_prompt=user_msg)
            scores.append(score)

    return scores


def print_report(scores: list[ScriptScore]):
    """Print evaluation report."""
    total = len(scores)
    if total == 0:
        print("No examples to evaluate.")
        return

    pass_count = sum(1 for s in scores if s.total_score >= 0.8)
    warn_count = sum(1 for s in scores if 0.6 <= s.total_score < 0.8)
    fail_count = sum(1 for s in scores if s.total_score < 0.6)
    avg_score = sum(s.total_score for s in scores) / total

    print(f"\n{'='*60}")
    print(f"Lua Script Evaluation Report")
    print(f"{'='*60}")
    print(f"Total examples: {total}")
    print(f"Average score:  {avg_score:.0%}")
    print(f"PASS (>=80%):   {pass_count}")
    print(f"WARN (60-79%):  {warn_count}")
    print(f"FAIL (<60%):    {fail_count}")

    # Dimension breakdown
    print(f"\n--- Dimension Breakdown ---")
    print(f"Syntax valid:     {sum(1 for s in scores if s.syntax_valid):3d}/{total}")
    print(f"Has headers:      {sum(1 for s in scores if s.has_intent_header and s.has_description_header):3d}/{total}")
    print(f"API correct:      {sum(1 for s in scores if s.api_correct):3d}/{total}")
    print(f"Issues commands:  {sum(1 for s in scores if s.issues_commands):3d}/{total}")
    print(f"Runtime OK:       {sum(1 for s in scores if s.runtime_ok):3d}/{total}")

    # API usage stats
    from collections import Counter
    all_ctx = Counter()
    all_bhv = Counter()
    for s in scores:
        all_ctx.update(s.ctx_methods_used)
        all_bhv.update(s.behavior_methods_used)

    print(f"\n--- ctx API Usage ({sum(all_ctx.values())} total calls) ---")
    for name, count in all_ctx.most_common(15):
        print(f"  {name:35s} {count:3d}")

    if all_bhv:
        print(f"\n--- Behavior Usage ({sum(all_bhv.values())} total calls) ---")
        for name, count in all_bhv.most_common(15):
            print(f"  {name:35s} {count:3d}")

    # Show failures
    failures = [s for s in scores if s.total_score < 0.8]
    if failures:
        print(f"\n--- Issues ({len(failures)} examples) ---")
        for s in failures:
            print(f"  #{s.example_index}: {s.summary()}")
            if s.user_prompt:
                print(f"    Prompt: {s.user_prompt[:60]}")


def main():
    parser = argparse.ArgumentParser(description="Evaluate Lua scripts for ClawedCommand")
    parser.add_argument("input", nargs="?", type=Path, help="JSONL file with training examples")
    parser.add_argument("--script", type=Path, help="Evaluate a single .lua file")
    parser.add_argument("--output", type=Path, help="Write detailed results as JSON")
    args = parser.parse_args()

    if args.script:
        script = args.script.read_text()
        score = evaluate_script(script, user_prompt=args.script.name)
        print(f"{score.summary()}")
        if args.output:
            with open(args.output, "w") as f:
                json.dump(asdict(score), f, indent=2)
        sys.exit(0 if score.total_score >= 0.8 else 1)

    if not args.input:
        parser.error("Either --script or a JSONL input file is required")

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    scores = evaluate_jsonl(args.input)
    print_report(scores)

    if args.output:
        with open(args.output, "w") as f:
            json.dump([asdict(s) for s in scores], f, indent=2)
        print(f"\nDetailed results written to {args.output}")

    # Exit with error if any scripts fail
    fail_count = sum(1 for s in scores if s.total_score < 0.6)
    if fail_count > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
