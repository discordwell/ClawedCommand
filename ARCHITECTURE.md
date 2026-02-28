# ClawedCommand Architecture

> Post-singularity cat RTS where players hybrid-control cute animal armies directly and through AI agents powered by fine-tuned Mistral models. Humanity uploaded, chose animal forms, and forgot why. You're a cat with an AI named Minstral pursuing world domination. Other Redwall-esque factions oppose you with their own (worse) AI agents. Light comedic tone, mechanically serious. Full game identity in **[GAME_DESIGN.md](./GAME_DESIGN.md)**.

## Vision

Players command armies through a dual interface: traditional RTS point-and-click micro **and** natural language instructions to an AI commander (Minstral) that generates strategy code. The AI agent uses MCP tools to issue game commands, creating a gameplay loop where strategic thinking and AI coaching are as important as mechanical skill.

---

## High-Level System Diagram

```
┌─────────────────────────────────────────────────────────┐
│                      CLIENT                             │
│                                                         │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Renderer │  │  Player Input │  │  Agent Interface  │  │
│  │(Isometric│  │ (Click/Select │  │ (Chat + Code     │  │
│  │ 2D/Bevy) │  │  /Hotkeys)   │  │  Display)         │  │
│  └────┬─────┘  └──────┬───────┘  └────────┬──────────┘  │
│       │               │                   │              │
│       │         ┌─────▼───────────────────▼─────┐       │
│       │         │     Command Dispatcher         │       │
│       │         │  (merges player + AI commands)  │       │
│       │         └─────────────┬──────────────────┘       │
│       │                       │                          │
│  ┌────▼───────────────────────▼──────────────────────┐   │
│  │              GAME SIMULATION (Bevy ECS)            │   │
│  │                                                    │   │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌───────┐  │   │
│  │  │  Units  │ │Buildings │ │Resources │ │Terrain│  │   │
│  │  │ System  │ │ System   │ │ System   │ │System │  │   │
│  │  └─────────┘ └──────────┘ └──────────┘ └───────┘  │   │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────┐            │   │
│  │  │ Combat  │ │Pathfind  │ │  Fog of  │            │   │
│  │  │ System  │ │ System   │ │  War     │            │   │
│  │  └─────────┘ └──────────┘ └──────────┘            │   │
│  └───────────────────────────────────────────────────┘   │
│                                                         │
└──────────────────────┬──────────────────────────────────┘
                       │
              ┌────────▼────────┐
              │   Networking    │
              │  (Lockstep or   │
              │   Client-Server)│
              └────────┬────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                      SERVER                              │
│                                                          │
│  ┌─────────────────┐  ┌──────────────────────────────┐   │
│  │  Game Server    │  │  AI Inference Service         │   │
│  │  (Authoritative │  │  (Fine-tuned Mistral)         │   │
│  │   Simulation)   │  │                               │   │
│  │                 │  │  - MCP Tool Server             │   │
│  │  - Match mgmt  │  │  - Code generation             │   │
│  │  - Anti-cheat   │  │  - Strategy execution          │   │
│  │  - Replay       │  │  - Sandboxed code runner       │   │
│  └─────────────────┘  └──────────────────────────────┘   │
│                                                          │
│  ┌─────────────────┐  ┌──────────────────────────────┐   │
│  │  Matchmaking    │  │  Player Data / Persistence    │   │
│  │  Service        │  │  (Profiles, replays, configs) │   │
│  └─────────────────┘  └──────────────────────────────┘   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## Core Layers

### 1. Game Simulation (Bevy ECS)

The deterministic game simulation is the heart of the system. All game state lives in the ECS.

**Entity Types:**
- **Units** — Pawdlers, Nuisances, Chonks, Hissers, etc. Components: `Position`, `Health`, `Attack`, `Movement`, `Owner`, `UnitType`, `AIControllable`
- **Buildings** — The Box, Cat Tree, Server Rack, etc. Components: `Position`, `Health`, `ProductionQueue`, `Owner`, `BuildingType`
- **Resources** — fish ponds, GPU deposits, Monkey Mines. Components: `Position`, `ResourceType`, `Amount`
- **Projectiles** — spit, laser beams, explosives. Components: `Position`, `Velocity`, `Damage`, `Target`
- **Terrain** — tiles with elevation, passability, resource slots

**Core Systems (tick order):**
1. `input_system` — process player commands from input queue
2. `ai_command_system` — process agent-generated commands from AI queue
3. `production_system` — handle build queues, unit spawning
4. `resource_system` — gathering, spending, income
5. `pathfinding_system` — A* / flowfield on isometric grid
6. `movement_system` — apply velocity, handle collision
7. `combat_system` — target acquisition, damage calculation, death
8. `projectile_system` — move projectiles, hit detection
9. `fog_of_war_system` — visibility updates per player
10. `cleanup_system` — despawn dead entities, garbage collect

**Design Constraints:**
- Simulation must be **deterministic** for lockstep multiplayer
- Fixed tick rate (e.g., 10 logic ticks/sec) decoupled from render framerate
- All state mutations go through the command queue — no direct writes

### 2. Command System

Unified command interface that both players and AI agents write to.

```rust
enum GameCommand {
    // Unit commands
    Move { unit_ids: Vec<EntityId>, target: GridPos },
    Attack { unit_ids: Vec<EntityId>, target: EntityId },
    Patrol { unit_ids: Vec<EntityId>, waypoints: Vec<GridPos> },
    Stop { unit_ids: Vec<EntityId> },
    HoldPosition { unit_ids: Vec<EntityId> },

    // Building commands
    Build { building_type: BuildingType, position: GridPos },
    SetRallyPoint { building_id: EntityId, position: GridPos },
    QueueUnit { building_id: EntityId, unit_type: UnitType },
    CancelQueue { building_id: EntityId, index: usize },

    // Economy commands
    GatherResource { unit_ids: Vec<EntityId>, resource_id: EntityId },

    // Meta commands
    SetAgentMode { unit_ids: Vec<EntityId>, enabled: bool },
}
```

**Command Dispatcher** merges commands from two sources:
- **Player input** — mouse clicks, keyboard shortcuts, UI buttons
- **AI agent** — MCP tool calls translated to `GameCommand`s

Priority/conflict resolution: player commands override AI commands for the same units (player always has final say).

### 3. Voice Command System

> Full technical details in **[VOICE.md](./VOICE.md)**.

Players vibecode Lua agent scripts in **construct mode** (an in-game LLM-powered scripting environment) and command them by voice during gameplay. Push-to-talk speech recognition feeds into a tiered intent classifier (keyword match → fuzzy match → Mistral agent), which triggers the matching Lua script. Scripts run in the WASM sandbox with access to the MCP game tools.

**Voice commands are a core game mechanic:** units touched by a voice-triggered script receive a temporary command-specific buff (e.g., attack → damage buff, retreat → speed/armor buff). This incentivizes voice use over clicking and creates strategic depth around cooldown management and script design.

The meta-game is the vibecoding itself — players use an LLM to generate Lua scripts in construct mode, iterating on their agent loadout. Better scripts = smarter agents = competitive advantage.

### 4. AI Agent Layer (MCP + Fine-tuned Mistral)

> Full technical details, code examples, and training data formats in **[MISTRAL.md](./MISTRAL.md)**.

This is the novel core of ClawedCommand.

**Architecture:**

```
Player ──(natural language)──► Agent Interface (Chat UI)
                                      │
                                      ▼
                              Fine-tuned Devstral
                              (understands game state,
                               generates tool calls)
                                      │
                                      ▼
                              MCP Tool Server
                              (game-specific tools)
                                      │
                                      ▼
                              Command Queue ──► ECS Simulation
```

**Model Selection:**

| Context | Model | Deployment | Latency Target |
|---------|-------|------------|----------------|
| Competitive/ranked | Devstral 2 (123B) | Mistral API (`devstral-2-2512`) | <2s per turn |
| Single-player/practice | Devstral Small 2 (24B) | Local vLLM/Ollama, Q4_K_M ~14GB | <5s per turn |

Both models are dense transformers (not MoE) with native tool use support and 256K context windows. The Rust client in `cc_agent` uses a single `reqwest`-based HTTP client that targets the OpenAI-compatible endpoint — identical code path for both API and local inference, just a different base URL.

**MCP Tools exposed to the model:**

| Tool | Description |
|------|-------------|
| `get_units(filter)` | Query own units by type, location, status |
| `get_buildings(filter)` | Query own buildings |
| `get_visible_enemies()` | What the player can currently see |
| `get_resources()` | Current resource counts |
| `get_map_info(region)` | Terrain data for a region |
| `move_units(ids, target)` | Issue move command |
| `attack_units(ids, target)` | Issue attack command |
| `build(type, position)` | Place a building |
| `train_unit(building, type)` | Queue unit production |
| `set_rally_point(building, pos)` | Set rally point |
| `patrol(ids, waypoints)` | Set patrol route |
| `gather_resource(workers, deposit)` | Send workers to gather |
| `execute_strategy(code)` | Run a sandboxed strategy script |

MCP tool definitions map to Mistral's function calling format with minimal conversion (`inputSchema` → `parameters`). See MISTRAL.md for full JSON schemas and conversion code.

**Fine-tuning approach:**
- **Server model**: Fine-tune `mistral-small-latest` or `codestral-latest` via Mistral API (~$5-10 per run)
- **Local model**: Fine-tune Devstral Small 2 weights via `mistral-finetune` repo (LoRA, rank 64)
- **Training data**: JSONL with `messages` (full tool-call conversations) + `tools` (function definitions)
- **Data pipeline**: game replays → replay converter → (game_state, instruction, tool_calls) JSONL
- **Data strategy**: bootstrap with 200-500 hand-authored examples → augment with self-play → enrich with human replays
- Tool call IDs must be exactly 9 random chars; arguments must be stringified JSON

**Cost estimation (competitive play):** ~$0.012 per player turn, ~$0.72 per player per game at ~60 turns.

**Code generation mode:**
- For complex strategies, the model generates reusable strategy scripts
- Scripts run in a WASM sandbox (Wasmtime) with access only to the game tools
- Players can save, edit, and share strategy scripts
- Scripts are versioned and can be rolled back

**Inference routing:**
- **Competitive/ranked** — Mistral API, Devstral 2, server-side (fair, anti-cheat)
- **Practice/single-player** — local vLLM or Ollama, Devstral Small 2 Q4_K_M (free, works offline, requires RTX 4090 or Mac 32GB+)

### 5. Renderer (Bevy 2D + Isometric)

**Isometric projection:**
- Tile-based map with diamond-shaped tiles
- Sprite-based units and buildings with directional animations
- Depth sorting based on y-position for correct overlap
- Smooth camera with zoom, pan, edge scrolling, minimap

**Visual layers (bottom to top):**
1. Terrain tiles
2. Resource deposits
3. Building foundations / shadows
4. Buildings
5. Unit shadows
6. Units (depth-sorted)
7. Projectiles / effects
8. Fog of war overlay
9. Selection indicators, health bars
10. UI overlay (HUD, minimap, chat, agent panel)

**Asset pipeline** (see [ASSET_PIPELINE.md](ASSET_PIPELINE.md)):
- Generation: Claude-in-Chrome → ChatGPT image gen with style reference + prompt templates
- Post-processing: `rembg` bg removal → resize/slice → grid verification → palette normalization
- Sprite sheets: sliced into frames, reassembled to exact grid, atlas manifest generated
- Atlas: `assets/atlas/atlas_manifest.yaml` → `TextureAtlasLayout::from_grid` in Bevy
- Catalog: `tools/asset_pipeline/config/asset_catalog.yaml` tracks every asset through `planned → generated → processed → game_ready`
- Tilemaps: `bevy_ecs_tilemap` (git branch `0.18`)
- Animations: frame-based sprite animation system using TextureAtlasLayout

### 6. Networking

**Model: Deterministic Lockstep** (preferred for RTS)

- Each client runs the full simulation
- Only **commands** are sent over the network (tiny bandwidth)
- All clients process the same commands on the same tick → identical state
- Requires strict determinism (no floats in simulation — use fixed-point math)

**Alternative: Client-Server Authoritative** (fallback if determinism is too hard)
- Server runs authoritative simulation
- Clients send commands, receive state snapshots
- More bandwidth, but tolerant of non-determinism
- Better anti-cheat (server validates everything)

**Networking stack:**
- Transport: QUIC or WebTransport (UDP-based, reliable + unreliable channels)
- Serialization: `bincode` or `rkyv` for minimal overhead
- Matchmaking: separate service (REST API + WebSocket lobby)

### 7. Economy & Tech Tree

**Resources:**
- **Food**: Gathered from fish ponds and berry bushes by Pawdlers (workers). Funds unit training and building upkeep.
- **GPU Cores**: Harvested from old-world tech ruins (limited deposits). Powers AI agent actions, advanced units, and buildings. Strategic tension: invest in AI or army?
- **NFTs**: Generated by captured Monkey Mines (neutral objectives guarded by feral Monkeys). Victory points + powerful upgrades. Map control driver.
- **Supply**: Population cap increased by Litter Boxes.

**Tech tree structure (cat faction):**
```
The Box ──► Cat Tree ──► Nuisance, Hisser, Yowler, Chonk
  │               └──► Scratching Post ──► Mouser, Catnapper, Mech Commander
  ├──► Fish Market ──► Pawdlers (enhanced gathering)
  ├──► Server Rack ──► AI upgrades, FerretSapper, Flying Fox
  ├──► Litter Box ──► Supply cap
  └──► Laser Pointer ──► Base defense
```

See [GAME_DESIGN.md](./GAME_DESIGN.md) for full unit roster, building details, and all six factions.

---

## Project Structure

```
ClawedCommand/
├── ARCHITECTURE.md
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── cc_core/                # Shared types, commands, fixed-point math
│   │   ├── src/
│   │   │   ├── commands.rs     # GameCommand enum
│   │   │   ├── components.rs   # ECS component definitions
│   │   │   ├── math.rs         # Fixed-point arithmetic
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── cc_sim/                 # Game simulation (ECS systems)
│   │   ├── src/
│   │   │   ├── systems/        # One file per system
│   │   │   ├── resources.rs    # Bevy resources (game clock, map data)
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── cc_client/              # Bevy app, renderer, input, UI
│   │   ├── src/
│   │   │   ├── renderer/       # Isometric rendering, camera, sprites
│   │   │   ├── input/          # Mouse/keyboard handling
│   │   │   ├── ui/             # HUD, minimap, agent chat panel
│   │   │   └── main.rs
│   │   ├── assets/             # Sprites, tilemaps, audio
│   │   └── Cargo.toml
│   ├── cc_voice/               # Voice command system
│   │   ├── src/
│   │   │   ├── speech.rs       # Speech-to-text (Web Speech API + Whisper fallback)
│   │   │   ├── intent.rs       # Tiered intent classification pipeline
│   │   │   ├── lua_runtime.rs  # Lua script execution in WASM sandbox
│   │   │   ├── buff.rs         # VoiceCommandBuff component + system
│   │   │   ├── construct.rs    # Construct mode state + LLM integration
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── cc_agent/               # AI agent integration
│   │   ├── src/
│   │   │   ├── mcp_server.rs   # MCP tool definitions
│   │   │   ├── inference.rs    # Model client (local + remote)
│   │   │   ├── sandbox.rs      # Strategy script sandbox
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── cc_net/                 # Networking (lockstep / client-server)
│   │   ├── src/
│   │   │   ├── protocol.rs     # Message types, serialization
│   │   │   ├── lockstep.rs     # Lockstep synchronization
│   │   │   ├── transport.rs    # QUIC/WebTransport layer
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   └── cc_server/              # Dedicated server binary
│       ├── src/
│       │   ├── matchmaking.rs
│       │   ├── game_server.rs
│       │   └── main.rs
│       └── Cargo.toml
├── tools/
│   ├── replay_converter/       # Convert replays to training data
│   └── asset_pipeline/         # Asset processing scripts
├── training/
│   ├── data/                   # JSONL training/eval datasets
│   ├── configs/                # Fine-tuning YAML configs (see MISTRAL.md)
│   └── scripts/                # Python fine-tuning job scripts
└── assets/
    ├── sprites/
    ├── tilemaps/
    ├── audio/
    └── ui/
```

---

## Development Phases

### Phase 1: Core Simulation
- Set up Bevy workspace with `cc_core` and `cc_sim`
- Implement basic ECS: units with movement, simple grid map
- Command system with player input
- Basic isometric renderer (placeholder sprites)
- **Milestone: units moving on an isometric grid via click commands**

### Phase 2: Combat & Economy
- Combat system (attack, damage, death)
- Resource gathering (workers, deposits)
- Building placement and production queues
- Basic tech tree
- **Milestone: playable skirmish loop — gather, build, fight**

### Phase 3: AI Agent + Voice Commands
- MCP tool server with game-state query tools
- Local Mistral inference integration
- Agent chat UI panel
- Command dispatcher (merge player + AI commands)
- Voice input pipeline (push-to-talk, speech-to-text, intent classification)
- Construct mode (in-game LLM-powered Lua script editor)
- Lua script runtime in WASM sandbox with MCP tool access
- Starter scripts (basic attack, retreat, build, gather, train)
- Voice command buff system (command-specific temporary buffs)
- **Milestone: vibecode a Lua script, issue voice command, watch agent execute with buff**

### Phase 4: Fine-tuning Pipeline
- **GPU platform**: [NVIDIA Brev](https://brev.nvidia.com) — $100 GPU budget, $15 Mistral API credits
- **Strategy**: QLoRA on L40S 48GB (~$1.25/hr) for fast iteration → final full LoRA on A100 80GB (~$2.50/hr) for the winner
- **Multi-model evaluation**: Compare Qwen2.5-Coder-32B, Devstral Small 2 (24B), Codestral (API), and xLAM-2-8B
- **Training data**: 50 gold hand-authored examples → 500-1000 synthetic variations via Claude → quality filtering
- **Quick baseline**: Codestral via Mistral API fine-tuning (~30 min turnaround) to validate data quality
- **QLoRA iteration**: Unsloth + TRL SFTTrainer with 4-bit quantized LoRA (rank 32, alpha 64) on L40S 48GB
- **Final LoRA**: Full-precision LoRA on A100 80GB for the winning model only
- **Budget-aware order**: Codestral API ($10) → xLAM QLoRA ($4) → Devstral QLoRA ($8) → Qwen QLoRA ($12) → eval ($8) → final LoRA ($8)
- **Format pipeline**: Validate → convert between Mistral/Qwen/xLAM chat templates → 90/10 train/eval split
- **Evaluation harness**: Tool call accuracy (>95%), instruction following (>85%), multi-step completion (>70%), no-tool accuracy (>90%), latency (<2s)
- Replay recording system + replay → training data converter
- See [training/BREV_GUIDE.md](training/BREV_GUIDE.md) for step-by-step Brev setup and training instructions
- **Milestone: fine-tuned model demonstrably outperforms base model at game-specific tool calling**

### Phase 5: Multiplayer
- Deterministic simulation verification
- Lockstep networking implementation
- Lobby / matchmaking service
- Server-side inference for competitive play
- **Milestone: two players can play a full match over the network**

### Phase 6: Polish & Content
- Full sprite art and animations
- Sound design
- Multiple factions / unit rosters
- Campaign / tutorial
- Strategy script sharing / marketplace
- **Milestone: shippable game**

---

## Key Technical Risks

| Risk | Mitigation |
|------|------------|
| Bevy pre-1.0 breaking changes | Pin Bevy version, update deliberately between milestones |
| Deterministic simulation is hard | Use fixed-point math from day 1, test with desync detection |
| Mistral inference latency too high for real-time | Devstral Small 2 (24B) for local at Q4_K_M; batch tool calls; cap at 10 rounds per turn |
| Fine-tuning data quality | Bootstrap with 200-500 hand-authored examples, augment with self-play, enrich with human replays |
| Competitive play inference costs (~$0.72/player/game) | Monitor token usage, optimize system prompts, consider caching common game state queries |
| Sandboxed code execution security | WASM sandbox for strategy scripts, strict resource limits |
| Web Speech API Chromium-only | Whisper.js fallback for non-Chromium browsers and offline/native clients |
| Voice buff balance | Start conservative (low magnitude, short duration), tune via playtesting; cooldown prevents spam |
| LLM-generated Lua quality | Starter scripts as examples, constrained API surface, sandbox catches runtime errors |
| Construct mode mid-mission distraction | Game doesn't pause — intentional risk/reward tradeoff for players |
| Isometric depth sorting edge cases | Well-tested sprite sorting system, handle edge cases early |
| Comedic tone may undercut competitive appeal | Humor is in flavor/lore, not mechanics. Gameplay is mechanically serious — the comedy is the juxtaposition |

---

## Technology Stack Summary

| Layer | Technology |
|-------|-----------|
| Language | Rust |
| Game Engine | Bevy 0.15+ |
| ECS | Bevy ECS (built-in) |
| Rendering | Bevy 2D with custom isometric plugin |
| Networking | Quinn (QUIC) or wtransport |
| Serialization | bincode / serde |
| AI Model (Server) | Devstral 2 123B via Mistral API (`devstral-2-2512`) |
| AI Model (Local) | Devstral Small 2 24B via vLLM/Ollama (`devstral-small-2-2512`) |
| AI Interface | MCP → Mistral function calling (OpenAI-compatible) |
| Fine-tuning | Unsloth + TRL (Qwen/Devstral/xLAM), Mistral API (Codestral) |
| Voice Script Language | Lua (via `rlua`/`mlua` in WASM sandbox) |
| Speech-to-Text | Web Speech API (primary), Whisper.js via Transformers.js (fallback) |
| Sandbox | Wasmtime (WASM runtime for strategy scripts + Lua voice scripts) |
| Training | Python + Unsloth + TRL + HuggingFace |
| Asset Authoring | Aseprite (sprites), Tiled (maps) |
| Build | Cargo workspaces |
| CI/CD | GitHub Actions |
