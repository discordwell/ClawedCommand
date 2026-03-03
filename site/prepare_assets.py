#!/usr/bin/env python3
"""Prepare web-optimized assets for the ClawedCommand site.

Landing page assets:
1. Copy 12 unit idle PNGs to site/assets/sprites/units/
2. Resize 7 building PNGs (1024->256) to site/assets/sprites/buildings/
3. Copy gameplay screenshot (if exists)
4. Generate 32x32 favicon from pawdler_idle.png

Game (WASM) assets:
5. Copy all sprite directories (units, buildings, terrain, resources, projectiles, heroes, portraits)
6. Copy campaign RON files
7. Copy atlas manifests
8. Copy scripts
9. Skip: voice/ (native-only), training data, models/
"""

import shutil
from pathlib import Path
from PIL import Image

PROJECT_ROOT = Path(__file__).resolve().parent.parent
SITE_DIR = PROJECT_ROOT / "site"
ASSETS_SRC = PROJECT_ROOT / "assets"
ASSETS_DST = SITE_DIR / "assets"
SRC_UNITS = ASSETS_SRC / "sprites" / "units"
SRC_BUILDINGS = ASSETS_SRC / "sprites" / "buildings"
SRC_SCREENSHOTS = ASSETS_SRC / "screenshots"
DST_UNITS = ASSETS_DST / "sprites" / "units"
DST_BUILDINGS = ASSETS_DST / "sprites" / "buildings"
DST_SCREENSHOTS = ASSETS_DST / "screenshots"

UNIT_SPRITES = [
    "mech_commander_idle.png",
    "warren_marshal_idle.png",
    "sentinel_idle.png",
    "rookclaw_idle.png",
    "murder_scrounger_idle.png",
    "plaguetail_idle.png",
    "pawdler_idle.png",
    "chonk_idle.png",
    "hisser_idle.png",
    "mouser_idle.png",
    "nuisance_idle.png",
    "nibblet_idle.png",
]

BUILDING_SPRITES = [
    "the_box.png",
    "the_burrow.png",
    "the_sett.png",
    "the_parliament.png",
    "the_dumpster.png",
    "the_grotto.png",
    "server_rack.png",
]

BUILDING_TARGET_SIZE = 256

# Sprite dirs to copy in full for WASM game
GAME_SPRITE_DIRS = [
    "terrain",
    "resources",
    "projectiles",
    "heroes",
    "portraits",
]

# Top-level asset dirs to copy in full for WASM game
GAME_ASSET_DIRS = [
    "campaign",
    "atlas",
    "scripts",
    "ui",
]

# Dirs to skip (native-only or too large)
SKIP_DIRS = {"voice", "models"}


def copy_units():
    DST_UNITS.mkdir(parents=True, exist_ok=True)
    copied = 0
    for name in UNIT_SPRITES:
        src = SRC_UNITS / name
        dst = DST_UNITS / name
        if src.exists():
            shutil.copy2(src, dst)
            copied += 1
            print(f"  [unit] {name} ({src.stat().st_size // 1024}KB)")
        else:
            print(f"  [unit] MISSING: {name}")
    print(f"  Copied {copied}/{len(UNIT_SPRITES)} unit sprites\n")


def resize_buildings():
    DST_BUILDINGS.mkdir(parents=True, exist_ok=True)
    resized = 0
    for name in BUILDING_SPRITES:
        src = SRC_BUILDINGS / name
        dst = DST_BUILDINGS / name
        if src.exists():
            img = Image.open(src)
            original_size = img.size
            img = img.resize(
                (BUILDING_TARGET_SIZE, BUILDING_TARGET_SIZE),
                Image.NEAREST,
            )
            img.save(dst, optimize=True, compress_level=9)
            resized += 1
            print(f"  [building] {name} {original_size} -> {BUILDING_TARGET_SIZE}x{BUILDING_TARGET_SIZE} ({dst.stat().st_size // 1024}KB)")
        else:
            print(f"  [building] MISSING: {name}")
    print(f"  Resized {resized}/{len(BUILDING_SPRITES)} building sprites\n")


def copy_screenshots():
    DST_SCREENSHOTS.mkdir(parents=True, exist_ok=True)
    gameplay = SRC_SCREENSHOTS / "gameplay.png"
    if gameplay.exists():
        shutil.copy2(gameplay, DST_SCREENSHOTS / "gameplay.png")
        print(f"  [screenshot] gameplay.png ({gameplay.stat().st_size // 1024}KB)\n")
    else:
        print("  [screenshot] No gameplay.png found -- skipping\n")


def generate_favicon():
    src = SRC_UNITS / "pawdler_idle.png"
    dst = ASSETS_DST / "favicon.png"
    if src.exists():
        img = Image.open(src)
        img = img.resize((32, 32), Image.NEAREST)
        img.save(dst, optimize=True)
        print(f"  [favicon] Generated 32x32 from pawdler_idle.png\n")
    else:
        print("  [favicon] MISSING: pawdler_idle.png -- cannot generate favicon\n")


def copy_game_sprite_dirs():
    """Copy full sprite subdirectories needed by the WASM game."""
    copied_total = 0
    for dirname in GAME_SPRITE_DIRS:
        src_dir = ASSETS_SRC / "sprites" / dirname
        dst_dir = ASSETS_DST / "sprites" / dirname
        if not src_dir.exists():
            print(f"  [sprites/{dirname}] MISSING -- skipping")
            continue
        dst_dir.mkdir(parents=True, exist_ok=True)
        count = 0
        for f in sorted(src_dir.iterdir()):
            if f.is_file():
                shutil.copy2(f, dst_dir / f.name)
                count += 1
        copied_total += count
        print(f"  [sprites/{dirname}] {count} files")

    # Also copy ALL unit sprites (not just the landing page subset)
    src_dir = SRC_UNITS
    if src_dir.exists():
        DST_UNITS.mkdir(parents=True, exist_ok=True)
        count = 0
        for f in sorted(src_dir.iterdir()):
            if f.is_file():
                shutil.copy2(f, DST_UNITS / f.name)
                count += 1
        copied_total += count
        print(f"  [sprites/units] {count} files (full)")

    # All building sprites at full size for the game (overwrites landing page resized copies)
    src_dir = SRC_BUILDINGS
    if src_dir.exists():
        DST_BUILDINGS.mkdir(parents=True, exist_ok=True)
        count = 0
        for f in sorted(src_dir.iterdir()):
            if f.is_file():
                shutil.copy2(f, DST_BUILDINGS / f.name)
                count += 1
        copied_total += count
        print(f"  [sprites/buildings] {count} files (full size)")

    print(f"  Total game sprites: {copied_total}\n")


def copy_game_asset_dirs():
    """Copy top-level asset directories needed by the WASM game."""
    for dirname in GAME_ASSET_DIRS:
        src_dir = ASSETS_SRC / dirname
        dst_dir = ASSETS_DST / dirname
        if not src_dir.exists():
            print(f"  [{dirname}] MISSING -- skipping")
            continue
        # Use copytree with dirs_exist_ok for clean overwrite
        if dst_dir.exists():
            shutil.rmtree(dst_dir)
        shutil.copytree(src_dir, dst_dir)
        count = sum(1 for _ in dst_dir.rglob("*") if _.is_file())
        print(f"  [{dirname}] {count} files")
    print()


def main():
    print("=== ClawedCommand Asset Preparation ===\n")

    print("-- Landing page assets --\n")

    print("Copying unit sprites (landing page)...")
    copy_units()

    print("Resizing building sprites (1024 -> 256px)...")
    resize_buildings()

    print("Copying screenshots...")
    copy_screenshots()

    print("Generating favicon...")
    generate_favicon()

    print("-- Game (WASM) assets --\n")

    print("Copying game sprite directories...")
    copy_game_sprite_dirs()

    print("Copying game asset directories...")
    copy_game_asset_dirs()

    print("=== Done ===")


if __name__ == "__main__":
    main()
