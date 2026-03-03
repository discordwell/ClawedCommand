#!/usr/bin/env python3
"""Assemble the final v2 training dataset from Phase 2-4 outputs.

Steps:
1. Load Phase 2 (expert SFT), Phase 4 (snapshot) examples
2. Deduplicate by Levenshtein distance on Lua scripts
3. Balance categories (none > 30%)
4. Split train/eval (90/10)
5. Output final JSONL files

Usage:
  python training/scripts/assemble_dataset.py

  # Custom inputs
  python training/scripts/assemble_dataset.py \
    --sft-inputs training/data/cc_v2_sft_raw.jsonl training/data/cc_v2_snapshot_examples.jsonl \
    --dpo-input training/data/cc_v2_dpo_raw.jsonl

Output:
  training/data/cc_v2_sft_train.jsonl
  training/data/cc_v2_sft_eval.jsonl
  training/data/cc_v2_dpo_train.jsonl
  training/data/cc_v2_dpo_eval.jsonl
"""

import argparse
import json
import random
import re
import sys
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent / "data"

DEFAULT_SFT_INPUTS = [
    DATA_DIR / "cc_v2_sft_raw.jsonl",
    DATA_DIR / "cc_v2_snapshot_examples.jsonl",
]
DEFAULT_DPO_INPUT = DATA_DIR / "cc_v2_dpo_raw.jsonl"


def load_jsonl(path: Path) -> list[dict]:
    """Load examples from a JSONL file."""
    if not path.exists():
        print(f"  Warning: {path} not found, skipping")
        return []
    examples = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    examples.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
    return examples


def extract_script_text(example: dict) -> str:
    """Extract the Lua script from an SFT example for deduplication."""
    msgs = example.get("messages", [])
    for msg in msgs:
        if msg.get("role") == "assistant":
            text = msg["content"]
            # Remove think block
            think_match = re.search(r'<think>.*?</think>', text, re.DOTALL)
            if think_match:
                text = text[think_match.end():]
            # Remove code fences
            text = re.sub(r'```\w*\n?', '', text)
            return text.strip()
    return ""


def line_jaccard_similarity(s1: str, s2: str) -> float:
    """Compute Jaccard similarity on line sets (0 = different, 1 = identical)."""
    if not s1 or not s2:
        return 0.0

    lines1 = set(s1.strip().splitlines())
    lines2 = set(s2.strip().splitlines())

    if not lines1 and not lines2:
        return 1.0

    intersection = lines1 & lines2
    union = lines1 | lines2

    return len(intersection) / max(len(union), 1)


def deduplicate_sft(examples: list[dict], threshold: float = 0.85) -> list[dict]:
    """Remove near-duplicate SFT examples based on script similarity."""
    if not examples:
        return []

    scripts = [extract_script_text(ex) for ex in examples]
    kept = []
    kept_scripts = []

    for i, (ex, script) in enumerate(zip(examples, scripts)):
        if not script:
            continue

        is_dup = False
        # Compare against last 50 kept scripts (performance optimization)
        for kept_script in kept_scripts[-50:]:
            if line_jaccard_similarity(script, kept_script) > threshold:
                is_dup = True
                break

        if not is_dup:
            kept.append(ex)
            kept_scripts.append(script)

    return kept


def classify_example(example: dict) -> str:
    """Classify an SFT example into a category based on content."""
    msgs = example.get("messages", [])
    if len(msgs) < 2:
        return "unknown"

    user_text = msgs[1].get("content", "").lower()
    assistant_text = msgs[2].get("content", "").lower() if len(msgs) > 2 else ""

    # Check for combat keywords
    combat_words = ["attack", "fight", "combat", "focus fire", "kite", "retreat",
                    "outnumbered", "push", "engage", "micro"]
    economy_words = ["gather", "worker", "food", "resource", "economy", "pawdler",
                     "deposit", "fish"]
    build_words = ["build", "construct", "cattree", "serverrack", "litterbox",
                   "supply", "barracks", "production"]
    composition_words = ["train", "produce", "army", "composition", "chonk",
                         "hisser", "units", "counter"]
    terrain_words = ["terrain", "cover", "elevation", "forest", "path",
                     "passable", "high ground"]
    strategic_words = ["decide", "should", "priority", "strategy", "plan",
                       "assess", "evaluate"]

    all_text = user_text + " " + assistant_text

    scores = {
        "combat_micro": sum(1 for w in combat_words if w in all_text),
        "economy": sum(1 for w in economy_words if w in all_text),
        "build_orders": sum(1 for w in build_words if w in all_text),
        "unit_composition": sum(1 for w in composition_words if w in all_text),
        "terrain_interaction": sum(1 for w in terrain_words if w in all_text),
        "strategic_decisions": sum(1 for w in strategic_words if w in all_text),
    }

    # Use metadata category if available
    metadata = example.get("metadata", {})
    if "category" in metadata:
        return metadata["category"]

    # Check source
    source = metadata.get("source", "")
    if source == "snapshot":
        phase = metadata.get("phase", "")
        if phase in ("outnumbered", "mid_combat", "advantage", "late_game"):
            return "combat_micro"
        elif phase == "early_game":
            return "economy"

    best = max(scores, key=scores.get)
    if scores[best] > 0:
        return best
    return "multi_behavior"


def balance_categories(examples: list[dict], max_pct: float = 0.30) -> list[dict]:
    """Ensure no category exceeds max_pct of total examples."""
    categorized = {}
    for ex in examples:
        cat = classify_example(ex)
        categorized.setdefault(cat, []).append(ex)

    total = len(examples)
    max_per_cat = int(total * max_pct)

    balanced = []
    overflow = []
    for cat, cat_examples in categorized.items():
        if len(cat_examples) > max_per_cat:
            random.shuffle(cat_examples)
            balanced.extend(cat_examples[:max_per_cat])
            overflow.extend(cat_examples[max_per_cat:])
        else:
            balanced.extend(cat_examples)

    # Fill remaining slots from overflow if we have space
    target_total = total
    remaining = target_total - len(balanced)
    if remaining > 0 and overflow:
        random.shuffle(overflow)
        balanced.extend(overflow[:remaining])

    return balanced


def split_train_eval(examples: list[dict], eval_ratio: float = 0.10) -> tuple[list, list]:
    """Split examples into train and eval sets."""
    random.shuffle(examples)
    eval_count = max(1, int(len(examples) * eval_ratio))
    return examples[eval_count:], examples[:eval_count]


def write_jsonl(examples: list[dict], path: Path):
    """Write examples to JSONL."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        for ex in examples:
            # Remove metadata before writing (it's internal)
            output = {k: v for k, v in ex.items() if k != "metadata"}
            f.write(json.dumps(output, ensure_ascii=False) + "\n")


def main():
    parser = argparse.ArgumentParser(
        description="Assemble final v2 training dataset"
    )
    parser.add_argument("--sft-inputs", nargs="+", type=Path,
                       default=DEFAULT_SFT_INPUTS,
                       help="SFT input JSONL files")
    parser.add_argument("--dpo-input", type=Path,
                       default=DEFAULT_DPO_INPUT,
                       help="DPO input JSONL file")
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR,
                       help="Output directory")
    parser.add_argument("--max-category-pct", type=float, default=0.30,
                       help="Max percentage for any category (default: 0.30)")
    parser.add_argument("--eval-ratio", type=float, default=0.10,
                       help="Eval set ratio (default: 0.10)")
    parser.add_argument("--dedup-threshold", type=float, default=0.85,
                       help="Deduplication similarity threshold (default: 0.85)")
    parser.add_argument("--seed", type=int, default=42,
                       help="Random seed for reproducibility")
    args = parser.parse_args()

    random.seed(args.seed)

    print("=== Dataset Assembly ===")

    # Load SFT examples
    all_sft = []
    for path in args.sft_inputs:
        examples = load_jsonl(path)
        print(f"  Loaded {len(examples)} from {path.name}")
        all_sft.extend(examples)
    print(f"  Total SFT raw: {len(all_sft)}")

    # Load DPO examples
    all_dpo = load_jsonl(args.dpo_input)
    print(f"  Total DPO raw: {len(all_dpo)}")

    # Step 1: Deduplicate SFT
    print(f"\nDeduplicating SFT (threshold={args.dedup_threshold})...")
    deduped_sft = deduplicate_sft(all_sft, args.dedup_threshold)
    removed = len(all_sft) - len(deduped_sft)
    print(f"  Removed {removed} duplicates → {len(deduped_sft)} unique")

    # Step 2: Categorize and report distribution
    print(f"\nCategory distribution:")
    cats = Counter(classify_example(ex) for ex in deduped_sft)
    for cat, count in cats.most_common():
        pct = count / max(len(deduped_sft), 1) * 100
        flag = " ⚠" if pct > args.max_category_pct * 100 else ""
        print(f"  {cat}: {count} ({pct:.0f}%){flag}")

    # Step 3: Balance categories
    print(f"\nBalancing categories (max {args.max_category_pct*100:.0f}%)...")
    balanced_sft = balance_categories(deduped_sft, args.max_category_pct)
    print(f"  After balancing: {len(balanced_sft)}")

    # Re-report distribution
    cats2 = Counter(classify_example(ex) for ex in balanced_sft)
    for cat, count in cats2.most_common():
        pct = count / max(len(balanced_sft), 1) * 100
        print(f"  {cat}: {count} ({pct:.0f}%)")

    # Step 4: Split train/eval
    print(f"\nSplitting train/eval ({1-args.eval_ratio:.0%}/{args.eval_ratio:.0%})...")
    sft_train, sft_eval = split_train_eval(balanced_sft, args.eval_ratio)
    print(f"  SFT train: {len(sft_train)}, eval: {len(sft_eval)}")

    dpo_train, dpo_eval = split_train_eval(all_dpo, args.eval_ratio)
    print(f"  DPO train: {len(dpo_train)}, eval: {len(dpo_eval)}")

    # Step 5: Write outputs
    print(f"\nWriting outputs...")
    sft_train_path = args.output_dir / "cc_v2_sft_train.jsonl"
    sft_eval_path = args.output_dir / "cc_v2_sft_eval.jsonl"
    dpo_train_path = args.output_dir / "cc_v2_dpo_train.jsonl"
    dpo_eval_path = args.output_dir / "cc_v2_dpo_eval.jsonl"

    write_jsonl(sft_train, sft_train_path)
    write_jsonl(sft_eval, sft_eval_path)
    write_jsonl(dpo_train, dpo_train_path)
    write_jsonl(dpo_eval, dpo_eval_path)

    print(f"  {sft_train_path.name}: {len(sft_train)} examples")
    print(f"  {sft_eval_path.name}: {len(sft_eval)} examples")
    print(f"  {dpo_train_path.name}: {len(dpo_train)} pairs")
    print(f"  {dpo_eval_path.name}: {len(dpo_eval)} pairs")

    # Summary
    print(f"\n{'='*60}")
    print(f"Dataset Assembly Complete")
    print(f"{'='*60}")
    print(f"SFT: {len(sft_train)} train + {len(sft_eval)} eval = {len(balanced_sft)} total")
    print(f"DPO: {len(dpo_train)} train + {len(dpo_eval)} eval = {len(all_dpo)} total")
    print(f"\nNext steps:")
    print(f"  1. Validate: python training/scripts/validate_pipeline.py {sft_train_path}")
    print(f"  2. Train SFT: See training/configs/devstral_24b_qlora.yaml")
    print(f"  3. Train DPO: python training/scripts/train_dpo.py")


if __name__ == "__main__":
    main()
