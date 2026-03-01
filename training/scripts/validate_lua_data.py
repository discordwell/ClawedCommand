#!/usr/bin/env python3
"""Validate ClawedCommand Lua generation JSONL data.

Unlike validate_data.py (which validates MCP tool-call format), this validates
the Lua script generation format used for agentic builder fine-tuning.

Checks:
- JSON syntax per line
- Message ordering: system → user → assistant (exactly 3 messages)
- System message contains ctx API reference
- User message is non-empty natural language prompt
- Assistant message contains valid Lua code
- Lua code references ctx: methods that exist in the API
- Script has Intent/Description header comments
- No invalid ctx API references

Outputs:
- Validation report to stdout
- Filtered (valid-only) dataset to --output if specified

Usage:
  python training/scripts/validate_lua_data.py training/data/gold_lua_examples.jsonl
  python training/scripts/validate_lua_data.py training/data/gold_lua_examples.jsonl --output validated.jsonl --strict
"""

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

from lua_api_surface import (
    VALID_CTX_METHODS, VALID_BEHAVIOR_METHODS,
    CTX_METHOD_PATTERN, BEHAVIOR_METHOD_PATTERN,
    INTENT_PATTERN, DESCRIPTION_PATTERN,
    check_lua_block_balance,
)


def validate_example(line_num: int, raw_line: str) -> tuple[dict | None, list[str], list[str]]:
    """Validate a single JSONL line. Returns (data, errors, warnings)."""
    errors = []
    warnings = []

    try:
        data = json.loads(raw_line)
    except json.JSONDecodeError as e:
        return None, [f"Line {line_num}: Invalid JSON: {e}"], []

    if not isinstance(data, dict):
        return None, [f"Line {line_num}: Top level must be object"], []

    # Check messages array
    if "messages" not in data:
        errors.append(f"Line {line_num}: Missing 'messages' key")
        return data, errors, warnings

    messages = data["messages"]
    if not isinstance(messages, list):
        errors.append(f"Line {line_num}: 'messages' must be an array")
        return data, errors, warnings

    if len(messages) != 3:
        errors.append(
            f"Line {line_num}: Expected exactly 3 messages (system, user, assistant), "
            f"got {len(messages)}"
        )
        return data, errors, warnings

    # Check message roles
    expected_roles = ["system", "user", "assistant"]
    for i, (msg, expected) in enumerate(zip(messages, expected_roles)):
        role = msg.get("role")
        if role != expected:
            errors.append(
                f"Line {line_num}: Message {i} should be '{expected}', got '{role}'"
            )

    # Check system message has ctx API content
    system_content = messages[0].get("content", "")
    if "ctx" not in system_content.lower():
        warnings.append(
            f"Line {line_num}: System message doesn't reference 'ctx' API"
        )

    # Check user message is non-empty
    user_content = messages[1].get("content", "")
    if not user_content or len(user_content.strip()) < 5:
        errors.append(f"Line {line_num}: User message is empty or too short")

    # Check assistant message is valid Lua
    lua_code = messages[2].get("content", "")
    if not lua_code or len(lua_code.strip()) < 10:
        errors.append(f"Line {line_num}: Assistant message (Lua code) is empty or too short")
        return data, errors, warnings

    # Check header comments
    if not INTENT_PATTERN.search(lua_code):
        warnings.append(f"Line {line_num}: Missing '-- Intent:' header comment")
    if not DESCRIPTION_PATTERN.search(lua_code):
        warnings.append(f"Line {line_num}: Missing '-- Description:' header comment")

    # Check ctx API references
    ctx_calls = CTX_METHOD_PATTERN.findall(lua_code)
    for method in ctx_calls:
        if method not in VALID_CTX_METHODS:
            errors.append(
                f"Line {line_num}: Unknown ctx method: ctx:{method}()"
            )

    behavior_calls = BEHAVIOR_METHOD_PATTERN.findall(lua_code)
    for method in behavior_calls:
        if method not in VALID_BEHAVIOR_METHODS:
            errors.append(
                f"Line {line_num}: Unknown behavior method: ctx.behaviors:{method}()"
            )

    # Check that the script references ctx at all
    if not ctx_calls and not behavior_calls:
        warnings.append(
            f"Line {line_num}: Script doesn't call any ctx methods"
        )

    # Check basic Lua syntax heuristics using shared block balance checker
    openers, closers = check_lua_block_balance(lua_code)
    if openers != closers:
        warnings.append(
            f"Line {line_num}: Possible unbalanced blocks "
            f"({openers} openers vs {closers} 'end')"
        )

    return data, errors, warnings


def main():
    parser = argparse.ArgumentParser(
        description="Validate ClawedCommand Lua generation training data"
    )
    parser.add_argument("input", type=Path, help="Input JSONL file")
    parser.add_argument("--output", type=Path, help="Write valid examples to this file")
    parser.add_argument(
        "--strict", action="store_true", help="Treat warnings as errors"
    )
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    lines = args.input.read_text().strip().split("\n")
    total = len(lines)
    valid_examples = []
    all_errors = []
    all_warnings = []

    print(f"Validating {total} Lua generation examples from {args.input}...\n")

    for i, line in enumerate(lines, 1):
        line = line.strip()
        if not line:
            continue
        data, errors, warnings = validate_example(i, line)
        all_errors.extend(errors)
        all_warnings.extend(warnings)
        if data and not errors:
            valid_examples.append(data)

    # API usage summary
    ctx_counts: Counter = Counter()
    behavior_counts: Counter = Counter()
    total_ctx_calls = 0
    total_behavior_calls = 0

    for ex in valid_examples:
        lua_code = ex["messages"][2]["content"]
        for method in CTX_METHOD_PATTERN.findall(lua_code):
            if method in VALID_CTX_METHODS:
                ctx_counts[method] += 1
                total_ctx_calls += 1
        for method in BEHAVIOR_METHOD_PATTERN.findall(lua_code):
            if method in VALID_BEHAVIOR_METHODS:
                behavior_counts[method] += 1
                total_behavior_calls += 1

    # Report
    print(f"Results: {len(valid_examples)}/{total} examples valid")
    print(f"Errors:   {len(all_errors)}")
    print(f"Warnings: {len(all_warnings)}")

    if all_errors:
        print("\n--- Errors ---")
        for e in all_errors:
            print(f"  {e}")

    if all_warnings:
        print("\n--- Warnings ---")
        for w in all_warnings:
            print(f"  {w}")

    print(f"\n--- ctx Method Usage ({total_ctx_calls} total calls) ---")
    for name, count in ctx_counts.most_common():
        pct = count / total_ctx_calls * 100 if total_ctx_calls else 0
        bar = "#" * int(pct / 2)
        print(f"  {name:35s} {count:4d} ({pct:5.1f}%) {bar}")

    if behavior_counts:
        print(f"\n--- Behavior Usage ({total_behavior_calls} total calls) ---")
        for name, count in behavior_counts.most_common():
            pct = count / total_behavior_calls * 100 if total_behavior_calls else 0
            bar = "#" * int(pct / 2)
            print(f"  {name:35s} {count:4d} ({pct:5.1f}%) {bar}")

    # Unused API methods
    used_ctx = set(ctx_counts.keys())
    used_bhv = set(behavior_counts.keys())
    unused_ctx = VALID_CTX_METHODS - used_ctx
    unused_bhv = VALID_BEHAVIOR_METHODS - used_bhv
    if unused_ctx:
        print(f"\n--- Unused ctx Methods ({len(unused_ctx)}) ---")
        for m in sorted(unused_ctx):
            print(f"  {m}")
    if unused_bhv:
        print(f"\n--- Unused Behavior Methods ({len(unused_bhv)}) ---")
        for m in sorted(unused_bhv):
            print(f"  {m}")

    # Write filtered output
    if args.output:
        with open(args.output, "w") as f:
            for ex in valid_examples:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"\nWrote {len(valid_examples)} valid examples to {args.output}")

    if all_errors or (args.strict and all_warnings):
        sys.exit(1)


if __name__ == "__main__":
    main()
