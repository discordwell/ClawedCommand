#!/usr/bin/env python3
"""
Process a sprite sheet: background removal, slice into frames, validate frame
consistency, reassemble into a clean grid.

Usage:
    python process_sheet.py <input> <output> --columns 4 --rows 1 --tile-width 128 --tile-height 128
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image

from image_utils import crop_to_content, remove_background, resize_to_fit


def detect_grid(img: Image.Image, columns: int, rows: int):
    """Detect frame boundaries by dividing the image into a grid."""
    frame_w = img.width // columns
    frame_h = img.height // rows

    frames = []
    for r in range(rows):
        for c in range(columns):
            x = c * frame_w
            y = r * frame_h
            frame = img.crop((x, y, x + frame_w, y + frame_h))
            frames.append(frame)

    return frames, frame_w, frame_h


def validate_frames(frames, tolerance=0.05):
    """Check that all frames have roughly similar content coverage."""
    coverages = []
    for i, frame in enumerate(frames):
        alpha = np.array(frame.split()[-1])
        coverage = np.count_nonzero(alpha > 0) / alpha.size
        coverages.append(coverage)

    if not coverages:
        return True, "No frames to validate"

    avg = sum(coverages) / len(coverages)
    issues = []
    for i, cov in enumerate(coverages):
        if cov < 0.01:
            issues.append(f"Frame {i}: nearly empty ({cov:.1%} coverage)")
        elif avg > 0 and abs(cov - avg) / max(avg, 0.01) > tolerance * 10:
            issues.append(f"Frame {i}: unusual coverage ({cov:.1%} vs avg {avg:.1%})")

    report_parts = [f"Frame {i}: {c:.1%}" for i, c in enumerate(coverages)]
    report = "Coverage: " + ", ".join(report_parts)

    if issues:
        return False, report + "\nIssues: " + "; ".join(issues)
    return True, report


def fit_to_tile(frame: Image.Image, tile_w: int, tile_h: int) -> Image.Image:
    """Fit a frame into exact tile dimensions, centering content."""
    alpha = np.array(frame.split()[-1])
    rows_mask = np.any(alpha > 0, axis=1)
    cols_mask = np.any(alpha > 0, axis=0)

    if not rows_mask.any():
        return Image.new("RGBA", (tile_w, tile_h), (0, 0, 0, 0))

    rmin, rmax = np.where(rows_mask)[0][[0, -1]]
    cmin, cmax = np.where(cols_mask)[0][[0, -1]]
    content = frame.crop((cmin, rmin, cmax + 1, rmax + 1))

    scale = min(tile_w / content.width, tile_h / content.height)
    if scale < 1.0:
        new_w = max(1, int(content.width * scale))
        new_h = max(1, int(content.height * scale))
        content = content.resize((new_w, new_h), Image.Resampling.LANCZOS)

    canvas = Image.new("RGBA", (tile_w, tile_h), (0, 0, 0, 0))
    offset_x = (tile_w - content.width) // 2
    offset_y = (tile_h - content.height) // 2
    canvas.paste(content, (offset_x, offset_y), content)
    return canvas


def reassemble_grid(frames, tile_w, tile_h, columns, rows):
    """Reassemble frames into a clean grid of exact tile dimensions.
    Returns (sheet, resized_frames) so individual frames can be saved at target size."""
    sheet_w = columns * tile_w
    sheet_h = rows * tile_h
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    resized_frames = []

    for i, frame in enumerate(frames):
        r = i // columns
        c = i % columns

        if frame.size != (tile_w, tile_h):
            frame = fit_to_tile(frame, tile_w, tile_h)

        resized_frames.append(frame)
        sheet.paste(frame, (c * tile_w, r * tile_h), frame)

    return sheet, resized_frames


def save_individual_frames(frames, output_path: Path, name_prefix: str):
    """Save individual frames alongside the sheet."""
    frames_dir = output_path.parent / f"{name_prefix}_frames"
    frames_dir.mkdir(exist_ok=True)
    for i, frame in enumerate(frames):
        frame_path = frames_dir / f"{name_prefix}_{i:02d}.png"
        frame.save(frame_path, "PNG")
    print(f"  Saved {len(frames)} individual frames to {frames_dir}")


def main():
    parser = argparse.ArgumentParser(description="Process a sprite sheet")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("output", help="Output image path")
    parser.add_argument("--columns", type=int, required=True, help="Number of columns in grid")
    parser.add_argument("--rows", type=int, required=True, help="Number of rows in grid")
    parser.add_argument("--tile-width", type=int, required=True, help="Target frame width")
    parser.add_argument("--tile-height", type=int, required=True, help="Target frame height")
    parser.add_argument("--no-bg-remove", action="store_true", help="Skip background removal")
    parser.add_argument("--no-save-frames", action="store_true",
                        help="Skip saving individual frames")
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.exists():
        print(f"Error: input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    img = Image.open(input_path).convert("RGBA")
    print(f"  Input: {img.size[0]}x{img.size[1]}")

    if not args.no_bg_remove:
        img = remove_background(img)

    print(f"  Slicing into {args.columns}x{args.rows} grid...")
    frames, detected_fw, detected_fh = detect_grid(img, args.columns, args.rows)
    print(f"  Detected frame size: {detected_fw}x{detected_fh}")

    valid, report = validate_frames(frames)
    print(f"  {report}")
    if not valid:
        print("  Warning: frame validation issues detected", file=sys.stderr)

    print(f"  Reassembling at {args.tile_width}x{args.tile_height} per frame...")
    sheet, resized_frames = reassemble_grid(frames, args.tile_width, args.tile_height, args.columns, args.rows)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output_path, "PNG")
    total_w = args.columns * args.tile_width
    total_h = args.rows * args.tile_height
    print(f"  Output: {output_path} ({total_w}x{total_h})")

    if not args.no_save_frames:
        name_prefix = output_path.stem
        save_individual_frames(resized_frames, output_path, name_prefix)


if __name__ == "__main__":
    main()
