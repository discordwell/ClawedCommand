#!/usr/bin/env python3
"""
Verify that a sprite sheet has proper grid alignment.

Checks:
- Image dimensions are exact multiples of expected tile size
- No content bleeds across grid boundaries
- All frames have non-trivial content
- Consistent content positioning across frames

Usage:
    python verify_grid.py <image> --columns 4 --rows 1
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def check_dimensions(img: Image.Image, columns: int, rows: int):
    """Check if image dimensions divide evenly into grid."""
    w, h = img.size
    frame_w = w / columns
    frame_h = h / rows

    issues = []
    if w % columns != 0:
        issues.append(f"Width {w} not evenly divisible by {columns} columns (frame_w={frame_w:.1f})")
    if h % rows != 0:
        issues.append(f"Height {h} not evenly divisible by {rows} rows (frame_h={frame_h:.1f})")

    return int(frame_w), int(frame_h), issues


def check_boundary_bleed(img: Image.Image, columns: int, rows: int, frame_w: int, frame_h: int):
    """Check for content bleeding across grid boundaries."""
    alpha = np.array(img.split()[-1])
    issues = []

    # Check vertical boundaries (column edges)
    for c in range(1, columns):
        x = c * frame_w
        # Check 2px strip on each side of boundary
        if x >= 2 and x + 2 <= alpha.shape[1]:
            left_strip = alpha[:, x - 2:x]
            right_strip = alpha[:, x:x + 2]
            left_content = np.count_nonzero(left_strip > 0)
            right_content = np.count_nonzero(right_strip > 0)
            if left_content > 0 and right_content > 0:
                # Content on both sides — might be bleeding
                # Only flag if content density is high (suggests actual sprite content, not noise)
                left_density = left_content / left_strip.size
                right_density = right_content / right_strip.size
                if left_density > 0.3 and right_density > 0.3:
                    issues.append(f"Possible bleed at column boundary x={x} (left={left_density:.0%}, right={right_density:.0%})")

    # Check horizontal boundaries (row edges)
    for r in range(1, rows):
        y = r * frame_h
        if y >= 2 and y + 2 <= alpha.shape[0]:
            top_strip = alpha[y - 2:y, :]
            bottom_strip = alpha[y:y + 2, :]
            top_content = np.count_nonzero(top_strip > 0)
            bottom_content = np.count_nonzero(bottom_strip > 0)
            if top_content > 0 and bottom_content > 0:
                top_density = top_content / top_strip.size
                bottom_density = bottom_content / bottom_strip.size
                if top_density > 0.3 and bottom_density > 0.3:
                    issues.append(f"Possible bleed at row boundary y={y} (top={top_density:.0%}, bottom={bottom_density:.0%})")

    return issues


def check_frame_content(img: Image.Image, columns: int, rows: int, frame_w: int, frame_h: int):
    """Check that each frame has meaningful content."""
    alpha = np.array(img.split()[-1])
    issues = []
    stats = []

    for r in range(rows):
        for c in range(columns):
            idx = r * columns + c
            y0 = r * frame_h
            x0 = c * frame_w
            frame_alpha = alpha[y0:y0 + frame_h, x0:x0 + frame_w]
            coverage = np.count_nonzero(frame_alpha > 0) / frame_alpha.size

            stats.append({"index": idx, "coverage": coverage})

            if coverage < 0.005:
                issues.append(f"Frame {idx} is nearly empty ({coverage:.2%} coverage)")

    return stats, issues


def check_position_consistency(img: Image.Image, columns: int, rows: int, frame_w: int, frame_h: int):
    """Check that content is positioned consistently across frames (center of mass)."""
    alpha = np.array(img.split()[-1])
    centers = []
    issues = []

    for r in range(rows):
        for c in range(columns):
            y0 = r * frame_h
            x0 = c * frame_w
            frame_alpha = alpha[y0:y0 + frame_h, x0:x0 + frame_w]

            if np.count_nonzero(frame_alpha > 0) < 10:
                centers.append(None)
                continue

            # Weighted center of mass
            ys, xs = np.where(frame_alpha > 0)
            cx = np.mean(xs) / frame_w
            cy = np.mean(ys) / frame_h
            centers.append((cx, cy))

    # Check deviation from mean center
    valid_centers = [c for c in centers if c is not None]
    if len(valid_centers) >= 2:
        avg_cx = sum(c[0] for c in valid_centers) / len(valid_centers)
        avg_cy = sum(c[1] for c in valid_centers) / len(valid_centers)

        for i, center in enumerate(centers):
            if center is None:
                continue
            dx = abs(center[0] - avg_cx)
            dy = abs(center[1] - avg_cy)
            if dx > 0.15 or dy > 0.15:
                issues.append(
                    f"Frame {i} center offset: ({center[0]:.2f}, {center[1]:.2f}) "
                    f"vs avg ({avg_cx:.2f}, {avg_cy:.2f})"
                )

    return issues


def main():
    parser = argparse.ArgumentParser(description="Verify sprite sheet grid alignment")
    parser.add_argument("image", help="Sprite sheet image path")
    parser.add_argument("--columns", type=int, required=True, help="Expected columns")
    parser.add_argument("--rows", type=int, required=True, help="Expected rows")
    args = parser.parse_args()

    img_path = Path(args.image)
    if not img_path.exists():
        print(f"Error: image not found: {img_path}", file=sys.stderr)
        sys.exit(1)

    img = Image.open(img_path).convert("RGBA")
    all_issues = []

    print(f"Verifying: {img_path} ({img.width}x{img.height})")
    print(f"Expected grid: {args.columns}x{args.rows}")

    # 1. Dimension check
    frame_w, frame_h, dim_issues = check_dimensions(img, args.columns, args.rows)
    all_issues.extend(dim_issues)
    print(f"Frame size: {frame_w}x{frame_h}")

    # 2. Boundary bleed
    bleed_issues = check_boundary_bleed(img, args.columns, args.rows, frame_w, frame_h)
    all_issues.extend(bleed_issues)

    # 3. Frame content
    stats, content_issues = check_frame_content(img, args.columns, args.rows, frame_w, frame_h)
    all_issues.extend(content_issues)
    for s in stats:
        print(f"  Frame {s['index']}: {s['coverage']:.1%} coverage")

    # 4. Position consistency
    pos_issues = check_position_consistency(img, args.columns, args.rows, frame_w, frame_h)
    all_issues.extend(pos_issues)

    # Report
    if all_issues:
        print(f"\n{len(all_issues)} issue(s) found:")
        for issue in all_issues:
            print(f"  ! {issue}")
        sys.exit(1)
    else:
        print("\nGrid verification passed.")
        sys.exit(0)


if __name__ == "__main__":
    main()
