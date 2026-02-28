# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T25:00:00Z** — Created GAME_DESIGN.md: full game identity document. Post-singularity cat RTS setting, 6 factions (cats/mice/badgers/corvids/raccoons/axolotls) with AI agent personalities, 3 resources (Food/GPU Cores/NFTs), 10 cat units, 8 buildings, 3 victory conditions, GPU economy mechanic. Updated ARCHITECTURE.md (economy, tech tree, risks), PLAN.md Phase 2 (new names), UnitKind enum (2→10 variants), asset catalog (cat-themed). Code reviewed (correctness + refactor). 33 tests pass.
- **2026-02-28T24:00:00Z** — Built complete asset pipeline in `tools/asset_pipeline/`. 6 Python scripts + shared image_utils.py. Config: asset_catalog.yaml, palette.yaml, 8 prompt templates. Pipeline flow: ChatGPT prompt → browser automation → rembg → resize/slice → atlas manifest. ASSET_PIPELINE.md created.
- **2026-02-28T23:00:00Z** — Created PLAN.md: detailed game creation plan Phases 1-3. Phase 1 actionable, 2-3 feature-level. Bevy 0.18, bevy_ecs_tilemap git branch.
- **2026-02-28T21:00:00Z** — Voice command system: vibecoded Lua scripts, push-to-talk, tiered intent classification, voice buffs. Web Speech API + Whisper.js fallback.
- **2026-02-28T17:00:00Z** — Project kickoff. ARCHITECTURE.md: Bevy/Rust isometric 2D RTS, fine-tuned Mistral AI, hybrid control, deterministic lockstep.

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
