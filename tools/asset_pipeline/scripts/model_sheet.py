#!/usr/bin/env python3
"""
Generate faction model sheets — composite images showing all units with their
animation states for visual review.

Layout:
  Rows = unit types (Pawdler, Nuisance, Chonk, ...)
  Columns = animation states (idle, walk, attack)
  Each cell shows the sprite (or a placeholder if not yet game_ready)
  Labels on left (unit name) and top (state)

Usage:
    python model_sheet.py catgpt                    # Generate catGPT model sheet
    python model_sheet.py all                       # Generate all factions
    python model_sheet.py catgpt --include-planned  # Show placeholders for planned assets too
"""

import argparse
import sys
from pathlib import Path

import yaml
from PIL import Image, ImageDraw, ImageFont

PIPELINE_ROOT = Path(__file__).resolve().parent.parent
CONFIG_DIR = PIPELINE_ROOT / "config"
CATALOG_PATH = CONFIG_DIR / "asset_catalog.yaml"
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
QC_DIR = PIPELINE_ROOT / "qc"

# Faction → unit name prefixes (base name without _idle/_walk/_attack)
FACTIONS = {
    "catgpt": {
        "label": "catGPT (Cats)",
        "units": [
            "pawdler", "nuisance", "chonk", "flying_fox", "hisser",
            "yowler", "mouser", "catnapper", "ferret_sapper", "mech_commander",
        ],
    },
    "the_clawed": {
        "label": "The Clawed (Mice)",
        "units": [
            "nibblet", "swarmer", "gnawer", "shrieker", "tunneler",
            "sparks", "quillback", "whiskerwitch", "plaguetail", "warren_marshal",
        ],
    },
    "seekers": {
        "label": "Seekers of the Deep (Badgers)",
        "units": [
            "delver", "ironhide", "cragback", "warden", "sapjaw",
            "wardenmother", "tunneler_seeker", "embermaw", "dustclaw", "gutripper",
        ],
    },
    "the_murder": {
        "label": "The Murder (Corvids)",
        "units": [
            "scrounger_murder", "sentinel", "rookclaw", "magpike", "magpyre",
            "jaycaller", "jayflicker", "dusktalon", "hootseer", "corvus_rex",
        ],
    },
    "croak": {
        "label": "Croak (Axolotls)",
        "units": [
            "ponderer", "regeneron", "broodmother", "gulper", "eftsaber",
            "croaker", "leapfrog", "shellwarden", "bogwhisper", "murk_commander",
        ],
    },
    "llama": {
        "label": "LLAMA (Raccoons)",
        "units": [
            "scrounger_llama", "bandit", "heap_titan", "glitch_rat", "patch_possum",
            "grease_monkey", "dead_drop", "wrecker", "dumpster_diver", "junkyard_king",
        ],
    },
}

# Animation states to look for (in column order)
ANIM_STATES = ["idle", "walk", "attack"]

# Layout constants
CELL_SIZE = 160          # Each cell is 160x160 (sprite + padding)
SPRITE_AREA = 140        # Sprite drawn within this area of the cell
LABEL_WIDTH = 180        # Left column for unit name labels
HEADER_HEIGHT = 50       # Top row for state labels
MARGIN = 20              # Outer margin
BG_COLOR = (30, 30, 35, 255)
CELL_BG = (40, 42, 48, 255)
CELL_BG_MISSING = (50, 35, 35, 255)
CELL_BG_PLANNED = (35, 40, 50, 255)
LABEL_COLOR = (220, 225, 230, 255)
SUBLABEL_COLOR = (140, 150, 160, 255)
GRID_COLOR = (55, 58, 65, 255)


def load_catalog():
    with open(CATALOG_PATH) as f:
        data = yaml.safe_load(f)
        return data if data else {}


def find_unit_assets(catalog, unit_base):
    """Find all animation states for a unit base name in the catalog."""
    units_cat = catalog.get("units", {})
    results = {}
    for state in ANIM_STATES:
        key = f"{unit_base}_{state}"
        if key in units_cat:
            entry = units_cat[key]
            results[state] = {
                "key": key,
                "entry": entry,
                "status": entry.get("status", "planned"),
                "game_path": entry.get("output", {}).get("game_path"),
                "params": entry.get("params", {}),
            }
    return results


def get_display_name(params):
    """Extract a clean display name from params."""
    name = params.get("name", "???")
    # Strip "The " prefix for compact display
    return name


def try_load_font(size):
    """Try to load a reasonable font, fall back to default."""
    font_paths = [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.dfont",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    for fp in font_paths:
        if Path(fp).exists():
            try:
                return ImageFont.truetype(fp, size)
            except Exception:
                continue
    return ImageFont.load_default()


def load_sprite(game_path, max_size=SPRITE_AREA):
    """Load a sprite from its game path, resize to fit display area."""
    if not game_path:
        return None
    full_path = PROJECT_ROOT / game_path
    if not full_path.exists():
        return None

    img = Image.open(full_path).convert("RGBA")

    # For sprite sheets, just show the first frame
    output_w, output_h = img.size
    # Heuristic: if width > 2x height, it's probably a sheet — crop first frame
    if output_w > output_h * 1.5:
        # Estimate columns from aspect ratio rather than hardcoding 4
        estimated_cols = max(1, round(output_w / output_h))
        frame_w = output_w // estimated_cols
        img = img.crop((0, 0, frame_w, output_h))

    # Fit into max_size square
    scale = min(max_size / img.width, max_size / img.height)
    if scale < 1.0:
        new_w = max(1, int(img.width * scale))
        new_h = max(1, int(img.height * scale))
        img = img.resize((new_w, new_h), Image.Resampling.LANCZOS)

    return img


def generate_model_sheet(faction_id, include_planned=False):
    """Generate a model sheet for a faction."""
    if faction_id not in FACTIONS:
        print(f"Error: unknown faction '{faction_id}'", file=sys.stderr)
        print(f"Available: {', '.join(FACTIONS.keys())}", file=sys.stderr)
        return None

    faction = FACTIONS[faction_id]
    catalog = load_catalog()

    # Collect unit data
    unit_rows = []
    for unit_base in faction["units"]:
        assets = find_unit_assets(catalog, unit_base)
        if not assets and not include_planned:
            continue
        # Only include if at least one asset is game_ready (or include_planned)
        has_ready = any(a["status"] == "game_ready" for a in assets.values())
        if not has_ready and not include_planned:
            continue
        # Get display name from first available entry
        display_name = unit_base.replace("_", " ").title()
        for state_data in assets.values():
            display_name = get_display_name(state_data["params"])
            break
        unit_rows.append((unit_base, display_name, assets))

    if not unit_rows and not include_planned:
        # No game_ready units yet — show everything as planned
        include_planned = True
        for unit_base in faction["units"]:
            assets = find_unit_assets(catalog, unit_base)
            display_name = unit_base.replace("_", " ").title()
            for state_data in assets.values():
                display_name = get_display_name(state_data["params"])
                break
            unit_rows.append((unit_base, display_name, assets))

    if not unit_rows:
        print(f"  No units found for {faction_id}")
        return None

    n_rows = len(unit_rows)
    n_cols = len(ANIM_STATES)

    # Calculate dimensions
    sheet_w = MARGIN * 2 + LABEL_WIDTH + n_cols * CELL_SIZE
    sheet_h = MARGIN * 2 + HEADER_HEIGHT + n_rows * CELL_SIZE

    # Create sheet
    sheet = Image.new("RGBA", (sheet_w, sheet_h), BG_COLOR)
    draw = ImageDraw.Draw(sheet)

    font_title = try_load_font(18)
    font_label = try_load_font(14)
    font_small = try_load_font(11)

    # Title
    title = f"{faction['label']} — Unit Model Sheet"
    draw.text((MARGIN, MARGIN // 2), title, fill=LABEL_COLOR, font=font_title)

    # Column headers
    for col_idx, state in enumerate(ANIM_STATES):
        x = MARGIN + LABEL_WIDTH + col_idx * CELL_SIZE + CELL_SIZE // 2
        y = MARGIN + HEADER_HEIGHT // 2
        # Center text
        bbox = draw.textbbox((0, 0), state.upper(), font=font_label)
        tw = bbox[2] - bbox[0]
        draw.text((x - tw // 2, y - 8), state.upper(), fill=SUBLABEL_COLOR, font=font_label)

    # Rows
    ready_count = 0
    total_cells = 0

    for row_idx, (unit_base, display_name, assets) in enumerate(unit_rows):
        row_y = MARGIN + HEADER_HEIGHT + row_idx * CELL_SIZE

        # Unit name label
        label_y = row_y + CELL_SIZE // 2 - 12
        draw.text((MARGIN + 8, label_y), display_name, fill=LABEL_COLOR, font=font_label)

        # Role subtitle from unit base name
        role = unit_base.replace("_", " ")
        draw.text((MARGIN + 8, label_y + 18), role, fill=SUBLABEL_COLOR, font=font_small)

        # Cells for each animation state
        for col_idx, state in enumerate(ANIM_STATES):
            total_cells += 1
            cell_x = MARGIN + LABEL_WIDTH + col_idx * CELL_SIZE
            cell_y = row_y

            state_data = assets.get(state)

            if state_data and state_data["status"] == "game_ready":
                # Draw sprite
                bg = CELL_BG
                sprite = load_sprite(state_data["game_path"])
                ready_count += 1
            elif state_data:
                # Planned/generated — show placeholder
                bg = CELL_BG_PLANNED
                sprite = None
            else:
                # No entry in catalog at all
                bg = CELL_BG_MISSING
                sprite = None

            # Cell background
            draw.rectangle(
                [cell_x + 2, cell_y + 2, cell_x + CELL_SIZE - 2, cell_y + CELL_SIZE - 2],
                fill=bg, outline=GRID_COLOR, width=1,
            )

            if sprite:
                # Center sprite in cell
                sx = cell_x + (CELL_SIZE - sprite.width) // 2
                sy = cell_y + (CELL_SIZE - sprite.height) // 2
                sheet.paste(sprite, (sx, sy), sprite)
            else:
                # Status label
                if state_data:
                    status = state_data["status"]
                    status_colors = {
                        "planned": (100, 110, 140, 255),
                        "generated": (80, 160, 180, 255),
                        "processed": (180, 160, 80, 255),
                    }
                    color = status_colors.get(status, SUBLABEL_COLOR)
                    bbox = draw.textbbox((0, 0), status, font=font_small)
                    tw = bbox[2] - bbox[0]
                    draw.text(
                        (cell_x + CELL_SIZE // 2 - tw // 2, cell_y + CELL_SIZE // 2 - 6),
                        status, fill=color, font=font_small,
                    )
                else:
                    # No catalog entry
                    bbox = draw.textbbox((0, 0), "no entry", font=font_small)
                    tw = bbox[2] - bbox[0]
                    draw.text(
                        (cell_x + CELL_SIZE // 2 - tw // 2, cell_y + CELL_SIZE // 2 - 6),
                        "no entry", fill=(80, 60, 60, 255), font=font_small,
                    )

    # Footer with stats
    footer_y = sheet_h - MARGIN + 4
    footer = f"{ready_count}/{total_cells} sprites ready"
    draw.text((MARGIN, footer_y), footer, fill=SUBLABEL_COLOR, font=font_small)

    return sheet


def main():
    parser = argparse.ArgumentParser(description="Generate faction model sheets")
    parser.add_argument("faction", help="Faction ID (catgpt, the_clawed, seekers, the_murder, croak, llama, all)")
    parser.add_argument("--include-planned", action="store_true",
                        help="Include units with planned/not-yet-ready sprites")
    parser.add_argument("--output", type=str, default=None,
                        help="Override output path")
    args = parser.parse_args()

    QC_DIR.mkdir(parents=True, exist_ok=True)

    factions_to_generate = list(FACTIONS.keys()) if args.faction == "all" else [args.faction]

    for faction_id in factions_to_generate:
        print(f"Generating model sheet: {faction_id}")
        sheet = generate_model_sheet(faction_id, include_planned=args.include_planned)
        if sheet is None:
            continue

        output_path = Path(args.output) if args.output else QC_DIR / f"model_sheet_{faction_id}.png"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        sheet.save(output_path, "PNG")
        print(f"  Saved: {output_path} ({sheet.width}x{sheet.height})")


if __name__ == "__main__":
    main()
