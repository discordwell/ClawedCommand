# TDL — To Do Later

## From Dream Sequence Implementation

- [ ] Generate Kell Fisher portrait (`assets/portraits/portrait_kell_fisher.png`) — Human USMC officer, stern
- [ ] Generate Rex Harmon portrait (`assets/portraits/portrait_rex_human.png`) — Rex Solstice's pre-singularity human form
- [ ] Generate Claude of the Lake portrait (`assets/portraits/portrait_claude_lake.png`) — Ethereal, code-themed figure in water
- [ ] Generate Kell Fisher hero sprite (`assets/sprites/heroes/kell_fisher_idle.png`) — Isometric human in military uniform
- [ ] Generate Claude of the Lake sprite (`assets/sprites/dream/claude_lake.png`) — Figure standing on island
- [ ] Generate code sword sprite (`assets/sprites/dream/code_sword.png`) — Sword made of glowing code lines
- [ ] Generate 5 modern military terrain tiles (128x128): concrete.png, linoleum.png, carpet_tile.png, metal_grate.png, drywall.png
- [x] Post-dream "code sword IRL" mission (Lua scripting in-game) — implemented as "The Strait" DEFCON-style drone warfare interlude
- [ ] Office prop sprites (polish pass): desk, vending machine, gym equipment, phone, bed, food, people

## From Strait Dream Sequence (DEFCON Interlude)

- [ ] DEFCON map background — dark navy, glowing coastline contours (or generated from terrain data)
- [ ] Drone icon sprite (friendly patrol) — small green glowing dot with range circle
- [ ] Enemy drone icon sprite (AA) — small red dot
- [ ] Tanker icon sprite — larger blue rectangle/ship silhouette
- [ ] Enemy launcher icon sprite — blinking red triangle when visible
- [ ] Missile trail VFX — bright amber arcing line
- [ ] Explosion flash VFX — tanker hit / missile intercept
- [ ] Radar sweep overlay animation — rotating green sector
- [ ] Compute budget bar UI elements — terminal-style green-on-dark
- [ ] Terminal HUD frame — dark translucent panel with monospace font
- [ ] Satellite scan visual — expanding ring/pulse effect
- [ ] Implement Lua `ctx.strait:` bindings in `lua_runtime.rs` (data types in `strait_bindings.rs`)
- [ ] Wire StraitCommand outputs from Lua back into Bevy ECS (patrol updates, satellite scans, zero-day builds)
- [ ] Add "scan", "exploit", "deploy" to voice keyword vocabulary
- [ ] Player click-to-interact: click drone to select, click map to set satellite scan target
- [ ] Strait mission sound design: radar ping, missile warning, intercept beep, ops center ambient

## From Unit Ability Design Review

- [x] Update ARCHITECTURE.md core systems list with: ability_system, aura_system, status_effect_system, stealth_system, tunnel_system
- [x] Add `GameCommand::ActivateAbility` variant to ARCHITECTURE.md command enum
- [x] Update PLAN.md Phase 2 with ability system sub-items or add Phase 2.5
- [ ] Add ~24 VFX asset entries to asset_catalog.yaml (aura rings, Shadow Network lines, Override beam, etc.)
- [ ] Add `vfx` top-level category to asset_catalog.yaml
- [ ] Add Chonk Loaf Mode sprite variant to asset catalog
- [ ] Add Ferret Sapper tunnel entrance/exit sprites to asset catalog

## From Faction Assembly Review

- [ ] Standardize Buildings heading levels: change `###` to `##` for The Clawed, Seekers, Croak, and LLAMA
- [ ] Standardize ability rules section naming across factions (e.g., `### General Ability Rules ({Faction} Addendum)`)
- [ ] Rename duplicate unit names: "Scrounger" (Murder vs LLAMA) and "Tunneler" (Clawed vs Seekers)
- [ ] Add mechanical detail to catGPT buildings (currently much less detailed than other factions)
- [ ] Add Implementation Notes sections for The Clawed, The Murder, and Croak (or move all to separate doc)
- [ ] Add Tech Trees for all factions (currently only LLAMA has one) or move to separate doc
- [ ] Rename duplicate ability names across factions: "Rally Cry" (Seekers vs Murder), "Undermine" (Clawed vs Seekers)
- [ ] Consider splitting GAME_DESIGN.md into per-faction files for navigability (1450+ lines)
- [ ] Add zodiac/astrology theming to The Murder's unit abilities (user intent noted in factions table but not yet reflected in unit designs)

## From Voice Vocabulary Expansion

- [ ] Extract shared `load_config()` into `training/voice/utils.py` (currently duplicated in generate_tts.py, dataset.py, train.py)
- [ ] Auto-generate `assets/voice/labels.txt` from `config.yaml` (add `--generate-labels` flag or build step) instead of manual sync
- [ ] Add pending-state timeout to `voice_intent_system` — if unit filter set but no agent command follows within ~2s, clear it
- [ ] Clarify building synonyms: barracks/post and refinery/market currently alias to same BuildingKind — split into separate variants if they become distinct buildings
- [ ] Add `UnitKind` variants for other factions (Clawed, Seekers, Murder, Croak, LLAMA) so voice unit names resolve instead of logging Ignored

## From Wet Test Analysis

- [x] Add staleness detection for stuck gatherers: workers with MoveTarget but no positional progress should have Gathering removed after N ticks (stale_ticks counter on Gathering component)
- [x] AI workers are all busy gathering during BuildUp, leaving no idle workers for building — consider allowing AI to pull a worker off gathering for construction
- [x] Consolidate scattered tuning constants (HARVEST_TICKS, CARRY_AMOUNT, ATTACK_MOVE_SIGHT_RANGE, etc.) into `cc_core::tuning` module when count grows further
- [x] ReturningToBase deposits resources when MoveTarget removed even if worker is not near a drop-off (pre-existing, proximity check needed)

## From Rendering Performance Review

- [ ] Replace 4,096 fog overlay entities with a single full-screen quad + 64x64 fog texture (write pixel alpha directly, use shader for isometric diamond mask). Eliminates all entity queries and material swaps for fog. Priority increases at 128x128+ map sizes.
- [ ] Consider replacing Gizmos terrain borders with spawned static Mesh2d line entities for 128x128+ maps (Gizmos are immediate-mode, rebuilt every frame)

## From Agent Harness Code Review

- [ ] Extract `run_behavior` helper in `cc_harness/src/server.rs` to de-duplicate ~18 behavior tools (~200 lines of boilerplate: lock sim, snapshot, create ScriptContext, take commands, inject, return result)
- [ ] Extract `run_query` helper in `cc_harness/src/server.rs` to de-duplicate ~11 query tools (lock sim, snapshot, create ScriptContext, return JSON)
- [ ] Deduplicate `HeadlessSim::snapshot()` in `cc_harness/src/headless.rs` with `build_snapshot()` in `cc_agent/src/snapshot.rs` (~180 lines of identical logic — delegate to build_snapshot instead)


## From Code Review (Agent Harness + Gameplay Fixes)

- [ ] `ToolRegistry::build_default()` called on every `execute_tool()` invocation — should use `OnceLock` or pass registry as parameter
- [ ] Hardcoded `ToolTier::Advanced` in `cc_client/src/ui/agent_chat.rs` — should read from `FactionToolStates` resource
- [ ] `FactionId::from_u8(player_id).unwrap_or(CatGPT)` duplicated 19× in `cc_harness/src/server.rs` — extract to `FactionId::for_player(id)` method
- [ ] Lua behavior binding registration boilerplate (~18 blocks) in `lua_runtime.rs` — consider a macro
- [x] `test_dream_siege_resets_on_target_change` was flaky — fixed by zeroing target damage in test (T2 damage-reset was interfering)
- [ ] Update training data scripts (`validate_data.py`, `generate_synthetic.py`, `evaluate.py`) to match current tool list after `execute_strategy` removal

## From Campaign System Code Review

- [ ] Extract shared `spawn_base_unit` helper from 5 duplicated unit-spawning patterns (wave_spawner.rs, campaign_integration.rs, integration.rs, harness/mod.rs, headless.rs) — prevents drift when unit component bundles change
- [ ] Add documentation comment in `cc_sim/src/campaign/mod.rs` about sim chain / campaign chain co-execution assumption (campaign system ordering constraints assume GameState::Playing is active simultaneously)

## From Phase 4B: Ability Implementation

- [ ] LoafMode should block pathing (grid occupancy system needed — currently only applies stat effects)
- [ ] Yowler network stacking: multiple Yowlers amplify each other's auras (Phase 4D)
- [x] Tilted CC trigger at 5 Annoyed stacks — implemented in status_effect_system (Phase 4C)
- [ ] Zoomies Chaos Trail: enemy slow zone left behind while Zoomies is active (needs trail entity spawning + slow zone system)
- [x] DreamSiege timer reset on Catnapper taking damage — implemented in ability_effect_system (Phase 4C)
- [x] `wave_eliminated_fires_when_all_dead` campaign test — fixed by using WaveTracker resource for condition evaluation

## From Phase 4C: Ability Implementation

- [ ] Hairball should block pathing (needs grid occupancy system — currently only spawns obstacle entity)
- [ ] DisgustMortar position targeting in client input UI (currently uses unit position as center)
- [ ] EcholocationPulse client-side fog reveal rendering
- [ ] ShapedCharge explosion VFX
- [ ] GravitationalChonk: don't pull through buildings (needs pathfinding query for line-of-sight)
- [ ] GravitationalPullCommand: add map bounds clamping (unlike RevulsionAoeCommand, currently can pull units off-map)
- [ ] Corroded stack decay guard: add `remaining_ticks > 0` check before `% 80 == 0` (fires spuriously on expiry tick)
- [ ] Supply cap should be granted on construction completion, not on build start (LitterBox +10 cap immediately on placement)
- [ ] ScratchingPost research queue not shown in building_info panel (only shows generic text)

## Code Quality (from code review)

- [ ] Extract `ensure_effect`/`refresh_or_add` into `StatusEffects::refresh_or_insert()` method (3 duplicate copies)
- [ ] Add `UnderConstruction::progress_f32()` method (construction progress computed 4× in different files)
- [ ] Add `BuildingKind::display_name()` method in cc_core (duplicated in build_menu.rs and building_info.rs)
- [ ] Move LaserPointer combat stats to tuning.rs constants (hardcoded in production_system.rs)
- [ ] Extract `BUILDING_SPRITE_SIZE: f32 = 28.0` constant (repeated in 4 renderer locations)

## Campaign Missions (Remaining RON Files)

- [ ] Act 2 M5: False Front (Seekers border, first encounter)
- [ ] Act 2 M6: Into the Deep (Seekers tunnels, escort mission)
- [ ] Act 2 M7: The Sett (Seekers stronghold, stealth/diplomacy)
- [ ] Act 2 M8: Betrayal (Escape from Seekers)
- [ ] Act 3 M9: Crow's Landing (First Murder encounter)
- [ ] Act 3 M10: The Parliament (Murder diplomacy/assassination)
- [ ] Act 3 M11: Memory Vision (Flashback mission)
- [ ] Act 3 M12: The Choice (Act 3 branching point — HelpRex/RefuseRex)
- [ ] Act 3 M13: Consequences (Branch-dependent aftermath)
- [ ] Act 4 M14: LLAMA Territory (Junkyard infiltration)
- [ ] Act 4 M15: Scrapfall Crossing (Granite confrontation)
- [ ] Act 4 M16: Four-faction Assault (Large multi-faction battle)
- [ ] Act 5 M17: Croak Approach (Swamp entry)
- [ ] Act 5 M18: The Grotto (Croak stronghold)
- [ ] Act 5 M19: Convergence (Five-faction battle)
- [ ] Act 5 M20: The Cloud (Wonder construction defense)
- [ ] Act 5 M21: Endings (4 branching endings: A/B/C/D)
- [ ] Missing hero units: Jinx (LLAMA), Ironjaw (Seekers), Zip (Murder) — needed for later act missions

## From Unit Training Flow Code Review

- [ ] **HIGH**: Q/W training hotkeys conflict with WASD camera pan — pressing W to train slot 1 also pans camera upward. Need to suppress camera pan when a producer building is selected and Q/W/E/R are pressed, or use different training hotkeys
- [ ] Consolidate `LOCAL_PLAYER` constant (duplicated 10x across cc_client with inconsistent types: u8 vs usize)
- [ ] Supply cap is granted at building spawn (builder arrival), not construction completion — consider deferring to `construction_system` completion
- [ ] No server-side guard against `GameCommand::Build { building_kind: TheBox }` — only protected by client hotkey menu omission
- [ ] Add visual indicator for BuildMenu mode (show available sub-keys on screen)

## From AI FSM Code Review

- [ ] Extract `try_build()` helper to de-duplicate 5 identical building construction blocks in BuildUp/MidGame phases (~50 lines saved)
- [ ] Extract `try_train_unit()` helper to de-duplicate 7 identical train-unit-if-affordable blocks (~40 lines saved)
- [ ] Extract `to_entity_ids(entities: &[Entity]) -> Vec<EntityId>` — repeated 12× across issue_attack/defend_commands
- [ ] Extract `set_rally_points()` and `defense_rally_pos()` helpers — rally point logic repeated 3× across phases
- [ ] Extract phase arms from `run_ai_fsm()` (~400 lines) into individual `phase_early_game()`, `phase_build_up()`, etc. functions for testability
- [x] Replace hardcoded building costs (100 food for FishMarket, 150 for CatTree, etc.) with references to `building_stats()` data to prevent silent desync
- [ ] Extract magic numbers: `4` (BuildUp→MidGame threshold), `6` (max MidGame workers), `15` (focus-fire search radius), `5` (flank offset tiles), `2` (melee forward offset), `3` (defense rally offset from box)
- [ ] Separate Strategic and Advanced tier match arms in `issue_attack_commands` (currently conflated; Advanced should add adaptive positioning per enum doc)
- [x] Rename `BuildingCensus` fields from catGPT names to role names (`has_hq`, `barracks_entity`, `tech_queue_len`, etc.) for consistency with FactionMap
- [x] Simplify `take_building_census` to compare against `fmap` fields instead of enumerating all faction building variants
- [x] Consolidate duplicate `BotConfig` structs (cc_sim::harness + cc_agent::arena) into `cc_sim::ai::fsm`
- [x] Consolidate duplicate helper functions between harness and arena (`spawn_starting_entities`, `spawn_combat_unit`, `headless_despawn_system`, `count_living_entities`, `determine_leader`) — made pub in harness, arena imports from there. `check_elimination` kept separate (different semantics: draw vs attacker advantage)

## From AI Training Pipeline Iterations

- [x] **Investigate P1 map advantage**: Fixed resource placement asymmetry in `place_tiered_resources()`, build position ring scan bias in `find_build_position()`, and command processing order via `drain_interleaved()`. Remaining P0 bias from ECS entity iteration order (P0 entities spawned first → processed first in target_acquisition, combat, movement).
- [ ] **Fix entity iteration order bias**: P0 wins mirrors because its entities are iterated first in all systems. Options: (1) snapshot-then-apply pattern for movement/targeting, (2) shuffle iteration order per tick via SimRng, (3) process systems in alternating player order per tick. Most principled: make target_acquisition and movement systems fully simultaneous.
- [ ] **Balance non-CatGpt faction unit stats**: All 5 non-CatGpt factions lose 5/5 games vs CatGpt at Basic tier. Unit stats need tuning per faction (tracked in wet_faction_vs_catgpt_basic_tier test balance report).
- [ ] Consider adding `--swap-spawns` flag to arena CLI to test positional advantage independently

## From Arena Module Code Review

- [ ] Make `cc_sim::harness` helpers `pub` and reuse in `cc_agent::arena` instead of duplicating: `spawn_starting_entities`, `spawn_combat_unit`, `headless_despawn_system`, `count_living_entities`, `check_elimination`, `determine_leader`, `BotConfig`
- [ ] Extract shared `make_headless_world()` from `make_harness_sim` and `make_arena_sim` (resource initialization is ~80% identical)
- [ ] Populate `damage_dealt`/`damage_taken` fields in `PlayerArenaStats` (currently always 0.0)
- [ ] Add bounds checking in `spawn_starting_entities` for Pawdler spawn offsets near map edge
- [ ] Count `MatchOutcome::Error` outcomes in arena CLI summary statistics
- [ ] Extract `extract_panic_message()` helper from duplicated `catch_unwind` downcast patterns

## From LLM Runner + Construct Mode Code Review

- [x] `resource_deposits` Lua binding bypasses compute budget — now routes through `ScriptContext::resource_deposits()` with `budget.spend(COST_SIMPLE)`
- [x] LLM pipeline disconnected: `AgentBridge::default()` creates dead channels, `spawn_llm_runner()` never called — need startup wiring in `AgentPlugin::build()` or game setup
- [x] Dead snapshot path: `process_request`'s `snapshot: Option<&GameStateSnapshot>` always called with `None` from `spawn_llm_runner` — either pass snapshot through channel or remove parameter
- [x] `ToolRegistry::build_default()` rebuilt on every call (already tracked above)
- [x] Hardcoded `player_id: 0` in construct mode UI and agent chat quick commands — now uses `LocalPlayer` resource
- [x] `deposit_to_lua_table` uses `"kind"` field name vs `resource_deposits` binding using both `"kind"` and `"resource_type"` — standardized to `"kind"` only

## From Voice Pipeline Implementation

- [ ] Run Python voice training tests after setting up PyTorch environment (`cd training/voice && python test_model.py`)
- [ ] Download Silero VAD v5 ONNX model to `assets/voice/silero_vad.onnx`
- [ ] Generate TTS training data (`cd training/voice && python generate_tts.py`)
- [ ] Train TC-ResNet8 keyword classifier (`cd training/voice && python train.py --data-dir data/tts`)
- [ ] Record real voice samples for each vocabulary word (`cd training/voice && python record.py --word <word> --count 20`)
- [ ] Add `NSMicrophoneUsageDescription` to Info.plist for macOS mic permission
- [ ] Test end-to-end: run game → hold V → say "stop" → units stop

## 3D Models (Promotional Video Only)

> **Decision**: Game uses 2D sprites for units/buildings on isometric terrain. 3D unit/building models via Tripo are for trailers/promotional video only — do NOT replace 2D sprites with 3D animated models. Isometric maps and 3D terrain are fine.

- [x] Tripo pipeline PoC validated — sprite → 3D model → retopo → GLB → Bevy 3D render
- [x] Pawdler GLB generated (4986 faces, 1k texture, 1.1MB) at `assets/models/units/pawdler.glb`
- [x] PoC example at `crates/cc_client/examples/poc_3d.rs` (run with `BEVY_ASSET_ROOT=. cargo run --example poc_3d`)
- [ ] Generate GLB models for remaining 9 cat unit types (via Tripo Studio web UI, ~40 credits each)
- [ ] Generate GLB models for other faction units as needed for video
- [ ] Generate building GLBs for cinematic shots
- [ ] Render promotional trailer scenes using Bevy 3D camera

## Dream Office Sprites
- [ ] Generate prop sprites for 30+ interactables via ChatGPT pipeline (regen_hero_walk.py pattern adapted for static props)
- [ ] Priority sprites: phone, bed, couch, TV, coffee machine, vending, gym rack (existing 3 need repositioning check)
- [ ] Secondary: SCIF door, server rack, pool table, arcade, chapel pew, helicopter, humvee
- [ ] Tertiary: bulletin board, water fountain, medical cabinet, locker, menu board, washing machine, etc.
