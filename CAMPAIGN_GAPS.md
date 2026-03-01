# Campaign Implementation Gaps

> Tracking all remaining work needed to bring the 23-mission campaign from narrative scripts to playable game.

---

## Priority 1: Core Systems (Blocking Playability)

### 1.1 Structured Mission Definitions
**Status**: Partially complete (Prologue + Act 1)
**What**: RON mission definition files that the game engine reads — spawn positions, objectives, win/lose conditions, trigger events, dialogue timing.
**Where**: `assets/campaign/` — prologue.ron, act1_m1-m4.ron
**Depends on**: Mission trigger system in cc_sim
**Notes**: Prologue + 4 Act 1 missions implemented and validated. Remaining 18 missions (Act 2-5) need RON files.

### 1.2 Campaign Map Generation
**Status**: Not started (map_gen exists for skirmish)
**What**: 22 unique campaign maps (Mission 0 uses a small tutorial arena). Each mission specifies terrain, chokepoints, and faction starting positions.
**Where**: `crates/cc_core/src/map_gen.rs` extensions or `assets/maps/`
**Key maps**:
- Prologue: Small river clearing (tutorial arena)
- Act 1: catGPT territory — fish ponds, rolling hills, Cat Tree bases
- Act 2: Seekers mountain fortress — tunnels, elevation, Sett interior
- Act 3: Murder territory — cliffs, nesting spires, Parliament roosts + LLAMA junkyard border
- Act 4: LLAMA junkyard — scrap heaps, wreck turrets, Scrapfall Crossing bridge
- Act 5: Croak swamp — flooded caves, Grotto assembly site, convergence battlefield

### 1.3 Enemy Wave / Force Compositions
**Status**: Partially complete (Act 1)
**What**: Unit compositions for each mission's enemy forces. Story scripts describe factions and named characters present; need exact unit counts, spawn timing, and AI behavior profiles.
**Where**: Mission definition files (RON). Wave spawner system (`crates/cc_sim/src/campaign/wave_spawner.rs`) handles entity creation with WaveMember tracking.
**Key battles**:
- Mission 3: Clawed flanking force (Tunnelers + Swarmers)
- Mission 8: Seekers pursuit force (Ironhides, non-lethal capture intent)
- Mission 13: Seekers assault on Murder territory (Ironhides + Cragbacks)
- Mission 15: Full Seekers offensive (Wardenmother's army)
- Mission 16: Four-faction simultaneous assault
- Mission 19: Five-faction convergence (largest battle)

### 1.4 Branching State Machine
**Status**: Complete (infrastructure)
**What**: Campaign state tracker for the Act 3 choice and its cascading consequences.
**Where**: `crates/cc_sim/src/campaign/state.rs` — `PersistentCampaignState` struct with all fields, `NextMission` enum (Fixed/Branching/None) in `cc_core::mission`
**Tracks**:
- `act3_choice`: HelpRex | RefuseRex
- `gemineye_fabrication_rate`: 20% (default) or 5% (HelpRex)
- `patches_status`: Free | Captured
- `murder_alliance`: true | false
- `flicker_subplot_progress`: 0/3 (stages completed)
- `ponderer_fragment_found`: bool
- `ending_d_eligible`: bool (all 3 prerequisites met)

### 1.5 Mission Trigger / Event System
**Status**: Complete (core system)
**What**: In-mission scripted events — dialogue triggers, objective changes, reinforcement spawns, cinematic camera moves.
**Where**: `crates/cc_sim/src/campaign/triggers.rs` — full trigger evaluation with 10 condition types, 6 action types
**Implemented trigger conditions**: AtTick, HeroAtPos, EnemyKillCount, AllEnemiesDead, WaveEliminated, FlagSet, TriggerFired, All, Any, HeroHpBelow, PersistentFlag
**Implemented trigger actions**: ShowDialogue, SpawnWave, SetFlag, CompleteObjective, PanCamera, SetPersistentFlag
**Still needed**: Player choice prompt UI (presentation layer)

---

## Priority 2: Presentation Layer (Blocking Campaign Feel)

### 2.1 Cutscene / Dialogue Format
**Status**: Not started
**What**: Runtime format for presenting `[STAGE]` directions, `"Dialogue"` lines, and `` `AI VOICE` `` speech. Needs text rendering, character portraits, camera scripting.
**Where**: `crates/cc_client/src/ui/`
**Options**: Simple text box overlay (MVP), VN-style portraits (stretch), in-engine camera choreography (ideal)
**Notes**: Story scripts are written with camera directions — would benefit from a simple camera scripting system even at MVP.

### 2.2 Voice-Over Script Export
**Status**: Not started
**What**: Extract all dialogue lines from story scripts into a structured VO casting document — character, line, context, emotion, duration estimate.
**Where**: `story/vo_script.md` or tooling to auto-extract
**Scale**: ~15 named characters, estimated 400-600 individual lines across 23 missions + 4 endings
**Notes**: Could be AI-generated placeholder VO initially. Distinct voice profiles per character and per AI.

### 2.3 Music / Atmosphere Cues
**Status**: Not started
**What**: Music and ambient sound direction for each mission and key moments.
**Where**: `story/music_cues.md` or integrated into mission definitions
**Key moments needing distinct audio**:
- Prologue: Server boot sequence (6 AI voices overlapping)
- Act 2 M7: Rex's whisper (tense, intimate)
- Act 3 M11: Memory vision (ethereal, pre-singularity)
- Act 4 M15: Granite confrontation (emotional climax)
- Act 5 M21: Each ending needs unique music
- Ending D: Unified AI voice (six harmonics)

### 2.4 Tutorial System Integration
**Status**: Not started
**What**: Prologue + early missions teach game mechanics through gameplay triggers. Need tutorial overlay system (highlight UI elements, constrain player actions, show tooltips).
**Where**: `crates/cc_client/src/ui/tutorial.rs`
**Tutorials by mission**:
- M0: Movement, terrain, basic combat, Polyglot Protocol
- M1: Unit selection, attack-move, pond defense economy
- M2: Stealth/recon, Mouser mechanics, minimap
- M3: Large battle, reinforcements, AI manipulation
- M5: Seekers Dug In passive, defensive mechanics
- M14: LLAMA scrap economy, Bandit jury-rigging

---

## Priority 3: Narrative Supplements (Enriching Existing Scripts)

### 3.1 Fork in the Code — Geppity/Claudeus Maximus Glitch Scenes
**Status**: Written (3 parts across act1, act3, act5)
**What**: STORYLINE.md describes mechanical glitching when catGPT and Clawed forces meet — units receive wrong commands, Nuisances twitch toward Swarmer spacing, Swarmers pause to analyze. This sibling-AI subplot needs in-mission narrative beats.
**When**: Act 1 Mission 3 (first noticed during counter-raid), Act 3 Mission 13 (exploited by Kelpie during escape), Ending D (Geppity/Claudeus Maximus reunification)

### 3.2 Ponderer Prophecy Scenes in Earlier Acts
**Status**: Written (3 prophecies across act2, act3, act4)
**What**: The Ponderer's prophecies (always 1 day wrong) appear before Act 5 to establish the running joke and foreshadowing.
**When**: Act 2 Mission 8 (Grok relays broadcast during escape), Act 3 Mission 13 (Gemineye intercepts broadcast), Act 4 Mission 14 (Grok relays third prophecy)

### 3.3 Patches' Internal Conflict Scenes
**Status**: Written (3 parts across act1, act2, act3)
**What**: Patches deliberates between reporting Kelpie's AI-hacking to Felix and staying quiet. Escalating internal conflict from omission to deliberate protection.
**When**: Act 1 post-Mission 3 (first omission to Felix), Act 2 post-mountains (partial report), Act 3 pre-Mission 12 (refuses to transmit)

### 3.4 Thimble / The Clawed Development
**Status**: Written (3 interludes across act1, act3, act4)
**What**: The Clawed perspective through non-interactive cutscenes. Establishes Thimble as sympathetic antagonist before his Act 5 hero confrontation.
**When**: Act 1 post-Mission 3 (War Room — counting casualties), Act 3 post-Mission 10 (Intelligence — processing otter's journey), Act 4 pre-Mission 16 (March — full mobilization)

### 3.5 Granite's Optional Philosophical Dialogue
**Status**: Written (1 scene in act5)
**What**: Optional Act 5 dialogue where Granite explains why humans chose to become animals. Placed on the eastern ridge between Mission 19 waves.
**When**: Act 5, between Missions 19-20 (optional interaction when Kelpie approaches Granite's paused forces)

---

## Priority 4: Post-Campaign / Extended Content

### 4.1 The Clawed Faction Campaign
**Status**: Not planned
**What**: The Clawed is the only faction never played. A DLC or bonus campaign from Thimble's perspective would fill this gap — showing the Clawed's "defensive aggression" doctrine and Claudeus Maximus's verbose strategic analysis.
**Notes**: Not required for main campaign, but a notable asymmetry.

### 4.2 Skirmish Mode Faction AI Personalities
**Status**: Partially implemented (AiPersonalityProfile exists)
**What**: Each faction's AI should play distinctly in skirmish mode, reflecting their campaign character. catGPT aggressive, Seekers defensive, Murder intel-focused, LLAMA chaotic/scrap-economic, Croak patient/regenerative, Clawed swarm.

### 4.3 Post-Game Epilogue Missions
**Status**: Not planned
**What**: Optional missions after each ending showing the world state. E.g., Ending A: Kelpie as diplomat mediating faction disputes without AI. Ending D: unified AI dealing with Jinx trying to steal from it.

### 4.4 New Game+ / Campaign Replay
**Status**: Not planned
**What**: Replay with opposite Act 3 choice, carrying over unit upgrades. See alternate consequences.

---

## Dependency Graph

```
Mission Definitions (1.1) ──┐
Campaign Maps (1.2) ────────┤
Enemy Compositions (1.3) ───┼──► Playable Campaign MVP
Branching State (1.4) ──────┤
Trigger System (1.5) ───────┘
        │
        ▼
Cutscene Format (2.1) ─────┐
Tutorial System (2.4) ──────┼──► Full Campaign Experience
Music Cues (2.3) ───────────┤
VO Script (2.2) ────────────┘
        │
        ▼
Narrative Supplements (3.x) ──► Enriched Campaign
        │
        ▼
Extended Content (4.x) ───────► Post-Launch
```
