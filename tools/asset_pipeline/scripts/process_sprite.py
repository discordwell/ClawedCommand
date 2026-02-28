#!/usr/bin/env python3
"""
Process a single sprite: background removal, resize, crop to content.

Usage:
    python process_sprite.py <input> <output> --width 128 --height 128
"""

import argparse
import sys
from pathlib import Path

from PIL import Image

from image_utils import crop_to_content, remove_background, resize_to_fit


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

    if not args.no_bg_remove:
        img = remove_background(img)

    if not args.no_crop:
        img = crop_to_content(img, padding=args.padding)

    img = resize_to_fit(img, args.width, args.height)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(output_path, "PNG")
    print(f"  Output: {output_path} ({args.width}x{args.height})")


if __name__ == "__main__":
    main()
