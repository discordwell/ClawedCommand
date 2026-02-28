# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T33:00:00Z** — Voice pipeline bugfixes from code review. Fixed 3 critical bugs: partial ring buffer read data loss (persisted chunk_offset across iterations), short utterances dropped (now classified via zero-padding when speech ends), mel padding mismatch (Python now uses log(eps) like Rust). Also fixed vad.rs test_stub cfg gating. Added 6 terrain integration tests + 64x64 default map. 102 tests pass (50+16+16+8+12). Committed & pushed.
- **2026-02-28T32:00:00Z** — Phase 2 complete: Bevy 0.15→0.18 migration. Fixed apply_deferred→ApplyDeferred, EventReader/Writer→MessageReader/Writer, OrthographicProjection→Projection enum, get_single→single, WindowResolution (u32,u32). Also fixed ort 2.0 API (inputs! macro, try_extract_tensor tuple, &mut Session::run) and cpal::Stream !Send via Box::leak. 73 tests pass (35+16+10+12). Code reviewed (correctness: all pass, no issues). Committed & pushed.
- **2026-02-28T30:00:00Z** — Implemented on-device voice recognition (cc_voice crate + Python training pipeline). TC-ResNet8 CNN for 31 keyword classes, Silero VAD gate, three-thread architecture (cpal audio → inference → Bevy messages), PTT on V key. Python pipeline: model.py, generate_tts.py, augment.py, dataset.py, record.py, train.py. Fixed Bevy 0.18 API: Event→Message, EventReader→MessageReader, EventWriter→MessageWriter, add_event→add_message. 76 tests pass (35+16+10+15). Updated ARCHITECTURE.md, TDL.md.
- **2026-02-28T28:00:00Z** — Assembled all 6 faction designs into GAME_DESIGN.md (~1450 lines). Renamed factions: catGPT (cats), The Clawed (mice), Seekers of the Deep (badgers), The Murder (corvids), LLAMA (raccoons), Croak (axolotls). LLAMA backronym: "Locally Leveraged Alliance for Material Appropriation." Inserted The Clawed, Seekers, and Murder faction sections via extraction from background agent outputs. Fixed stale references, duplicate headings, grammar. Updated asset_catalog.yaml. Added 9 deferred items to TDL.md. Code reviewed (correctness + refactor). 33 tests pass.
- **2026-02-28T26:00:00Z** — Designed The Eternal Pond faction (axolotls/frogs/newts/turtles) with AI agent Grok. 10 units, 8 buildings, synergy map. Core mechanics: Water Affinity (water tile bonuses), Limb Economy (axolotl resource), Waterlogged debuff, terrain creation (Bog Patches, Tidal Zones, Primordial Soup). Species split: 5 axolotl, 3 frog, 1 newt, 1 turtle. Faction identity: attrition warfare, regeneration, water terrain advantage. Added to GAME_DESIGN.md between cat buildings and victory conditions.
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
