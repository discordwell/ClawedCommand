#!/usr/bin/env python3
"""Convert ClawedCommand training data between model chat template formats.

Supported formats:
- mistral: Mistral/Devstral native tool-calling format (OpenAI-compatible)
- qwen: Qwen2.5 chat template format
- xlam: Salesforce xLAM function-calling format

Also handles train/eval split.
"""

import argparse
import json
import random
import sys
from pathlib import Path


def to_mistral_format(example: dict) -> dict:
    """Mistral format is the base format — pass through."""
    return example


def to_qwen_format(example: dict) -> dict:
    """Convert to Qwen2.5-Coder chat template format.

    Qwen uses the same OpenAI-compatible tool format but wraps tool results
    differently and expects <|im_start|> / <|im_end|> delimiters
    (handled by the tokenizer, not in the JSONL).
    """
    # Qwen accepts OpenAI-format tool calling natively since Qwen2.5
    # The main difference is in the chat template applied by the tokenizer,
    # not in the JSONL structure. We pass through but ensure compatibility.
    converted = {"messages": [], "tools": example.get("tools", [])}

    for msg in example["messages"]:
        new_msg = dict(msg)
        # Qwen expects tool results with role "tool" (same as OpenAI)
        # No structural changes needed for Qwen2.5-Coder
        converted["messages"].append(new_msg)

    return converted


def to_xlam_format(example: dict) -> dict:
    """Convert to xLAM function-calling format.

    xLAM uses a different tool representation:
    - Tools are described in the system prompt as a JSON array
    - Tool calls use a specific format in assistant content
    - No separate 'tools' key at top level
    """
    tools = example.get("tools", [])
    messages = example.get("messages", [])

    # Build tool descriptions for system prompt
    tool_descriptions = []
    for tool in tools:
        fn = tool.get("function", {})
        tool_descriptions.append({
            "name": fn.get("name", ""),
            "description": fn.get("description", ""),
            "parameters": fn.get("parameters", {}),
        })

    converted_messages = []
    for msg in messages:
        role = msg.get("role")

        if role == "system":
            # Prepend tool descriptions to system prompt
            system_content = msg.get("content", "")
            tool_prompt = (
                "You have access to the following tools:\n"
                f"{json.dumps(tool_descriptions, indent=2)}\n\n"
                "When you need to call a tool, respond with a JSON object in this format:\n"
                '[{"name": "tool_name", "arguments": {"arg": "value"}}]\n\n'
                f"{system_content}"
            )
            converted_messages.append({"role": "system", "content": tool_prompt})

        elif role == "assistant" and "tool_calls" in msg and msg["tool_calls"]:
            # Convert tool_calls to xLAM inline format
            calls = []
            for tc in msg["tool_calls"]:
                fn = tc.get("function", {})
                args_str = fn.get("arguments", "{}")
                try:
                    args = json.loads(args_str)
                except json.JSONDecodeError:
                    args = {}
                calls.append({"name": fn.get("name", ""), "arguments": args})

            converted_messages.append({
                "role": "assistant",
                "content": json.dumps(calls),
            })

        elif role == "tool":
            # xLAM treats tool results as user messages with observation tag
            content = msg.get("content", "")
            converted_messages.append({
                "role": "user",
                "content": f"[TOOL_RESULT] {content}",
            })

        else:
            converted_messages.append(dict(msg))

    # xLAM format has no top-level tools key
    return {"messages": converted_messages}


FORMAT_CONVERTERS = {
    "mistral": to_mistral_format,
    "qwen": to_qwen_format,
    "xlam": to_xlam_format,
}


def split_data(
    examples: list[dict], eval_ratio: float = 0.1, seed: int = 42
) -> tuple[list[dict], list[dict]]:
    """Split into train/eval sets."""
    rng = random.Random(seed)
    shuffled = list(examples)
    rng.shuffle(shuffled)
    split_idx = max(1, int(len(shuffled) * eval_ratio))
    return shuffled[split_idx:], shuffled[:split_idx]


def main():
    parser = argparse.ArgumentParser(
        description="Convert training data between model formats"
    )
    parser.add_argument("input", type=Path, help="Input JSONL file (Mistral format)")
    parser.add_argument(
        "--format",
        choices=FORMAT_CONVERTERS.keys(),
        required=True,
        help="Target format",
    )
    parser.add_argument("--output-dir", type=Path, default=Path("../data"))
    parser.add_argument(
        "--split", action="store_true", help="Split into train/eval (90/10)"
    )
    parser.add_argument("--eval-ratio", type=float, default=0.1)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--prefix", default="cc", help="Output filename prefix")
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: {args.input} not found", file=sys.stderr)
        sys.exit(1)

    args.output_dir.mkdir(parents=True, exist_ok=True)

    # Load
    examples = []
    with open(args.input) as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))

    print(f"Loaded {len(examples)} examples from {args.input}")

    # Convert
    converter = FORMAT_CONVERTERS[args.format]
    converted = [converter(ex) for ex in examples]

    if args.split:
        train, eval_set = split_data(converted, args.eval_ratio, args.seed)
        train_path = args.output_dir / f"{args.prefix}_train_{args.format}.jsonl"
        eval_path = args.output_dir / f"{args.prefix}_eval_{args.format}.jsonl"

        with open(train_path, "w") as f:
            for ex in train:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")

        with open(eval_path, "w") as f:
            for ex in eval_set:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")

        print(f"Train: {len(train)} → {train_path}")
        print(f"Eval:  {len(eval_set)} → {eval_path}")
    else:
        out_path = args.output_dir / f"{args.prefix}_{args.format}.jsonl"
        with open(out_path, "w") as f:
            for ex in converted:
                f.write(json.dumps(ex, ensure_ascii=False) + "\n")
        print(f"Wrote {len(converted)} examples → {out_path}")


if __name__ == "__main__":
    main()
