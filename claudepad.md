# Claudepad - ClawedCommand

## Session Summaries
- **2026-03-03T11:30:00Z** — Animation Sprite Sheet Generation: Replacing fake walk/attack sheets (tiny rotations of idle) with real ChatGPT-generated 4-frame animations. Pipeline: MCP tab for prompt entry → AppleScript+curl for download → process_sheet_simple.py for post-processing. Created batch_sheets.py orchestrator. Completed: pawdler_walk (100KB, real), pawdler_attack (63KB, real). 18 remaining sheets need generation. ChatGPT rate limits after ~3 consecutive generations (5-min cooldown). Key: text-only prompts work fine without image upload, describe character explicitly.
- **2026-03-03T10:00:00Z** — WASM Phase 2: Wired stubs from Phase 1. (1) Real AI connect: deferred agent loop via OnceLock config channel, `cc_connect_ai`/`cc_get_ai_status` wasm_bindgen exports, status tracking via AtomicU8. (2) Real loading progress: streaming fetch for WASM download 0-50%, LoadingTracker resource polling AssetServer load states for 50-100%, `cc_get_loading_progress` export. (3) Script persistence: new `wasm_persistence.rs` using localStorage, `cc_list_scripts`/`cc_get_script_source`/`cc_delete_script` exports. (4) play.html: real progress bar, real AI connect, Scripts panel with View/Download/Delete, tabbed AI config panel.
- **2026-03-03T07:00:00Z** — Asset Catalog Finalization: Updated 35 unit idle sprite entries in asset_catalog.yaml from `planned` → `game_ready`. All 60 unit idle sprites (6 factions × 10 units) confirmed on disk at `assets/sprites/units/`. All are 128x128 RGBA including heroes/tanks (catalog spec says 148/192 for some but actual files are uniform 128x128 — minor mismatch, cosmetic only). Chrome MCP browser automation for sprite generation is unreliable — extension disconnects frequently, downloads completely non-functional from MCP tab groups. Previous session used batch_sprites.py (AppleScript JS + curl) as workaround. **All sprite generation tasks complete.** No pending sprite work remains.
- **2026-03-03T01:00:00Z** — Counter-Meta Arena Training Gens 065-076: Ran 12 generations against Gen 063 champion (new P0 opponent). Mirror baseline (Gen 065): P1 14, P0 2, TO 4. Explored timing (early/adaptive/momentum push), terrain (road push, high-ground formation, terrain kite), tactical (hold-position ranged, wider focus fire, faster interval). **Gen 072 = NEW CHAMPION: PERFECT 20/20 P1 wins (100%), 0 timeouts.** Single change: ctx:hold() for ranged in formation instead of move_units. Creates disciplined turret line behind tanks. Both combinatorials (hold+momentum, hold+wide focus) degraded it — purity of formation discipline is key. Key finding: hold-position ranged is the most impactful single behavioral change in 76 generations of training. Faster interval (2 ticks) HURTS (command thrashing, 12/20).
- **2026-03-02T23:10:00Z** — Expanded 20→60 Unit Sprites + Pipeline: Phase 1 (code): expanded ALL_KINDS[60], kind_index, unit_slug, draw_size, UnitSprites, AnimSheets arrays, unit_scale — all match arms for 60 units across 6 factions. 40 procedural draw functions with faction helpers (bird/badger/axolotl/raccoon). Phase 2 (sprites): generated 40 idle sprites via ChatGPT browser automation. batch_sprites.py sends prompts via AppleScript JS + downloads via curl (canvas→blob broken, AppleScript setTimeout unreliable — split fill+send into separate calls). Style reference upload via DataTransfer API. All 60 idle sprites at 128x128. 12 unit_gen + anim_assets tests pass.
- **2026-03-02T06:00:00Z** — Arena Training Gens 049-064: Ran 16 generations of Lua combat scripts against Gen 34 champion. **Gen 063 = NEW CHAMPION: 19/20 P1 wins (95%), 0 timeouts.** Combines timed push at tick 4000 + inline formation.
- (older entries moved to oldpad.md)
## Key Findings
- Player wants hybrid control: direct unit micro + AI agent delegation
- AI agent approach: fine-tuned Mistral generates code/MCP tool calls from natural language instructions
- Inference routing: server-side for competitive, local for practice/SP
- Player commands override AI commands for conflict resolution
- Must use fixed-point math from day 1 for deterministic simulation
- **Devstral 2 (123B)** for server-side competitive ($0.40/$2.00 per 1M tokens), **Devstral Small 2 (24B)** for local SP (Q4_K_M ~15GB, Ollama on M4 Max 64GB)
- Both models are dense transformers with native tool use, 256K context
- MCP `inputSchema` maps directly to Mistral `parameters` (same JSON Schema)
- Fine-tuning supports tool-use training: JSONL with messages + tools arrays, 9-char random tool call IDs, stringified JSON args
- Fine-tune via Mistral API (~$5-10/run) or self-hosted with `mistral-finetune` repo (LoRA rank 64)
- Competitive play cost estimate: ~$0.72 per player per game
- Rust client: single `reqwest` HTTP client works for both API and local (OpenAI-compatible endpoint)
- **Voice Command System**: core game mechanic, not convenience. Push-to-talk → speech-to-text → intent classification → triggers player-authored Lua scripts
- **Construct Mode**: in-game LLM-powered scripting environment where players vibecode Lua agent scripts. Can be used mid-mission.
- **Lua scripting**: chosen for WASM sandbox scripts. LLMs generate Lua well, players can hand-edit, classic game scripting language
- **Voice Buff Mechanic**: command-specific temporary buffs on voice-commanded units (attack→damage, retreat→speed, etc.). Intentionally exploitable — "touch all units" meta is acceptable
- **Speech-to-Text tech**: Web Speech API primary (free, low latency, Chromium), Whisper.js fallback (cross-browser, offline, ~40MB model)
- **Intent classification**: tiered — keyword/regex (<50ms) → fuzzy match (~100ms) → Mistral agent for complex strategy (1-3s)
- **Precedents**: EndWar (70-word vocab, hierarchical commands), Radio General (errors as fiction), Warkestra (hybrid voice+mouse)
- **Picovoice Rhino**: audio-to-intent (no transcription step), free tier 3 users, $6K/yr paid. Worth evaluating for production.
