# TDL — To Do Later

## From Unit Ability Design Review

- [x] Update ARCHITECTURE.md core systems list with: ability_system, aura_system, status_effect_system, stealth_system, tunnel_system
- [x] Add `GameCommand::ActivateAbility` variant to ARCHITECTURE.md command enum
- [x] Update PLAN.md Phase 2 with ability system sub-items or add Phase 2.5
- [ ] Add ~24 VFX asset entries to asset_catalog.yaml (aura rings, Shadow Network lines, Override beam, etc.)
- [ ] Add `vfx` top-level category to asset_catalog.yaml
- [ ] Add Chonk Loaf Mode sprite variant to asset catalog
- [ ] Add Ferret Sapper tunnel entrance/exit sprites to asset catalog

## From Faction Assembly Review

- [x] Standardize Buildings heading levels: change `###` to `##` for The Clawed, Seekers, Croak, and LLAMA
- [x] Standardize ability rules section naming across factions (e.g., `### General Ability Rules ({Faction} Addendum)`)
- [x] Rename duplicate unit names: "Scrounger" (Murder vs LLAMA) and "Tunneler" (Clawed vs Seekers) — Murder Scrounger→Pilferer, Clawed Tunneler→Burrower
- [ ] Add mechanical detail to catGPT buildings (currently much less detailed than other factions)
- [ ] Add Implementation Notes sections for The Clawed, The Murder, and Croak (or move all to separate doc)
- [ ] Add Tech Trees for all factions (currently only LLAMA has one) or move to separate doc
- [x] Rename duplicate ability names across factions: "Rally Cry" (Seekers vs Murder), "Undermine" (Clawed vs Seekers) — Seekers Rally Cry→Bulwark Cry, Murder Rally Cry→Ascendant Call, Clawed Undermine→Destabilize
- [ ] Consider splitting GAME_DESIGN.md into per-faction files for navigability (1450+ lines)
- [x] Add zodiac/astrology theming to The Murder's unit abilities (user intent noted in factions table but not yet reflected in unit designs) — renamed 8 abilities with celestial/astrological names, added Zodiac Theming rule

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

- [x] Extract `run_behavior` helper in `cc_harness/src/server.rs` to de-duplicate ~18 behavior tools (~200 lines of boilerplate: lock sim, snapshot, create ScriptContext, take commands, inject, return result)
- [x] Extract `run_query` helper in `cc_harness/src/server.rs` to de-duplicate ~11 query tools (lock sim, snapshot, create ScriptContext, return JSON)


## From Code Review (Agent Harness + Gameplay Fixes)

- [x] `ToolRegistry::build_default()` called on every `execute_tool()` invocation — fixed: `LazyLock<ToolRegistry>` singleton in `mcp_tools.rs`
- [x] Hardcoded `ToolTier::Advanced` in `cc_client/src/ui/agent_chat.rs` — already fixed: reads from `FactionToolStates` resource
- [x] `FactionId::from_u8(player_id).unwrap_or(CatGPT)` duplicated 19× in `cc_harness/src/server.rs` — extract to `FactionId::for_player(id)` method
- [ ] Lua behavior binding registration boilerplate (~18 blocks) in `lua_runtime.rs` — consider a macro
- [x] `test_dream_siege_resets_on_target_change` was flaky — fixed by zeroing target damage in test (T2 damage-reset was interfering)
- [ ] Update training data scripts (`validate_data.py`, `generate_synthetic.py`, `evaluate.py`) to match current tool list after `execute_strategy` removal

## From Campaign System Code Review

- [x] Extract shared `spawn_base_unit` helper from 5 duplicated unit-spawning patterns (wave_spawner.rs, campaign_integration.rs, integration.rs, harness/mod.rs, headless.rs) — prevents drift when unit component bundles change
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
- [x] GravitationalPullCommand: add map bounds clamping (unlike RevulsionAoeCommand, currently can pull units off-map)
- [x] Corroded stack decay guard: add `remaining_ticks > 0` check before `% 80 == 0` (fires spuriously on expiry tick)
- [x] Supply cap should be granted on construction completion, not on build start (LitterBox +10 cap immediately on placement)
- [ ] ScratchingPost research queue not shown in building_info panel (only shows generic text)

## Code Quality (from code review)

- [x] Extract `ensure_effect`/`refresh_or_add` into `StatusEffects::refresh_or_insert()` method (3 duplicate copies)
- [x] Add `UnderConstruction::progress_f32()` method (construction progress computed 4× in different files)
- [x] Add `BuildingKind::display_name()` method in cc_core (duplicated in build_menu.rs and building_info.rs)
- [x] Move LaserPointer combat stats to tuning.rs constants (hardcoded in production_system.rs)
- [x] Extract `BUILDING_SPRITE_SIZE: f32 = 28.0` constant (repeated in 4 renderer locations)

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
- [ ] Gate `resource_hud` behind `#[cfg(not(feature = "native"))]` to prevent duplicate display with egui `resource_bar` when native feature is active
- [x] Supply cap is granted at building spawn (builder arrival), not construction completion — consider deferring to `construction_system` completion
- [x] No server-side guard against `GameCommand::Build { building_kind: TheBox }` — only protected by client hotkey menu omission
- [ ] Add visual indicator for BuildMenu mode (show available sub-keys on screen)

## From AI FSM Code Review

- [x] Extract `try_build()` helper to de-duplicate 5 identical building construction blocks in BuildUp/MidGame phases (~50 lines saved)
- [x] Extract `try_train_unit()` helper to de-duplicate 7 identical train-unit-if-affordable blocks (~40 lines saved)
- [x] Extract `to_entity_ids(entities: &[Entity]) -> Vec<EntityId>` — repeated 12× across issue_attack/defend_commands
- [x] Extract `set_rally_points()` and `defense_rally_pos()` helpers — rally point logic repeated 3× across phases
- [ ] Extract phase arms from `run_ai_fsm()` (~400 lines) into individual `phase_early_game()`, `phase_build_up()`, etc. functions for testability
- [x] Replace hardcoded building costs (100 food for FishMarket, 150 for CatTree, etc.) with references to `building_stats()` data to prevent silent desync
- [ ] Extract magic numbers: `4` (BuildUp→MidGame threshold), `6` (max MidGame workers), `15` (focus-fire search radius), `5` (flank offset tiles), `2` (melee forward offset), `3` (defense rally offset from box)
- [ ] Separate Strategic and Advanced tier match arms in `issue_attack_commands` (currently conflated; Advanced should add adaptive positioning per enum doc)
- [ ] Rename `BuildingCensus` fields from catGPT names to role names (`has_hq`, `barracks_entity`, `tech_queue_len`, etc.) for consistency with FactionMap
- [ ] Simplify `take_building_census` to compare against `fmap` fields instead of enumerating all faction building variants (~200 lines → ~30 lines)
- [ ] Consolidate duplicate `BotConfig` structs (cc_sim::harness + cc_agent::arena) into `cc_sim::ai`
- [ ] Consolidate duplicate helper functions between harness and arena (`spawn_starting_entities`, `spawn_combat_unit`, `headless_despawn_system`, etc.)

## From AI Training Pipeline Iterations

- [ ] **Investigate P1 map advantage**: P1 wins 70-80% of arena matches regardless of scripts. Likely causes: (1) FSM defense_pos `box_pos + (3,3)` puts P1 near map edge on 64x64, (2) `find_build_position` search direction bias, (3) process_commands execution order, (4) terrain generator not perfectly rotationally symmetric. Fix: mirror defense_pos offset based on spawn quadrant, or test with explicitly symmetric maps.
- [ ] Consider adding `--swap-spawns` flag to arena CLI to test positional advantage independently

## From Arena Module Code Review

- [ ] Make `cc_sim::harness` helpers `pub` and reuse in `cc_agent::arena` instead of duplicating: `spawn_starting_entities`, `spawn_combat_unit`, `headless_despawn_system`, `count_living_entities`, `check_elimination`, `determine_leader`, `BotConfig`
- [ ] Extract shared `make_headless_world()` from `make_harness_sim` and `make_arena_sim` (resource initialization is ~80% identical)
- [ ] Populate `damage_dealt`/`damage_taken` fields in `PlayerArenaStats` (currently always 0.0)
- [ ] Add bounds checking in `spawn_starting_entities` for Pawdler spawn offsets near map edge
- [ ] Count `MatchOutcome::Error` outcomes in arena CLI summary statistics
- [ ] Extract `extract_panic_message()` helper from duplicated `catch_unwind` downcast patterns

## From LLM Runner + Construct Mode Code Review

- [x] `resource_deposits` Lua binding bypasses compute budget — should route through `ScriptContext` method with `budget.spend(COST_SIMPLE)` like other query bindings
- [ ] LLM pipeline disconnected: `AgentBridge::default()` creates dead channels, `spawn_llm_runner()` never called — need startup wiring in `AgentPlugin::build()` or game setup
- [ ] Dead snapshot path: `process_request`'s `snapshot: Option<&GameStateSnapshot>` always called with `None` from `spawn_llm_runner` — either pass snapshot through channel or remove parameter
- [x] `ToolRegistry::build_default()` rebuilt on every call (fixed: `LazyLock` singleton in `mcp_tools.rs`)
- [ ] Hardcoded `player_id: 0` in construct mode UI and agent chat quick commands — should use `LocalPlayer` resource
- [ ] `deposit_to_lua_table` uses `"kind"` field name vs `resource_deposits` binding using both `"kind"` and `"resource_type"` — standardize across all deposit APIs

## From Voice Pipeline Implementation

- [ ] Run Python voice training tests after setting up PyTorch environment (`cd training/voice && python test_model.py`)
- [ ] Download Silero VAD v5 ONNX model to `assets/voice/silero_vad.onnx`
- [ ] Generate TTS training data (`cd training/voice && python generate_tts.py`)
- [ ] Train TC-ResNet8 keyword classifier (`cd training/voice && python train.py --data-dir data/tts`)
- [ ] Record real voice samples for each vocabulary word (`cd training/voice && python record.py --word <word> --count 20`)
- [ ] Add `NSMicrophoneUsageDescription` to Info.plist for macOS mic permission
- [ ] Test end-to-end: run game → hold V → say "stop" → units stop

## 3D Renderer Migration

- [ ] Evaluate Tripo GLB output quality — decide image-to-model vs text-to-model vs multi-view
- [ ] Generate GLB models for all 10 cat unit types via Tripo API
- [ ] Generate GLB models for 8 cat building types
- [ ] Generate terrain tile 3D meshes (grass, dirt, forest, water, etc.)
- [ ] Replace 2D isometric renderer with 3D orthographic camera in cc_client
- [ ] Implement 3D depth sorting (replaces sprite-based Y-sort)
- [ ] Port zoom LOD system to 3D (mesh LOD levels instead of sprite swap)
- [ ] Port health bars/selection rings to 3D billboard quads
- [ ] Port fog of war to 3D (shader-based or geometry overlay)
- [ ] Animate GLB models (idle, walk, attack) — either skeletal or swap meshes
- [ ] Team color tinting system for 3D materials (per-faction color multiply)
- [ ] Performance profiling: ensure 60fps with full 64x64 map in 3D
