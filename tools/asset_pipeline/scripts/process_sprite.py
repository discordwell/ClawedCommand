#!/usr/bin/env python3
"""
Process a single sprite: background removal, resize, crop to content.

Usage:
    python process_sprite.py <input> <output> --width 128 --height 128
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image

try:
    from rembg import remove as rembg_remove
    HAS_REMBG = True
except ImportError:
    HAS_REMBG = False


def remove_background(img: Image.Image) -> Image.Image:
    """Remove background using rembg, or skip if already has alpha."""
    if img.mode == "RGBA":
        # Check if alpha channel is already meaningful (not all 255)
        alpha = np.array(img.split()[-1])
        if alpha.min() < 250:
            print("  Alpha channel detected, skipping bg removal")
            return img

    if not HAS_REMBG:
        print("  Warning: rembg not installed, skipping bg removal", file=sys.stderr)
        return img.convert("RGBA")

    print("  Removing background with rembg...")
    return rembg_remove(img).convert("RGBA")


def crop_to_content(img: Image.Image, padding: int = 2) -> Image.Image:
    """Crop to bounding box of non-transparent pixels with optional padding."""
    alpha = np.array(img.split()[-1])
    rows = np.any(alpha > 0, axis=1)
    cols = np.any(alpha > 0, axis=0)

    if not rows.any() or not cols.any():
        print("  Warning: image is fully transparent after bg removal", file=sys.stderr)
        return img

    rmin, rmax = np.where(rows)[0][[0, -1]]
    cmin, cmax = np.where(cols)[0][[0, -1]]

    # Add padding
    rmin = max(0, rmin - padding)
    rmax = min(img.height - 1, rmax + padding)
    cmin = max(0, cmin - padding)
    cmax = min(img.width - 1, cmax + padding)

    cropped = img.crop((cmin, rmin, cmax + 1, rmax + 1))
    print(f"  Cropped: {img.size} → {cropped.size}")
    return cropped


def resize_to_fit(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """Resize image to fit within target dimensions, preserving aspect ratio.
    Centers the result on a transparent canvas of exact target size."""
    # Calculate scale to fit within target
    scale = min(target_w / img.width, target_h / img.height)

    if scale >= 1.0:
        # Image already fits — just center it
        new_w, new_h = img.width, img.height
    else:
        new_w = int(img.width * scale)
        new_h = int(img.height * scale)
        img = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
        print(f"  Resized to: {new_w}x{new_h}")

    # Center on target canvas
    canvas = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    offset_x = (target_w - new_w) // 2
    offset_y = (target_h - new_h) // 2
    canvas.paste(img, (offset_x, offset_y), img)
    return canvas


def main():
    parser = argparse.ArgumentParser(description="Process a single sprite")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("output", help="Output image path")
    parser.add_argument("--width", type=int, default=128, help="Target width")
    parser.add_argument("--height", type=int, default=128, help="Target height")
    parser.add_argument("--no-bg-remove", action="store_true", help="Skip background removal")
    parser.add_argument("--no-crop", action="store_true", help="Skip crop to content")
    parser.add_argument("--padding", type=int, default=2, help="Padding around content after crop")
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.exists():
        print(f"Error: input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    img = Image.open(input_path).convert("RGBA")
    print(f"  Input: {img.size[0]}x{img.size[1]}")

    # Step 1: Background removal
    if not args.no_bg_remove:
        img = remove_background(img)

    # Step 2: Crop to content
    if not args.no_crop:
        img = crop_to_content(img, padding=args.padding)

    # Step 3: Resize to target
    img = resize_to_fit(img, args.width, args.height)

    # Save
    output_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(output_path, "PNG")
    print(f"  Output: {output_path} ({args.width}x{args.height})")


if __name__ == "__main__":
    main()
