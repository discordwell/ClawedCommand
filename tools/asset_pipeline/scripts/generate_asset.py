#!/usr/bin/env python3
"""
ClawedCommand Asset Pipeline — Main Orchestrator

Usage:
    python generate_asset.py prompt <asset_name>       Assemble and print the ChatGPT prompt
    python generate_asset.py status                    Show catalog status overview
    python generate_asset.py process <asset_name>      Post-process a raw asset into game-ready form
    python generate_asset.py add <category> <name>     Add a new asset entry to the catalog
    python generate_asset.py model-sheet <faction>     Generate faction unit model sheet
    python generate_asset.py map-preview               Generate isometric map preview
    python generate_asset.py qc <asset_name|--all>     Run quality checks
    python generate_asset.py batch-faction <faction>   List all assets for a faction with prompts
    python generate_asset.py style-report              Show v1 vs v2 style version breakdown
    python generate_asset.py cleanup [--dry-run]       Clean up stale raws and sync catalog
    python generate_asset.py replace <name> <file>     Replace sprite with new version (archives old)
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


# Map from descriptive faction strings in catalog to faction file IDs
FACTION_ALIASES = {
    "catgpt": "catgpt",
    "catgpt (cats)": "catgpt",
    "clawed": "clawed",
    "the clawed": "clawed",
    "the clawed (mice)": "clawed",
    "murder": "murder",
    "the murder": "murder",
    "the murder (corvids)": "murder",
    "seekers": "seekers",
    "seekers of the deep": "seekers",
    "seekers of the deep (badgers)": "seekers",
    "croak": "croak",
    "croak (axolotls)": "croak",
    "llama": "llama",
    "llama (raccoons)": "llama",
}


def resolve_faction_id(faction_str):
    """Resolve a descriptive faction string to a canonical faction file ID."""
    if not faction_str:
        return None
    return FACTION_ALIASES.get(faction_str.lower().strip())


def load_faction_style(faction_id):
    """Load a faction style overlay file. Returns empty string if not found."""
    if not faction_id:
        return ""
    path = PROMPTS_DIR / f"faction_{faction_id}.txt"
    return path.read_text() if path.exists() else ""


def assemble_prompt(entry):
    """Build a complete ChatGPT prompt from base style + faction overlay + template.

    Prompt layer order (style-first — models weight early tokens more):
      1. base_style.txt      — identical for ALL assets
      2. faction_*.txt        — per-faction shape/color/material (if applicable)
      3. template + params    — category-specific content description
    """
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

    # Resolve faction overlay
    faction_str = params.get("faction", "")
    faction_id = resolve_faction_id(faction_str)
    faction_style = load_faction_style(faction_id)

    # Fill placeholders
    try:
        filled = template_text.format(**params)
    except KeyError as e:
        print(f"Error: missing param {e} for template '{template_name}'", file=sys.stderr)
        print(f"Available params: {list(params.keys())}", file=sys.stderr)
        sys.exit(1)

    # Assemble: base style → faction overlay → content (style-first order)
    layers = [base_style.strip()]
    if faction_style:
        layers.append(faction_style.strip())
    layers.append(filled.strip())
    prompt = "\n\n".join(layers)
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
    style_counts = {1: 0, 2: 0}
    rows = []

    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        for name, entry in assets.items():
            status = entry.get("status", "planned")
            status_counts[status] = status_counts.get(status, 0) + 1
            sv = entry.get("style_version", 1)
            style_counts[sv] = style_counts.get(sv, 0) + 1
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

    # Style version summary
    print(f"Style versions:")
    print(f"  v1 (legacy):  {style_counts.get(1, 0):3d}")
    print(f"  v2 (layered): {style_counts.get(2, 0):3d}")
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

    # Update catalog status and stamp style version
    catalog[category][key]["status"] = "game_ready"
    catalog[category][key]["style_version"] = 2
    save_catalog(catalog)
    print(f"  Status: game_ready (style_version: 2)")
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


def cmd_batch_faction(args):
    """List all catalog entries for a faction with assembled prompts."""
    catalog = load_catalog()
    target = args.faction.lower().strip()
    target_id = FACTION_ALIASES.get(target, target)

    entries = []
    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        for name, entry in assets.items():
            faction_str = entry.get("params", {}).get("faction", "")
            fid = resolve_faction_id(faction_str)
            if fid == target_id:
                sv = entry.get("style_version", 1)
                status = entry.get("status", "planned")
                entries.append((category, name, entry, sv, status))

    if not entries:
        print(f"No assets found for faction '{args.faction}'.", file=sys.stderr)
        print(f"Valid factions: {', '.join(sorted(set(FACTION_ALIASES.values())))}", file=sys.stderr)
        sys.exit(1)

    v1_count = sum(1 for e in entries if e[3] == 1)
    v2_count = sum(1 for e in entries if e[3] == 2)
    print(f"── Batch: {args.faction} ({len(entries)} assets, v1: {v1_count}, v2: {v2_count}) ──\n")

    for i, (category, name, entry, sv, status) in enumerate(entries, 1):
        version_tag = "\033[33mv1\033[0m" if sv == 1 else "\033[32mv2\033[0m"
        print(f"  [{i:2d}] {name:<30s}  {category:<12s}  {status:<10s}  {version_tag}")

    if not args.show_prompts:
        print(f"\nAdd --prompts to show assembled prompts for each asset.")
        return

    print("\n" + "=" * 60)
    for i, (category, name, entry, sv, status) in enumerate(entries, 1):
        prompt = assemble_prompt(entry)
        print(f"\n── [{i}] {name} ({category}) ──\n")
        print(prompt)
        print(f"\n── End ({len(prompt)} chars) ──")


def cmd_style_report(args):
    """Show v1 vs v2 style version breakdown per faction."""
    catalog = load_catalog()
    faction_assets = {}
    unfactioned = {"v1": [], "v2": []}

    for category, assets in catalog.items():
        if not isinstance(assets, dict):
            continue
        for name, entry in assets.items():
            faction_str = entry.get("params", {}).get("faction", "")
            fid = resolve_faction_id(faction_str)
            sv = entry.get("style_version", 1)
            status = entry.get("status", "planned")

            if fid:
                if fid not in faction_assets:
                    faction_assets[fid] = {"v1": [], "v2": []}
                faction_assets[fid][f"v{sv}"].append((name, category, status))
            else:
                unfactioned[f"v{sv}"].append((name, category, status))

    print("── Style Version Report ──\n")

    for fid in sorted(faction_assets.keys()):
        fa = faction_assets[fid]
        v1 = len(fa["v1"])
        v2 = len(fa["v2"])
        total = v1 + v2
        pct = (v2 / total * 100) if total > 0 else 0
        bar = "\033[32m█\033[0m" * v2 + "\033[33m░\033[0m" * v1
        print(f"  {fid:<12s}  {v2:3d}/{total:3d} v2 ({pct:5.1f}%)  {bar}")
        if args.verbose and fa["v1"]:
            for name, cat, status in fa["v1"]:
                print(f"    v1: {name} ({cat}, {status})")

    # Unfactioned assets
    v1 = len(unfactioned["v1"])
    v2 = len(unfactioned["v2"])
    total = v1 + v2
    if total > 0:
        pct = (v2 / total * 100) if total > 0 else 0
        bar = "\033[32m█\033[0m" * v2 + "\033[33m░\033[0m" * v1
        print(f"  {'(none)':<12s}  {v2:3d}/{total:3d} v2 ({pct:5.1f}%)  {bar}")
        if args.verbose and unfactioned["v1"]:
            for name, cat, status in unfactioned["v1"]:
                print(f"    v1: {name} ({cat}, {status})")

    print()


def cmd_cleanup(args):
    """Clean up stale raw files and sync catalog with disk state."""
    catalog = load_catalog()

    raw_units = RAW_DIR / "units"
    output_units = ASSETS_DIR / "sprites" / "units"
    archive_dir = RAW_DIR / "archive"

    # 1. Find stale v1 raws superseded by v2
    stale_v1 = []
    if raw_units.exists():
        for v2_raw in sorted(raw_units.glob("*_idle_raw.png")):
            slug = v2_raw.stem.replace("_idle_raw", "")
            v1_raw = raw_units / f"{slug}_raw.png"
            if v1_raw.exists():
                stale_v1.append(v1_raw)

    # 2. Find orphaned raws (no corresponding output)
    stale_v1_set = set(f.name for f in stale_v1)
    orphaned = []
    if raw_units.exists():
        for raw_file in sorted(raw_units.glob("*.png")):
            if raw_file.name in stale_v1_set:
                continue  # already handled above
            stem = raw_file.stem
            if not stem.endswith("_raw"):
                continue
            base = stem[:-4]  # e.g. "chonk_walk_raw" → "chonk_walk"
            # Check direct match and _idle variant (v1 raws use slug_raw, output is slug_idle)
            candidates = [
                output_units / f"{base}.png",
                output_units / f"{base}_idle.png",
            ]
            if not any(c.exists() for c in candidates):
                orphaned.append(raw_file)

    # 3. Sync catalog status with disk
    synced = 0
    units = catalog.get("units", {})
    for key, entry in units.items():
        if entry.get("status") == "planned":
            game_path = entry.get("output", {}).get("game_path", "")
            if game_path:
                full = PROJECT_ROOT / game_path
                if full.exists():
                    entry["status"] = "game_ready"
                    synced += 1

    if args.dry_run:
        print("── Cleanup Dry Run ──\n")
        if stale_v1:
            print(f"Would delete {len(stale_v1)} stale v1 raws:")
            for f in stale_v1:
                print(f"  rm {f.relative_to(PIPELINE_ROOT)}")
        if orphaned:
            print(f"\nWould {'archive' if args.archive else 'delete'} {len(orphaned)} orphaned raws:")
            for f in orphaned:
                print(f"  {'mv' if args.archive else 'rm'} {f.relative_to(PIPELINE_ROOT)}")
        if synced:
            print(f"\nWould sync {synced} catalog entries: planned → game_ready")
        if not stale_v1 and not orphaned and not synced:
            print("Nothing to clean up!")
        return

    # Execute
    deleted = 0
    archived = 0

    for f in stale_v1:
        f.unlink()
        deleted += 1
        print(f"  Deleted stale v1: {f.name}")

    for f in orphaned:
        if args.archive:
            archive_dir.mkdir(parents=True, exist_ok=True)
            dest = archive_dir / f.name
            shutil.move(str(f), str(dest))
            archived += 1
            print(f"  Archived orphan: {f.name}")
        else:
            f.unlink()
            deleted += 1
            print(f"  Deleted orphan: {f.name}")

    if synced:
        save_catalog(catalog)
        print(f"  Synced {synced} catalog entries → game_ready")

    # Clean up any empty subdirs
    if raw_units.exists():
        for subdir in raw_units.iterdir():
            if subdir.is_dir():
                try:
                    subdir.rmdir()
                    print(f"  Removed empty dir: {subdir.name}/")
                except OSError:
                    pass

    print(f"\nDone: {deleted} deleted, {archived} archived, {synced} catalog synced")


def cmd_replace(args):
    """Replace a sprite with a new version, archiving the old one."""
    catalog = load_catalog()
    category, key, entry = find_asset(catalog, args.name)
    if entry is None:
        print(f"Error: asset '{args.name}' not found in catalog", file=sys.stderr)
        sys.exit(1)

    game_path = entry.get("output", {}).get("game_path", "")
    if not game_path:
        print(f"Error: asset '{args.name}' has no game_path", file=sys.stderr)
        sys.exit(1)

    current = PROJECT_ROOT / game_path
    new_file = Path(args.new_file)

    if not new_file.exists():
        print(f"Error: new file not found: {new_file}", file=sys.stderr)
        sys.exit(1)

    # Archive the current version
    archive_dir = RAW_DIR / "archive"
    archive_dir.mkdir(parents=True, exist_ok=True)

    if current.exists():
        # Timestamp the archive copy
        import time
        ts = time.strftime("%Y%m%d_%H%M%S")
        archive_name = f"{args.name}_{ts}.png"
        dest = archive_dir / archive_name
        shutil.copy2(str(current), str(dest))
        print(f"  Archived: {current.name} → archive/{archive_name}")

    # Copy new file into place
    shutil.copy2(str(new_file), str(current))
    print(f"  Replaced: {current.name}")

    # Update catalog
    entry["status"] = "game_ready"
    entry["style_version"] = 2
    save_catalog(catalog)
    print(f"  Catalog: {args.name} → style_version=2, game_ready")


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

    # batch-faction
    p_bf = subparsers.add_parser("batch-faction", help="List all assets for a faction with prompts")
    p_bf.add_argument("faction", help="Faction ID (catgpt, clawed, murder, seekers, croak, llama)")
    p_bf.add_argument("--prompts", dest="show_prompts", action="store_true",
                       help="Show assembled prompts for each asset")

    # style-report
    p_sr = subparsers.add_parser("style-report", help="Show v1 vs v2 style version breakdown")
    p_sr.add_argument("--verbose", "-v", action="store_true", help="List individual v1 assets")

    # cleanup
    p_cl = subparsers.add_parser("cleanup", help="Clean up stale raws and sync catalog")
    p_cl.add_argument("--dry-run", action="store_true", help="Show what would be done without doing it")
    p_cl.add_argument("--archive", action="store_true", help="Archive orphaned files instead of deleting")

    # replace
    p_rp = subparsers.add_parser("replace", help="Replace a sprite with a new version, archiving the old")
    p_rp.add_argument("name", help="Asset name from catalog (e.g. scrounger_idle)")
    p_rp.add_argument("new_file", help="Path to the new sprite file")

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
    elif args.command == "batch-faction":
        cmd_batch_faction(args)
    elif args.command == "style-report":
        cmd_style_report(args)
    elif args.command == "cleanup":
        cmd_cleanup(args)
    elif args.command == "replace":
        cmd_replace(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
