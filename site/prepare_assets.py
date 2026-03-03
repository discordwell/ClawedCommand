#!/usr/bin/env python3
"""Prepare web-optimized assets for the ClawedCommand landing page.

1. Copy 12 unit idle PNGs to site/assets/sprites/units/
2. Resize 7 building PNGs (1024→256) to site/assets/sprites/buildings/
3. Copy gameplay screenshot (if exists)
4. Generate 32x32 favicon from pawdler_idle.png
"""

import shutil
from pathlib import Path
from PIL import Image

PROJECT_ROOT = Path(__file__).resolve().parent.parent
SITE_DIR = PROJECT_ROOT / "site"
SRC_UNITS = PROJECT_ROOT / "assets" / "sprites" / "units"
SRC_BUILDINGS = PROJECT_ROOT / "assets" / "sprites" / "buildings"
SRC_SCREENSHOTS = PROJECT_ROOT / "assets" / "screenshots"
DST_UNITS = SITE_DIR / "assets" / "sprites" / "units"
DST_BUILDINGS = SITE_DIR / "assets" / "sprites" / "buildings"
DST_SCREENSHOTS = SITE_DIR / "assets" / "screenshots"

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
            print(f"  [building] {name} {original_size} → {BUILDING_TARGET_SIZE}x{BUILDING_TARGET_SIZE} ({dst.stat().st_size // 1024}KB)")
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
        print("  [screenshot] No gameplay.png found — skipping\n")


def generate_favicon():
    src = SRC_UNITS / "pawdler_idle.png"
    dst = SITE_DIR / "assets" / "favicon.png"
    if src.exists():
        img = Image.open(src)
        img = img.resize((32, 32), Image.NEAREST)
        img.save(dst, optimize=True)
        print(f"  [favicon] Generated 32x32 from pawdler_idle.png\n")
    else:
        print("  [favicon] MISSING: pawdler_idle.png — cannot generate favicon\n")


def main():
    print("=== ClawedCommand Asset Preparation ===\n")

    print("Copying unit sprites...")
    copy_units()

    print("Resizing building sprites (1024 → 256px)...")
    resize_buildings()

    print("Copying screenshots...")
    copy_screenshots()

    print("Generating favicon...")
    generate_favicon()

    print("=== Done ===")


if __name__ == "__main__":
    main()
