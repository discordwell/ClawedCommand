# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T130:00:00Z** — Agent Harness Code Review Fixes: Fixed 3 critical issues from code review: (1) added player_id param to all 8+13 MCP tool param structs, (2) replaced all GameMap::new() with sim.map() using scoped block pattern for borrow safety, (3) renamed distance_between→distance_squared_between. Fixed FactionId::CatGPT hardcoding (25 occurrences) to use from_u8(player_id). Fixed economy.rs borrow checker errors. All 238 tests pass (109 cc_core + 35 cc_sim + 89 cc_agent + 5 cc_harness).
- **2026-02-28T120:00:00Z** — Tiered Agent Tools: Implemented 4-tier tool unlock system (Basic/Tactical/Strategic/Advanced). Created tool_tier.rs, refactored behaviors.rs → behaviors/ module (economy.rs, tactical.rs, strategic.rs), added 10 new behavior primitives. Wired tier-gating into MCP tools, Lua runtime, agent bridge, runner, harness server. 89 cc_agent + 109 cc_core + 5 cc_harness + 80 cc_sim tests pass (1 pre-existing sim failure).
- **2026-02-28T110:00:00Z** — Wet Test Continuation: Fixed rmcp exclusion, SpiteCarryBuff, supply cap test, idle worker regression. 355 tests + 10 wet tests pass.
- **2026-02-28T105:00:00Z** — Campaign System Implementation. Hero system, mission definitions, campaign state machine, dialogue UI, 6 AI personality profiles, prologue mission. 360 tests pass.
- **2026-02-28T99:00:00Z** — Agent Harness Implementation. Extended cc_agent ScriptContext, behaviors.rs with 8 composable primitives, cc_harness crate with HeadlessSim + MCP server. 62 cc_agent + 5 cc_harness tests pass.
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
