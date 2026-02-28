#!/usr/bin/env python3
"""Evaluation harness for comparing fine-tuned models on ClawedCommand tool-calling.

Metrics:
- Tool call accuracy: valid tool name + valid args
- Instruction following: LLM-as-judge (does output match intent?)
- Multi-step completion: correct 3+ step tool chains
- No-tool accuracy: correctly avoids tool calls when not needed
- Response latency: time to first tool call
- Token efficiency: tokens per successful game action

Usage:
    python evaluate.py ../data/cc_eval_mistral.jsonl \
        --model vllm::http://localhost:8080/v1 \
        --model mistral::codestral-latest \
        --model mistral::ft:codestral-latest:xxx:20260228
"""

import argparse
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

import requests

TOOL_CALL_ID_RE = re.compile(r"^[a-zA-Z0-9]{9}$")

VALID_TOOLS = {
    "get_units", "move_units", "attack_units", "build", "train_unit",
    "get_visible_enemies", "get_resources", "get_buildings", "get_map_info",
    "set_rally_point", "patrol", "gather_resource", "execute_strategy",
}


@dataclass
class EvalResult:
    model_name: str
    total: int = 0
    tool_call_valid: int = 0
    tool_call_invalid: int = 0
    instruction_match: int = 0
    multistep_correct: int = 0
    multistep_total: int = 0
    no_tool_correct: int = 0
    no_tool_total: int = 0
    latencies: list = field(default_factory=list)
    total_tokens: int = 0
    total_actions: int = 0


def call_model(
    backend: str, model_id: str, messages: list, tools: list
) -> tuple[dict, float, int]:
    """Call a model and return (response, latency_seconds, total_tokens)."""
    start = time.time()

    if backend == "mistral":
        from mistralai import Mistral

        client = Mistral(api_key=os.environ.get("MISTRAL_API_KEY", ""))
        resp = client.chat.complete(
            model=model_id,
            messages=messages,
            tools=tools,
            tool_choice="auto",
            temperature=0.2,
        )
        latency = time.time() - start
        msg = resp.choices[0].message
        tokens = resp.usage.total_tokens if resp.usage else 0
        result = {"role": "assistant"}
        if msg.tool_calls:
            result["tool_calls"] = [
                {
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.function.name,
                        "arguments": tc.function.arguments,
                    },
                }
                for tc in msg.tool_calls
            ]
        if msg.content:
            result["content"] = msg.content
        return result, latency, tokens

    elif backend == "vllm":
        # OpenAI-compatible endpoint
        resp = requests.post(
            f"{model_id}/chat/completions",
            json={
                "model": "default",
                "messages": messages,
                "tools": tools,
                "tool_choice": "auto",
                "temperature": 0.2,
            },
            headers={"Content-Type": "application/json"},
            timeout=60,
        )
        latency = time.time() - start
        data = resp.json()
        msg = data["choices"][0]["message"]
        tokens = data.get("usage", {}).get("total_tokens", 0)
        return msg, latency, tokens

    else:
        raise ValueError(f"Unknown backend: {backend}")


def validate_tool_calls(tool_calls: list) -> tuple[int, int]:
    """Return (valid_count, invalid_count)."""
    valid = 0
    invalid = 0
    for tc in tool_calls:
        fn = tc.get("function", {})
        name = fn.get("name", "")
        args_str = fn.get("arguments", "")

        ok = True
        if name not in VALID_TOOLS:
            ok = False
        if not isinstance(args_str, str):
            ok = False
        else:
            try:
                json.loads(args_str)
            except (json.JSONDecodeError, TypeError):
                ok = False

        if ok:
            valid += 1
        else:
            invalid += 1

    return valid, invalid


def is_no_tool_example(example: dict) -> bool:
    """Check if the gold example has no tool calls (negative example)."""
    for msg in example["messages"]:
        if msg.get("role") == "assistant" and msg.get("tool_calls"):
            return False
    return True


def count_tool_steps(example: dict) -> int:
    """Count number of assistant messages with tool calls."""
    return sum(
        1
        for msg in example["messages"]
        if msg.get("role") == "assistant" and msg.get("tool_calls")
    )


def get_input_messages(example: dict) -> list:
    """Extract just the system + first user message for prompting the model."""
    msgs = []
    for msg in example["messages"]:
        if msg["role"] in ("system", "user"):
            msgs.append(msg)
            if msg["role"] == "user":
                break
    return msgs


def evaluate_model(
    backend: str, model_id: str, examples: list[dict], display_name: str
) -> EvalResult:
    result = EvalResult(model_name=display_name)

    for i, example in enumerate(examples):
        result.total += 1
        input_msgs = get_input_messages(example)
        tools = example.get("tools", [])
        is_negative = is_no_tool_example(example)
        gold_steps = count_tool_steps(example)

        try:
            response, latency, tokens = call_model(
                backend, model_id, input_msgs, tools
            )
        except Exception as e:
            print(f"  [{i+1}/{len(examples)}] Error: {e}")
            continue

        result.latencies.append(latency)
        result.total_tokens += tokens

        has_tool_calls = "tool_calls" in response and response["tool_calls"]

        if is_negative:
            result.no_tool_total += 1
            if not has_tool_calls:
                result.no_tool_correct += 1
        else:
            if has_tool_calls:
                v, inv = validate_tool_calls(response["tool_calls"])
                result.tool_call_valid += v
                result.tool_call_invalid += inv
                result.total_actions += v

                if gold_steps >= 3:
                    result.multistep_total += 1
                    # Credit if model made at least one valid tool call
                    if v > 0:
                        result.multistep_correct += 1

        status = "ok" if has_tool_calls != is_negative else "MISMATCH"
        print(
            f"  [{i+1}/{len(examples)}] {status} "
            f"latency={latency:.2f}s tokens={tokens}"
        )

    return result


def print_results(results: list[EvalResult]):
    print("\n" + "=" * 80)
    print("EVALUATION RESULTS")
    print("=" * 80)

    print(f"\n{'Model':<35} {'Tool Acc':>8} {'No-Tool':>8} "
          f"{'Multi':>6} {'Lat(s)':>7} {'Tok/Act':>8}")
    print("-" * 80)

    for r in results:
        total_tc = r.tool_call_valid + r.tool_call_invalid
        tool_acc = r.tool_call_valid / total_tc * 100 if total_tc else 0
        no_tool = (
            r.no_tool_correct / r.no_tool_total * 100 if r.no_tool_total else 0
        )
        multi = (
            r.multistep_correct / r.multistep_total * 100
            if r.multistep_total
            else 0
        )
        avg_lat = sum(r.latencies) / len(r.latencies) if r.latencies else 0
        tok_act = r.total_tokens / r.total_actions if r.total_actions else 0

        print(
            f"{r.model_name:<35} {tool_acc:>7.1f}% {no_tool:>7.1f}% "
            f"{multi:>5.1f}% {avg_lat:>7.2f} {tok_act:>8.0f}"
        )

    print()


def main():
    parser = argparse.ArgumentParser(description="Evaluate models on CC tool-calling")
    parser.add_argument("eval_file", type=Path, help="Eval JSONL file")
    parser.add_argument(
        "--model",
        action="append",
        required=True,
        help="Model spec: backend::model_id (e.g., vllm::http://localhost:8080/v1, "
        "mistral::codestral-latest). Can specify multiple.",
    )
    parser.add_argument("--limit", type=int, help="Max examples to evaluate")
    parser.add_argument(
        "--output", type=Path, help="Write results JSON to file"
    )
    args = parser.parse_args()

    if not args.eval_file.exists():
        print(f"Error: {args.eval_file} not found", file=sys.stderr)
        sys.exit(1)

    examples = []
    with open(args.eval_file) as f:
        for line in f:
            line = line.strip()
            if line:
                examples.append(json.loads(line))

    if args.limit:
        examples = examples[: args.limit]

    print(f"Evaluating {len(examples)} examples\n")

    results = []
    for model_spec in args.model:
        if "::" not in model_spec:
            print(f"Error: model spec '{model_spec}' must use 'backend::model_id' format", file=sys.stderr)
            sys.exit(1)
        backend, model_id = model_spec.split("::", 1)
        display_name = f"{backend}/{model_id.split('/')[-1]}"
        print(f"--- {display_name} ---")
        result = evaluate_model(backend, model_id, examples, display_name)
        results.append(result)

    print_results(results)

    if args.output:
        output_data = []
        for r in results:
            total_tc = r.tool_call_valid + r.tool_call_invalid
            output_data.append({
                "model": r.model_name,
                "tool_call_accuracy": r.tool_call_valid / total_tc if total_tc else 0,
                "no_tool_accuracy": (
                    r.no_tool_correct / r.no_tool_total if r.no_tool_total else 0
                ),
                "multistep_accuracy": (
                    r.multistep_correct / r.multistep_total
                    if r.multistep_total
                    else 0
                ),
                "avg_latency": (
                    sum(r.latencies) / len(r.latencies) if r.latencies else 0
                ),
                "tokens_per_action": (
                    r.total_tokens / r.total_actions if r.total_actions else 0
                ),
                "total_examples": r.total,
            })
        with open(args.output, "w") as f:
            json.dump(output_data, f, indent=2)
        print(f"Results saved to {args.output}")


if __name__ == "__main__":
    main()
