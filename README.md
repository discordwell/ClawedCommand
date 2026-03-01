# ClawedCommand

An isometric 2D RTS where you command armies through both traditional point-and-click micro **and** natural language instructions to a fine-tuned LLM that writes Lua strategy scripts in real-time. Built in Rust with [Bevy 0.18](https://bevyengine.org/).

## The Premise

After the singularity, humanity uploaded themselves, chose animal forms, and promptly forgot why. Now six factions of cute-but-deadly critters wage war across a post-digital landscape, each guided by a rival AI. You play as **catGPT** — a cat army advised by the AI agent **Geppity** — pursuing world domination one laser pointer at a time.

Light comedic tone. Mechanically serious.

## What Makes This Different

**Hybrid control.** You can click-to-move like any RTS, *or* open Construct Mode and describe what you want in plain English. The AI generates a Lua combat script, binds it to a voice keyword, and you shout "focus fire!" during battle to execute it. Both input paths flow through the same deterministic command system — player commands always override AI suggestions.

**Voice commands.** Push-to-talk triggers an on-device keyword classifier (TC-ResNet8, 119 classes, runs in a background thread with zero frame impact). Recognized keywords execute bound Lua scripts instantly.

**AI that learns.** 49 generations of automated arena matches evolved the combat scripts from 0% to 95% win rate. The best strategies (centroid focus fire, conditional kiting, terrain-aware retreat) are baked into starter scripts that ship with the game.

## Factions

| Faction | Animals | AI Agent | Playstyle |
|---|---|---|---|
| **catGPT** | Cats, bats, ferrets | Geppity | Balanced, stealth ops, strong individuals |
| **The Clawed** | Mice, shrews, voles | Claudeus Maximus | Swarm tactics, cheap units, guerrilla warfare |
| **Seekers of the Deep** | Badgers, moles, wolverines | Deepseek | Defensive fortresses, heavy armor |
| **The Murder** | Corvids (crows, ravens, magpies) | Gemineye | Intel/espionage, aerial dominance, astrology-themed abilities |
| **LLAMA** | Raccoons, possums, rats | Llhama | Scavengers, salvage wrecks for parts, jury-rigged tech |
| **Croak** | Axolotls, frogs, newts, turtles | Grok | Regeneration, water advantage, extremely hard to kill |

## Architecture

A Cargo workspace with six crates:

```
crates/
├── cc_core      Engine-agnostic types: components, commands, coords, map, terrain, fixed-point math
├── cc_sim       Bevy ECS simulation: 18-system FixedUpdate chain at 10Hz, deterministic lockstep
├── cc_client    Bevy app: isometric renderer, input handling, full HUD, camera, VFX
├── cc_voice     On-device voice recognition: Silero VAD + TC-ResNet8 classifier (ONNX)
├── cc_agent     AI layer: Lua runtime (mlua), ScriptContext API, LLM client, MCP tools, arena trainer
└── cc_harness   Headless sim wrapper + MCP server (35 tools via rmcp) for testing and AI training
```

### Simulation

The simulation runs in `FixedUpdate` at 10 ticks/second with a strict system ordering:

```
tick → commands → abilities → status_effects → auras → stat_modifiers → production →
research → gathering → target_acquisition → combat → tower_combat → projectiles →
movement → builder → grid_sync → cleanup → victory
```

All math uses `FixedI32<U16>` for deterministic replay. Faction-aware A\* pathfinding with terrain costs and elevation modifiers.

### Rendering

Full isometric 2D renderer with:
- **Zoom LOD**: Tactical view (< 2.0x) shows full sprites, health bars, VFX. Strategic view (>= 2.0x) switches to colored-dot icons with inverse-scaled labels.
- **Animation**: 4-frame sprite sheets (idle/walk/attack) driven by ECS state
- **VFX**: Lightweight particle system (200 cap) with trail + impact bursts
- **Fog of War**: Per-tile visibility overlays
- **Minimap**: Click-to-jump overview with unit dots
- **Autotile terrain**: Borders, water animation, terrain atlas

### AI Agent

The AI operates as a **code generator above the runtime**, not an in-loop decision maker:

1. Player describes intent in natural language
2. Fine-tuned LLM (Devstral Small 2, 24B, LoRA adapter) generates a Lua script
3. Script executes via `ScriptContext` with a 500-point compute budget
4. Available primitives: 25+ query methods, 15+ command methods, 20 composable behaviors
5. Scripts persist in a library, bindable to voice keywords

### Voice Pipeline

Three-thread architecture (audio capture → VAD → classification), entirely on-device:
- **VAD**: Silero v5 (2.3MB ONNX) detects speech segments
- **Classifier**: TC-ResNet8 (262K params, 1MB ONNX), 99.8% validation accuracy
- **Push-to-talk**: V key, with visual feedback on the HUD

## Campaign

A 23-mission narrative campaign across a Prologue and 5 Acts, with 4 branching endings.

**Protagonist**: Kelpie, a young otter who can simultaneously interface with all 6 faction AIs — making them the most dangerous individual alive.

**Named heroes**: Commander Felix Nine (catGPT), Marshal Thimble (The Clawed), Mother Granite (Seekers), Rex Solstice (The Murder), King Ringtail (LLAMA), The Eternal (Croak).

Mission definitions are RON files in `assets/campaign/` with inline maps, hero spawns, wave definitions, objectives, and mutators (LavaRise, ToxicTide, VoiceOnlyControl, etc.).

## Economy

| Resource | Source | Use |
|---|---|---|
| **Food** | Fish ponds, berry bushes | Unit training, building construction |
| **GPU Cores** | Tech ruins | AI actions, research, advanced units |
| **NFTs** | Monkey Mines (neutral objectives) | Victory points, special upgrades |

Server Racks increase your AI action rate cap. Destroying enemy racks degrades their AI.

## Running

```bash
# Standard game
cargo run -p cc_client

# Demo modes
cargo run -p cc_client -- --demo canyon       # Canyon battle scenario
cargo run -p cc_client -- --demo canyon 3     # Canyon with hero units
cargo run -p cc_client -- --demo showcase     # Building showcase
cargo run -p cc_client -- --demo cutscene 1   # Faction cutscene with dialogue
cargo run -p cc_client -- --demo voice        # Voice command demo
cargo run -p cc_client -- --demo match        # AI mirror match
```

### Controls

| Input | Action |
|---|---|
| Left-click | Select unit |
| Shift+click | Add to selection |
| Left-drag | Box select |
| Right-click | Move / Attack-move |
| H | Stop (halt) |
| Shift+H | Hold position |
| Esc | Deselect all |
| Q / W / E / R | Train units |
| V (hold) | Push-to-talk |
| Scroll wheel | Zoom |
| WASD / edge scroll | Pan camera |

### AI Arena (training)

```bash
cargo run -p cc_agent --bin arena --features harness -- \
  --seeds 1,2,3 \
  --p0-scripts training/arena/gen_042/player_0/ \
  --shared-scripts training/arena/gen_042/player_1/
```

### MCP Server (for LLM integration)

```bash
cargo run -p cc_harness
```

Exposes 35 tools (11 query, 10 command, 6 behavior, 8 sim-control) over the Model Context Protocol.

## Test Suite

592+ tests across all crates:

```bash
cargo test --workspace
```

| Crate | Tests |
|---|---|
| cc_core | 108 |
| cc_sim (unit) | 29 |
| cc_sim (integration) | 199 |
| cc_agent | 63 |
| cc_agent (arena) | 44 |
| cc_harness | 79 |
| cc_client | 24 |
| cc_voice | 23 |

## Training

### Lua Script Evolution

The `training/arena/` directory contains 49 generations of AI script evolution. Key discoveries:

- **Group focus fire** (centroid-based, all attackers target same enemy) is the single most impactful behavior
- **Conditional kiting** for ranged units when outnumbered prevents army loss without causing stalemates
- **Terrain-aware retreat** checks `movement_cost` and tries perpendicular escape routes when the flee path is blocked
- **Closest-to-centroid targeting is critical** — switching to weakest/lowest-HP targeting is catastrophic (20% win rate)

### LLM Fine-Tuning

Devstral Small 2 (24B) fine-tuned with LoRA (r=32, 184M trainable params) on 550 Lua script examples. The adapter lives in `training/lora_checkpoints/`. Key technical details in the codebase MEMORY files.

### Voice Model

TC-ResNet8 trained via knowledge distillation from a larger teacher model. 2975 synthetic TTS samples, 119 keyword classes. Pipeline in `training/voice/`.

## Asset Pipeline

`tools/asset_pipeline/` — a Python pipeline that orchestrates sprite generation, post-processing (background removal, palette normalization, sheet slicing), and atlas manifest generation. Art style: *Into the Breach* meets *Redwall* — clean, minimal, readable, cute animals with tactical clarity.

## Project Documentation

| File | Contents |
|---|---|
| `ARCHITECTURE.md` | Full system architecture (7 layers) |
| `GAME_DESIGN.md` | Complete game design document |
| `STORYLINE.md` | 23-mission campaign narrative |
| `PLAN.md` | Phase 1-3 implementation roadmap |
| `CAMPAIGN_GAPS.md` | Remaining campaign work |
| `ASSET_PIPELINE.md` | Asset pipeline documentation |
| `TDL.md` | To-do-later backlog |

## Tech Stack

- **Engine**: Bevy 0.18 (Rust)
- **Fixed-point math**: `fixed` crate (`FixedI32<U16>`)
- **Scripting**: mlua (Luau sandbox)
- **AI inference**: Devstral Small 2 (24B) via OpenAI-compatible API
- **Voice**: ONNX Runtime (Silero VAD + TC-ResNet8)
- **Audio capture**: cpal
- **MCP server**: rmcp 0.17
- **Map format**: RON
- **Asset processing**: Python (Pillow, rembg, NumPy)
