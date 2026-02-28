#!/usr/bin/env python3
"""Validate ClawedCommand fine-tuning JSONL data.

Checks:
- JSON syntax per line
- Required top-level keys (messages, tools)
- Message ordering (system? → user → assistant → tool → ...)
- Tool call ID format (9 alphanumeric chars)
- Arguments are stringified JSON (not parsed objects)
- Tool names match the known game tool set
- Argument keys match tool parameter schemas
- Balanced tool distribution (no single tool >30% of calls)
- No empty assistant messages

Outputs:
- Validation report to stdout
- Filtered (valid-only) dataset to --output if specified
"""

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

VALID_TOOLS = {
    "get_units", "move_units", "attack_units", "build", "train_unit",
    "get_visible_enemies", "get_resources", "get_buildings", "get_map_info",
    "set_rally_point", "patrol", "gather_resource", "execute_strategy",
}

TOOL_REQUIRED_PARAMS = {
    "move_units": {"unit_ids", "target"},
    "attack_units": {"unit_ids"},
    "build": {"building_type", "position"},
    "train_unit": {"building_id", "unit_type"},
    "get_map_info": {"region"},
    "set_rally_point": {"building_id", "position"},
    "patrol": {"unit_ids", "waypoints"},
    "gather_resource": {"worker_ids", "resource_id"},
    "execute_strategy": {"code"},
}

TOOL_CALL_ID_PATTERN = re.compile(r"^[a-zA-Z0-9]{9}$")


def validate_tool_call_id(tc_id: str) -> list[str]:
    errors = []
    if not isinstance(tc_id, str):
        errors.append(f"Tool call ID must be string, got {type(tc_id).__name__}")
    elif not TOOL_CALL_ID_PATTERN.match(tc_id):
        errors.append(
            f"Tool call ID '{tc_id}' must be exactly 9 alphanumeric chars"
        )
    return errors


def validate_tool_call(tc: dict) -> list[str]:
    errors = []

    if "id" not in tc:
        errors.append("Tool call missing 'id'")
    else:
        errors.extend(validate_tool_call_id(tc["id"]))

    if "type" not in tc:
        errors.append("Tool call missing 'type'")
    elif tc["type"] != "function":
        errors.append(f"Tool call type must be 'function', got '{tc['type']}'")

    fn = tc.get("function", {})
    if "name" not in fn:
        errors.append("Tool call function missing 'name'")
    else:
        name = fn["name"]
        if name not in VALID_TOOLS:
            errors.append(f"Unknown tool name: '{name}'")

    if "arguments" not in fn:
        errors.append("Tool call function missing 'arguments'")
    elif not isinstance(fn["arguments"], str):
        errors.append(
            f"Arguments must be stringified JSON (string), got {type(fn['arguments']).__name__}"
        )
    else:
        try:
            parsed = json.loads(fn["arguments"])
            if not isinstance(parsed, dict):
                errors.append("Parsed arguments must be a JSON object")
            else:
                name = fn.get("name", "")
                required = TOOL_REQUIRED_PARAMS.get(name, set())
                missing = required - set(parsed.keys())
                if missing:
                    errors.append(
                        f"Tool '{name}' missing required params: {missing}"
                    )
        except json.JSONDecodeError as e:
            errors.append(f"Arguments are not valid JSON: {e}")

    return errors


def validate_message_ordering(messages: list[dict]) -> list[str]:
    errors = []
    if not messages:
        return ["Empty messages array"]

    # First message can be system or user
    roles = [m.get("role") for m in messages]

    if roles[0] not in ("system", "user"):
        errors.append(f"First message must be 'system' or 'user', got '{roles[0]}'")

    # Check: after system comes user, after user comes assistant, etc.
    prev_role = None
    for i, msg in enumerate(messages):
        role = msg.get("role")
        if role not in ("system", "user", "assistant", "tool"):
            errors.append(f"Message {i}: invalid role '{role}'")
            continue

        if role == "tool":
            if prev_role not in ("assistant", "tool"):
                errors.append(
                    f"Message {i}: 'tool' must follow 'assistant' or 'tool', got '{prev_role}'"
                )
            if "tool_call_id" not in msg:
                errors.append(f"Message {i}: tool message missing 'tool_call_id'")

        if role == "assistant":
            has_tool_calls = "tool_calls" in msg and msg["tool_calls"]
            has_content = "content" in msg and msg["content"]
            if not has_tool_calls and not has_content:
                errors.append(
                    f"Message {i}: assistant message has neither tool_calls nor content"
                )

        prev_role = role

    # Last message should be assistant with content (final response)
    last = messages[-1]
    if last.get("role") != "assistant":
        errors.append(f"Last message should be 'assistant', got '{last.get('role')}'")
    elif "tool_calls" in last and last["tool_calls"] and not last.get("content"):
        errors.append("Last message is a tool call with no final text response")

    return errors


def validate_tools_array(tools: list) -> list[str]:
    errors = []
    if not tools:
        errors.append("Tools array is empty")
        return errors

    for i, tool in enumerate(tools):
        if tool.get("type") != "function":
            errors.append(f"Tool {i}: type must be 'function'")
        fn = tool.get("function", {})
        if "name" not in fn:
            errors.append(f"Tool {i}: missing function name")
        if "parameters" not in fn:
            errors.append(f"Tool {i}: missing parameters schema")

    return errors


def validate_example(line_num: int, raw_line: str) -> tuple[dict | None, list[str]]:
    errors = []

    try:
        data = json.loads(raw_line)
    except json.JSONDecodeError as e:
        return None, [f"Line {line_num}: Invalid JSON: {e}"]

    if not isinstance(data, dict):
        return None, [f"Line {line_num}: Top level must be object"]

    if "messages" not in data:
        errors.append(f"Line {line_num}: Missing 'messages' key")
    else:
        msg_errors = validate_message_ordering(data["messages"])
        errors.extend(f"Line {line_num}: {e}" for e in msg_errors)

        # Validate all tool calls in assistant messages
        for msg in data["messages"]:
            if msg.get("role") == "assistant" and "tool_calls" in msg:
                for tc in msg["tool_calls"]:
                    tc_errors = validate_tool_call(tc)
                    errors.extend(f"Line {line_num}: {e}" for e in tc_errors)

    if "tools" not in data:
        errors.append(f"Line {line_num}: Missing 'tools' key")
    else:
        tool_errors = validate_tools_array(data["tools"])
        errors.extend(f"Line {line_num}: {e}" for e in tool_errors)

    return data, errors


def check_tool_distribution(examples: list[dict]) -> list[str]:
    """Check that no single tool dominates >30% of all tool calls."""
    tool_counts: Counter = Counter()
    total_calls = 0

    for ex in examples:
        for msg in ex.get("messages", []):
            if msg.get("role") == "assistant" and "tool_calls" in msg:
                for tc in msg["tool_calls"]:
                    name = tc.get("function", {}).get("name", "unknown")
                    tool_counts[name] += 1
                    total_calls += 1

    warnings = []
    if total_calls > 0:
        for tool_name, count in tool_counts.most_common():
            pct = count / total_calls * 100
            if pct > 30:
                warnings.append(
                    f"Tool '{tool_name}' used in {pct:.1f}% of calls "
                    f"({count}/{total_calls}) — exceeds 30% threshold"
                )

    return warnings


def main():
    parser = argparse.ArgumentParser(description="Validate ClawedCommand training data")
    parser.add_argument("input", type=Path, help="Input JSONL file")
    parser.add_argument("--output", type=Path, help="Write valid examples to this file")
    parser.add_argument("--strict", action="store_true", help="Treat warnings as errors")
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    lines = args.input.read_text().strip().split("\n")
    total = len(lines)
    valid_examples = []
    all_errors = []

    print(f"Validating {total} examples from {args.input}...\n")

    for i, line in enumerate(lines, 1):
        line = line.strip()
        if not line:
            continue
        data, errors = validate_example(i, line)
        if errors:
            all_errors.extend(errors)
        if data and not errors:
            valid_examples.append(data)

    # Distribution check
    dist_warnings = check_tool_distribution(valid_examples)

    # Report
    print(f"Results: {len(valid_examples)}/{total} examples valid")
    print(f"Errors: {len(all_errors)}")

    if all_errors:
        print("\n--- Errors ---")
        for e in all_errors:
            print(f"  {e}")

    if dist_warnings:
        print("\n--- Distribution Warnings ---")
        for w in dist_warnings:
            print(f"  {w}")

    # Tool usage summary
    tool_counts: Counter = Counter()
    total_calls = 0
    for ex in valid_examples:
        for msg in ex.get("messages", []):
            if msg.get("role") == "assistant" and "tool_calls" in msg:
                for tc in msg["tool_calls"]:
                    name = tc.get("function", {}).get("name", "unknown")
                    tool_counts[name] += 1
                    total_calls += 1

    print(f"\n--- Tool Usage ({total_calls} total calls) ---")
    for name, count in tool_counts.most_common():
        pct = count / total_calls * 100 if total_calls else 0
        bar = "#" * int(pct / 2)
        print(f"  {name:25s} {count:4d} ({pct:5.1f}%) {bar}")

    # Negative example count (no tool calls)
    neg_count = sum(
        1 for ex in valid_examples
        if not any(
            "tool_calls" in m and m["tool_calls"]
            for m in ex.get("messages", [])
            if m.get("role") == "assistant"
        )
    )
    print(f"\nNegative examples (no tool calls): {neg_count}/{len(valid_examples)}")

    # Write filtered output
    if args.output:
        with open(args.output, "w") as f:
            for ex in valid_examples:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"\nWrote {len(valid_examples)} valid examples to {args.output}")

    if all_errors or (args.strict and dist_warnings):
        sys.exit(1)


if __name__ == "__main__":
    main()
