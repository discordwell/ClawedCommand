#!/usr/bin/env python3
"""
Generate an isometric map preview using available terrain tiles.

Renders a sample map showing all game_ready terrain tiles arranged in a
proper 2:1 isometric grid. Useful for checking tile consistency, seaming,
and overall visual feel.

Usage:
    python map_preview.py                          # Default 12x12 map with all available tiles
    python map_preview.py --size 8                 # 8x8 map
    python map_preview.py --tiles grass_base,water_base  # Only specific tiles
"""

import argparse
import random
import sys
from pathlib import Path

import yaml
from PIL import Image, ImageDraw, ImageFont

PIPELINE_ROOT = Path(__file__).resolve().parent.parent
CONFIG_DIR = PIPELINE_ROOT / "config"
CATALOG_PATH = CONFIG_DIR / "asset_catalog.yaml"
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
QC_DIR = PIPELINE_ROOT / "qc"

# Isometric projection constants (matching the game's 2:1 ratio)
TILE_W = 128
TILE_H = 64  # Diamond is 128x64 (2:1), but full tile image is 128x128 (includes height zone)
TILE_IMG_H = 128  # Full tile image height (diamond + height zone)

BG_COLOR = (25, 28, 32, 255)
LABEL_COLOR = (200, 205, 210, 255)
SUBLABEL_COLOR = (120, 130, 140, 255)


def load_catalog():
    with open(CATALOG_PATH) as f:
        data = yaml.safe_load(f)
        return data if data else {}


def get_terrain_tiles(catalog, filter_tiles=None):
    """Get all terrain tiles and their game paths, filtered by status."""
    terrain = catalog.get("terrain", {})
    tiles = {}

    for name, entry in terrain.items():
        status = entry.get("status", "planned")
        game_path = entry.get("output", {}).get("game_path")
        if not game_path:
            continue

        full_path = PROJECT_ROOT / game_path
        if filter_tiles and name not in filter_tiles:
            continue

        tiles[name] = {
            "status": status,
            "game_path": game_path,
            "full_path": full_path,
            "exists": full_path.exists(),
            "tags": entry.get("tags", []),
        }

    return tiles


def load_tile_image(tile_info):
    """Load a terrain tile image."""
    if not tile_info["exists"]:
        return None
    try:
        return Image.open(tile_info["full_path"]).convert("RGBA")
    except Exception:
        return None


def create_placeholder_tile(label, color):
    """Create a placeholder diamond tile with a label."""
    img = Image.new("RGBA", (TILE_W, TILE_IMG_H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Diamond shape in the bottom half (rows 64-128)
    cy = TILE_IMG_H - TILE_H // 2  # center of diamond = row 96
    points = [
        (TILE_W // 2, cy - TILE_H // 2),   # top
        (TILE_W, cy),                        # right
        (TILE_W // 2, cy + TILE_H // 2),    # bottom
        (0, cy),                             # left
    ]
    draw.polygon(points, fill=color, outline=(60, 65, 70, 255))

    # Label
    try:
        font = ImageFont.load_default()
        bbox = draw.textbbox((0, 0), label, font=font)
        tw = bbox[2] - bbox[0]
        draw.text((TILE_W // 2 - tw // 2, cy - 5), label, fill=(180, 180, 180, 255), font=font)
    except Exception:
        pass

    return img


def iso_to_screen(grid_x, grid_y):
    """Convert grid coordinates to screen pixel coordinates (top-left of tile)."""
    screen_x = (grid_x - grid_y) * (TILE_W // 2)
    screen_y = (grid_x + grid_y) * (TILE_H // 2)
    return screen_x, screen_y


def generate_map_layout(size, available_tiles):
    """Generate a simple map layout using available tiles."""
    layout = []
    tile_names = list(available_tiles.keys())

    if not tile_names:
        return layout

    # Separate tiles by type for a more natural-looking map
    passable = [n for n in tile_names if "impassable" not in str(available_tiles[n].get("tags", []))]
    water_tiles = [n for n in tile_names if "liquid" in str(available_tiles[n].get("tags", []))]
    land_tiles = [n for n in tile_names if n not in water_tiles]

    if not land_tiles:
        land_tiles = tile_names
    if not water_tiles:
        water_tiles = land_tiles

    random.seed(42)  # Deterministic layout

    for y in range(size):
        row = []
        for x in range(size):
            # Simple terrain distribution: mostly grass, some variety
            # Water on edges, special tiles scattered
            dist_from_edge = min(x, y, size - 1 - x, size - 1 - y)

            if dist_from_edge == 0 and water_tiles:
                # Edge tiles — water
                tile = random.choice(water_tiles)
            elif dist_from_edge == 1 and len(tile_names) > 2:
                # Near-edge — transition tiles (sand, shallows)
                transition = [n for n in tile_names if any(t in str(available_tiles[n].get("tags", []))
                              for t in ["coastal", "slow"])]
                tile = random.choice(transition) if transition else random.choice(land_tiles)
            else:
                # Interior — weighted random from land tiles
                # Favor the first tile (usually grass) heavily
                weights = [4 if i == 0 else 1 for i in range(len(land_tiles))]
                tile = random.choices(land_tiles, weights=weights, k=1)[0]

            row.append(tile)
        layout.append(row)

    return layout


def generate_map_preview(catalog, map_size=12, filter_tiles=None):
    """Generate an isometric map preview image."""
    tiles = get_terrain_tiles(catalog, filter_tiles)

    if not tiles:
        print("  No terrain tiles found in catalog")
        return None

    # Separate available (has image) from planned
    available = {k: v for k, v in tiles.items() if v["exists"] and v["status"] == "game_ready"}
    planned = {k: v for k, v in tiles.items() if not v["exists"] or v["status"] != "game_ready"}

    # Load tile images
    tile_images = {}
    for name, info in available.items():
        img = load_tile_image(info)
        if img:
            tile_images[name] = img

    # Create placeholders for planned tiles
    placeholder_colors = {
        "grass": (90, 160, 70, 200),
        "dirt": (160, 130, 80, 200),
        "sand": (200, 180, 130, 200),
        "water": (60, 120, 180, 200),
        "stone": (110, 110, 110, 200),
        "rock": (100, 100, 100, 200),
        "forest": (60, 100, 40, 200),
        "road": (130, 115, 90, 200),
        "shallows": (80, 150, 200, 200),
        "ramp": (140, 120, 90, 200),
        "tech": (50, 100, 100, 200),
    }

    for name, info in planned.items():
        # Pick color based on name
        color = (80, 80, 80, 200)
        for key, c in placeholder_colors.items():
            if key in name:
                color = c
                break
        tile_images[name] = create_placeholder_tile(name[:8], color)

    all_tile_names = list(tile_images.keys())
    if not all_tile_names:
        print("  No tile images (real or placeholder) available")
        return None

    # Generate layout
    layout = generate_map_layout(map_size, tiles)
    if not layout:
        # Fallback: show a grid of each unique tile
        n = len(all_tile_names)
        side = int(n ** 0.5) + 1
        layout = []
        idx = 0
        for y in range(side):
            row = []
            for x in range(side):
                if idx < n:
                    row.append(all_tile_names[idx])
                    idx += 1
                else:
                    row.append(all_tile_names[0])
            layout.append(row)
        map_size = side

    # Calculate canvas size
    # Isometric grid: screen coords range from iso_to_screen(0,size-1) to iso_to_screen(size-1,0)
    # Width: from leftmost tile to rightmost tile
    min_sx = iso_to_screen(0, map_size - 1)[0]
    max_sx = iso_to_screen(map_size - 1, 0)[0] + TILE_W
    min_sy = iso_to_screen(0, 0)[1]
    max_sy = iso_to_screen(map_size - 1, map_size - 1)[1] + TILE_IMG_H

    margin = 60
    canvas_w = max_sx - min_sx + margin * 2
    canvas_h = max_sy - min_sy + margin * 2 + 40  # Extra for title + legend
    offset_x = -min_sx + margin
    offset_y = -min_sy + margin + 30

    canvas = Image.new("RGBA", (canvas_w, canvas_h), BG_COLOR)
    draw = ImageDraw.Draw(canvas)

    # Title
    try:
        font_paths = [
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
        font = None
        for fp in font_paths:
            if Path(fp).exists():
                try:
                    font = ImageFont.truetype(fp, 16)
                    break
                except Exception:
                    continue
        if font is None:
            font = ImageFont.load_default()
    except Exception:
        font = ImageFont.load_default()

    real_count = len(available)
    total_count = len(tiles)
    title = f"Map Preview — {real_count}/{total_count} terrain tiles ready ({map_size}x{map_size})"
    draw.text((margin, 8), title, fill=LABEL_COLOR, font=font)

    # Render tiles (back to front for correct depth sorting)
    for y in range(map_size):
        for x in range(map_size):
            tile_name = layout[y][x]
            tile_img = tile_images.get(tile_name)
            if tile_img is None:
                continue

            sx, sy = iso_to_screen(x, y)
            px = sx + offset_x
            py = sy + offset_y

            # Resize tile if needed
            if tile_img.size != (TILE_W, TILE_IMG_H):
                tile_img = tile_img.resize((TILE_W, TILE_IMG_H), Image.Resampling.LANCZOS)

            canvas.paste(tile_img, (px, py), tile_img)

    # Legend at bottom
    legend_y = canvas_h - 25
    font_small = ImageFont.load_default()
    legend_items = []
    if available:
        legend_items.append(f"Real: {', '.join(sorted(available.keys())[:6])}")
    if planned:
        legend_items.append(f"Placeholder: {', '.join(sorted(planned.keys())[:6])}")
    legend_text = " | ".join(legend_items)
    draw.text((margin, legend_y), legend_text, fill=SUBLABEL_COLOR, font=font_small)

    return canvas


def main():
    parser = argparse.ArgumentParser(description="Generate isometric map preview")
    parser.add_argument("--size", type=int, default=12, help="Map grid size (default: 12)")
    parser.add_argument("--tiles", type=str, default=None,
                        help="Comma-separated list of tile names to include")
    parser.add_argument("--output", type=str, default=None, help="Override output path")
    args = parser.parse_args()

    catalog = load_catalog()
    QC_DIR.mkdir(parents=True, exist_ok=True)

    filter_tiles = args.tiles.split(",") if args.tiles else None

    print("Generating map preview...")
    preview = generate_map_preview(catalog, map_size=args.size, filter_tiles=filter_tiles)

    if preview is None:
        print("  Failed to generate preview")
        sys.exit(1)

    output_path = Path(args.output) if args.output else QC_DIR / "map_preview.png"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    preview.save(output_path, "PNG")
    print(f"  Saved: {output_path} ({preview.width}x{preview.height})")


if __name__ == "__main__":
    main()
