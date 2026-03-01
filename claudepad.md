# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T230:00:00Z** — Unit Training Flow: Completed plan steps 4-10. Added BUILDER_PROXIMITY tuning constant, build hotkeys (B+sub-keys), training hotkeys (Q/W/E/R/X), bevy_ui resource HUD, Stop/Move build-cancel refunds, 5 new integration tests. MissionStarted refactored to named struct with mission_id for mission change detection. 431 workspace tests pass. Fixed stale build cache issue with wave_spawner.rs.
- **2026-03-01T013:00:00Z** — Web Build Phase W1+W2: Added `native` feature flag to cc_client, gated cc_agent/cc_voice/bevy_egui behind it. All 11 UI modules cfg-gated. Screenshot and filesystem access gated for wasm32. Trunk.toml + index.html in crates/cc_client/. WASM builds to 152MB debug (1GB raw .wasm). Trunk serve running at localhost:8080. 115+34 tests still pass (pre-existing integration test compile errors unrelated).
- **2026-02-28T220:00:00Z** — Wet Test Session #4: Fixed duplicate test fns in mission.rs, Rust 2024 `ref` patterns, WaveTracker missing from campaign tests, WaveEliminated using entity queries instead of WaveTracker resource. Added ability_effect_system + builder_system to harness chain (correctness review finding). Removed redundant Aura insert in command_system.rs. Updated victory test seeds. 431 workspace + 10 wet tests pass, all games produce victories.
- **2026-02-28T210:00:00Z** — Campaign Gaps Phase 1: Core Systems. Registered wave_spawner module. Added PersistentCampaignState, NextMission enum + ai_tool_tier. WaveEliminated uses WaveTracker resource. 8 new wave integration tests + RON validation. 280 tests pass. 4 Act 1 mission RON files validated.
- **2026-02-28T180:00:00Z** — Phase 4B: First 10 Abilities. 10 abilities across 3 activation types. New systems: ability_effect_system, aura_system, 4 deferred commands. 15 new integration tests. 201 workspace tests pass.
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
