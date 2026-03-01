#!/usr/bin/env python3
"""
Simple sprite sheet processor that doesn't depend on rembg.
Assumes ChatGPT output already has proper alpha channel.

Usage:
    python process_sheet_simple.py <input> <output> --columns 4 --rows 1 --tile-width 128 --tile-height 128
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def process_sheet(input_path: Path, output_path: Path, columns: int, rows: int,
                  tile_w: int, tile_h: int):
    """Process a sprite sheet: find content, split into grid, reassemble at target tile size."""
    img = Image.open(input_path).convert("RGBA")
    print(f"  Input: {img.size[0]}x{img.size[1]}")

    # Find content bounding box
    alpha = np.array(img.split()[-1])
    rows_mask = np.any(alpha > 10, axis=1)
    cols_mask = np.any(alpha > 10, axis=0)

    if not rows_mask.any():
        print("  Error: image is fully transparent", file=sys.stderr)
        return False

    rmin, rmax = np.where(rows_mask)[0][[0, -1]]
    cmin, cmax = np.where(cols_mask)[0][[0, -1]]
    content = img.crop((cmin, rmin, cmax + 1, rmax + 1))
    print(f"  Content: {content.size[0]}x{content.size[1]} (from ({cmin},{rmin}) to ({cmax},{rmax}))")

    # Split into grid
    frame_w = content.width // columns
    frame_h = content.height // rows
    print(f"  Frame size: {frame_w}x{frame_h}")

    # Reassemble at target size
    sheet = Image.new("RGBA", (tile_w * columns, tile_h * rows), (0, 0, 0, 0))

    for r in range(rows):
        for c in range(columns):
            idx = r * columns + c
            x0 = c * frame_w
            y0 = r * frame_h
            frame = content.crop((x0, y0, x0 + frame_w, y0 + frame_h))

            # Scale to fit tile
            scale = min(tile_w / frame.width, tile_h / frame.height)
            if scale < 1.0:
                new_w = max(1, int(frame.width * scale))
                new_h = max(1, int(frame.height * scale))
                frame = frame.resize((new_w, new_h), Image.Resampling.LANCZOS)

            # Center on tile
            ox = (tile_w - frame.width) // 2
            oy = (tile_h - frame.height) // 2
            sheet.paste(frame, (c * tile_w + ox, r * tile_h + oy), frame)
            print(f"  Frame {idx}: {frame.width}x{frame.height}")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output_path, "PNG")
    print(f"  Output: {output_path} ({sheet.size[0]}x{sheet.size[1]})")
    return True


def main():
    parser = argparse.ArgumentParser(description="Process sprite sheet (no rembg)")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("output", help="Output image path")
    parser.add_argument("--columns", type=int, default=4, help="Number of columns")
    parser.add_argument("--rows", type=int, default=1, help="Number of rows")
    parser.add_argument("--tile-width", type=int, default=128, help="Target tile width")
    parser.add_argument("--tile-height", type=int, default=128, help="Target tile height")
    args = parser.parse_args()

    success = process_sheet(
        Path(args.input), Path(args.output),
        args.columns, args.rows,
        args.tile_width, args.tile_height,
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
