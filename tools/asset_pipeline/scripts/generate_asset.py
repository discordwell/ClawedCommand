#!/usr/bin/env python3
"""
ClawedCommand Asset Pipeline — Main Orchestrator

Usage:
    python generate_asset.py prompt <asset_name>    Assemble and print the ChatGPT prompt
    python generate_asset.py status                 Show catalog status overview
    python generate_asset.py process <asset_name>   Post-process a raw asset into game-ready form
    python generate_asset.py add <category> <name>  Add a new asset entry to the catalog
    python generate_asset.py model-sheet <faction>  Generate faction unit model sheet
    python generate_asset.py map-preview            Generate isometric map preview
    python generate_asset.py qc <asset_name|--all>  Run quality checks
"""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

import yaml

PIPELINE_ROOT = Path(__file__).resolve().parent.parent
CONFIG_DIR = PIPELINE_ROOT / "config"
CATALOG_PATH = CONFIG_DIR / "asset_catalog.yaml"
PROMPTS_DIR = CONFIG_DIR / "prompts"
RAW_DIR = PIPELINE_ROOT / "raw"
PROCESSED_DIR = PIPELINE_ROOT / "processed"
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
ASSETS_DIR = PROJECT_ROOT / "assets"
SCRIPTS_DIR = PIPELINE_ROOT / "scripts"


def load_catalog():
    with open(CATALOG_PATH) as f:
        data = yaml.safe_load(f)
        return data if data else {}


def save_catalog(catalog):
    with open(CATALOG_PATH, "w") as f:
        yaml.dump(catalog, f, default_flow_style=False, sort_keys=False, width=120)


def find_asset(catalog, name):
    """Find an asset by name across all categories. Returns (category, key, entry)."""
    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        if name in assets:
            return category, name, assets[name]
    return None, None, None


def load_template(template_name):
    path = PROMPTS_DIR / f"{template_name}.txt"
    if not path.exists():
        print(f"Error: template '{template_name}' not found at {path}", file=sys.stderr)
        sys.exit(1)
    return path.read_text()


def load_base_style():
    path = PROMPTS_DIR / "base_style.txt"
    return path.read_text() if path.exists() else ""


def assemble_prompt(entry):
    """Build a complete ChatGPT prompt from catalog entry + template + base style."""
    template_name = entry["template"]
    params = dict(entry.get("params", {}))

    # Auto-fill palette_colors for terrain entries from palette.yaml
    if template_name == "terrain" and "palette_colors" not in params:
        palette_path = CONFIG_DIR / "palette.yaml"
        if palette_path.exists():
            with open(palette_path) as f:
                palette = yaml.safe_load(f) or {}
            terrain_colors = palette.get("terrain", {})
            color_list = [f"{name}: {hex_val}" for name, hex_val in terrain_colors.items()
                          if isinstance(hex_val, str) and hex_val.startswith("#")]
            params["palette_colors"] = ", ".join(color_list) if color_list else "natural muted tones"
        else:
            params["palette_colors"] = "natural muted tones"

    template_text = load_template(template_name)
    base_style = load_base_style()

    # Fill placeholders
    try:
        filled = template_text.format(**params)
    except KeyError as e:
        print(f"Error: missing param {e} for template '{template_name}'", file=sys.stderr)
        print(f"Available params: {list(params.keys())}", file=sys.stderr)
        sys.exit(1)

    prompt = filled.strip() + "\n\n" + base_style.strip()
    return prompt


# ── Subcommands ─────────────────────────────────────────────


def cmd_prompt(args):
    catalog = load_catalog()
    category, key, entry = find_asset(catalog, args.name)
    if entry is None:
        print(f"Error: asset '{args.name}' not found in catalog", file=sys.stderr)
        sys.exit(1)

    prompt = assemble_prompt(entry)
    print(f"── Prompt for: {args.name} ({category}) ──\n")
    print(prompt)
    print(f"\n── End prompt ({len(prompt)} chars) ──")


def cmd_status(args):
    catalog = load_catalog()
    status_counts = {"planned": 0, "generated": 0, "processed": 0, "game_ready": 0}
    rows = []

    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        for name, entry in assets.items():
            status = entry.get("status", "planned")
            status_counts[status] = status_counts.get(status, 0) + 1
            output = entry.get("output", {})
            game_path = output.get("game_path", "—")
            rows.append((category, name, status, game_path))

    # Summary
    total = sum(status_counts.values())
    print(f"Asset Catalog — {total} assets\n")
    for s, c in status_counts.items():
        bar = "█" * c + "░" * (total - c)
        print(f"  {s:12s}  {c:3d}  {bar}")
    print()

    # Table
    cat_w = max(len(r[0]) for r in rows) if rows else 8
    name_w = max(len(r[1]) for r in rows) if rows else 8
    status_w = 10

    header = f"  {'Category':<{cat_w}}  {'Name':<{name_w}}  {'Status':<{status_w}}  Path"
    print(header)
    print("  " + "─" * (len(header) - 2))

    current_cat = None
    for category, name, status, game_path in rows:
        cat_display = category if category != current_cat else ""
        current_cat = category

        # Color status
        if status == "game_ready":
            status_display = f"\033[32m{status}\033[0m"
        elif status == "processed":
            status_display = f"\033[33m{status}\033[0m"
        elif status == "generated":
            status_display = f"\033[36m{status}\033[0m"
        else:
            status_display = f"\033[90m{status}\033[0m"

        print(f"  {cat_display:<{cat_w}}  {name:<{name_w}}  {status_display:<{status_w + 9}}  {game_path}")


def cmd_process(args):
    catalog = load_catalog()
    category, key, entry = find_asset(catalog, args.name)
    if entry is None:
        print(f"Error: asset '{args.name}' not found in catalog", file=sys.stderr)
        sys.exit(1)

    output = entry.get("output", {})
    asset_type = output.get("type", "single")
    game_path = output.get("game_path")

    # Find the raw file
    raw_path = RAW_DIR / category / f"{args.name}_raw.png"
    if not raw_path.exists():
        # Try without _raw suffix
        raw_path = RAW_DIR / category / f"{args.name}.png"
    if not raw_path.exists():
        print(f"Error: raw file not found. Expected at:", file=sys.stderr)
        print(f"  {RAW_DIR / category / f'{args.name}_raw.png'}", file=sys.stderr)
        print(f"  {RAW_DIR / category / f'{args.name}.png'}", file=sys.stderr)
        sys.exit(1)

    processed_path = PROCESSED_DIR / category / f"{args.name}.png"
    processed_path.parent.mkdir(parents=True, exist_ok=True)

    final_path = PROJECT_ROOT / game_path if game_path else None
    if final_path:
        final_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Processing: {args.name} ({category}, {asset_type})")
    print(f"  Raw:       {raw_path}")
    print(f"  Processed: {processed_path}")
    if final_path:
        print(f"  Game:      {final_path}")

    # Determine target dimensions from params
    params = entry.get("params", {})

    if asset_type == "sheet":
        columns = output.get("columns", 4)
        rows_count = output.get("rows", 1)
        tile_size = output.get("tile_size", [128, 128])

        # Run process_sheet.py
        cmd = [
            sys.executable, str(SCRIPTS_DIR / "process_sheet.py"),
            str(raw_path), str(processed_path),
            "--columns", str(columns),
            "--rows", str(rows_count),
            "--tile-width", str(tile_size[0]),
            "--tile-height", str(tile_size[1]),
        ]
        print(f"  Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  Error in process_sheet:\n{result.stderr}", file=sys.stderr)
            sys.exit(1)
        if result.stdout.strip():
            print(f"  {result.stdout.strip()}")

        # Verify grid
        cmd_verify = [
            sys.executable, str(SCRIPTS_DIR / "verify_grid.py"),
            str(processed_path),
            "--columns", str(columns),
            "--rows", str(rows_count),
        ]
        print(f"  Verifying grid...")
        result = subprocess.run(cmd_verify, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  Grid verification warning:\n{result.stderr}", file=sys.stderr)
        if result.stdout.strip():
            print(f"  {result.stdout.strip()}")

        # Update atlas manifest
        cmd_atlas = [
            sys.executable, str(SCRIPTS_DIR / "generate_atlas_meta.py"),
            "--name", args.name,
            "--path", game_path or str(processed_path),
            "--columns", str(columns),
            "--rows", str(rows_count),
            "--tile-width", str(tile_size[0]),
            "--tile-height", str(tile_size[1]),
        ]
        print(f"  Updating atlas manifest...")
        result = subprocess.run(cmd_atlas, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  Atlas manifest error:\n{result.stderr}", file=sys.stderr)
        if result.stdout.strip():
            print(f"  {result.stdout.strip()}")

    else:
        # Single sprite
        target_w = params.get("width", 128)
        target_h = params.get("height", 128)

        cmd = [
            sys.executable, str(SCRIPTS_DIR / "process_sprite.py"),
            str(raw_path), str(processed_path),
            "--width", str(target_w),
            "--height", str(target_h),
        ]
        print(f"  Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  Error in process_sprite:\n{result.stderr}", file=sys.stderr)
            sys.exit(1)
        if result.stdout.strip():
            print(f"  {result.stdout.strip()}")

    # Copy to final game path
    if final_path and processed_path.exists():
        shutil.copy2(processed_path, final_path)
        print(f"  Copied to: {final_path}")

    # Update catalog status
    catalog[category][key]["status"] = "game_ready"
    save_catalog(catalog)
    print(f"  Status: game_ready")
    print("Done.")

    # Auto-trigger QC and review sheets
    _auto_trigger_qc(args.name, category)


def cmd_add(args):
    """Add a new asset entry to the catalog."""
    catalog = load_catalog()
    category = args.category
    name = args.name

    if category not in catalog:
        catalog[category] = {}

    if name in catalog[category]:
        print(f"Error: asset '{name}' already exists in '{category}'", file=sys.stderr)
        sys.exit(1)

    # Infer template from category
    template_map = {
        "terrain": "terrain",
        "buildings": "building",
        "units": "unit_static",
        "resources": "resource",
        "projectiles": "projectile",
        "ui": "ui",
        "portraits": "portrait",
    }
    template = template_map.get(category, "unit_static")

    # Determine if this should be a sheet (name ends with _walk, _attack, etc.)
    sheet_animations = ("_walk", "_run", "_attack", "_death", "_cast")
    is_sheet = any(name.endswith(suffix) for suffix in sheet_animations)
    if is_sheet:
        template = "unit_sheet"

    entry = {
        "template": template,
        "status": "planned",
        "params": {
            "name": name.replace("_", " ").title(),
            "description": f"TODO: describe {name}",
            "width": 128,
            "height": 128,
            "extra_notes": "",
        },
        "output": {
            "type": "sheet" if is_sheet else "single",
            "game_path": f"assets/sprites/{category}/{name}.png",
        },
    }

    if is_sheet:
        entry["params"].update({
            "animation": name.split("_")[-1] + " cycle",
            "frame_count": 4,
            "frame_width": 128,
            "frame_height": 128,
            "sheet_width": 512,
            "direction": "south-east",
        })
        entry["output"].update({
            "columns": 4,
            "rows": 1,
            "tile_size": [128, 128],
        })
    else:
        # Add template-specific fields
        if template == "unit_static":
            entry["params"]["pose"] = "idle"
            entry["params"]["direction"] = "south-east"
        elif template == "building":
            entry["params"]["faction"] = "generic sci-fi military"
            entry["params"]["footprint"] = "2x2"
        elif template == "terrain":
            entry["params"]["terrain_type"] = name
            entry["params"]["palette_colors"] = "See palette.yaml"
        elif template == "projectile":
            entry["params"]["frame_count"] = 1
        elif template == "ui":
            entry["params"]["context"] = "HUD overlay"
        elif template == "portrait":
            entry["params"]["animal"] = "TODO: animal species"
            entry["params"]["faction"] = "TODO: faction name"
            entry["params"]["role"] = "TODO: character role"

    catalog[category][name] = entry
    save_catalog(catalog)
    print(f"Added '{name}' to '{category}' (template: {template})")
    print(f"Edit {CATALOG_PATH} to fill in description and params.")


def _auto_trigger_qc(asset_name, category):
    """Auto-trigger QC checks and review sheets after processing."""
    # Run QC on the processed asset
    print(f"\n── Auto QC ──")
    cmd = [sys.executable, str(SCRIPTS_DIR / "qc_check.py"), asset_name]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.stdout.strip():
        print(result.stdout.strip())
    if result.returncode != 0 and result.stderr.strip():
        print(result.stderr.strip())

    # Auto-regenerate model sheet if a unit was processed
    if category == "units":
        _auto_regen_model_sheet(asset_name)

    # Auto-regenerate map preview if terrain was processed
    if category == "terrain":
        print(f"\n── Auto Map Preview ──")
        cmd = [sys.executable, str(SCRIPTS_DIR / "map_preview.py")]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.stdout.strip():
            print(result.stdout.strip())


def _auto_regen_model_sheet(asset_name):
    """Determine which faction a unit belongs to and regenerate its model sheet."""
    # Import faction data from model_sheet module
    sys.path.insert(0, str(SCRIPTS_DIR))
    try:
        from model_sheet import FACTIONS
    except ImportError:
        return

    # Find which faction this unit belongs to
    unit_base = asset_name.rsplit("_", 1)[0]  # e.g. "nuisance_idle" → "nuisance"
    # Also try the full name without last segment for multi-word units
    for faction_id, faction_data in FACTIONS.items():
        if unit_base in faction_data["units"]:
            print(f"\n── Auto Model Sheet ({faction_id}) ──")
            cmd = [sys.executable, str(SCRIPTS_DIR / "model_sheet.py"), faction_id]
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.stdout.strip():
                print(result.stdout.strip())
            return

    # Try matching with progressively shorter prefixes
    parts = asset_name.split("_")
    for i in range(len(parts) - 1, 0, -1):
        candidate = "_".join(parts[:i])
        for faction_id, faction_data in FACTIONS.items():
            if candidate in faction_data["units"]:
                print(f"\n── Auto Model Sheet ({faction_id}) ──")
                cmd = [sys.executable, str(SCRIPTS_DIR / "model_sheet.py"), faction_id]
                result = subprocess.run(cmd, capture_output=True, text=True)
                if result.stdout.strip():
                    print(result.stdout.strip())
                return


def cmd_model_sheet(args):
    """Generate a faction model sheet."""
    cmd = [sys.executable, str(SCRIPTS_DIR / "model_sheet.py"), args.faction]
    if args.include_planned:
        cmd.append("--include-planned")
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


def cmd_map_preview(args):
    """Generate a map preview."""
    cmd = [sys.executable, str(SCRIPTS_DIR / "map_preview.py")]
    if args.size:
        cmd.extend(["--size", str(args.size)])
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


def cmd_qc(args):
    """Run quality checks."""
    cmd = [sys.executable, str(SCRIPTS_DIR / "qc_check.py")]
    if args.qc_all:
        cmd.append("--all")
    elif args.name:
        cmd.append(args.name)
    if hasattr(args, "include_planned") and args.include_planned:
        cmd.append("--include-planned")
    if hasattr(args, "category") and args.category:
        cmd.extend(["--category", args.category])
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


# ── Main ────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(description="ClawedCommand Asset Pipeline")
    subparsers = parser.add_subparsers(dest="command")

    # prompt
    p_prompt = subparsers.add_parser("prompt", help="Assemble and print the ChatGPT prompt for an asset")
    p_prompt.add_argument("name", help="Asset name from catalog")

    # status
    subparsers.add_parser("status", help="Show catalog status overview")

    # process
    p_process = subparsers.add_parser("process", help="Post-process a raw asset")
    p_process.add_argument("name", help="Asset name from catalog")

    # add
    p_add = subparsers.add_parser("add", help="Add a new asset entry to the catalog")
    p_add.add_argument("category", help="Category (terrain, buildings, units, resources, projectiles, ui)")
    p_add.add_argument("name", help="Asset name (e.g. battle_cat_idle)")

    # model-sheet
    p_ms = subparsers.add_parser("model-sheet", help="Generate faction unit model sheet")
    p_ms.add_argument("faction", help="Faction ID (catgpt, the_clawed, seekers, the_murder, croak, llama, all)")
    p_ms.add_argument("--include-planned", action="store_true", help="Show placeholders for planned assets")

    # map-preview
    p_mp = subparsers.add_parser("map-preview", help="Generate isometric map preview")
    p_mp.add_argument("--size", type=int, default=12, help="Map grid size (default: 12)")

    # qc
    p_qc = subparsers.add_parser("qc", help="Run quality checks on assets")
    p_qc.add_argument("name", nargs="?", help="Asset name to check")
    p_qc.add_argument("--all", dest="qc_all", action="store_true", help="Check all game_ready assets")
    p_qc.add_argument("--category", type=str, help="Check all assets in a category")
    p_qc.add_argument("--include-planned", action="store_true", help="Also check planned assets")

    args = parser.parse_args()

    if args.command == "prompt":
        cmd_prompt(args)
    elif args.command == "status":
        cmd_status(args)
    elif args.command == "process":
        cmd_process(args)
    elif args.command == "add":
        cmd_add(args)
    elif args.command == "model-sheet":
        cmd_model_sheet(args)
    elif args.command == "map-preview":
        cmd_map_preview(args)
    elif args.command == "qc":
        cmd_qc(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
