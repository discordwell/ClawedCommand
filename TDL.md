# TDL — To Do Later

## From Unit Ability Design Review

- [ ] Update ARCHITECTURE.md core systems list with: ability_system, aura_system, status_effect_system, stealth_system, tunnel_system
- [ ] Add `GameCommand::ActivateAbility` variant to ARCHITECTURE.md command enum
- [ ] Update PLAN.md Phase 2 with ability system sub-items or add Phase 2.5
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

## From Rendering Performance Review

- [ ] Replace 4,096 fog overlay entities with a single full-screen quad + 64x64 fog texture (write pixel alpha directly, use shader for isometric diamond mask). Eliminates all entity queries and material swaps for fog. Priority increases at 128x128+ map sizes.
- [ ] Consider replacing Gizmos terrain borders with spawned static Mesh2d line entities for 128x128+ maps (Gizmos are immediate-mode, rebuilt every frame)

## From Voice Pipeline Implementation

- [ ] Run Python voice training tests after setting up PyTorch environment (`cd training/voice && python test_model.py`)
- [ ] Download Silero VAD v5 ONNX model to `assets/voice/silero_vad.onnx`
- [ ] Generate TTS training data (`cd training/voice && python generate_tts.py`)
- [ ] Train TC-ResNet8 keyword classifier (`cd training/voice && python train.py --data-dir data/tts`)
- [ ] Record real voice samples for each vocabulary word (`cd training/voice && python record.py --word <word> --count 20`)
- [ ] Add `NSMicrophoneUsageDescription` to Info.plist for macOS mic permission
- [ ] Test end-to-end: run game → hold V → say "stop" → units stop
