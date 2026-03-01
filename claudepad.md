# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T110:00:00Z** — Wet Test Continuation: Ran wet tests after linter changes. Fixed: cc_harness excluded from workspace (rmcp 72 compile errors, added to TDL.md), missing SpiteCarryBuff match arm in stat_modifier_system, incorrect test_supply_cap_from_buildings (Build command needs builder walk, not instant), idle worker GatherResource regression (re-added after linter removed). Linter also committed stale gatherer detection, deposit proximity checks, tuning constants consolidation, campaign system, AI personalities. 355 workspace tests + 10 wet tests pass. All games produce victories (0 violations).
- **2026-02-28T105:00:00Z** — Campaign System Implementation. Hero system, mission definitions, campaign state machine, dialogue UI, 6 AI personality profiles, prologue mission. 360 tests pass. Pushed as 95464a9.
- **2026-02-28T99:00:00Z** — Agent Harness Implementation. Extended cc_agent ScriptContext, behaviors.rs with 8 composable primitives, cc_harness crate with HeadlessSim + MCP server. 62 cc_agent + 5 cc_harness tests pass.
- **2026-02-28T95:00:00Z** — Wet Test Run & Fixes: Ran full wet test suite, found and fixed 3 issues. All seeds produce victories. 296 workspace tests + 10 wet tests pass.
- **2026-02-28T90:00:00Z** — Wet Test Harness Fixes: Fixed 3 critical issues (dead entity cleanup, elimination counting, AttackMove sight range). 287 workspace tests + 9 wet tests pass.
- **2026-02-28T85:00:00Z** — Phase 4A: Abilities, Buildings, Tech Tree. 258 tests pass.
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
