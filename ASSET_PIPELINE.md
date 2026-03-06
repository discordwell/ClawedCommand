# Asset Pipeline

Claude-in-Chrome automated asset generation pipeline. Uses ChatGPT and Gemini image generation (via MCP browser automation) with post-processing to produce game-ready sprites. Supports parallel dual-provider generation for 2x throughput.

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
│   ├── generate_mcp.py         # MCP-based generation CLI helper
│   ├── process_sprite.py       # Single sprite processing
│   ├── process_sheet.py        # Sprite sheet processing
│   ├── normalize_palette.py    # Color palette normalization
│   ├── verify_grid.py          # Grid alignment verification
│   ├── generate_atlas_meta.py  # Bevy atlas manifest generator
│   ├── image_utils.py          # Shared processing + QC validation
│   └── providers/              # Provider abstraction for browser automation
│       ├── __init__.py
│       ├── base.py             # Interface contract + shared JS utilities
│       ├── chatgpt.py          # ChatGPT DOM selectors + JS snippets
│       └── gemini.py           # Gemini DOM selectors + JS snippets
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

## Browser Automation Flow (MCP — Dual Provider)

Claude uses MCP browser tools (`javascript_tool`, `find`, `read_page`) with provider-specific JS snippets from `scripts/providers/`.

### Single-provider flow

1. Create browser tab via `tabs_create_mcp`
2. Navigate to provider URL (`chatgpt.com` or `gemini.google.com/app`)
3. Optionally upload `references/master_style.png` via DataTransfer JS
4. Fill prompt using provider's `fill_prompt_js()`, click send via `click_send_js()`
5. Poll for image completion using `image_check_js()` + `image_loaded_js()`
6. Extract image URL via `image_src_js()`, download with `curl`
7. Register via `generate_mcp.py received <name> <raw_path>`
8. Process via `generate_mcp.py process <name>`

### Parallel dual-provider flow

1. Create 2 tabs — tab A → ChatGPT, tab B → Gemini
2. Send prompt to provider A (tab A)
3. Immediately send prompt to provider B (tab B)
4. Poll both tabs for completion (image gen takes 15-60s)
5. Download from whichever finishes first, or both for A/B comparison
6. Rate limit on one provider? Switch to the other

### Provider selection modes

- **single** (`--provider chatgpt` or `--provider gemini`) — use one provider only
- **both** — generate same asset from both, user picks the better result
- **round-robin** — alternate between providers for batch work
- **fallback** — try primary, switch to secondary on rate limit

### MCP CLI helper

```bash
# Show what needs generating
python scripts/generate_mcp.py queue

# Get the prompt for an asset
python scripts/generate_mcp.py prompt pawdler_idle

# Register a downloaded raw image
python scripts/generate_mcp.py received pawdler_idle /tmp/sprite.png --provider chatgpt

# Run post-processing
python scripts/generate_mcp.py process pawdler_idle

# Show pipeline status
python scripts/generate_mcp.py status

# List available providers
python scripts/generate_mcp.py providers
```

### Provider modules

Each provider module (`providers/chatgpt.py`, `providers/gemini.py`) exports:
- `URL` — base URL to navigate to
- `fill_prompt_js(text)` — JS to inject prompt text
- `click_send_js()` — JS to click send
- `image_check_js()` / `image_loaded_js()` / `image_src_js()` — JS for image polling
- `rate_limit_check_js()` — JS to detect rate limiting
- `upload_reference_js(b64)` — JS to upload a reference image

Gemini's DOM changes frequently. When selectors break, use MCP `find`/`read_page` to discover current selectors and update `gemini.py`.

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
