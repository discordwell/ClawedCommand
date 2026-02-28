# Claudepad - ClawedCommand

## Session Summaries
- **2026-02-28T24:00:00Z** — Built complete asset pipeline in `tools/asset_pipeline/`. 6 Python scripts (generate_asset.py orchestrator, process_sprite.py, process_sheet.py, verify_grid.py, generate_atlas_meta.py, normalize_palette.py) + shared image_utils.py. Config: asset_catalog.yaml (24 starter assets), palette.yaml, 8 prompt templates. Pipeline flow: assemble ChatGPT prompt → browser automation download → rembg bg removal → resize/slice → grid verify → atlas manifest → copy to assets/. ASSET_PIPELINE.md docs created. ARCHITECTURE.md updated. All scripts tested. Code reviewed (correctness + refactoring).
- **2026-02-28T23:00:00Z** — Created PLAN.md: detailed game creation plan covering Phases 1-3. Phase 1 is actionable (file paths, Rust signatures, crate deps). Phases 2-3 are feature-level. Bevy 0.18 confirmed as target. bevy_ecs_tilemap needs git branch for 0.18 compat. Key crates: fixed (deterministic math), mlua (Lua scripts), reqwest (Mistral API), wasmtime (sandbox).
- **2026-02-28T21:00:00Z** — Voice command system research. Designed core mechanic: players vibecode Lua scripts in "construct mode" (in-game LLM-powered editor), bind scripts to voice command intents, issue commands via push-to-talk during gameplay. Voice-commanded units get command-specific temporary buffs (damage for attack, speed for retreat, etc.). Tech recommendation: Web Speech API + tiered intent classification (keyword match → fuzzy → Mistral agent). Lua chosen for scripting language. Singleplayer focus first.
- **2026-02-28T17:00:00Z** — Project kickoff. Established core architecture: Isometric 2D RTS in Bevy/Rust with fine-tuned Mistral AI agents using MCP tools. Hybrid player+AI control model. Classic base-building economy. Deterministic lockstep multiplayer. Created ARCHITECTURE.md with full system design, project structure, 6 development phases, and risk assessment.

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
