# Asset Pipeline

Claude-in-Chrome automated asset generation pipeline. Uses ChatGPT image generation (via browser automation) with post-processing to produce game-ready sprites.

## Art Style

- Clean vector art, flat colors, bold dark outlines (2-3px)
- Into the Breach / Northgard aesthetic
- Isometric perspective (~30 degrees)
- 128x128 base tiles, units 128-192px
- Transparent PNG backgrounds
- Consistent top-left lighting

## Directory Layout

```
tools/asset_pipeline/
├── config/
│   ├── asset_catalog.yaml      # Master manifest — all assets + status
│   ├── palette.yaml            # Target color palette
│   └── prompts/                # Prompt templates per category
│       ├── base_style.txt      # Style fragment appended to ALL prompts
│       ├── terrain.txt
│       ├── building.txt
│       ├── unit_sheet.txt      # Animated sprite sheets
│       ├── unit_static.txt     # Single unit poses
│       ├── resource.txt
│       ├── projectile.txt
│       └── ui.txt
├── references/                 # Style reference images
│   └── master_style.png        # THE master ref — uploaded with every request
├── raw/                        # Untouched ChatGPT outputs
├── processed/                  # Post-processed intermediates
├── scripts/
│   ├── generate_asset.py       # Main orchestrator
│   ├── process_sprite.py       # Single sprite processing
│   ├── process_sheet.py        # Sprite sheet processing
│   ├── normalize_palette.py    # Color palette normalization
│   ├── verify_grid.py          # Grid alignment verification
│   └── generate_atlas_meta.py  # Bevy atlas manifest generator
└── requirements.txt

assets/                         # Final game-ready (Bevy loads from here)
├── sprites/{terrain,buildings,units,resources,projectiles}/
├── atlas/atlas_manifest.yaml
└── ui/
```

## Setup

```bash
cd tools/asset_pipeline
pip install -r requirements.txt
```

## Usage

### See what assets exist and their status

```bash
python scripts/generate_asset.py status
```

Status flow: `planned` → `generated` → `processed` → `game_ready`

### Generate a prompt for ChatGPT

```bash
python scripts/generate_asset.py prompt infantry_idle
```

This assembles the prompt from the catalog entry + template + base style. Copy it into ChatGPT along with `master_style.png` as a reference upload.

### Add a new asset

```bash
python scripts/generate_asset.py add units battle_cat_idle
```

Then edit `config/asset_catalog.yaml` to fill in the description and params.

### Process a raw image into game-ready form

After downloading from ChatGPT to `raw/{category}/{name}_raw.png`:

```bash
python scripts/generate_asset.py process infantry_idle
```

This runs the full chain:
1. Background removal (rembg)
2. Resize/crop (single) or slice/reassemble (sheet)
3. Grid verification (sheets only)
4. Atlas manifest update (sheets only)
5. Copy to `assets/sprites/...`
6. Update catalog status → `game_ready`

### Normalize colors to palette

```bash
python scripts/normalize_palette.py processed/units/infantry_idle.png output.png
```

### Verify a sprite sheet grid

```bash
python scripts/verify_grid.py sprites/units/infantry_walk.png --columns 4 --rows 1
```

## Browser Automation Flow (Claude-in-Chrome)

When Claude automates asset generation via ChatGPT:

1. Get/create browser tab
2. Navigate to `chatgpt.com`
3. Upload `references/master_style.png` via file input
4. Type the assembled prompt, send
5. Wait for image generation (~15-60s), polling with screenshots
6. Download the generated image
7. Move from `~/Downloads/` to `raw/{category}/`
8. Run `generate_asset.py process <name>`

## Adding New Asset Categories

1. Create a new prompt template in `config/prompts/`
2. Add entries to `config/asset_catalog.yaml`
3. Create subdirectories in `raw/`, `processed/`, and `assets/sprites/`

## Bevy Integration

The atlas manifest at `assets/atlas/atlas_manifest.yaml` maps sprite sheets to `TextureAtlasLayout::from_grid` parameters:

```yaml
sheets:
  infantry_walk:
    path: assets/sprites/units/infantry_walk.png
    tile_size: [128, 128]
    columns: 4
    rows: 1
    frame_count: 4
```

In Rust:
```rust
let layout = TextureAtlasLayout::from_grid(
    UVec2::new(128, 128),  // tile_size
    4,                      // columns
    1,                      // rows
    None,                   // padding
    None,                   // offset
);
```
