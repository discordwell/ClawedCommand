# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T60:00:00Z** — Standard RTS Controls: Filled control gaps. Gutted duplicate Bevy text HUD (kept egui panels). Added building info panel (HP, construction %, production queue, rally point). Cancel production button + queue status in command card. Rally point via right-click (building selected → right-click ground). Rally auto-move (trained units walk to rally via MoveTarget). Rally flag visual (green diamond when building selected). Double-click select all of same UnitKind (300ms window, visible units only). 203 tests pass (45+14+61+19+41+23). Code reviewed twice.
- **2026-02-28T55:00:00Z** — Wet Test Harness: Implemented AI-vs-AI battle harness with voice integration. Feature-gated behind `harness`. New harness module (mod.rs, snapshot.rs, minimap.rs, invariants.rs, report.rs) in cc_sim. Multi-player AI (MultiAiState, BotPersonality, multi_ai_decision_system). Headless minimap PNG rendering, game state snapshots, invariant checking (health/bounds/supply/friendly-fire/stuck), JSON reports. Voice injection tests (stop/hold/gather). Binary runner outputs to wet_tests/. 9 wet tests + 201 workspace tests pass. Bevy 0.18 gotcha: World::query() needs &mut World. Voice resolution duplicated inline (circular dep with cc_voice).
- **2026-02-28T50:00:00Z** — Phase E Completion + Full Wiring: Completed vibecode interface (LLM+Lua). Fixed mlua 0.10 API (set_interrupt/sandbox instead of set_hook), bevy_egui 0.39 ctx_mut() returns Result. Wired cc_agent into cc_client: AgentPlugin, construct_mode UI, agent_chat UI, egui input priority blocking. Fixed multi_ai_decision_system reference, harness module feature-gating. 201 tests pass (45+12+61+60+23). All 5 phases complete. Code reviewed twice.
- **2026-02-28T45:00:00Z** — Phase 2 Completion: Playable RTS Loop. Added starting base (TheBox per player, 4 Pawdlers + 2 Nuisance), building visuals (spawn_building_visuals + sync_building_sprites), unit visual spawner (spawn_unit_visuals fixes invisible trained-unit bug), building selection in mouse input (1.2 radius), building placement mode (BuildPlacement InputMode + ghost preview), build buttons in command card (CatTree/FishMkt/LitterBox), reactive scripted AI FSM (EarlyGame→BuildUp→MidGame→Attack→Defend with BotPersonality + AiDifficulty), victory/defeat conditions (victory_system checks TheBox survival), game over overlay (egui VICTORY/DEFEAT), GameState run_if on main sim chain, SpawnPositions resource. 12 new integration tests (economy loop, train unit, build+train loop, victory/defeat, supply cap). 161 tests pass (61+19+41+45+12+23). Code reviewed twice.
- **2026-02-28T40:00:00Z** — Code review bug fixes: Fixed 6 bugs from dual code review. CRITICAL: fog shared one material (all tiles same alpha) → per-tile materials; fog Z=-5 behind units → Z=100. HIGH: selection overwrote dead sprite color → Without<Dead> filter; setup_game in PreStartup missed sprites → moved to Startup with ordering. MODERATE: death scale drift → stored original in DeathTimer; props Z overlap → adjusted. Also incorporated linter additions: buildings renderer, expanded egui UI modules, cc_agent crate, victory system. 121 tests pass. Pushed to GitHub.
- (older entries moved to oldpad.md)
## Key Findings
- Player wants hybrid control: direct unit micro + AI agent delegation
- AI agent approach: fine-tuned Mistral generates code/MCP tool calls from natural language instructions
- Inference routing: server-side for competitive, local for practice/SP
- Player commands override AI commands for conflict resolution
- Must use fixed-point math from day 1 for deterministic simulation
- **Devstral 2 (123B)** for server-side competitive ($0.40/$2.00 per 1M tokens), **Devstral Small 2 (24B)** for local SP (Q4_K_M ~14GB, fits RTX 4090)
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
