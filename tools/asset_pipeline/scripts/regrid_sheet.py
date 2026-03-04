#!/usr/bin/env python3
"""Re-grid a sprite sheet so each sprite is centered in its 128x128 cell.

ChatGPT generates 4 sprites at arbitrary positions. This script finds the
4 sprite blobs via connected-component analysis and recenters each into
a proper 128x128 grid cell, producing a clean 512x128 sheet.

Usage: python3 regrid_sheet.py <input.png> [output.png]
  If output is omitted, overwrites input in-place.

Batch: python3 regrid_sheet.py --batch
  Re-grids all *_walk.png and *_attack.png in assets/sprites/units/
"""
import sys
from pathlib import Path
from PIL import Image
import numpy as np


def find_sprite_blobs(img: Image.Image, min_area: int = 200) -> list[tuple[int, int, int, int]]:
    """Find connected non-transparent regions (sprites) via flood fill.

    Returns list of (x0, y0, x1, y1) bounding boxes sorted left-to-right.
    """
    arr = np.array(img)
    alpha = arr[:, :, 3]
    mask = alpha > 10  # Non-transparent pixels
    visited = np.zeros_like(mask, dtype=bool)

    blobs = []
    h, w = mask.shape

    for start_y in range(h):
        for start_x in range(w):
            if mask[start_y, start_x] and not visited[start_y, start_x]:
                # BFS flood fill
                queue = [(start_x, start_y)]
                visited[start_y, start_x] = True
                min_x, min_y = start_x, start_y
                max_x, max_y = start_x, start_y
                area = 0

                while queue:
                    cx, cy = queue.pop(0)
                    area += 1
                    min_x = min(min_x, cx)
                    min_y = min(min_y, cy)
                    max_x = max(max_x, cx)
                    max_y = max(max_y, cy)

                    for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                        nx, ny = cx + dx, cy + dy
                        if 0 <= nx < w and 0 <= ny < h and mask[ny, nx] and not visited[ny, nx]:
                            visited[ny, nx] = True
                            queue.append((nx, ny))

                if area >= min_area:
                    blobs.append((min_x, min_y, max_x + 1, max_y + 1))

    # Sort left-to-right by center x
    blobs.sort(key=lambda b: (b[0] + b[2]) / 2)
    return blobs


def merge_nearby_blobs(blobs: list[tuple[int, int, int, int]],
                       max_gap: int = 8) -> list[tuple[int, int, int, int]]:
    """Merge blobs that are close together (e.g. disconnected parts of same sprite)."""
    if not blobs:
        return blobs

    merged = [blobs[0]]
    for b in blobs[1:]:
        prev = merged[-1]
        # Check if this blob's left edge is close to previous blob's right edge
        gap = b[0] - prev[2]
        if gap < max_gap:
            # Merge
            merged[-1] = (
                min(prev[0], b[0]),
                min(prev[1], b[1]),
                max(prev[2], b[2]),
                max(prev[3], b[3]),
            )
        else:
            merged.append(b)

    return merged


def regrid_column(img: Image.Image, cell_w: int = 128, cell_h: int = 128,
                  n_frames: int = 4) -> Image.Image | None:
    """Column-based fallback: slice sheet into n equal-width zones, recenter content.

    Works for attack sheets with detached effects (projectiles, slashes, sonic waves)
    that confuse blob detection. Each zone's non-transparent content gets centered
    in its target cell.
    """
    arr = np.array(img)
    alpha = arr[:, :, 3]
    w = img.size[0]
    zone_w = w // n_frames

    sheet = Image.new("RGBA", (cell_w * n_frames, cell_h), (0, 0, 0, 0))
    has_content = False

    for i in range(n_frames):
        zone = img.crop((i * zone_w, 0, (i + 1) * zone_w, cell_h))
        bbox = zone.getbbox()
        if not bbox:
            continue
        has_content = True
        sprite = zone.crop(bbox)
        sw, sh = sprite.size

        if sw > cell_w or sh > cell_h:
            sprite.thumbnail((cell_w, cell_h), Image.LANCZOS)
            sw, sh = sprite.size

        cx = i * cell_w + (cell_w - sw) // 2
        cy = (cell_h - sh) // 2
        sheet.paste(sprite, (cx, cy))

    return sheet if has_content else None


def regrid_sheet(img: Image.Image, cell_w: int = 128, cell_h: int = 128,
                 n_frames: int = 4) -> Image.Image | None:
    """Find sprites and recenter each in its grid cell.

    Returns new 512x128 image, or None if can't find exactly 4 sprites.
    Falls back to column-based splitting if blob detection fails.
    """
    blobs = find_sprite_blobs(img)

    # Merge nearby blobs (disconnected parts of same sprite)
    blobs = merge_nearby_blobs(blobs, max_gap=8)

    # If we still don't have 4, try merging more aggressively
    if len(blobs) != n_frames:
        blobs_orig = find_sprite_blobs(img)
        blobs = merge_nearby_blobs(blobs_orig, max_gap=20)

    if len(blobs) != n_frames:
        # Fallback: column-based splitting for sheets with detached effects
        return regrid_column(img, cell_w, cell_h, n_frames)

    # Create new sheet
    sheet = Image.new("RGBA", (cell_w * n_frames, cell_h), (0, 0, 0, 0))

    for i, (x0, y0, x1, y1) in enumerate(blobs):
        sprite = img.crop((x0, y0, x1, y1))
        sw, sh = sprite.size

        # Scale down if sprite is larger than cell
        if sw > cell_w or sh > cell_h:
            sprite.thumbnail((cell_w, cell_h), Image.LANCZOS)
            sw, sh = sprite.size

        # Center in cell
        cx = i * cell_w + (cell_w - sw) // 2
        cy = (cell_h - sh) // 2
        sheet.paste(sprite, (cx, cy))

    return sheet


def process_file(path: Path, out_path: Path | None = None) -> bool:
    """Process a single sheet file."""
    img = Image.open(str(path)).convert("RGBA")

    if img.size[0] != 512 or img.size[1] != 128:
        print(f"  SKIP {path.name}: unexpected size {img.size}")
        return False

    # Check if already properly gridded (each 128x128 cell has centered content)
    needs_regrid = False
    for i in range(4):
        frame = img.crop((i * 128, 0, (i + 1) * 128, 128))
        bbox = frame.getbbox()
        if not bbox:
            needs_regrid = True
            break
        # Check if content spills to both edges (likely cross-frame bleed)
        if bbox[0] == 0 and bbox[2] == 128:
            needs_regrid = True
            break

    if not needs_regrid:
        # Double check: are there visible gaps between frames?
        blobs = find_sprite_blobs(img)
        blobs = merge_nearby_blobs(blobs, max_gap=8)
        if len(blobs) == 4:
            # Check if blobs align with grid
            for i, (x0, y0, x1, y1) in enumerate(blobs):
                cell_start = i * 128
                cell_end = (i + 1) * 128
                if x0 < cell_start - 2 or x1 > cell_end + 2:
                    needs_regrid = True
                    break

    if not needs_regrid:
        print(f"  OK   {path.name}: already properly gridded")
        return True

    result = regrid_sheet(img)
    if result is None:
        print(f"  FAIL {path.name}: no content found")
        return False

    target = out_path or path
    result.save(str(target))
    print(f"  FIXED {path.name}")
    return True


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--batch":
        units_dir = Path("/Users/discordwell/Projects/ClawedCommand/assets/sprites/units")
        ok, fixed, failed = 0, 0, 0
        for suffix in ["_walk.png", "_attack.png"]:
            for f in sorted(units_dir.glob(f"*{suffix}")):
                result = process_file(f)
                if result:
                    if "FIXED" in str(result):
                        fixed += 1
                    else:
                        ok += 1
                else:
                    failed += 1
        print(f"\nDone: {ok} OK, {fixed} fixed, {failed} failed")
    elif len(sys.argv) >= 2:
        path = Path(sys.argv[1])
        out = Path(sys.argv[2]) if len(sys.argv) > 2 else None
        process_file(path, out)
    else:
        print("Usage: python3 regrid_sheet.py <input.png> [output.png]")
        print("       python3 regrid_sheet.py --batch")


if __name__ == "__main__":
    main()
