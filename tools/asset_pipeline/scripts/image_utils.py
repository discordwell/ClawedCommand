"""Shared image processing utilities for the asset pipeline."""

import sys

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
    scale = min(target_w / img.width, target_h / img.height)

    if scale >= 1.0:
        new_w, new_h = img.width, img.height
    else:
        new_w = int(img.width * scale)
        new_h = int(img.height * scale)
        img = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
        print(f"  Resized to: {new_w}x{new_h}")

    canvas = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    offset_x = (target_w - new_w) // 2
    offset_y = (target_h - new_h) // 2
    canvas.paste(img, (offset_x, offset_y), img)
    return canvas
