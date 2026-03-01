# ClawedCommand Architecture

> Post-singularity cat RTS where players hybrid-control cute animal armies directly and through AI agents powered by fine-tuned Mistral models. Humanity uploaded, chose animal forms, and forgot why. You're a cat with an AI named Geppity pursuing world domination. Other Redwall-esque factions oppose you with their own (worse) AI agents. Light comedic tone, mechanically serious. Full game identity in **[GAME_DESIGN.md](./GAME_DESIGN.md)**.

## Vision

Players command armies through a dual interface: traditional RTS point-and-click micro **and** natural language instructions to an AI commander (Geppity) that generates strategy code. The AI agent uses MCP tools to issue game commands, creating a gameplay loop where strategic thinking and AI coaching are as important as mechanical skill.

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

## System Layers

The architecture separates into five distinct layers, each with a clear responsibility and interface to the layers above and below it.

```
┌─────────────────────────────────────────┐
│  FRONTEND (cc_client)                   │
│  Renderer, UI, input handling           │
├─────────────────────────────────────────┤
│  BACKEND (cc_sim, cc_core)              │  ← tells frontend what to display
│  Bevy ECS simulation, commands, physics │
├─────────────────────────────────────────┤
│  SCRIPTED AI (cc_agent scripts)         │  ← uses ctx API
│  Player Lua scripts, enemy AI, FSM      │
├─────────────────────────────────────────┤
│  INTELLIGENCE LAYER                     │  ← creates scripts / comprehends voice
│  Agentic builders + voice comprehension │
│  (Fine-tuned Devstral Small 2)          │
├─────────────────────────────────────────┤
│  FINE-TUNING PIPELINE                   │  ← trains the intelligence layer
│  (Not in-game — Brev GPU + Unsloth)    │
└─────────────────────────────────────────┘
```

**Layer descriptions (top to bottom):**

1. **Frontend** — The Bevy 2D renderer, camera, input handlers, and UI panels in `cc_client`. Reads ECS state from the backend and renders it. Sends player inputs (mouse, keyboard, voice PTT) down as `GameCommand`s.

2. **Backend** — The deterministic ECS simulation in `cc_sim` and `cc_core`. Processes commands from both the player and scripted AI through a unified command queue. Runs at a fixed 10Hz tick rate. This is the authoritative game state — the frontend only displays what the backend computes.

3. **Scripted AI** — Lua scripts that execute each tick via the `ScriptContext` (ctx) API. These include hand-authored starter scripts (basic_attack, basic_retreat, etc.), player-created scripts from construct mode, and the enemy AI FSM. Scripts call ctx methods like `ctx:nearest_enemy()`, `ctx:attack()`, `ctx:move_to()` to issue commands. They run inside a sandboxed Luau runtime with instruction-count limits.

4. **Intelligence Layer** — The fine-tuned Devstral Small 2 model that *generates* Lua scripts on player request (the "agentic builder") and comprehends voice commands to trigger the right script. This layer does not run every tick — it activates on player interaction (chat input, voice command) and produces artifacts (Lua scripts, command triggers) that the Scripted AI layer then executes. This is the key architectural insight: the LLM is a *code generator* that sits above the runtime, not an in-loop decision maker.

5. **Fine-Tuning Pipeline** — Offline training infrastructure (Brev GPU, Unsloth, TRL) that produces the weights used by the Intelligence Layer. Not part of the running game. Consumes replay data and hand-authored examples, outputs LoRA adapters.

**Data flow across layers:** Player speaks a voice command → Frontend captures audio → Intelligence Layer classifies intent and selects/generates a Lua script → Scripted AI executes that script each tick via ctx API → Backend processes the resulting commands → Frontend renders the outcome.

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

**Two layers — keyword spotting (implemented) and Lua scripting (future):**

**Layer 1 — On-device keyword spotting (`cc_voice` crate, Phase 1 complete):**
- **Model**: TC-ResNet8 (~80-120K params, <300KB ONNX) classifies 1-second mel spectrograms into 31 keyword classes
- **VAD gate**: Silero VAD (~1-2MB ONNX) detects speech before running classifier
- **Audio**: `cpal` → lock-free ring buffer → inference thread → crossbeam channel → Bevy messages. Three-thread architecture: zero frame impact
- **Training**: TTS synthetic (macOS `say` × 8 voices × 5 speeds) + real recordings + augmentation (noise, pitch, speed, SpecAugment)
- **Vocabulary**: 12 command verbs, 4 directions, 4 meta, 6 units, 3 buildings + unknown/silence = 31 classes
- **Latency**: sub-10ms inference, fully offline, PTT on V key
- **Intent mapping**: `stop`/`hold` → `GameCommand::Stop`; parameterized commands (attack, move, build) stubbed for context resolution

**Layer 2 — Lua construct mode + voice buffs (future, per VOICE.md):**
Players vibecode Lua agent scripts in **construct mode** (an in-game LLM-powered scripting environment) and command them by voice during gameplay. Matched Lua scripts run in the WASM sandbox with access to the MCP game tools.

**Voice commands are a core game mechanic:** units touched by a voice-triggered script receive a temporary command-specific buff (e.g., attack → damage buff, retreat → speed/armor buff). This incentivizes voice use over clicking and creates strategic depth around cooldown management and script design.

The meta-game is the vibecoding itself — players use an LLM to generate Lua scripts in construct mode, iterating on their agent loadout. Better scripts = smarter agents = competitive advantage.

### 4. AI Agent Layer (MCP + Fine-tuned Mistral)

> Full technical details, code examples, and training data formats in **[MISTRAL.md](./MISTRAL.md)**.

This is the novel core of ClawedCommand. The AI agent layer is split into three distinct sub-systems that operate at different timescales and abstraction levels:

#### 4a. Scripted AI (runs every tick)

Lua scripts that execute each simulation tick through the `ScriptContext` (ctx) API. These are the actual "brains" that control units in real time.

**Sources of scripts:**
- **Hand-authored starters** — `basic_attack.lua`, `basic_retreat.lua`, `basic_gather.lua`, `basic_build.lua`, `basic_train.lua` ship with the game as examples and defaults
- **Player-created** — written in construct mode (the in-game LLM-powered Lua editor) or by hand
- **Enemy AI FSM** — faction-specific behavior trees implemented as Lua scripts

**Runtime:**
- Sandboxed Luau via `mlua` with instruction-count limits (budget-gated)
- `ScriptContext` exposes 25+ methods: `ctx:nearest_enemy()`, `ctx:attack()`, `ctx:move_to()`, `ctx:units_in_range()`, `ctx:build()`, etc.
- 8 composable behavior primitives in the `behaviors` module
- `SpatialIndex` for efficient spatial queries within scripts
- Scripts produce `GameCommand`s that feed into the same unified command queue as player input

**Key point:** scripts run deterministically at simulation tick rate. They do not call the LLM. They are pure game logic.

#### 4b. Agentic Builder — the Intelligence Layer (runs on player request)

The fine-tuned Devstral model that *generates* Lua scripts and issues strategic commands. This is one layer above the scripted AI — it produces the scripts that the scripted AI then runs.

**Architecture:**

```
Player ──(natural language)──► Agent Interface (Chat UI)
                                      │
                                      ▼
                              Fine-tuned Devstral
                              (understands game state,
                               generates Lua scripts +
                               tool calls)
                                      │
                          ┌───────────┴───────────┐
                          ▼                       ▼
                   MCP Tool Server          Lua Script Output
                   (game-specific tools)    (saved to script library)
                          │                       │
                          ▼                       ▼
                   Command Queue          Scripted AI Layer
                          │              (runs script each tick)
                          ▼                       │
                   ECS Simulation ◄───────────────┘
```

**When it activates:**
- Player types a chat message ("build a forward base near the GPU deposit")
- Player requests a new script in construct mode ("write me a kiting script for Hissers")
- Voice comprehension triggers a script-generation request for a novel intent

**What it produces:**
- Direct `GameCommand`s via MCP tool calls (for immediate one-shot actions)
- Lua scripts (for ongoing behaviors that persist across ticks)
- Script modifications (refining existing scripts based on player feedback)

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

**Inference routing:**
- **Competitive/ranked** — Mistral API, Devstral 2, server-side (fair, anti-cheat)
- **Practice/single-player** — local vLLM or Ollama, Devstral Small 2 Q4_K_M (free, works offline, requires RTX 4090 or Mac 32GB+)

#### 4c. Voice Comprehension (bridges voice to scripts)

Voice comprehension sits in the Intelligence Layer alongside the agentic builder. It translates spoken commands into script activations.

**Pipeline:**
1. `cc_voice` captures audio and runs on-device keyword spotting (TC-ResNet8 + Silero VAD)
2. Classified keywords are mapped to intents (e.g., "attack" + "north" → attack-north intent)
3. Simple intents trigger pre-existing scripts directly (e.g., `basic_attack.lua` with a direction parameter)
4. Complex or ambiguous intents escalate to the agentic builder, which may generate a new script or select the best match from the player's script library

**Key insight:** voice commands and chat commands converge at the same Intelligence Layer. The difference is input modality (audio vs. text), not processing layer. Both ultimately produce Lua scripts or direct commands that flow through the Scripted AI layer into the simulation.

See [VOICE.md](./VOICE.md) for the full keyword spotting technical details.

### 5. Renderer (Isometric — 2D Sprites on 3D Terrain)

**Isometric map rendering:**
- Tile-based map with diamond-shaped tiles
- Smooth camera with zoom (0.5x-3.5x), pan, edge scrolling, minimap
- Isometric 3D terrain is on the table (elevation, camera orbit) — the maps look great

**Units and buildings — 2D sprites (permanent):**
- Sprite-based units and buildings with directional animations
- Depth sorting based on y-position for correct overlap
- Two-tier zoom LOD with hysteresis: Tactical (< 2.0x) shows full sprites/health bars/props; Strategic (>= 2.0x) shows simplified colored-dot icons
- 2x sprite resolution for crisp close-up zoom (drawn at 1x, nearest-neighbor upscaled)
- Do NOT replace with 3D animated models — 2D sprites are the art style

**3D unit/building models (promotional video only):**
- AI-generated GLB models via Tripo Studio (~40 credits/model, ~5K faces after retopo)
- PoC: `crates/cc_client/examples/poc_3d.rs` (Bevy 3D isometric camera, terrain grid, team color tinting)
- Assets: `assets/models/units/pawdler.glb`, generation script: `tools/asset_pipeline/scripts/generate_3d_model.py`
- Tracked in TDL.md under "3D Models (Promotional Video Only)"

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

### 7. Campaign Mutator System

Data-driven mission modifiers that make each campaign mission mechanically unique. Defined in `cc_core::mutator::MissionMutator` enum (RON-serializable), stored as `mutators: Vec<MissionMutator>` in `MissionDefinition`.

**Mutator Categories:**
- **Environmental Hazards**: `LavaRise` (directional tile flooding with `FLAG_LAVA`), `ToxicTide` (shrinking ring with `FLAG_TOXIC`), `Tremors` (random epicenter `FLAG_TEMP_BLOCKED`), `Flooding` (elevation-based water conversion), `WindStorm` (periodic gust windows), `DenseFog` (vision reduction with periodic clearing), `DamageZone` (arbitrary tile damage)
- **Control Restrictions**: `VoiceOnlyControl`, `NoAiControl`, `NoBuildMode`, `AiOnlyControl`, `RestrictedUnits`
- **Gameplay Modifiers**: `TimeLimit`, `ResourceScarcity`, `DamageMultiplier`, `SpeedMultiplier`

**Runtime Resources** (`cc_sim::campaign::mutator_state`):
- `MutatorState` — per-mutator active flags, hazard advance counters, water level
- `ControlRestrictions` — gates input systems (keyboard/mouse, voice, AI, building)
- `FogState` — vision reduction consumed by client rendering

**Systems** (chained after `mission_objective_system` in FixedUpdate):
1. `environmental_hazard_system` — modifies map tile flags per-tick
2. `hazard_damage_system` — applies `ApplyDamageCommand` to units on hazardous tiles
3. `mutator_tick_system` — checks `TimeLimit`, fires `MissionFailedEvent`

**Command Filtering**: Commands carry `CommandSource` (PlayerInput/VoiceCommand/AiAgent/Script). `process_commands` filters based on `ControlRestrictions`. Client input handlers (keyboard, mouse, voice) also gate early via `Option<Res<ControlRestrictions>>`.

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
│   ├── cc_voice/               # Voice command recognition (on-device CNN)
│   │   ├── src/
│   │   │   ├── mel.rs          # Mel spectrogram computation (matches Python pipeline)
│   │   │   ├── vad.rs          # Silero VAD wrapper (speech detection)
│   │   │   ├── classifier.rs   # TC-ResNet8 ONNX keyword classifier
│   │   │   ├── audio.rs        # cpal mic capture → lock-free ring buffer
│   │   │   ├── pipeline.rs     # Three-thread orchestrator (audio → inference → Bevy)
│   │   │   ├── intent.rs       # Keyword → GameCommand mapping
│   │   │   ├── events.rs       # VoiceCommandEvent, VoiceStateChanged messages
│   │   │   └── lib.rs          # VoicePlugin, VoiceConfig, VoiceState
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
│   ├── voice/                  # TC-ResNet8 keyword spotting training pipeline
│   │   ├── config.yaml         # Audio params, vocabulary, hyperparameters
│   │   ├── model.py            # TC-ResNet8 PyTorch model + ONNX export
│   │   ├── generate_tts.py     # macOS TTS synthetic data generator
│   │   ├── augment.py          # Audio augmentation (noise, pitch, speed, SpecAugment)
│   │   ├── dataset.py          # PyTorch Dataset (WAV → mel spectrogram)
│   │   ├── record.py           # CLI recording tool for real samples
│   │   ├── train.py            # Training loop (AdamW + cosine LR + ONNX export)
│   │   └── test_model.py       # Model + pipeline tests
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
| ONNX Runtime distribution size (~20-50MB dylib) | `load-dynamic` feature loads at runtime; voice feature is optional |
| Mel spectrogram Rust/Python mismatch | Cross-validated with reference signals in test fixtures |
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
| Rendering | Bevy 2D isometric (current) → Bevy 3D orthographic (planned, PoC validated) |
| 3D Model Gen | Tripo API (image-to-3D GLB) |
| Networking | Quinn (QUIC) or wtransport |
| Serialization | bincode / serde |
| AI Model (Server) | Devstral 2 123B via Mistral API (`devstral-2-2512`) |
| AI Model (Local) | Devstral Small 2 24B via vLLM/Ollama (`devstral-small-2-2512`) |
| AI Interface | MCP → Mistral function calling (OpenAI-compatible) |
| Fine-tuning | Unsloth + TRL (Qwen/Devstral/xLAM), Mistral API (Codestral) |
| Voice Script Language | Lua (via `rlua`/`mlua` in WASM sandbox) |
| Voice Keywords | TC-ResNet8 CNN via ONNX Runtime + Silero VAD (on-device, sub-10ms) |
| Voice Scripts | Lua construct mode + Mistral agent (future, see VOICE.md) |
| Sandbox | Wasmtime (WASM runtime for strategy scripts + Lua voice scripts) |
| Training | Python + Unsloth + TRL + HuggingFace |
| Asset Authoring | Aseprite (sprites), Tiled (maps) |
| Build | Cargo workspaces |
| CI/CD | GitHub Actions |
