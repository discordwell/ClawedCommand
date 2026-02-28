# ClawedCommand — Game Creation Plan

> Phases 1-3: from empty repo to the full novel gameplay loop (move, fight, build, vibecode, voice command).
> Phase 1 is actionable (file paths, deps, signatures). Phases 2-3 are feature-level.

**Bevy version:** 0.18.0 (released Jan 2026). Pin in workspace Cargo.toml.
**bevy_ecs_tilemap:** use git branch `0.18` until crates.io catches up.

---

## Phase 1: Core Simulation

**Milestone:** Units moving on an isometric grid via click commands.

### 1.1 Project Scaffolding

- [ ] **Initialize Cargo workspace**
  - Workspace `Cargo.toml` at project root with members: `crates/cc_core`, `crates/cc_sim`, `crates/cc_client`
  - Shared workspace dependencies:
    ```toml
    [workspace.dependencies]
    bevy = { version = "0.18", features = ["2d"] }
    serde = { version = "1", features = ["derive"] }
    fixed = "1"   # fixed-point arithmetic
    ```
  - `cc_core` is a lib crate (no Bevy dependency — pure types + math)
  - `cc_sim` depends on `cc_core` + `bevy`
  - `cc_client` depends on `cc_core` + `cc_sim` + `bevy` — this is the binary crate

- [ ] **Create crate skeletons**
  - `crates/cc_core/src/lib.rs` — re-exports modules
  - `crates/cc_sim/src/lib.rs` — `SimPlugin` that registers all sim systems
  - `crates/cc_client/src/main.rs` — Bevy `App::new()` with `DefaultPlugins` + `SimPlugin`

- [ ] **Basic CI**
  - `.github/workflows/ci.yml`: `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace`
  - `.gitignore` for Rust + Bevy (target/, *.wasm, etc.)

- [ ] **Verify it runs** — `cargo run -p cc_client` opens a blank Bevy window

### 1.2 cc_core: Foundation Types

All files in `crates/cc_core/src/`:

- [ ] **`math.rs` — Fixed-point arithmetic**
  - Type alias: `type Fixed = fixed::FixedI32<fixed::types::extra::U16>` (16.16 format)
  - Helper functions: `fixed_from_f32()`, `fixed_to_f32()`, `fixed_mul()`, `fixed_div()`
  - This is used everywhere the simulation does math — ensures determinism

- [ ] **`coords.rs` — Coordinate system**
  - `GridPos { x: i32, y: i32 }` — logical tile position on the isometric grid
  - `WorldPos { x: Fixed, y: Fixed }` — sub-tile world position (simulation space)
  - `ScreenPos { x: f32, y: f32 }` — pixel position on screen (rendering only)
  - Conversion functions:
    - `grid_to_world(GridPos) -> WorldPos`
    - `world_to_grid(WorldPos) -> GridPos`
    - `world_to_screen(WorldPos, camera: &CameraState) -> ScreenPos`
    - `screen_to_world(ScreenPos, camera: &CameraState) -> WorldPos`
  - Isometric transform: standard diamond projection
    ```
    screen_x = (grid_x - grid_y) * TILE_HALF_WIDTH
    screen_y = (grid_x + grid_y) * TILE_HALF_HEIGHT
    ```

- [ ] **`components.rs` — ECS component definitions**
  ```rust
  // Spatial
  Position { world: WorldPos }
  Velocity { dx: Fixed, dy: Fixed }
  GridCell { pos: GridPos }        // cached grid position, updated from Position

  // Identity
  Owner { player_id: u8 }
  UnitType { kind: UnitKind }
  enum UnitKind { Worker, Infantry }   // expand later

  // Stats
  Health { current: Fixed, max: Fixed }
  MovementSpeed { speed: Fixed }

  // State
  Selected                          // marker component — this unit is selected
  MoveTarget { target: WorldPos }   // unit is moving toward this
  ```

- [ ] **`commands.rs` — GameCommand enum**
  ```rust
  enum GameCommand {
      Move { unit_ids: Vec<Entity>, target: GridPos },
      Stop { unit_ids: Vec<Entity> },
      Select { unit_ids: Vec<Entity> },
      Deselect,
  }
  ```
  Start minimal. Attack, Build, etc. added in Phase 2.

- [ ] **`map.rs` — Map data structures**
  ```rust
  struct TileData { passable: bool, elevation: u8 }
  struct GameMap {
      width: u32,
      height: u32,
      tiles: Vec<TileData>,  // row-major: tiles[y * width + x]
  }
  impl GameMap {
      fn get(&self, pos: GridPos) -> Option<&TileData>;
      fn is_passable(&self, pos: GridPos) -> bool;
      fn neighbors(&self, pos: GridPos) -> Vec<GridPos>;  // 8-directional
  }
  ```

- [ ] **Tests for cc_core**
  - Coordinate round-trips: `grid_to_world` → `world_to_grid` = identity
  - Fixed-point arithmetic: basic operations, no precision loss for game-scale values
  - Map: bounds checking, neighbor generation, passability queries

### 1.3 cc_sim: Simulation Systems

All files in `crates/cc_sim/src/`:

- [ ] **`resources.rs` — Bevy resources**
  ```rust
  #[derive(Resource)]
  struct CommandQueue { commands: Vec<GameCommand> }

  #[derive(Resource)]
  struct SimClock { tick: u64 }

  #[derive(Resource)]
  struct MapResource { map: GameMap }
  ```

- [ ] **`systems/mod.rs` — System registration**
  - `SimPlugin` registers all systems in a `FixedUpdate` schedule
  - Fixed timestep: 10 ticks/second (100ms per tick) via Bevy's `Time<Fixed>`

- [ ] **`systems/command_system.rs` — Process command queue**
  - Drain `CommandQueue` each tick
  - `GameCommand::Move` → set `MoveTarget` component on each unit
  - `GameCommand::Stop` → remove `MoveTarget`, zero `Velocity`
  - `GameCommand::Select` / `Deselect` → add/remove `Selected` marker

- [ ] **`systems/movement_system.rs` — Unit movement**
  - Query entities with `(Position, MoveTarget, MovementSpeed)`
  - Calculate direction vector from current position to target
  - Set `Velocity` = normalized direction * speed
  - If within threshold distance of target → snap to target, remove `MoveTarget`, zero `Velocity`
  - Apply `Velocity` to `Position` each tick

- [ ] **`systems/grid_sync_system.rs` — Sync GridCell from Position**
  - Query `(Position, &mut GridCell)`
  - Recompute `GridCell.pos` from `Position.world` using `world_to_grid()`
  - This keeps the grid overlay in sync with sub-tile movement

- [ ] **`systems/pathfinding.rs` — A* on isometric grid**
  - `fn find_path(map: &GameMap, from: GridPos, to: GridPos) -> Option<Vec<GridPos>>`
  - Standard A* with 8-directional movement
  - Diagonal cost = Fixed::sqrt(2), cardinal cost = 1
  - Heuristic: Chebyshev distance (appropriate for 8-dir)
  - Path is stored on the entity; movement_system follows waypoints
  - Add `Path { waypoints: VecDeque<GridPos>, current_idx: usize }` component

- [ ] **Tests for cc_sim**
  - Command processing: issue Move → entity gets MoveTarget
  - Movement: entity moves toward target over multiple ticks, arrives
  - Pathfinding: finds path around obstacles, returns None when blocked
  - Grid sync: Position change → GridCell updates

### 1.4 cc_client: Renderer & Input

Files in `crates/cc_client/src/`:

- [ ] **`main.rs` — App setup**
  ```rust
  fn main() {
      App::new()
          .add_plugins(DefaultPlugins.set(WindowPlugin {
              primary_window: Some(Window {
                  title: "ClawedCommand".into(),
                  resolution: (1280., 720.).into(),
                  ..default()
              }),
              ..default()
          }))
          .add_plugins(SimPlugin)
          .add_plugins(RenderPlugin)    // our custom rendering
          .add_plugins(InputPlugin)     // our custom input handling
          .add_systems(Startup, setup_game)
          .run();
  }

  fn setup_game(mut commands: Commands) {
      // Spawn camera, load map, spawn initial units
  }
  ```

- [ ] **`renderer/mod.rs` — Isometric rendering plugin**
  - `RenderPlugin` struct implementing Bevy `Plugin`
  - Registers: tilemap rendering system, unit sprite system, selection indicator system, depth sorting system

- [ ] **`renderer/camera.rs` — Camera system**
  - Spawn `Camera2d` at world center
  - Pan: edge scrolling (mouse near screen edge) + middle-click drag + WASD
  - Zoom: scroll wheel, clamp between min/max
  - `CameraState` resource tracking current position + zoom level
  - Screen-to-world coordinate conversion using camera transform

- [ ] **`renderer/tilemap.rs` — Isometric tilemap rendering**
  - Use `bevy_ecs_tilemap` (git branch `0.18`) for efficient tile rendering
  - OR: manual sprite-per-tile approach with placeholder colored diamonds
    - Green = passable, dark gray = impassable, blue = water
  - Tiles rendered at isometric positions using `grid_to_world` → `world_to_screen`
  - Only render tiles visible in camera frustum (basic culling)

- [ ] **`renderer/units.rs` — Unit sprite rendering**
  - Spawn units with `Sprite` components (colored rectangles as placeholders)
  - Color by owner (player = blue, future enemies = red)
  - Sync sprite `Transform` from `Position` each frame (interpolated between sim ticks)
  - Workers slightly smaller than infantry

- [ ] **`renderer/selection.rs` — Selection indicators**
  - Render a green circle/ring under selected units
  - Show move-target indicator (X marker) at the destination when a move is issued

- [ ] **`renderer/depth.rs` — Isometric depth sorting**
  - Set sprite Z-order based on isometric Y position
  - Entities further "south" (higher screen Y) render in front
  - `z = -(world_y + world_x)` ensures correct overlap

- [ ] **`input/mod.rs` — Input handling plugin**
  - `InputPlugin` struct implementing Bevy `Plugin`
  - Registers: mouse click handler, keyboard handler, selection system

- [ ] **`input/mouse.rs` — Mouse interaction**
  - Left-click on unit: select it (add `Selected` component)
  - Left-click on empty ground: deselect all
  - Shift+left-click: add to selection
  - Box select: left-click drag draws a rectangle, selects all units inside
  - Right-click on ground: issue `GameCommand::Move` for selected units to clicked grid position
  - Convert screen coordinates → world → grid for all mouse interactions

- [ ] **`input/keyboard.rs` — Keyboard shortcuts**
  - `S` — Stop selected units
  - `Escape` — Deselect all
  - `1-9` — Control groups (TDL — not implemented in Phase 1, just the keybindings)

- [ ] **`setup.rs` — Initial game state**
  - Generate a small test map (32x32 tiles, some impassable terrain)
  - Spawn 5-10 units at starting positions
  - Spawn the isometric tilemap

### 1.5 Integration & Testing

- [ ] **Playtest loop**
  - Run the game, click to select units, right-click to move them
  - Units pathfind around obstacles on the isometric grid
  - Camera pans and zooms smoothly
  - Selection indicators and move targets display correctly

- [ ] **Determinism test harness**
  - Record a sequence of commands with tick timestamps
  - Replay them twice, assert final positions are identical
  - This validates the simulation is deterministic from day 1

- [ ] **Write integration tests**
  - Headless sim test: spawn units, issue commands, advance ticks, assert positions
  - Pathfinding stress test: random start/end on 64x64 map, verify path validity

---

## Phase 2: Combat & Economy

**Milestone:** Playable skirmish loop — gather, build, fight.

### 2.1 Combat System

- [ ] **Attack command** — add `GameCommand::Attack { unit_ids, target }` and `GameCommand::AttackMove { unit_ids, target_pos }`
- [ ] **Target acquisition** — units auto-acquire targets within attack range if idle or on attack-move
- [ ] **Damage calculation** — `attack_system` runs per tick, applies damage based on `AttackStats { damage, range, cooldown }`
- [ ] **Death & cleanup** — units at 0 HP despawn, play death animation (placeholder), drop debris entity
- [ ] **Projectile system** — ranged units spawn projectile entities with `Velocity` + `Target`, projectile_system handles flight + hit detection
- [ ] **Unit types** — full cat faction roster from GAME_DESIGN.md: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, Catnapper, FerretSapper, MechCommander
- [ ] **Combat tests** — unit attacks target, target takes damage, target dies, attacker re-acquires

### 2.2 Resource System

- [ ] **Resource deposits** — fish ponds, berry bushes (Food), old-world tech ruins (GPU Cores), Monkey Mines (NFTs) as entities with `ResourceDeposit { resource_type, amount }`
- [ ] **Resource tracking** — `PlayerResources { food: Fixed, gpu_cores: Fixed, nfts: u32, supply: u32, supply_cap: u32 }` resource per player
- [ ] **Gathering** — `GameCommand::GatherResource`, Pawdler moves to deposit → gathers over time → returns to nearest Fish Market → deposits → repeats
- [ ] **Spending** — building/training costs deducted from PlayerResources, commands rejected if insufficient
- [ ] **Monkey Mines** — neutral objective structures guarded by feral Monkeys, capture for passive NFT generation
- [ ] **HUD** — display resource counts (Food, GPU Cores, NFTs, supply/cap) at top of screen

### 2.3 Building System

- [ ] **Building placement** — `GameCommand::Build { building_type, position }`, ghost preview while placing, validity check (passable, no overlap, near existing buildings)
- [ ] **Construction** — Pawdler moves to site, building spawns as "under construction" with health ticking up over time
- [ ] **Production queues** — `ProductionQueue` component on Cat Tree/Server Rack, `GameCommand::QueueUnit`, trains units over time, spawns at rally point
- [ ] **Rally points** — `GameCommand::SetRallyPoint`, newly produced units auto-move to rally
- [ ] **Building types** — implement the tech tree skeleton from ARCHITECTURE.md: The Box, Cat Tree, Fish Market, Server Rack, Scratching Post, Litter Box, Cat Flap, Laser Pointer

### 2.4 Tech Tree

- [ ] **Prerequisites** — buildings require other buildings to be constructed first (e.g., Scratching Post requires Cat Tree, Server Rack requires The Box)
- [ ] **UI** — tech tree display panel showing what's available, what's locked, what's in progress
- [ ] **Unit unlocks** — advanced unit types gated behind tech buildings (e.g., Mech Commander requires Scratching Post)

### 2.5 Fog of War

- [ ] **Visibility grid** — per-player bitfield tracking which tiles are visible, explored, or unexplored
- [ ] **Vision range** — units and buildings have `VisionRange` component, update visibility each tick
- [ ] **Rendering** — darken unexplored tiles, dim explored-but-not-visible tiles, fully render visible tiles
- [ ] **Enemy hiding** — enemy entities outside fog are hidden from rendering and from game queries

### 2.6 Basic AI Opponent

- [ ] **Scripted AI** — simple state machine opponent for single-player testing: build workers → gather → build army → attack
- [ ] **Difficulty tiers** — Easy (slow, dumb), Medium (reasonable), Hard (fast expand + aggression)
- [ ] **This doubles as replay training data** — recording scripted AI games provides bootstrap fine-tuning data

### 2.7 UI Expansion

- [ ] **Unit info panel** — shows selected unit stats (HP, attack, speed, type)
- [ ] **Building info panel** — shows production queue, rally point
- [ ] **Minimap** — small overview of the full map in corner, click to jump camera
- [ ] **Command card** — context-sensitive buttons based on selection (move, attack, stop, hold, build menu)

### 2.8 Audio Foundation

- [ ] **Sound effects** — attack sounds, movement sounds, building placement, unit selection acknowledgments
- [ ] **Ambient** — background music loop, environment ambiance
- [ ] **Spatial audio** — sounds positioned relative to camera (louder when zoomed in to action)

### 2.9 Replay System

- [ ] **Recording** — serialize all `GameCommand`s with tick timestamps to a replay file
- [ ] **Playback** — load replay, feed commands into simulation at correct ticks, render as normal
- [ ] **This is a prerequisite for the Phase 4 fine-tuning pipeline** — replays become training data

---

## Phase 3: AI Agent + Voice Commands

**Milestone:** Vibecode a Lua script in construct mode, issue a voice command, watch the agent execute it with a buff.

### 3.1 MCP Tool Server (`cc_agent`)

- [ ] **Define MCP tools** — implement the 13 tools from ARCHITECTURE.md as Rust functions that query/mutate ECS state
- [ ] **Tool serialization** — each tool returns JSON-serializable results (unit lists, resource counts, status confirmations)
- [ ] **Fog-of-war filtering** — query tools only return data the player can see (no cheating through the agent)
- [ ] **Rate limiting** — cap tool calls per game tick to prevent AI from overwhelming the simulation

### 3.2 Mistral Inference Client (`cc_agent`)

- [ ] **`inference.rs`** — `MistralClient` with `remote()` and `local()` constructors per MISTRAL.md
- [ ] **Tool call loop** — send messages + tools → receive tool_calls → execute → return results → repeat up to 10 rounds
- [ ] **Streaming** — stream partial responses for the agent chat UI
- [ ] **Error handling** — timeout, malformed tool calls, API errors → graceful degradation with player notification
- [ ] **Configuration** — model selection, base URL, API key, temperature — read from config file

### 3.3 Agent Chat UI (`cc_client`)

- [ ] **Chat panel** — collapsible side panel showing conversation history with the AI agent
- [ ] **Input box** — text input for typing instructions to the agent
- [ ] **Tool call display** — show what tools the agent is calling (collapsed by default, expandable)
- [ ] **Status indicators** — "thinking...", "executing...", "done" states
- [ ] **Agent mode toggle** — per-unit or per-group toggle between player control and AI delegation

### 3.4 Command Dispatcher Integration

- [ ] **Merge three command sources** — player clicks, AI agent tool calls, voice script execution → all flow into `CommandQueue`
- [ ] **Priority system** — player overrides AI overrides voice for the same units
- [ ] **Command attribution** — track which source issued each command (for replay data + UI feedback)

### 3.5 Lua Script Runtime (`cc_voice`)

- [ ] **Lua engine** — integrate `mlua` crate with Bevy, configure sandbox (no os/io/debug libraries)
- [ ] **Script API** — expose `ctx` object with all methods from VOICE.md Script API table
- [ ] **ctx → MCP bridge** — Lua `ctx:move_units()` calls translate to the same `GameCommand` as the MCP tools
- [ ] **Resource limits** — bounded execution time (e.g., 10ms per script invocation), bounded memory
- [ ] **Error handling** — Lua runtime errors caught and displayed as HUD notifications, never crash the game
- [ ] **Script loading** — load `.lua` files from player's script library directory, hot-reload on change

### 3.6 Voice Input Pipeline (`cc_voice`)

- [ ] **Speech-to-text abstraction** — trait with Web Speech API implementation and Whisper.js fallback
- [ ] **Push-to-talk** — bind a key (e.g., `V`), start recognition on press, stop on release
- [ ] **Interim results** — show partial transcription on HUD as player speaks
- [ ] **Intent classifier — Tier 1** — keyword/regex match against registered script intents + synonym lists
- [ ] **Intent classifier — Tier 2** — fuzzy match using Levenshtein distance for speech recognition errors
- [ ] **Intent classifier — Tier 3** — fall through to Mistral agent for unrecognized complex commands
- [ ] **Contextual narrowing** — bias intent matching based on current selection (unit selected → unit commands)

### 3.7 Voice Command Buff System (`cc_voice`)

- [ ] **VoiceCommandBuff component** — as specified in VOICE.md (buff_type, magnitude, remaining_ticks, source_intent)
- [ ] **voice_buff_system** — apply modifiers to stat calculations, tick down duration, remove expired buffs
- [ ] **Buff application** — when a voice script executes, all units touched by tool calls receive the appropriate buff
- [ ] **Visual feedback** — buff icon/glow on affected units, HUD notification showing buff applied
- [ ] **Cooldown system** — per-intent cooldown preventing voice command spam
- [ ] **Balance tuning** — start conservative (small magnitude, short duration), expose as config values for easy iteration

### 3.8 Construct Mode (`cc_voice` + `cc_client`)

- [ ] **Construct mode state** — toggle in/out via hotkey, game continues running (intentional risk/reward)
- [ ] **Script editor panel** — display current script with Lua syntax highlighting
- [ ] **LLM chat** — player describes desired behavior in natural language, LLM generates Lua script
- [ ] **Script iteration** — player can say "change X" and LLM edits the existing script
- [ ] **Intent binding** — UI to map script to voice intent keywords
- [ ] **Script library** — browse, rename, delete, duplicate saved scripts
- [ ] **Test runner** — simulate a voice command against current game state without actually executing it
- [ ] **Starter scripts** — ship the 5 starter scripts from VOICE.md (basic_attack, basic_retreat, basic_build, basic_gather, basic_train)

### 3.9 Integration Testing

- [ ] **End-to-end agent test** — issue natural language instruction → Mistral generates tool calls → commands execute in simulation → verify game state changed correctly
- [ ] **End-to-end voice test** — simulate speech input → intent classification → Lua script execution → commands execute → buff applied → verify
- [ ] **Construct mode test** — generate a script via LLM → bind to intent → trigger by voice → verify behavior
- [ ] **Latency profiling** — measure end-to-end time from player instruction to visible game action for both agent and voice paths

---

## Dependency Graph

```
Phase 1                          Phase 2                          Phase 3
────────                         ────────                         ────────
1.1 Scaffolding                  2.1 Combat ──────────────────┐
 │                                │                           │
 ▼                               2.2 Resources                │
1.2 cc_core types                 │                           │
 │                               2.3 Buildings                ├──► 3.1 MCP Tools
 ▼                                │                           │     │
1.3 cc_sim systems               2.4 Tech Tree                │     ▼
 │                                │                           │   3.2 Inference Client
 ▼                               2.5 Fog of War ──────────────┤     │
1.4 cc_client render+input        │                           │     ▼
 │                               2.6 Scripted AI              │   3.3 Agent Chat UI
 ▼                                │                           │     │
1.5 Integration tests            2.7 UI Expansion             │     ▼
                                  │                           │   3.4 Command Dispatcher
                                 2.8 Audio                    │
                                  │                           │
                                 2.9 Replay System ───────────┘   3.5 Lua Runtime
                                                                   │
                                                                  3.6 Voice Pipeline
                                                                   │
                                                                  3.7 Buff System
                                                                   │
                                                                  3.8 Construct Mode
                                                                   │
                                                                  3.9 Integration Tests
```

## Key Dependencies & Crates

| Crate | Purpose | Used In |
|-------|---------|---------|
| `bevy` 0.18 | Game engine | cc_sim, cc_client |
| `bevy_ecs_tilemap` (git 0.18) | Efficient isometric tilemap | cc_client |
| `fixed` | Fixed-point arithmetic for determinism | cc_core |
| `serde` / `bincode` | Serialization | cc_core, cc_net |
| `mlua` | Lua scripting runtime | cc_voice |
| `reqwest` | HTTP client for Mistral API | cc_agent |
| `tokio` | Async runtime for inference calls | cc_agent |
| `wasmtime` | WASM sandbox for scripts | cc_voice, cc_agent |
| `rodio` / `bevy_audio` | Audio playback | cc_client |

## Open Decisions (to resolve during implementation)

1. **Tilemap approach**: `bevy_ecs_tilemap` git branch vs. manual sprite-per-tile. Try the plugin first; fall back to manual if the 0.18 branch is unstable.
2. **Pathfinding crate**: Write A* from scratch (simple, deterministic) vs. use `pathfinding` crate. Recommend writing it — it's ~100 lines and we control determinism.
3. **Construct mode LLM**: Same Devstral model as the agent, or a separate code-gen model? Start with the same model; if Lua quality is poor, evaluate Codestral.
4. **Native vs. WASM client**: Phase 1-3 target native desktop. WASM build for browser is a Phase 6 stretch goal. Voice/speech APIs need platform abstraction either way.
5. **Multiplayer architecture**: Lockstep vs. client-server. Defer final decision to Phase 5 but design for lockstep (stricter determinism requirements = better code).
