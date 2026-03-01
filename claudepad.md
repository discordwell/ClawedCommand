# Claudepad - ClawedCommand

## Session Summaries
- **2026-03-01T270:00:00Z** — Voice Command Gaps Implementation: Wired 4 stubbed voice command categories. (1) CursorGridPos resource: shared cursor position updated each frame in mouse.rs, read by voice systems. (2) "Nearby" selector: filters units within 10-tile Chebyshev distance of cursor (119 keywords total). (3) Position-targeted commands: Move/Patrol/Scout use direction offsets (~15 tiles, screen-relative→isometric), Charge→AttackMove, Flank→perpendicular offset, Rally→SetRallyPoint. Direction keywords (NSEW) set pending_direction Local state consumed by next agent command. (4) Build command: voice "tower build" → find nearest idle Pawdler + ring-scan valid build site near cursor → GameCommand::Build. voice_building_to_game_building maps generic names to catGPT buildings. Bonus fixes: map center hardcode (32,32) → MapResource-based, cmd_queue.push → push_for_player(0, c). 11 new tests, 572 workspace tests pass.
- **2026-03-01T260:00:00Z** — Voice KD Pipeline Running on Brev L40S. Teacher pretrain COMPLETE: TC-ResNet14-Wide (2.1M params) on Speech Commands v2 (105K samples, 35 classes) = 97.0% val acc. Teacher finetune IN PROGRESS: 118 game classes, 26.7K unified samples, epoch 10/120, val 98.3%, all layers unfrozen. Pipeline: pretrain→finetune→distill→student→eval, running autonomously via nohup. Fixed setup_gpu.sh and run_pipeline.sh /workspace→$HOME paths. Brev instance 3zwo0eqwo, $0.86/hr.
- **2026-03-01T250:00:00Z** — AI Training Gen 24-29 + Worker Spawn Symmetry Fix: Fixed worker spawn offset asymmetry (always +x, favoring P0). P0 baseline dropped from 90% to 80%. Shifted training to P1 (disadvantaged side) — scripts should help the underdog. Gen 26 NEW BEST: conditional kite (ranged units flee when outnumbered) + aggressive push (when >=3 army advantage) + HQ targeting + building rally = P1 40% decisive wins, 75% effective dominance (vs 15% baseline). Gen 24 (always kite) = too many timeouts. Gen 25 (no kite) = regression. Gen 27 (critical-only kite) = regression. Gen 28 (base defense recall) = slightly worse. Gen 29 (worker harassment) = CATASTROPHIC (split army dies). Shared scripts amplify positional advantage (P0 90%). Key: scripts benefit the underdog most; kiting when outnumbered is essential; never split army.
- **2026-03-01T240:00:00Z** — Devstral Small 2 QLoRA Training Running on Brev L40S. Mistral hackathon. Generated 550 training examples (50 gold + 500 synthetic via 5 parallel Opus agents). Resolved 10+ compatibility issues with Devstral Small 2: fp8 quantization_config stripping, TokenizersBackend→PreTrainedTokenizerFast, extra_special_tokens list→dict, Mistral3ForConditionalGeneration model type, BitsAndBytes 4-bit. Training in progress: 13.4GB VRAM, 100% GPU utilization, ~90 min ETA. Files: training/scripts/train_peft.py (PEFT+TRL, no Unsloth), training/configs/devstral_24b_lua_qlora.yaml. Brev: ssh -o RequestTTY=no clawedcommand-voice-training, PID 20899.
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
