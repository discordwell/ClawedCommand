#!/usr/bin/env python3
"""Clean a ChatGPT-generated sprite sheet and process to game-ready format.

Handles both real-transparent and baked-checkered-background images.
Extracts the first row of sprites (ignores shadows/effects rows below).

Usage:
    python clean_and_process.py <input> <output> [--columns 4] [--rows 1] [--tile-width 128] [--tile-height 128]
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def remove_checkered_bg(img: Image.Image, threshold_brightness=220, threshold_saturation=15) -> Image.Image:
    """Remove baked checkered transparency pattern from ChatGPT images."""
    arr = np.array(img)
    if arr[:, :, 3].min() < 200:
        # Image already has real transparency
        return img

    r, g, b = arr[:, :, 0].astype(float), arr[:, :, 1].astype(float), arr[:, :, 2].astype(float)
    brightness = (r + g + b) / 3
    max_rgb = np.maximum(np.maximum(r, g), b)
    min_rgb = np.minimum(np.minimum(r, g), b)
    saturation = max_rgb - min_rgb

    bg_mask = (brightness > threshold_brightness) & (saturation < threshold_saturation)
    arr[bg_mask, 3] = 0
    return Image.fromarray(arr)


def extract_first_row(img: Image.Image) -> Image.Image:
    """Extract just the first content row (ignoring shadow/effect rows below)."""
    alpha = np.array(img.split()[-1])
    rows_mask = np.any(alpha > 10, axis=1)

    in_content = False
    row_regions = []
    start = None
    gap_count = 0
    MIN_GAP = 20  # minimum empty rows to count as a separator

    for y in range(alpha.shape[0]):
        if rows_mask[y]:
            if not in_content:
                start = y
                in_content = True
            gap_count = 0
        else:
            gap_count += 1
            if in_content and gap_count >= MIN_GAP:
                row_regions.append((start, y - gap_count + 1))
                in_content = False

    if in_content:
        row_regions.append((start, alpha.shape[0]))

    if not row_regions:
        return img

    # Take first (largest) region
    first = max(row_regions, key=lambda r: r[1] - r[0])
    return img.crop((0, first[0], img.width, first[1]))


def process_to_sheet(img: Image.Image, columns: int, rows: int,
                     tile_w: int, tile_h: int) -> Image.Image:
    """Process image into a clean sprite sheet."""
    alpha = np.array(img.split()[-1])
    rows_mask = np.any(alpha > 10, axis=1)
    cols_mask = np.any(alpha > 10, axis=0)

    if not rows_mask.any():
        print("  Error: image is fully transparent", file=sys.stderr)
        return None

    rmin, rmax = np.where(rows_mask)[0][[0, -1]]
    cmin, cmax = np.where(cols_mask)[0][[0, -1]]
    content = img.crop((cmin, rmin, cmax + 1, rmax + 1))

    frame_w = content.width // columns
    frame_h = content.height // rows

    sheet = Image.new("RGBA", (tile_w * columns, tile_h * rows), (0, 0, 0, 0))

    for r in range(rows):
        for c in range(columns):
            x0 = c * frame_w
            y0 = r * frame_h
            frame = content.crop((x0, y0, x0 + frame_w, y0 + frame_h))

            scale = min(tile_w / frame.width, tile_h / frame.height)
            if scale < 1.0:
                new_w = max(1, int(frame.width * scale))
                new_h = max(1, int(frame.height * scale))
                frame = frame.resize((new_w, new_h), Image.Resampling.LANCZOS)

            ox = (tile_w - frame.width) // 2
            oy = (tile_h - frame.height) // 2
            sheet.paste(frame, (c * tile_w + ox, r * tile_h + oy), frame)

    return sheet


def main():
    parser = argparse.ArgumentParser(description="Clean and process ChatGPT sprite sheet")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("output", help="Output image path")
    parser.add_argument("--columns", type=int, default=4)
    parser.add_argument("--rows", type=int, default=1)
    parser.add_argument("--tile-width", type=int, default=128)
    parser.add_argument("--tile-height", type=int, default=128)
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    img = Image.open(input_path).convert("RGBA")
    print(f"  Input: {img.size[0]}x{img.size[1]}")

    # Step 1: Remove checkered background if needed
    img = remove_checkered_bg(img)
    transparent_pct = (np.array(img.split()[-1]) == 0).sum() / (img.size[0] * img.size[1])
    print(f"  Transparency: {transparent_pct:.1%}")

    # Step 2: Extract first sprite row
    img = extract_first_row(img)
    print(f"  After row extraction: {img.size[0]}x{img.size[1]}")

    # Step 3: Process into sheet
    sheet = process_to_sheet(img, args.columns, args.rows, args.tile_width, args.tile_height)
    if sheet is None:
        sys.exit(1)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output_path, "PNG")
    print(f"  Output: {output_path} ({sheet.size[0]}x{sheet.size[1]})")


if __name__ == "__main__":
    main()
