#!/usr/bin/env python3
"""CLI helper for MCP-based asset generation (Claude browser automation).

Subcommands:
    queue    [--category CAT] [--provider chatgpt|gemini]  Show assets needing generation
    prompt   <asset_name>                                   Assemble and print the prompt
    received <asset_name> <raw_path> [--provider P]         Register a downloaded raw image, run QC
    process  <asset_name>                                   Run post-processing chain
    status                                                  Show generation queue with provider info
    providers                                               List available providers and their URLs
"""

import argparse
import sys
from pathlib import Path

# Add scripts dir to path so we can import siblings
SCRIPTS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_DIR))

from generate_asset import (
    load_catalog,
    save_catalog,
    find_asset,
    assemble_prompt,
    CATALOG_PATH,
    RAW_DIR,
    PROJECT_ROOT,
)
from image_utils import download_and_validate


def cmd_queue(args):
    """Show assets that need generation, optionally filtered by category."""
    catalog = load_catalog()
    pending = []

    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        if args.category and category != args.category:
            continue
        for name, entry in assets.items():
            status = entry.get("status", "planned")
            if status in ("planned", "generated"):
                game_path = entry.get("output", {}).get("game_path", "")
                # Check if file already exists on disk
                if game_path and (PROJECT_ROOT / game_path).exists():
                    continue
                pending.append((category, name, status, entry))

    if not pending:
        print("All assets are generated!")
        return

    print(f"Assets needing generation: {len(pending)}\n")
    for category, name, status, entry in pending:
        template = entry.get("template", "?")
        print(f"  {category:<12s}  {name:<30s}  {status:<10s}  ({template})")


def cmd_prompt(args):
    """Assemble and print the prompt for an asset."""
    catalog = load_catalog()
    category, key, entry = find_asset(catalog, args.name)
    if entry is None:
        print(f"Error: asset '{args.name}' not found in catalog", file=sys.stderr)
        sys.exit(1)

    prompt = assemble_prompt(entry)
    # Print raw prompt text for Claude to copy into the provider
    print(prompt)


def cmd_received(args):
    """Register a downloaded raw image and run QC validation."""
    catalog = load_catalog()
    category, key, entry = find_asset(catalog, args.name)
    if entry is None:
        print(f"Error: asset '{args.name}' not found in catalog", file=sys.stderr)
        sys.exit(1)

    raw_path = Path(args.raw_path)
    if not raw_path.exists():
        print(f"Error: raw file not found: {raw_path}", file=sys.stderr)
        sys.exit(1)

    # Determine sprite type from template
    template = entry.get("template", "")
    if "sheet" in template:
        sprite_type = "sheet"
    elif "portrait" in template:
        sprite_type = "portrait"
    else:
        sprite_type = "idle"

    # Determine crop size from params
    params = entry.get("params", {})
    output = entry.get("output", {})
    if output.get("type") == "sheet":
        crop_size = None  # Sheets keep original dimensions
    else:
        w = params.get("width", 128)
        h = params.get("height", 128)
        crop_size = (w, h)

    # Copy raw to canonical location
    raw_dest = RAW_DIR / category / f"{args.name}_raw.png"
    raw_dest.parent.mkdir(parents=True, exist_ok=True)

    import shutil
    shutil.copy2(str(raw_path), str(raw_dest))

    # Provider suffix for A/B testing
    if args.provider:
        suffixed = RAW_DIR / category / f"{args.name}_{args.provider}_raw.png"
        shutil.copy2(str(raw_path), str(suffixed))
        print(f"  Raw saved: {suffixed.name}")

    # Run QC (validate only — don't overwrite raw, use a temp output)
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        qc_out = tmp.name
    success, reason = download_and_validate(
        str(raw_dest), qc_out,
        sprite_type=sprite_type,
        crop_size=crop_size,
    )
    Path(qc_out).unlink(missing_ok=True)

    if success:
        print(f"  QC passed: {reason}")
        catalog[category][key]["status"] = "generated"
        save_catalog(catalog)
        print(f"  Status: generated")
    else:
        print(f"  QC FAILED: {reason}")
        sys.exit(1)


def cmd_process(args):
    """Run the full post-processing chain via generate_asset.py."""
    import subprocess
    cmd = [sys.executable, str(SCRIPTS_DIR / "generate_asset.py"), "process", args.name]
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


def cmd_status(args):
    """Show generation status overview."""
    catalog = load_catalog()

    counts = {"planned": 0, "generated": 0, "processed": 0, "game_ready": 0}
    by_category = {}

    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        if category not in by_category:
            by_category[category] = {"planned": 0, "generated": 0, "processed": 0, "game_ready": 0}
        for name, entry in assets.items():
            status = entry.get("status", "planned")
            counts[status] = counts.get(status, 0) + 1
            by_category[category][status] = by_category[category].get(status, 0) + 1

    total = sum(counts.values())
    print(f"Asset Pipeline Status — {total} total\n")

    for status, count in counts.items():
        pct = count / total * 100 if total else 0
        print(f"  {status:<12s}  {count:3d}  ({pct:5.1f}%)")

    print(f"\nBy category:")
    for cat, sc in sorted(by_category.items()):
        cat_total = sum(sc.values())
        ready = sc.get("game_ready", 0)
        print(f"  {cat:<14s}  {ready}/{cat_total} ready")


def cmd_providers(args):
    """List available providers."""
    from providers.base import PROVIDERS
    print("Available providers:\n")
    for name, module_path in PROVIDERS.items():
        try:
            from providers.base import get_provider
            mod = get_provider(name)
            print(f"  {name:<10s}  {mod.URL}")
        except Exception as e:
            print(f"  {name:<10s}  (error: {e})")


def main():
    parser = argparse.ArgumentParser(description="MCP-based asset generation helper")
    subparsers = parser.add_subparsers(dest="command")

    # queue
    p_queue = subparsers.add_parser("queue", help="Show assets needing generation")
    p_queue.add_argument("--category", type=str, help="Filter by category")

    # prompt
    p_prompt = subparsers.add_parser("prompt", help="Assemble and print the prompt")
    p_prompt.add_argument("name", help="Asset name from catalog")

    # received
    p_recv = subparsers.add_parser("received", help="Register a downloaded raw image")
    p_recv.add_argument("name", help="Asset name from catalog")
    p_recv.add_argument("raw_path", help="Path to the downloaded raw image")
    p_recv.add_argument("--provider", type=str, help="Provider that generated it (for A/B)")

    # process
    p_proc = subparsers.add_parser("process", help="Run post-processing chain")
    p_proc.add_argument("name", help="Asset name from catalog")

    # status
    subparsers.add_parser("status", help="Show generation status overview")

    # providers
    subparsers.add_parser("providers", help="List available providers")

    args = parser.parse_args()

    commands = {
        "queue": cmd_queue,
        "prompt": cmd_prompt,
        "received": cmd_received,
        "process": cmd_process,
        "status": cmd_status,
        "providers": cmd_providers,
    }

    if args.command in commands:
        commands[args.command](args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
