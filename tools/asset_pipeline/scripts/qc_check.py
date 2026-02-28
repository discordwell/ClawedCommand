#!/usr/bin/env python3
"""
Automated quality checks for processed game assets.

Checks:
  - Dimension verification (matches catalog spec)
  - Transparency (has alpha, no opaque background)
  - Content coverage (not too empty, not too full)
  - Palette adherence (how close to target palette)
  - Grid alignment (for sprite sheets)

Usage:
    python qc_check.py <asset_name>     # Check a specific asset
    python qc_check.py --all            # Check all game_ready assets
    python qc_check.py --category units # Check all units
"""

import argparse
import sys
from pathlib import Path

import numpy as np
import yaml
from PIL import Image

PIPELINE_ROOT = Path(__file__).resolve().parent.parent
CONFIG_DIR = PIPELINE_ROOT / "config"
CATALOG_PATH = CONFIG_DIR / "asset_catalog.yaml"
PALETTE_PATH = CONFIG_DIR / "palette.yaml"
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
QC_DIR = PIPELINE_ROOT / "qc"


def load_catalog():
    with open(CATALOG_PATH) as f:
        data = yaml.safe_load(f)
        return data if data else {}


def load_palette():
    with open(PALETTE_PATH) as f:
        return yaml.safe_load(f)


def hex_to_rgb(hex_str):
    hex_str = hex_str.lstrip("#")
    return tuple(int(hex_str[i:i + 2], 16) for i in (0, 2, 4))


def get_palette_colors():
    palette = load_palette()
    colors = []
    for category, entries in palette.items():
        if not isinstance(entries, dict):
            continue
        for name, hex_val in entries.items():
            if isinstance(hex_val, str) and hex_val.startswith("#"):
                colors.append(hex_to_rgb(hex_val))
    return colors


# ── Individual Checks ────────────────────────────────────────


def check_dimensions(img, entry):
    """Verify image dimensions match catalog spec."""
    output = entry.get("output", {})
    params = entry.get("params", {})
    asset_type = output.get("type", "single")

    if asset_type == "sheet":
        tile_size = output.get("tile_size", [128, 128])
        columns = output.get("columns", 4)
        rows = output.get("rows", 1)
        expected_w = tile_size[0] * columns
        expected_h = tile_size[1] * rows
    else:
        expected_w = params.get("width", 128)
        expected_h = params.get("height", 128)

    actual_w, actual_h = img.size
    passed = actual_w == expected_w and actual_h == expected_h

    return {
        "name": "dimensions",
        "passed": passed,
        "expected": f"{expected_w}x{expected_h}",
        "actual": f"{actual_w}x{actual_h}",
        "severity": "fail" if not passed else "pass",
    }


def check_transparency(img):
    """Verify image has meaningful transparency (not solid background)."""
    if img.mode != "RGBA":
        return {
            "name": "transparency",
            "passed": False,
            "detail": "Image has no alpha channel",
            "severity": "fail",
        }

    alpha = np.array(img.split()[-1])
    fully_opaque = np.count_nonzero(alpha == 255)
    fully_transparent = np.count_nonzero(alpha == 0)
    total = alpha.size

    opaque_pct = fully_opaque / total
    transparent_pct = fully_transparent / total

    if transparent_pct < 0.05:
        return {
            "name": "transparency",
            "passed": False,
            "detail": f"Only {transparent_pct:.1%} transparent — likely has opaque background",
            "severity": "warn",
        }

    if opaque_pct < 0.01:
        return {
            "name": "transparency",
            "passed": False,
            "detail": f"Only {opaque_pct:.1%} opaque — image may be nearly invisible",
            "severity": "warn",
        }

    return {
        "name": "transparency",
        "passed": True,
        "detail": f"{opaque_pct:.1%} opaque, {transparent_pct:.1%} transparent",
        "severity": "pass",
    }


def check_content_coverage(img):
    """Check that content coverage is reasonable (not too empty, not too full)."""
    alpha = np.array(img.split()[-1]) if img.mode == "RGBA" else np.full(img.size[::-1], 255)

    coverage = np.count_nonzero(alpha > 0) / alpha.size

    if coverage < 0.05:
        severity = "fail"
        detail = f"{coverage:.1%} — nearly empty"
    elif coverage < 0.15:
        severity = "warn"
        detail = f"{coverage:.1%} — very sparse"
    elif coverage > 0.95:
        severity = "warn"
        detail = f"{coverage:.1%} — very full (may need bg removal)"
    else:
        severity = "pass"
        detail = f"{coverage:.1%}"

    return {
        "name": "content_coverage",
        "passed": severity == "pass",
        "detail": detail,
        "severity": severity,
    }


def check_palette_adherence(img, tolerance=20.0):
    """Check how well image colors match the target palette."""
    palette_colors = get_palette_colors()
    if not palette_colors:
        return {
            "name": "palette_adherence",
            "passed": True,
            "detail": "No palette defined",
            "severity": "pass",
        }

    data = np.array(img)
    if img.mode == "RGBA":
        rgb = data[:, :, :3]
        alpha = data[:, :, 3]
        # Only check opaque pixels
        mask = alpha > 128
        pixels = rgb[mask]
    else:
        pixels = data.reshape(-1, 3)

    if len(pixels) == 0:
        return {
            "name": "palette_adherence",
            "passed": True,
            "detail": "No opaque pixels to check",
            "severity": "pass",
        }

    # Sample pixels for performance (check up to 5000)
    if len(pixels) > 5000:
        indices = np.random.RandomState(42).choice(len(pixels), 5000, replace=False)
        pixels = pixels[indices]

    palette_arr = np.array(palette_colors, dtype=np.float64)

    # Simple RGB distance (not perceptual, but fast)
    pixels_f = pixels.astype(np.float64)
    min_distances = np.full(len(pixels_f), float("inf"))

    for pc in palette_arr:
        dist = np.sqrt(np.sum((pixels_f - pc) ** 2, axis=1))
        min_distances = np.minimum(min_distances, dist)

    within_tolerance = np.count_nonzero(min_distances <= tolerance)
    adherence = within_tolerance / len(min_distances)

    if adherence < 0.3:
        severity = "warn"
    elif adherence < 0.6:
        severity = "info"
    else:
        severity = "pass"

    avg_dist = np.mean(min_distances)

    return {
        "name": "palette_adherence",
        "passed": adherence >= 0.3,
        "detail": f"{adherence:.1%} within tolerance, avg dist={avg_dist:.1f}",
        "severity": severity,
    }


def check_sheet_grid(img, entry):
    """For sprite sheets: check grid alignment and frame consistency."""
    output = entry.get("output", {})
    if output.get("type") != "sheet":
        return None  # Not a sheet, skip

    columns = output.get("columns", 4)
    rows = output.get("rows", 1)
    tile_size = output.get("tile_size", [128, 128])
    tile_w, tile_h = tile_size

    w, h = img.size
    issues = []

    # Check dimensions divide evenly
    if w % columns != 0:
        issues.append(f"Width {w} not divisible by {columns} columns")
    if h % rows != 0:
        issues.append(f"Height {h} not divisible by {rows} rows")

    # Check frame content
    alpha = np.array(img.split()[-1]) if img.mode == "RGBA" else np.full((h, w), 255)

    frame_w = w // columns
    frame_h = h // rows
    coverages = []

    for r in range(rows):
        for c in range(columns):
            y0 = r * frame_h
            x0 = c * frame_w
            frame_alpha = alpha[y0:y0 + frame_h, x0:x0 + frame_w]
            cov = np.count_nonzero(frame_alpha > 0) / frame_alpha.size
            coverages.append(cov)

    # Check for empty frames
    empty = [i for i, c in enumerate(coverages) if c < 0.01]
    if empty:
        issues.append(f"Empty frames: {empty}")

    # Check coverage consistency
    if coverages:
        avg = sum(coverages) / len(coverages)
        for i, cov in enumerate(coverages):
            if avg > 0 and abs(cov - avg) / max(avg, 0.01) > 0.5:
                issues.append(f"Frame {i} coverage ({cov:.1%}) differs significantly from avg ({avg:.1%})")

    if issues:
        return {
            "name": "sheet_grid",
            "passed": False,
            "detail": "; ".join(issues),
            "severity": "warn",
        }

    cov_str = ", ".join(f"{c:.0%}" for c in coverages)
    return {
        "name": "sheet_grid",
        "passed": True,
        "detail": f"All {columns * rows} frames OK. Coverage: [{cov_str}]",
        "severity": "pass",
    }


# ── Main Logic ───────────────────────────────────────────────


def run_checks(name, entry, category):
    """Run all applicable checks on an asset."""
    output = entry.get("output", {})
    game_path = output.get("game_path")

    if not game_path:
        return None

    full_path = PROJECT_ROOT / game_path
    if not full_path.exists():
        return {"name": name, "category": category, "error": f"File not found: {full_path}"}

    img = Image.open(full_path).convert("RGBA")

    results = {
        "name": name,
        "category": category,
        "path": str(game_path),
        "checks": [],
    }

    results["checks"].append(check_dimensions(img, entry))
    results["checks"].append(check_transparency(img))
    results["checks"].append(check_content_coverage(img))
    results["checks"].append(check_palette_adherence(img))

    grid_check = check_sheet_grid(img, entry)
    if grid_check:
        results["checks"].append(grid_check)

    return results


def print_results(results):
    """Pretty-print QC results."""
    if "error" in results:
        print(f"\n  {results['name']} ({results['category']})")
        print(f"    ERROR: {results['error']}")
        return False

    name = results["name"]
    category = results["category"]
    checks = results["checks"]

    all_passed = all(c["passed"] for c in checks)
    status_icon = "\033[32mPASS\033[0m" if all_passed else "\033[33mWARN\033[0m"

    print(f"\n  {name} ({category}) [{status_icon}]")

    for check in checks:
        sev = check["severity"]
        if sev == "pass":
            icon = "\033[32m+\033[0m"
        elif sev == "warn":
            icon = "\033[33m~\033[0m"
        elif sev == "info":
            icon = "\033[36m-\033[0m"
        else:
            icon = "\033[31mx\033[0m"

        detail = check.get("detail") or check.get("actual", "")
        if "expected" in check and not check["passed"]:
            detail = f"expected {check['expected']}, got {check['actual']}"

        print(f"    [{icon}] {check['name']}: {detail}")

    return all_passed


def main():
    parser = argparse.ArgumentParser(description="Asset quality checks")
    parser.add_argument("name", nargs="?", help="Asset name to check")
    parser.add_argument("--all", action="store_true", help="Check all game_ready assets")
    parser.add_argument("--category", type=str, help="Check all assets in a category")
    parser.add_argument("--include-planned", action="store_true",
                        help="Also check planned/generated assets (if files exist)")
    args = parser.parse_args()

    catalog = load_catalog()

    if not args.name and not args.all and not args.category:
        parser.print_help()
        sys.exit(1)

    assets_to_check = []

    if args.name:
        # Single asset
        for cat, entries in catalog.items():
            if not isinstance(entries, dict):
                continue
            if args.name in entries:
                assets_to_check.append((cat, args.name, entries[args.name]))
                break
        else:
            print(f"Error: asset '{args.name}' not found", file=sys.stderr)
            sys.exit(1)

    elif args.all or args.category:
        for cat, entries in catalog.items():
            if not isinstance(entries, dict):
                continue
            if args.category and cat != args.category:
                continue
            for name, entry in entries.items():
                status = entry.get("status", "planned")
                if status == "game_ready" or args.include_planned:
                    assets_to_check.append((cat, name, entry))

    if not assets_to_check:
        print("No assets to check. Use --include-planned to check non-ready assets.")
        sys.exit(0)

    print(f"QC Check — {len(assets_to_check)} asset(s)")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for cat, name, entry in assets_to_check:
        results = run_checks(name, entry, cat)
        if results is None:
            skip_count += 1
            continue
        if print_results(results):
            pass_count += 1
        else:
            fail_count += 1

    print(f"\n{'─' * 40}")
    print(f"Results: {pass_count} passed, {fail_count} warnings, {skip_count} skipped")

    sys.exit(0 if fail_count == 0 else 1)


if __name__ == "__main__":
    main()
