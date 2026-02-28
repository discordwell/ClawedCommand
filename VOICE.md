# Voice Command System

> Players vibecode Lua agent scripts and command them by voice during gameplay. Voice-commanded units receive temporary command-specific buffs, making voice a core game mechanic — not just a convenience input.

---

## Overview

The voice command system has three interlocking layers:

1. **Construct Mode** — In-game LLM-powered Lua scripting environment
2. **Voice Input Pipeline** — Push-to-talk speech recognition with tiered intent classification
3. **Command Buffs** — Temporary stat bonuses on voice-commanded units

```
┌─────────────────────────────────────────────────────────────────┐
│                     CONSTRUCT MODE (Pre-game / Mid-mission)     │
│                                                                 │
│  Player ──(natural language)──► LLM ──► Lua Script              │
│  "build me a script that grabs the nearest idle builder         │
│   and constructs whatever building I name"                      │
│                                                                 │
│  Scripts saved to player's script library                       │
│  Scripts declare which voice intents they handle                │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                     VOICE INPUT PIPELINE (During gameplay)       │
│                                                                 │
│  [Push-to-Talk Key] ──► [Speech-to-Text] ──► [Intent Classify]  │
│                                                      │          │
│                              ┌────────────────────────┤          │
│                              ▼                        ▼          │
│                     Tier 1: Keyword Match    Tier 3: Mistral     │
│                     (<50ms, exact)           Agent (1-3s,        │
│                              │               complex strategy)   │
│                              ▼                        │          │
│                     Tier 2: Fuzzy Match               │          │
│                     (~100ms, tolerant)                 │          │
│                              │                        │          │
│                              ▼                        ▼          │
│                     ┌─────────────────────────────────┐          │
│                     │  Matched Lua Script Execution   │          │
│                     │  (WASM sandbox, MCP tool access) │          │
│                     └────────────┬────────────────────┘          │
│                                  │                               │
└──────────────────────────────────┼───────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────┐
│                     COMMAND BUFF SYSTEM                          │
│                                                                 │
│  Voice-triggered commands apply temporary buffs:                │
│    Attack  → damage buff       Retreat → speed/armor buff       │
│    Build   → construction speed  Patrol → vision range buff     │
│    Gather  → gathering speed     Hold   → defense buff          │
│                                                                 │
│  Buff applied to all units touched by the script execution      │
│  Duration: N ticks (tunable per command type)                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Construct Mode

The in-game scripting environment where players create and edit their Lua agent scripts using an LLM.

### Player Experience

1. Player enters construct mode (hotkey or menu)
2. Chat-style interface appears — player describes what they want in natural language
3. LLM generates a Lua script targeting the game's MCP tool API
4. Player can test, tweak, rename, and save the script
5. Player binds the script to a voice intent (or the script declares its own intent bindings)

### When It's Available

- **Pre-mission:** Full access, no time pressure. Build your script loadout.
- **Mid-mission:** Available but risky — the game doesn't pause. Strategic tradeoff: spend time vibecoding a new script vs. managing units manually.

### Script Structure

Scripts are Lua modules that declare their intent bindings and implement a handler:

```lua
-- build_nearest.lua
-- Voice intent: "build"
-- Description: Finds the nearest idle builder within the player's base
-- and constructs the requested building type.

return {
    intents = { "build" },

    handle = function(ctx, args)
        local building_type = args.target  -- e.g. "power plant"

        -- Query idle builders inside base perimeter
        local builders = ctx:get_units({
            type = "worker",
            status = "idle",
            near = ctx:get_buildings({ type = "command_center" })[1].position,
            radius = 30
        })

        if #builders == 0 then
            ctx:notify("No idle builders available")
            return
        end

        -- Pick closest builder to a valid build site
        local site = ctx:find_build_site(building_type)
        if not site then
            ctx:notify("No valid site for " .. building_type)
            return
        end

        local closest = ctx:nearest(builders, site)
        ctx:move_units({ closest.id }, site)
        ctx:build(building_type, site)
        ctx:notify("Building " .. building_type .. " with " .. closest.name)
    end
}
```

### Script API

Scripts access the game through a context object (`ctx`) that wraps the MCP tool layer:

| Method | Description |
|--------|-------------|
| `ctx:get_units(filter)` | Query own units by type, status, location, radius |
| `ctx:get_buildings(filter)` | Query own buildings |
| `ctx:get_visible_enemies()` | Visible enemy units and buildings |
| `ctx:get_resources()` | Current resource counts |
| `ctx:get_map_info(region)` | Terrain data for a region |
| `ctx:move_units(ids, target)` | Issue move command |
| `ctx:attack_units(ids, target)` | Issue attack command |
| `ctx:build(type, position)` | Place a building |
| `ctx:train_unit(building, type)` | Queue unit production |
| `ctx:set_rally_point(building, pos)` | Set rally point |
| `ctx:patrol(ids, waypoints)` | Set patrol route |
| `ctx:gather_resource(workers, deposit)` | Send workers to gather |
| `ctx:find_build_site(building_type)` | Find valid placement near base |
| `ctx:nearest(units, position)` | Return unit closest to position |
| `ctx:notify(message)` | Show HUD notification to player |

Scripts run in the WASM sandbox (Wasmtime) with resource limits — bounded execution time, bounded memory, no filesystem or network access.

### Starter Scripts

Players begin with a basic set of scripts to learn from and use immediately:

- **basic_attack.lua** — Attack-move selected units toward a target
- **basic_retreat.lua** — Pull selected units back toward command center
- **basic_build.lua** — Build a named structure with the nearest idle worker
- **basic_gather.lua** — Send idle workers to the nearest resource deposit
- **basic_train.lua** — Queue a unit type at the appropriate production building

These are intentionally simple. The game encourages players to improve them or build new ones in construct mode.

---

## Voice Input Pipeline

### Speech-to-Text

**Primary: Web Speech API**
- Free, zero dependencies, ~200-500ms latency for short phrases
- Works on Chromium browsers (Chrome, Edge, Opera) — the expected target for Bevy WASM builds
- Push-to-talk mode avoids timeout issues and false positives from game audio
- `interimResults: true` allows showing partial transcription on the HUD as the player speaks

**Fallback: Browser-local Whisper (Transformers.js)**
- `whisper-tiny.en` model, ~40MB quantized, runs entirely in-browser via WASM/WebGPU
- Cross-browser (any browser with WebAssembly support)
- Works offline — no server dependency
- Higher latency (~0.5-1.5s for short clips) but acceptable for push-to-talk flow
- Loaded on demand, only if Web Speech API is unavailable

**Production consideration: Picovoice Rhino**
- Audio-to-intent engine — skips transcription entirely, goes from audio straight to structured intent
- Extremely fast and accurate for constrained vocabularies
- Free tier: 3 active users/month. Paid: starts at $6K/year.
- Worth evaluating once the command vocabulary stabilizes. Define a context grammar matching the game's intents and Rhino outputs `{intent: "build", slots: {target: "power plant"}}` directly from audio.

### Intent Classification

Once speech is transcribed to text, a tiered pipeline maps it to a script intent:

**Tier 1 — Keyword/Regex Match (<50ms)**

Fast exact matching against known command vocabulary. Handles the common case.

```
"build a power plant"   → intent: build,   target: "power plant"
"attack the north base" → intent: attack,  target: "north base"
"retreat"               → intent: retreat, target: null
"hold position"         → intent: hold,    target: null
```

Synonym lists map variations to canonical intents:
- attack, strike, hit, engage, fire → `attack`
- retreat, fall back, pull back, withdraw → `retreat`
- build, construct, place, make → `build`
- move, go, advance, march → `move`

**Tier 2 — Fuzzy Match (~100ms)**

Catches speech recognition errors using Levenshtein distance. "Atack" → "attack", "retreet" → "retreat". Also normalizes verb forms ("attacking" → "attack") using lightweight NLP.

**Tier 3 — Mistral Agent (1-3s)**

Unrecognized commands fall through to the existing Mistral chat agent. This handles complex, multi-step strategic instructions that don't map to a single script:

- "Send cavalry to flank from the north while infantry holds the bridge"
- "Set up a defensive perimeter around the refinery with overlapping fields of fire"

These get processed through the full agent pipeline (Mistral → MCP tool calls → command queue) as designed in the core architecture.

### Contextual Narrowing

Following EndWar's design, the intent classifier narrows its search space based on game state:

- If units are selected → bias toward unit commands (attack, move, patrol, hold)
- If a building is selected → bias toward production commands (train, set rally, cancel)
- If in build mode → bias toward building types
- If no selection → bias toward global commands (select all, group commands)

This reduces ambiguity and improves recognition accuracy.

---

## Command Buff System

Voice commands apply temporary stat buffs to all units touched by the script execution. This is the core incentive for using voice over clicking.

### Buff Types

| Voice Intent | Buff | Effect | Rationale |
|-------------|------|--------|-----------|
| Attack | Damage | +X% damage for N ticks | Inspired assault |
| Retreat | Speed + Armor | +X% move speed, +Y% damage resist | Disciplined withdrawal |
| Build | Construction Speed | +X% build rate | Focused effort |
| Gather | Gather Rate | +X% resource income | Motivated workers |
| Patrol | Vision Range | +X% sight radius | Heightened alertness |
| Hold Position | Defense | +X% damage resist, +Y% range | Dug-in bonus |
| Move | Speed | +X% move speed | Double-time march |

### ECS Representation

```rust
#[derive(Component)]
struct VoiceCommandBuff {
    buff_type: BuffType,
    magnitude: FixedPoint,    // Percentage modifier
    remaining_ticks: u32,     // Countdown to expiration
    source_intent: String,    // Which voice command triggered this
}

enum BuffType {
    Damage,
    Speed,
    Armor,
    ConstructionSpeed,
    GatherRate,
    VisionRange,
    Defense,
}
```

A new `voice_buff_system` runs each tick to:
1. Apply active buff modifiers to relevant stat calculations
2. Decrement `remaining_ticks`
3. Remove expired buffs

### Balance Considerations

- **Buff duration and magnitude are tunable per command type.** Start conservative and adjust based on playtesting.
- **Cooldown or charge system** prevents spam. Options:
  - Global cooldown: one voice command every N seconds
  - Per-intent cooldown: each command type has its own cooldown
  - Charge-based: accumulate charges over time, spend one per voice command
- **The "touch all units" exploit is intentional.** A player who writes a script that briefly touches every unit to apply a shallow buff is spending their voice command cooldown on a broad, shallow bonus instead of a deep targeted one. This is a valid strategic tradeoff.
- **Multiplayer:** Script complexity limits (execution time, instruction count) prevent scripts from becoming an unfair advantage through sheer computational power. The buff system itself is deterministic and reproducible.

---

## Integration with Existing Architecture

### New Crate: `cc_voice`

```
crates/
├── cc_voice/
│   ├── src/
│   │   ├── speech.rs        # Speech-to-text abstraction (Web Speech API + Whisper fallback)
│   │   ├── intent.rs        # Tiered intent classification pipeline
│   │   ├── lua_runtime.rs   # Lua script execution within WASM sandbox
│   │   ├── buff.rs          # VoiceCommandBuff component and voice_buff_system
│   │   ├── construct.rs     # Construct mode UI state and LLM integration
│   │   └── lib.rs
│   └── Cargo.toml
```

### Construct Mode UI: `cc_client`

Construct mode extends the client UI with:
- Script editor panel (code display with syntax highlighting)
- LLM chat interface (reuses agent chat UI patterns)
- Script library browser (list saved scripts, rename, delete, duplicate)
- Intent binding configuration (map intents to scripts)
- Test runner (simulate a voice command against current game state)

### Command Flow Integration

Voice commands feed into the same `GameCommand` queue as player clicks and the Mistral agent:

```
Voice Input ──► Intent Classifier ──► Lua Script ──► GameCommand queue ──► ECS
                                           │
                                           ▼
                                    VoiceCommandBuff applied
                                    to touched units
```

Player click commands still override voice-script commands for the same units (player always has final say, consistent with existing architecture).

### Processing Order Update

The `voice_buff_system` slots into the existing tick order after `ai_command_system`:

1. `input_system` — player commands
2. `ai_command_system` — Mistral agent commands
3. **`voice_script_system`** — voice-triggered Lua script commands
4. **`voice_buff_system`** — apply/tick/expire voice command buffs
5. `production_system` — build queues, unit spawning
6. _(remaining systems unchanged)_

---

## Technology Recommendations

| Component | Recommended | Fallback | Notes |
|-----------|-------------|----------|-------|
| Speech-to-text | Web Speech API | Whisper.js (Transformers.js) | Primary is free + fast; fallback adds ~40MB for offline/cross-browser |
| Intent classification | Keyword/regex + fuzzy match | Mistral agent (Tier 3) | Fast path handles 80%+ of commands; Mistral handles the rest |
| Script language | Lua | — | LLMs generate good Lua, players can hand-edit, classic game scripting choice |
| Lua runtime | `rlua` or `mlua` (Rust Lua bindings) in WASM sandbox | — | Integrates with existing Wasmtime sandbox architecture |
| Audio-to-intent (production) | Picovoice Rhino | — | Evaluate once command vocab stabilizes; replaces STT + Tier 1/2 classification in one step |

---

## Design Precedents

**Tom Clancy's EndWar (2008)** — 70-word vocabulary with hierarchical, composable commands (`Unit 1 → attack → hostile 2`). Key insight: narrow the recognition space based on current selection context. Players could play the entire game voice-only.

**Radio General (2020)** — RTS where you radio orders to units. Recognition errors feel like radio static and miscommunication — the game's fiction absorbs imperfect recognition gracefully. ClawedCommand could adopt a similar approach where garbled commands produce partial or degraded script execution rather than failure.

**Warkestra (2024)** — Hybrid voice + mouse/keyboard control. Players speak troop commands while using mouse for character movement. Closest existing precedent to ClawedCommand's vision.

---

## Open Questions

- **Construct mode LLM:** Does the construct mode LLM use the same Mistral model as the agent, or a separate code-generation-focused model? A code model may produce better Lua.
- **Script sharing:** Can players share scripts? Import from a marketplace? This is listed in Phase 6 of the main architecture but has implications for the script format and sandboxing.
- **Voice feedback:** Should the game acknowledge commands with synthesized speech (Web Speech Synthesis API)? Or is HUD text + unit response animation sufficient?
- **Buff stacking:** Can multiple voice command buffs stack on the same unit? If so, is there a cap?
- **Native client:** If ClawedCommand ships a native (non-WASM) client, the Web Speech API isn't available. The Whisper fallback covers this, but the native path should be planned.
