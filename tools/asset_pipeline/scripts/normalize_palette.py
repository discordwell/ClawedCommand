#!/usr/bin/env python3
"""
Normalize sprite colors to the target palette defined in palette.yaml.

Maps each non-transparent pixel's color to the nearest palette color using
delta-E (CIE76) perceptual distance. Only remaps if the distance is within
the configured tolerance.

Usage:
    python normalize_palette.py <input> <output> [--tolerance 15.0] [--preview]
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image
import yaml

PIPELINE_ROOT = Path(__file__).resolve().parent.parent
PALETTE_PATH = PIPELINE_ROOT / "config" / "palette.yaml"


def hex_to_rgb(hex_str: str) -> tuple:
    hex_str = hex_str.lstrip("#")
    return tuple(int(hex_str[i:i + 2], 16) for i in (0, 2, 4))


def rgb_to_lab(rgb: np.ndarray) -> np.ndarray:
    """Convert RGB (0-255) to CIE-LAB. Simplified conversion for color distance."""
    # Normalize to 0-1
    rgb_norm = rgb.astype(np.float64) / 255.0

    # sRGB to linear
    mask = rgb_norm > 0.04045
    rgb_lin = np.where(mask, ((rgb_norm + 0.055) / 1.055) ** 2.4, rgb_norm / 12.92)

    # Linear RGB to XYZ (D65)
    r, g, b = rgb_lin[..., 0], rgb_lin[..., 1], rgb_lin[..., 2]
    x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375
    y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750
    z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041

    # XYZ to LAB (D65 white point)
    xn, yn, zn = 0.95047, 1.0, 1.08883
    x, y, z = x / xn, y / yn, z / zn

    epsilon = 0.008856
    kappa = 903.3
    fx = np.where(x > epsilon, np.cbrt(x), (kappa * x + 16) / 116)
    fy = np.where(y > epsilon, np.cbrt(y), (kappa * y + 16) / 116)
    fz = np.where(z > epsilon, np.cbrt(z), (kappa * z + 16) / 116)

    L = 116 * fy - 16
    a = 500 * (fx - fy)
    b_val = 200 * (fy - fz)

    return np.stack([L, a, b_val], axis=-1)


def delta_e_cie76(lab1: np.ndarray, lab2: np.ndarray) -> np.ndarray:
    """CIE76 color difference (Euclidean distance in LAB space)."""
    return np.sqrt(np.sum((lab1 - lab2) ** 2, axis=-1))


def load_palette():
    """Load palette.yaml and extract all colors as a flat list of (name, rgb)."""
    with open(PALETTE_PATH) as f:
        palette = yaml.safe_load(f)

    colors = []
    tolerance = palette.pop("match_tolerance", 15.0) if "match_tolerance" in palette else 15.0

    for category, entries in palette.items():
        if not isinstance(entries, dict):
            continue
        for name, hex_val in entries.items():
            if isinstance(hex_val, str) and hex_val.startswith("#"):
                colors.append((f"{category}.{name}", hex_to_rgb(hex_val)))

    return colors, tolerance


def normalize_image(img: Image.Image, palette_colors, tolerance: float):
    """Remap image colors to nearest palette color within tolerance."""
    data = np.array(img)
    rgb = data[:, :, :3]
    alpha = data[:, :, 3]

    # Build palette array
    palette_rgb = np.array([c[1] for c in palette_colors], dtype=np.float64)
    palette_lab = rgb_to_lab(palette_rgb)

    # Convert image to LAB
    img_lab = rgb_to_lab(rgb)

    # For each pixel, find nearest palette color
    remapped = 0
    skipped = 0
    result = rgb.copy()

    # Flatten for vectorized distance computation
    h, w = rgb.shape[:2]
    flat_lab = img_lab.reshape(-1, 3)
    flat_alpha = alpha.reshape(-1)
    flat_result = result.reshape(-1, 3)

    # Only process non-transparent pixels
    opaque_mask = flat_alpha > 0
    opaque_indices = np.where(opaque_mask)[0]

    if len(opaque_indices) == 0:
        return img, 0, 0

    opaque_lab = flat_lab[opaque_indices]

    # Compute distances to all palette colors (N_pixels x N_palette)
    distances = np.zeros((len(opaque_lab), len(palette_lab)))
    for i, p_lab in enumerate(palette_lab):
        distances[:, i] = delta_e_cie76(opaque_lab, p_lab)

    # Find nearest
    nearest_idx = np.argmin(distances, axis=1)
    nearest_dist = distances[np.arange(len(nearest_idx)), nearest_idx]

    # Apply remapping where within tolerance
    within = nearest_dist <= tolerance
    for i, (idx, is_within) in enumerate(zip(nearest_idx, within)):
        if is_within:
            flat_result[opaque_indices[i]] = palette_rgb[idx].astype(np.uint8)
            remapped += 1
        else:
            skipped += 1

    result = flat_result.reshape(h, w, 3)
    out = np.dstack([result, alpha])
    return Image.fromarray(out.astype(np.uint8), "RGBA"), remapped, skipped


def main():
    parser = argparse.ArgumentParser(description="Normalize sprite colors to target palette")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("output", help="Output image path")
    parser.add_argument("--tolerance", type=float, default=None,
                        help="Override palette match tolerance (delta-E)")
    parser.add_argument("--preview", action="store_true",
                        help="Show before/after side-by-side (requires display)")
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.exists():
        print(f"Error: input not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    palette_colors, default_tolerance = load_palette()
    tolerance = args.tolerance if args.tolerance is not None else default_tolerance

    print(f"Palette: {len(palette_colors)} colors, tolerance: {tolerance}")

    img = Image.open(input_path).convert("RGBA")
    print(f"Input: {img.size[0]}x{img.size[1]}")

    result, remapped, skipped = normalize_image(img, palette_colors, tolerance)

    total = remapped + skipped
    print(f"Remapped: {remapped}/{total} pixels ({remapped/max(total,1):.1%})")
    print(f"Outside tolerance: {skipped}/{total} pixels")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    result.save(output_path, "PNG")
    print(f"Output: {output_path}")

    if args.preview:
        try:
            side_by_side = Image.new("RGBA", (img.width * 2 + 4, img.height), (40, 40, 40, 255))
            side_by_side.paste(img, (0, 0))
            side_by_side.paste(result, (img.width + 4, 0))
            side_by_side.show()
        except Exception as e:
            print(f"Preview unavailable: {e}", file=sys.stderr)


if __name__ == "__main__":
    main()
