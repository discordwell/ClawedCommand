# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T210:00:00Z** — Campaign Gaps Phase 1: Core Systems. Registered wave_spawner module (WaveTracker, MissionStarted, wave_spawner_system, wave_tracking_system). Added PersistentCampaignState (Act3Choice, PatchesStatus, flags). Added NextMission enum + ai_tool_tier to MissionDefinition. Added PersistentFlag condition + SetPersistentFlag action. Fixed WaveMembership→WaveMember rename. WaveEliminated uses WaveTracker resource. Fixed DreamSiege `attack` typo. 8 new wave integration tests + RON validation test. All 280 tests pass (120 cc_core + 36 cc_sim + 43 campaign + 66 integration + 15 abilities). 4 Act 1 mission RON files validated. Updated CAMPAIGN_GAPS.md + TDL.md.
- **2026-02-28T180:00:00Z** — Phase 4B: First 10 Abilities. Implemented all 10 abilities across 3 activation types. New systems: ability_effect_system (bridge), full aura_system, 4 deferred commands (ApplyStatusCommand, AoeCcCommand, RevulsionAoeCommand, + existing ApplyDamage). 15 new integration tests in separate file (phase4b_abilities.rs) to avoid linter interference. 201 workspace tests pass + 1 pre-existing campaign failure. Also fixed linter-introduced fsm.rs BuildOrder query mismatch.
- **2026-02-28T140:00:00Z** — Wet Test Session #3: Fixed linter-introduced regressions. Pushed commit 829c3cf. Ongoing: linter persistently toggles PersistentFlag enum variants.
- **2026-02-28T130:00:00Z** — Agent Harness Code Review Fixes: Fixed 3 critical issues from code review. All 238 tests pass.
- **2026-02-28T120:00:00Z** — Tiered Agent Tools: 4-tier tool unlock system. 89 cc_agent + 109 cc_core + 5 cc_harness + 80 cc_sim tests pass.
- **2026-02-28T110:00:00Z** — Wet Test Continuation: Fixed rmcp exclusion, SpiteCarryBuff, supply cap test, idle worker regression. 355 tests + 10 wet tests pass.
- **2026-02-28T105:00:00Z** — Campaign System Implementation. Hero system, mission definitions, campaign state machine, dialogue UI, 6 AI personality profiles, prologue mission. 360 tests pass.
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
