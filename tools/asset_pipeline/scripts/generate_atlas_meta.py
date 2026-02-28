#!/usr/bin/env python3
"""
Generate/update the Bevy TextureAtlasLayout manifest.

Emits atlas_manifest.yaml mapping sprite sheets to TextureAtlasLayout::from_grid params.

Usage:
    python generate_atlas_meta.py --name infantry_walk --path assets/sprites/units/infantry_walk.png \
        --columns 4 --rows 1 --tile-width 128 --tile-height 128
"""

import argparse
import sys
from pathlib import Path

import yaml

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
ATLAS_DIR = PROJECT_ROOT / "assets" / "atlas"
MANIFEST_PATH = ATLAS_DIR / "atlas_manifest.yaml"


def load_manifest():
    if MANIFEST_PATH.exists():
        with open(MANIFEST_PATH) as f:
            data = yaml.safe_load(f)
            return data if data else {"sheets": {}}
    return {"sheets": {}}


def save_manifest(manifest):
    ATLAS_DIR.mkdir(parents=True, exist_ok=True)
    with open(MANIFEST_PATH, "w") as f:
        yaml.dump(manifest, f, default_flow_style=False, sort_keys=False)


def main():
    parser = argparse.ArgumentParser(description="Generate atlas manifest entry")
    parser.add_argument("--name", required=True, help="Sprite sheet name (catalog key)")
    parser.add_argument("--path", required=True, help="Game asset path")
    parser.add_argument("--columns", type=int, required=True)
    parser.add_argument("--rows", type=int, required=True)
    parser.add_argument("--tile-width", type=int, required=True)
    parser.add_argument("--tile-height", type=int, required=True)
    args = parser.parse_args()

    manifest = load_manifest()

    # Bevy TextureAtlasLayout::from_grid params
    entry = {
        "path": args.path,
        "tile_size": [args.tile_width, args.tile_height],
        "columns": args.columns,
        "rows": args.rows,
        "frame_count": args.columns * args.rows,
        # Bevy code equivalent:
        # TextureAtlasLayout::from_grid(
        #     UVec2::new(tile_width, tile_height),
        #     columns,
        #     rows,
        #     None,  // padding
        #     None,  // offset
        # )
    }

    manifest["sheets"][args.name] = entry
    save_manifest(manifest)
    print(f"  Atlas manifest updated: {args.name} → {args.path} ({args.columns}x{args.rows} @ {args.tile_width}x{args.tile_height})")


if __name__ == "__main__":
    main()
