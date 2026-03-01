#!/usr/bin/env python3
"""
Generate walk/attack sprite sheets and projectile sprites from idle frames.

Walk sheets: 4-frame bounce animation (512x128, frames at 128x128)
Attack sheets: 4-frame strike animation (512x128, frames at 128x128)
Projectile sprites: Styled per-kind PNGs (32x32)

Usage:
    python generate_sheets.py --sprites-dir ../../assets/sprites
"""

import argparse
import math
from pathlib import Path

from PIL import Image, ImageDraw

# Unit slugs matching cc_client::renderer::unit_gen::ALL_KINDS order
UNIT_SLUGS = [
    "pawdler", "nuisance", "chonk", "flying_fox", "hisser",
    "yowler", "mouser", "catnapper", "ferret_sapper", "mech_commander",
]

FRAME_SIZE = 128
SHEET_FRAMES = 4
SHEET_WIDTH = FRAME_SIZE * SHEET_FRAMES  # 512

PROJECTILE_KINDS = {
    "spit":       {"color": (76, 230, 51),    "shape": "blob",    "size": 24},
    "laser_beam": {"color": (255, 51, 51),    "shape": "beam",    "size": 32},
    "sonic_wave": {"color": (179, 76, 255),   "shape": "ring",    "size": 28},
    "mech_shot":  {"color": (76, 230, 255),   "shape": "diamond", "size": 24},
    "explosive":  {"color": (255, 153, 25),   "shape": "bomb",    "size": 28},
    "generic":    {"color": (255, 230, 76),   "shape": "dot",     "size": 20},
}


def load_idle(sprites_dir: Path, slug: str) -> Image.Image:
    """Load a unit's idle sprite, resized to FRAME_SIZE, or return a placeholder."""
    path = sprites_dir / "units" / f"{slug}_idle.png"
    if path.exists():
        img = Image.open(path).convert("RGBA")
        if img.size != (FRAME_SIZE, FRAME_SIZE):
            img = img.resize((FRAME_SIZE, FRAME_SIZE), Image.Resampling.LANCZOS)
        return img
    # Placeholder: gray circle on transparent bg
    img = Image.new("RGBA", (FRAME_SIZE, FRAME_SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    cx, cy = FRAME_SIZE // 2, FRAME_SIZE // 2
    r = 40
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(180, 180, 180, 255),
                 outline=(40, 40, 40, 255), width=2)
    return img


def make_walk_sheet(idle: Image.Image) -> Image.Image:
    """Create a 4-frame walk sheet with bounce/bob animation."""
    sheet = Image.new("RGBA", (SHEET_WIDTH, FRAME_SIZE), (0, 0, 0, 0))

    # Bounce offsets (Y) and slight lean (rotation degrees)
    #   Frame 0: neutral
    #   Frame 1: up + slight right lean (mid-step)
    #   Frame 2: neutral
    #   Frame 3: up + slight left lean (other step)
    offsets = [
        (0,  0,  0.0),   # neutral
        (0, -6,  3.0),   # bounce up, lean right
        (0,  0,  0.0),   # neutral
        (0, -6, -3.0),   # bounce up, lean left
    ]

    for i, (dx, dy, angle) in enumerate(offsets):
        frame = idle.copy()
        if angle != 0.0:
            frame = frame.rotate(angle, resample=Image.Resampling.BICUBIC,
                                 expand=False, center=(FRAME_SIZE // 2, FRAME_SIZE // 2))
        canvas = Image.new("RGBA", (FRAME_SIZE, FRAME_SIZE), (0, 0, 0, 0))
        canvas.paste(frame, (dx, dy), frame)
        sheet.paste(canvas, (i * FRAME_SIZE, 0), canvas)

    return sheet


def make_attack_sheet(idle: Image.Image) -> Image.Image:
    """Create a 4-frame attack sheet with wind-up/strike animation."""
    sheet = Image.new("RGBA", (SHEET_WIDTH, FRAME_SIZE), (0, 0, 0, 0))

    # Attack frames:
    #   Frame 0: pull back (scale down slightly, shift back)
    #   Frame 1: wind-up (lean back)
    #   Frame 2: STRIKE (lunge forward + scale up)
    #   Frame 3: recovery (back to neutral)
    transforms = [
        {"dx": -3, "dy": 0, "scale": 0.95, "angle": -5.0},   # pull back
        {"dx": -5, "dy": -2, "scale": 0.93, "angle": -8.0},   # wind-up
        {"dx": 5,  "dy": 2,  "scale": 1.05, "angle": 6.0},    # STRIKE
        {"dx": 0,  "dy": 0,  "scale": 1.0,  "angle": 0.0},    # recovery
    ]

    for i, t in enumerate(transforms):
        frame = idle.copy()
        # Scale
        if t["scale"] != 1.0:
            new_size = int(FRAME_SIZE * t["scale"])
            frame = frame.resize((new_size, new_size), Image.Resampling.LANCZOS)
        # Rotate
        if t["angle"] != 0.0:
            frame = frame.rotate(t["angle"], resample=Image.Resampling.BICUBIC,
                                 expand=False)
        # Place on canvas with offset
        canvas = Image.new("RGBA", (FRAME_SIZE, FRAME_SIZE), (0, 0, 0, 0))
        ox = (FRAME_SIZE - frame.width) // 2 + t["dx"]
        oy = (FRAME_SIZE - frame.height) // 2 + t["dy"]
        canvas.paste(frame, (ox, oy), frame)
        sheet.paste(canvas, (i * FRAME_SIZE, 0), canvas)

    return sheet


def make_projectile_sprite(kind: str, props: dict) -> Image.Image:
    """Generate a styled projectile sprite."""
    size = props["size"]
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    r, g, b = props["color"]
    cx, cy = size // 2, size // 2

    shape = props["shape"]

    if shape == "blob":
        # Spit: gooey blob with glow
        for radius in range(size // 2, 2, -2):
            alpha = int(255 * (1.0 - radius / (size // 2)) ** 0.5)
            bright = 1.0 - (radius / (size // 2)) * 0.4
            cr = min(255, int(r * bright))
            cg = min(255, int(g * bright))
            cb = min(255, int(b * bright))
            draw.ellipse([cx - radius, cy - radius, cx + radius, cy + radius],
                         fill=(cr, cg, cb, alpha))
        # Core
        draw.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], fill=(r, g, b, 255))

    elif shape == "beam":
        # Laser: elongated rectangle with hot center
        beam_h = size // 4
        y0, y1 = cy - beam_h, cy + beam_h
        # Outer glow
        draw.rectangle([2, y0 - 2, size - 2, y1 + 2], fill=(r, g, b, 100))
        # Core
        draw.rectangle([4, y0, size - 4, y1], fill=(r, g, b, 220))
        # Hot center line
        draw.rectangle([4, cy - 1, size - 4, cy + 1], fill=(255, 255, 255, 200))

    elif shape == "ring":
        # Sonic wave: concentric rings
        for radius in [size // 2 - 2, size // 3, size // 5]:
            alpha = int(200 * radius / (size // 2))
            draw.ellipse([cx - radius, cy - radius, cx + radius, cy + radius],
                         outline=(r, g, b, alpha), width=2)
        draw.ellipse([cx - 2, cy - 2, cx + 2, cy + 2], fill=(r, g, b, 255))

    elif shape == "diamond":
        # Mech shot: diamond/rhombus
        points = [(cx, cy - size // 3), (cx + size // 3, cy),
                  (cx, cy + size // 3), (cx - size // 3, cy)]
        draw.polygon(points, fill=(r, g, b, 200), outline=(255, 255, 255, 150))
        # Inner diamond
        inner = [(cx, cy - size // 6), (cx + size // 6, cy),
                 (cx, cy + size // 6), (cx - size // 6, cy)]
        draw.polygon(inner, fill=(min(255, r + 60), min(255, g + 60), min(255, b + 60), 255))

    elif shape == "bomb":
        # Explosive: round with fuse spark
        # Body
        draw.ellipse([cx - 8, cy - 6, cx + 8, cy + 8], fill=(80, 60, 40, 255),
                     outline=(40, 30, 20, 255), width=2)
        # Highlight
        draw.ellipse([cx - 4, cy - 2, cx + 2, cy + 4], fill=(120, 90, 60, 180))
        # Fuse
        draw.line([(cx + 4, cy - 6), (cx + 8, cy - 10)], fill=(100, 80, 60, 255), width=2)
        # Spark
        for angle in range(0, 360, 45):
            sx = cx + 8 + int(4 * math.cos(math.radians(angle)))
            sy = cy - 10 + int(4 * math.sin(math.radians(angle)))
            draw.line([(cx + 8, cy - 10), (sx, sy)], fill=(r, g, b, 200), width=1)

    elif shape == "dot":
        # Generic: simple glowing dot
        for radius in range(size // 2, 1, -1):
            alpha = int(255 * (1.0 - (radius / (size // 2))) ** 0.7)
            draw.ellipse([cx - radius, cy - radius, cx + radius, cy + radius],
                         fill=(r, g, b, alpha))

    return img


def main():
    parser = argparse.ArgumentParser(description="Generate animation sheets and projectile sprites")
    parser.add_argument("--sprites-dir", type=Path, default=Path("../../assets/sprites"),
                        help="Path to sprites directory")
    parser.add_argument("--only-walk", action="store_true", help="Only generate walk sheets")
    parser.add_argument("--only-attack", action="store_true", help="Only generate attack sheets")
    parser.add_argument("--only-projectiles", action="store_true", help="Only generate projectiles")
    args = parser.parse_args()

    sprites_dir = args.sprites_dir.resolve()
    units_dir = sprites_dir / "units"
    proj_dir = sprites_dir / "projectiles"

    generate_all = not (args.only_walk or args.only_attack or args.only_projectiles)

    units_dir.mkdir(parents=True, exist_ok=True)

    # Walk sheets
    if generate_all or args.only_walk:
        print("=== Walk Sheets ===")
        for slug in UNIT_SLUGS:
            idle = load_idle(sprites_dir, slug)
            sheet = make_walk_sheet(idle)
            out = units_dir / f"{slug}_walk.png"
            sheet.save(out, "PNG")
            print(f"  {out.name}: {sheet.size[0]}x{sheet.size[1]}")

    # Attack sheets
    if generate_all or args.only_attack:
        print("=== Attack Sheets ===")
        for slug in UNIT_SLUGS:
            idle = load_idle(sprites_dir, slug)
            sheet = make_attack_sheet(idle)
            out = units_dir / f"{slug}_attack.png"
            sheet.save(out, "PNG")
            print(f"  {out.name}: {sheet.size[0]}x{sheet.size[1]}")

    # Projectile sprites
    if generate_all or args.only_projectiles:
        print("=== Projectile Sprites ===")
        proj_dir.mkdir(parents=True, exist_ok=True)
        for kind, props in PROJECTILE_KINDS.items():
            img = make_projectile_sprite(kind, props)
            out = proj_dir / f"{kind}.png"
            img.save(out, "PNG")
            print(f"  {out.name}: {img.size[0]}x{img.size[1]}")

    print("\nDone!")


if __name__ == "__main__":
    main()
