#!/usr/bin/env python3
"""Validate all idle sprites and portraits for quality issues.

Checks transparency (idle only), sharpness (Laplacian variance), and content.
Non-zero exit on any failure.

Usage: python3 validate_all_sprites.py
"""
import sys
from pathlib import Path

from PIL import Image
from image_utils import validate_sprite_quality

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
UNIT_DIR = PROJECT / "assets/sprites/units"
PORTRAIT_DIR = PROJECT / "assets/sprites/portraits"

# Cat faction slugs (generated separately, included for completeness)
CAT_SLUGS = {"pawdler", "nuisance", "chonk", "flying_fox", "hisser",
             "yowler", "mouser", "catnapper", "ferret_sapper", "mech_commander"}


def validate_idles() -> tuple[int, int, list[str]]:
    """Validate all *_idle.png sprites. Returns (passed, failed, failed_names)."""
    passed = 0
    failed_list = []
    for p in sorted(UNIT_DIR.glob("*_idle.png")):
        slug = p.stem.replace("_idle", "")
        img = Image.open(str(p)).convert("RGBA")
        ok, reason = validate_sprite_quality(img, sprite_type="idle")
        tag = "CAT " if slug in CAT_SLUGS else "    "
        if ok:
            print(f"  PASS {tag}{slug}: {reason}")
            passed += 1
        else:
            print(f"  FAIL {tag}{slug}: {reason}")
            failed_list.append(slug)
    return passed, len(failed_list), failed_list


def validate_portraits() -> tuple[int, int, list[str]]:
    """Validate all portrait PNGs. Returns (passed, failed, failed_names)."""
    passed = 0
    failed_list = []
    for p in sorted(PORTRAIT_DIR.glob("*.png")):
        slug = p.stem
        img = Image.open(str(p)).convert("RGBA")
        ok, reason = validate_sprite_quality(img, sprite_type="portrait")
        if ok:
            print(f"  PASS {slug}: {reason}")
            passed += 1
        else:
            print(f"  FAIL {slug}: {reason}")
            failed_list.append(slug)
    return passed, len(failed_list), failed_list


def main():
    total_pass = 0
    total_fail = 0
    all_failed = []

    print("=" * 60)
    print("  Idle Sprites")
    print("=" * 60)
    p, f, names = validate_idles()
    total_pass += p
    total_fail += f
    all_failed.extend(names)

    print(f"\n  Idle: {p} passed, {f} failed")

    print()
    print("=" * 60)
    print("  Portraits")
    print("=" * 60)
    if not PORTRAIT_DIR.exists():
        print("  (no portrait directory found)")
    else:
        p, f, names = validate_portraits()
        total_pass += p
        total_fail += f
        all_failed.extend(names)
        print(f"\n  Portraits: {p} passed, {f} failed")

    print()
    print("=" * 60)
    print(f"  TOTAL: {total_pass} passed, {total_fail} failed")
    print("=" * 60)

    if all_failed:
        print(f"\nFailed assets:")
        for name in all_failed:
            print(f"  - {name}")
        sys.exit(1)
    else:
        print("\nAll assets pass quality checks!")


if __name__ == "__main__":
    main()
