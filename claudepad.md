# Claudepad - ClawedCommand

## Session Summaries
- **2026-03-01T230:00:00Z** — Multi-Faction Ability Implementation + Gen 024 NEW BEST: Implemented 40+ ability effects across all 6 factions. Added 7 new StatusEffectIds (Stunned, Silenced, Entrenched, SpeedBuff, ArmorBuff, DamageBuff, PlayingDead), 3 AuraTypes (WhiskerWeave, SwarmTremorSense, PanopticGaze), 14 tuning constants. command_system.rs: ~40 ability match arms (toggle auras, AoE CC, AoE damage, self-buffs). ability_effect_system.rs: ~20 self-buff bridges. stat_modifier_system.rs: 8 new StatusEffect→StatModifiers mappings. 22 new integration tests (multi_faction_abilities.rs). Gen 024: activate_abilities.lua with TacticalUplink toggle, lower DissonantScreech threshold (2 enemies), aggressive Zoomies (40% HP) = 7 wins, 0 losses, 3 timeouts = 70% decisive, 100% effective dominance. Key: abilities > extra units; TacticalUplink pushed 50%→70%.
- **2026-03-01T220:00:00Z** — P1 Win Bias Fix + Wet Test Observation: Investigated why P1 always won CatGpt mirror matches. Root causes found and fixed: (1) `place_tiered_resources()` hardcoded directional offsets, (2) `find_build_position()` ring scan bias, (3) `CommandQueue` last-mover advantage, (4) AI eval order. BuildingCensus refactored to role-based field names (linter). 561 workspace + 19 wet tests pass.
- **2026-03-01T210:00:00Z** — AI Training Gen 17-23 Combat Micro Breakthrough: Gen 21 BREAKTHROUGH: pure combat_micro.lua = 50% P0 win rate (0% baseline). Gen 22 (abilities + combat) = 0% (interference). Gen 23 (per-unit focus) = 0% (scattered damage). Key: concentrated group focus fire is #1 script behavior.
- **2026-03-01T200:00:00Z** — AI Training Gen 18b + FSM Symmetry Fix: Fixed 4 FSM asymmetries. Gen 18b: activate_abilities.lua + baselines = 80% effective dominance. Key: ability activation > extra unit production.
- **2026-03-01T190:00:00Z** — AI Training Gen 10-15 Breakthrough: Gen 12 BREAKTHROUGH: smart_fill.lua = 53% P0 win rate (7% baseline). Key: idle building production fill is most impactful script behavior.
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
