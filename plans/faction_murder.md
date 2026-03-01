# Implementation Plan: The Murder (Corvids / Gemineye)

> Making The Murder a fully playable faction in ClawedCommand.

---

## Table of Contents

1. [Scope Summary](#scope-summary)
2. [Phase 1: Data Layer (cc_core)](#phase-1-data-layer-cc_core)
   - [1A. UnitKind Variants](#1a-unitkind-variants)
   - [1B. BuildingKind Variants](#1b-buildingkind-variants)
   - [1C. Unit Base Stats](#1c-unit-base-stats)
   - [1D. Building Base Stats](#1d-building-base-stats)
   - [1E. AbilityId Variants](#1e-abilityid-variants)
   - [1F. AbilityDef Definitions](#1f-abilitydef-definitions)
   - [1G. UpgradeType Variants](#1g-upgradetype-variants)
3. [Phase 2: Faction-Specific Components (cc_core)](#phase-2-faction-specific-components-cc_core)
   - [2A. Aerial Marker](#2a-aerial-marker)
   - [2B. Fabrication System](#2b-fabrication-system)
   - [2C. Exposed Debuff](#2c-exposed-debuff)
   - [2D. Murder-Specific Trackers](#2d-murder-specific-trackers)
   - [2E. AuraType Extensions](#2e-auratype-extensions)
4. [Phase 3: Simulation Systems (cc_sim)](#phase-3-simulation-systems-cc_sim)
   - [3A. Production System Updates](#3a-production-system-updates)
   - [3B. Aerial Pathing System](#3b-aerial-pathing-system)
   - [3C. Aerial Combat Rules](#3c-aerial-combat-rules)
   - [3D. Fabrication System](#3d-fabrication-system)
   - [3E. Exposed Debuff System](#3e-exposed-debuff-system)
   - [3F. Building-Completion Specialization](#3f-building-completion-specialization)
5. [Phase 4: AI FSM Updates (cc_sim)](#phase-4-ai-fsm-updates-cc_sim)
   - [4A. Personality Profile](#4a-personality-profile)
   - [4B. Building Census](#4b-building-census)
   - [4C. Build Order Logic](#4c-build-order-logic)
   - [4D. Unit Production Logic](#4d-unit-production-logic)
   - [4E. Worker Type Mapping](#4e-worker-type-mapping)
6. [Phase 5: Client / Rendering (cc_client)](#phase-5-client--rendering-cc_client)
7. [Phase 6: Agent / Harness (cc_agent, cc_harness)](#phase-6-agent--harness-cc_agent-cc_harness)
8. [Phase 7: Tests](#phase-7-tests)
9. [System Modification Matrix](#system-modification-matrix)
10. [Dependency Graph](#dependency-graph)
11. [Open Questions](#open-questions)

---

## Scope Summary

The Murder is an aerial intel/espionage faction led by AI agent Gemineye. Currently the faction exists only as:
- A `Faction::TheMurder` enum variant in `components.rs`
- An AI personality profile in `fsm.rs` that references cat unit proxies (`FlyingFox`, `Mouser`, `Nuisance`)
- Full design docs in `GAME_DESIGN.md` (10 units, 8 buildings, 30 abilities, faction mechanics)

To make The Murder fully playable we need to add:
- **10 UnitKind variants** with base stats
- **8 BuildingKind variants** with base stats
- **30 AbilityId variants** with AbilityDef definitions
- **~5 UpgradeType variants** for Murder-specific research
- **4+ new components** (Aerial, FabricationSource, ExposedDebuff, murder-specific trackers)
- **3+ new systems** (aerial pathing, fabrication, exposed debuff tick-down)
- **Modifications to ~8 existing systems** (production, combat, target acquisition, cleanup, AI FSM, building census, client rendering, harness MCP tools)

---

## Phase 1: Data Layer (cc_core)

### 1A. UnitKind Variants

Add 10 new variants to the `UnitKind` enum in `crates/cc_core/src/components.rs`:

| Variant | Role | Animal | Equivalent Cat Role |
|---------|------|--------|---------------------|
| `Scrounger` | Worker | Crow | Pawdler |
| `Sentinel` | Ranged Scout | Crow | Mouser/Hisser hybrid |
| `Rookclaw` | Melee Dive Striker | Crow | Nuisance |
| `Magpike` | Disruptor/Thief | Magpie | (unique) |
| `Magpyre` | Saboteur | Magpie | FerretSapper |
| `Jaycaller` | Support/Buffer | Jay | Yowler |
| `Jayflicker` | Illusion Specialist | Jay | (unique) |
| `Dusktalon` | Stealth Assassin | Owl | Mouser |
| `Hootseer` | Area Denial/Debuffer | Owl | (unique) |
| `CorvusRex` | Hero/Heavy | Crow (Augmented) | MechCommander |

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add variants to `UnitKind` enum, update `Display`, `FromStr`, add to tests

### 1B. BuildingKind Variants

Add 8 new variants to the `BuildingKind` enum in `crates/cc_core/src/components.rs`:

| Variant | Role | Cat Equivalent |
|---------|------|----------------|
| `TheParliament` | Command Center (HQ) | TheBox |
| `Rookery` | Barracks | CatTree |
| `CarrionCache` | Resource Depot | FishMarket |
| `AntennaArray` | Tech Building | ServerRack |
| `Panopticon` | Research (unique, limit 1) | ScratchingPost |
| `NestBox` | Supply Depot | LitterBox |
| `ThornHedge` | Defensive Wall | CatFlap |
| `Watchtower` | Defense Tower | LaserPointer |

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add variants to `BuildingKind` enum, update `Display`, `FromStr`, add to tests

### 1C. Unit Base Stats

Add 10 match arms to `base_stats()` in `crates/cc_core/src/unit_stats.rs`. Proposed stats (all values at 10hz tick rate, Fixed-point where applicable):

```
Scrounger (Worker)
  HP: 55, Speed: 0.14, Damage: 3, Range: 1, AttackSpeed: 15, Type: Melee
  Food: 50, GPU: 0, Supply: 1, TrainTime: 50

Sentinel (Ranged Scout)
  HP: 60, Speed: 0.16, Damage: 12, Range: 6, AttackSpeed: 14, Type: Ranged
  Food: 75, GPU: 0, Supply: 1, TrainTime: 60

Rookclaw (Melee Dive Striker)
  HP: 70, Speed: 0.20, Damage: 10, Range: 1, AttackSpeed: 10, Type: Melee
  Food: 75, GPU: 0, Supply: 1, TrainTime: 55

Magpike (Disruptor/Thief)
  HP: 55, Speed: 0.18, Damage: 6, Range: 4, AttackSpeed: 12, Type: Ranged
  Food: 100, GPU: 25, Supply: 2, TrainTime: 80

Magpyre (Saboteur)
  HP: 50, Speed: 0.17, Damage: 8, Range: 3, AttackSpeed: 15, Type: Ranged
  Food: 100, GPU: 50, Supply: 2, TrainTime: 90

Jaycaller (Support/Buffer)
  HP: 85, Speed: 0.14, Damage: 5, Range: 4, AttackSpeed: 15, Type: Ranged
  Food: 100, GPU: 50, Supply: 2, TrainTime: 100

Jayflicker (Illusion Specialist)
  HP: 60, Speed: 0.16, Damage: 7, Range: 3, AttackSpeed: 12, Type: Ranged
  Food: 125, GPU: 50, Supply: 2, TrainTime: 90

Dusktalon (Stealth Assassin)
  HP: 65, Speed: 0.20, Damage: 15, Range: 1, AttackSpeed: 8, Type: Melee
  Food: 125, GPU: 25, Supply: 2, TrainTime: 80

Hootseer (Area Denial/Debuffer)
  HP: 100, Speed: 0.10, Damage: 8, Range: 5, AttackSpeed: 18, Type: Ranged
  Food: 150, GPU: 50, Supply: 3, TrainTime: 120

CorvusRex (Hero/Heavy)
  HP: 450, Speed: 0.10, Damage: 16, Range: 4, AttackSpeed: 15, Type: Ranged
  Food: 400, GPU: 200, Supply: 6, TrainTime: 250
```

**Design rationale:**
- Murder units are generally **lower HP** than cat equivalents (fragile faction identity)
- **Higher speed** across the board (aerial mobility)
- Scrounger is cheaper HP-wise than Pawdler (55 vs 60) but faster (0.14 vs 0.12)
- Sentinel has extraordinary range (6) but low HP -- a glass cannon scout
- Rookclaw is fast and bursty but fragile -- the dive striker role
- Dusktalon has the highest base damage of non-hero Murder units (15) for assassination
- CorvusRex is slightly weaker than MechCommander (450 vs 500 HP, 16 vs 18 damage) reflecting Murder's fragility, but has longer range (4 vs 3)

**Files to modify:**
- `crates/cc_core/src/unit_stats.rs` -- add 10 match arms, update tests
- Replace the `other => unimplemented!()` fallthrough or add before it

### 1D. Building Base Stats

Add 8 match arms to `building_stats()` in `crates/cc_core/src/building_stats.rs`:

```
TheParliament (HQ)
  HP: 450, BuildTime: 0 (pre-built), Food: 0, GPU: 0
  Supply: 10, CanProduce: [Scrounger]

Rookery (Barracks)
  HP: 275, BuildTime: 140 (14s), Food: 150, GPU: 0
  Supply: 0, CanProduce: [Sentinel, Rookclaw, Magpike, Jaycaller]

CarrionCache (Resource Depot)
  HP: 180, BuildTime: 100 (10s), Food: 100, GPU: 0
  Supply: 0, CanProduce: []

AntennaArray (Tech Building)
  HP: 225, BuildTime: 120 (12s), Food: 100, GPU: 75
  Supply: 0, CanProduce: [Magpyre, Jayflicker, Dusktalon, Hootseer, CorvusRex]

Panopticon (Research, limit 1)
  HP: 200, BuildTime: 120 (12s), Food: 125, GPU: 75
  Supply: 0, CanProduce: []

NestBox (Supply Depot)
  HP: 90, BuildTime: 75 (7.5s), Food: 75, GPU: 0
  Supply: 10, CanProduce: []

ThornHedge (Defensive Wall)
  HP: 120, BuildTime: 40 (4s), Food: 30, GPU: 0
  Supply: 0, CanProduce: []

Watchtower (Defense Tower)
  HP: 140, BuildTime: 80 (8s), Food: 75, GPU: 25
  Supply: 0, CanProduce: []
```

**Design rationale:**
- Murder buildings are generally **slightly less durable** than cat equivalents (fragile faction)
- TheParliament has 450 HP vs TheBox's 500 -- harder to turtle
- ThornHedge is cheap and fast (30 food, 4s) but fragile (120 HP) -- spam defense
- Panopticon is unique-limited (1 per player) -- addressed by a new component/system (see Phase 2)
- Watchtower costs the same as LaserPointer but has slightly lower HP (140 vs 150) -- offset by Glintwatch ability

**Files to modify:**
- `crates/cc_core/src/building_stats.rs` -- add 8 match arms, update tests

### 1E. AbilityId Variants

Add 30 new variants to the `AbilityId` enum in `crates/cc_core/src/abilities.rs`:

```rust
// --- Murder Faction ---
// Scrounger (worker)
TrinketStash,
Scavenge,
MimicCall,
// Sentinel (ranged scout)
Glintwatch,
Overwatch,
EvasiveAscent,
// Rookclaw (melee dive striker)
TalonDive,
MurdersMark,
CarrionInstinct,
// Magpike (disruptor/thief)
Pilfer,
GlitterBomb,
TrinketWard,
// Magpyre (saboteur)
SignalJam,
DecoyNest,
Rewire,
// Jaycaller (support/buffer)
RallyCry,
AlarmCall,
Cacophony,
// Jayflicker (illusion specialist)
PhantomFlock,
MirrorPosition,
Refraction,
// Dusktalon (stealth assassin)
Nightcloak,
SilentStrike,
PreySense,
// Hootseer (area denial/debuffer)
PanopticGaze,
DreadAura,
Omen,
// CorvusRex (hero)
CorvidNetwork,
AllSeeingLie,
OculusUplink,
```

**Files to modify:**
- `crates/cc_core/src/abilities.rs` -- add 30 variants to `AbilityId` enum

### 1F. AbilityDef Definitions

Add 30 match arms to `ability_def()` in `crates/cc_core/src/abilities.rs`, and add 10 match arms to `unit_abilities()`. Below are the proposed definitions with activation type, cooldown (in ticks at 10hz), GPU cost, duration (ticks), range (tiles), and max charges:

```
--- Scrounger ---
TrinketStash:    Passive,  cd=0,    gpu=0,  dur=0,   range=0,  charges=3
Scavenge:        Activated, cd=50,  gpu=0,  dur=20,  range=0,  charges=0
MimicCall:       Activated, cd=200, gpu=2,  dur=50,  range=6,  charges=0

--- Sentinel ---
Glintwatch:      Passive,  cd=0,    gpu=0,  dur=0,   range=12, charges=0
Overwatch:       Toggle,   cd=15,   gpu=0,  dur=0,   range=8,  charges=0
EvasiveAscent:   Passive,  cd=150,  gpu=0,  dur=20,  range=0,  charges=0

--- Rookclaw ---
TalonDive:       Activated, cd=100, gpu=0,  dur=5,   range=8,  charges=0
MurdersMark:     Passive,  cd=0,    gpu=0,  dur=150, range=0,  charges=0
CarrionInstinct: Passive,  cd=0,    gpu=0,  dur=0,   range=6,  charges=0

--- Magpike ---
Pilfer:          Activated, cd=180, gpu=0,  dur=0,   range=4,  charges=0
GlitterBomb:     Activated, cd=150, gpu=0,  dur=30,  range=5,  charges=0
TrinketWard:     Passive,  cd=0,    gpu=0,  dur=0,   range=0,  charges=0

--- Magpyre ---
SignalJam:       Activated, cd=300, gpu=4,  dur=100, range=8,  charges=0
DecoyNest:       Activated, cd=200, gpu=0,  dur=600, range=0,  charges=2
Rewire:          Activated, cd=250, gpu=5,  dur=0,   range=3,  charges=0

--- Jaycaller ---
RallyCry:        Activated, cd=200, gpu=0,  dur=80,  range=5,  charges=0
AlarmCall:       Passive,  cd=80,   gpu=0,  dur=30,  range=0,  charges=0
Cacophony:       Activated, cd=250, gpu=0,  dur=30,  range=4,  charges=0

--- Jayflicker ---
PhantomFlock:    Activated, cd=250, gpu=4,  dur=120, range=4,  charges=0
MirrorPosition:  Activated, cd=180, gpu=0,  dur=5,   range=8,  charges=0
Refraction:      Passive,  cd=0,    gpu=0,  dur=0,   range=6,  charges=0

--- Dusktalon ---
Nightcloak:      Passive,  cd=0,    gpu=0,  dur=0,   range=0,  charges=0
SilentStrike:    Activated, cd=200, gpu=0,  dur=0,   range=1,  charges=0
PreySense:       Passive,  cd=0,    gpu=0,  dur=0,   range=10, charges=0

--- Hootseer ---
PanopticGaze:    Toggle,   cd=10,   gpu=0,  dur=0,   range=6,  charges=0
DreadAura:       Passive,  cd=0,    gpu=0,  dur=0,   range=5,  charges=0
Omen:            Activated, cd=300, gpu=3,  dur=100, range=0,  charges=0

--- CorvusRex ---
CorvidNetwork:   Passive,  cd=0,    gpu=0,  dur=0,   range=10, charges=0
AllSeeingLie:    Activated, cd=900, gpu=8,  dur=30,  range=0,  charges=0
OculusUplink:    Passive,  cd=0,    gpu=0,  dur=0,   range=10, charges=0
```

Add `unit_abilities()` match arms (10 new entries, following the `[AbilityId; 3]` pattern):

```
Scrounger  => [TrinketStash, Scavenge, MimicCall]
Sentinel   => [Glintwatch, Overwatch, EvasiveAscent]
Rookclaw   => [TalonDive, MurdersMark, CarrionInstinct]
Magpike    => [Pilfer, GlitterBomb, TrinketWard]
Magpyre    => [SignalJam, DecoyNest, Rewire]
Jaycaller  => [RallyCry, AlarmCall, Cacophony]
Jayflicker => [PhantomFlock, MirrorPosition, Refraction]
Dusktalon  => [Nightcloak, SilentStrike, PreySense]
Hootseer   => [PanopticGaze, DreadAura, Omen]
CorvusRex  => [CorvidNetwork, AllSeeingLie, OculusUplink]
```

**Files to modify:**
- `crates/cc_core/src/abilities.rs` -- add 30 `AbilityId` variants, 30 `ability_def()` match arms, 10 `unit_abilities()` match arms, update tests

### 1G. UpgradeType Variants

Add Murder-specific upgrades to `UpgradeType` in `crates/cc_core/src/components.rs` and `upgrade_stats()`:

| Variant | Effect | Food | GPU | Research Ticks |
|---------|--------|------|-----|----------------|
| `SharperTalons` | +2 damage for all Murder combat units | 100 | 50 | 200 |
| `HardenedPlumage` | +20 HP for all Murder combat units | 100 | 50 | 200 |
| `SwiftWings` | +10% speed for all Murder units | 75 | 75 | 250 |
| `AssassinTraining` | Unlocks Dusktalon at AntennaArray | 150 | 100 | 300 |
| `RexPrototype` | Unlocks CorvusRex at AntennaArray | 200 | 150 | 400 |

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add variants to `UpgradeType`, update `Display`/`FromStr`
- `crates/cc_core/src/upgrade_stats.rs` -- add 5 match arms (or wherever upgrade_stats lives)

---

## Phase 2: Faction-Specific Components (cc_core)

### 2A. Aerial Marker

A new marker component indicating that a unit ignores terrain pathing and is immune to melee unless Grounded:

```rust
/// Marker: this unit is aerial — ignores terrain pathing, immune to melee unless Grounded.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Aerial;

/// Marker: aerial unit is temporarily grounded — can be hit by melee, cannot fly.
/// Removed when the grounded duration expires.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Grounded {
    pub remaining_ticks: u32,
}
```

**Which Murder units are Aerial:** All except Dusktalon (owl assassin, ground-based stealth) and CorvusRex (too heavy, ground-based hero). Specifically: Scrounger, Sentinel, Rookclaw, Magpike, Magpyre, Jaycaller, Jayflicker, Hootseer.

Design note: Dusktalon uses stealth (ground-based, like Mouser) rather than flight. CorvusRex is a heavy armored crow -- augmented too much to fly. This gives the Murder ground presence for melee engagement.

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add `Aerial` and `Grounded` components

### 2B. Fabrication System

Gemineye's signature mechanic: ~20% of intel data is fabricated. This is a global faction resource, not per-unit:

```rust
/// Tracks Gemineye's fabrication state for a player.
/// Used to determine if scouting data is real or fabricated.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
pub struct FabricationState {
    /// Base chance of fabrication (0-100). Default 20.
    pub base_chance: u32,
    /// Modifier from Panopticon (reduces to 10 if built).
    pub panopticon_reduction: u32,
    /// Modifier from destroyed Antenna Arrays (+5 per destroyed).
    pub array_penalty: u32,
    /// Modifier from Corvid Network cross-referencing (halves chance for networked units).
    pub network_reduction_active: bool,
}

impl FabricationState {
    /// Effective fabrication chance (clamped 0-100).
    pub fn effective_chance(&self) -> u32 {
        let base = self.base_chance
            .saturating_sub(self.panopticon_reduction)
            .saturating_add(self.array_penalty);
        base.min(100)
    }

    /// Deterministic check: is this event fabricated?
    /// Seeded from game tick + source entity ID for lockstep compatibility.
    pub fn is_fabricated(&self, tick: u64, source_id: u64) -> bool {
        let hash = simple_hash(tick, source_id);
        (hash % 100) < self.effective_chance() as u64
    }
}
```

**Files to modify:**
- New file or section in `crates/cc_core/src/components.rs` (or a new `crates/cc_core/src/fabrication.rs` module)

### 2C. Exposed Debuff

Murder-specific debuff that is richer than the existing `VisibleThroughFog`:

```rust
/// Murder-specific debuff: target is visible through fog to all Murder units,
/// and their HP, ability cooldowns, and active buffs are revealed.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct Exposed {
    pub remaining_ticks: u32,
    /// Which player applied the Exposed (for faction filtering).
    pub source_player: u8,
}
```

This is distinct from `VisibleThroughFog` (cat faction's Tagged) because Exposed also reveals unit state info, not just position. The existing `VisibleThroughFog` component can coexist -- different debuffs, different sources.

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add `Exposed` component

### 2D. Murder-Specific Trackers

New components for murder-specific ability state tracking:

```rust
/// Tracks Murder's Mark debuff on an enemy unit.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct MurdersMarkDebuff {
    pub remaining_ticks: u32,
}

/// Tracks Magpike's Trinket Ward passive stacking.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct TrinketWardTracker {
    pub trinkets_collected: u32,
}

/// Tracks Dusktalon's Nightcloak stealth state.
/// Re-uses existing Stealth component but with murder-specific re-stealth logic.
/// (May be handleable via the existing Stealth component with adjusted params.)

/// Tracks Hootseer's Panoptic Gaze cone direction.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct PanopticGazeCone {
    /// Direction of the cone center in radians (0 = east, PI/2 = north).
    pub direction: Fixed,
    /// Half-angle of the cone (60 degrees = PI/3 radians for 120-degree cone).
    pub half_angle: Fixed,
}

/// Tracks Rookclaw's Grounded state from Talon Dive.
/// Uses the Grounded component from 2A.

/// Unique building limit tracker.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
pub struct UniqueBuildingLimit;
```

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add tracker components

### 2E. AuraType Extensions

Add new aura types for Murder units to the `AuraType` enum:

```rust
// Add to existing AuraType enum:
DreadAura,        // Hootseer: -10% accuracy, -10% ability effectiveness
CorvidNetwork,    // CorvusRex: shared vision, -15% cooldowns, halved fabrication
OculusUplink,     // CorvusRex: 50% GPU cost reduction for AI commands
```

**Files to modify:**
- `crates/cc_core/src/components.rs` -- add variants to `AuraType` enum

---

## Phase 3: Simulation Systems (cc_sim)

### 3A. Production System Updates

**File:** `crates/cc_sim/src/systems/production_system.rs`

The production system spawns units with components based on `UnitKind`. Additions needed:

1. **Aerial component attachment:** After the base entity spawn, check if the unit kind belongs to Murder aerial units and insert `Aerial`:
   ```
   let murder_aerial = matches!(kind,
       UnitKind::Scrounger | UnitKind::Sentinel | UnitKind::Rookclaw |
       UnitKind::Magpike | UnitKind::Magpyre | UnitKind::Jaycaller |
       UnitKind::Jayflicker | UnitKind::Hootseer
   );
   if murder_aerial {
       entity_cmds.insert(Aerial);
   }
   ```

2. **Murder-specific spawn components** (matching the pattern of DreamSiegeTimer for Catnapper, Aura+NineLivesTracker for Chonk):
   - `Dusktalon` => insert `Stealth { stealthed: true, detection_radius: Fixed(3) }` + `Nightcloak` marker
   - `Hootseer` => insert `Aura { aura_type: AuraType::DreadAura, radius: Fixed(5), active: true }` + `PanopticGazeCone { direction: Fixed(0), half_angle: Fixed(PI/3) }`
   - `CorvusRex` => insert `Aura { aura_type: AuraType::CorvidNetwork, radius: Fixed(10), active: true }`
   - `Magpike` => insert `TrinketWardTracker { trinkets_collected: 0 }`
   - `Sentinel` => (Glintwatch is passive on the vision system, no special component beyond `Aerial`)

3. **Worker auto-gather for Scrounger:** The existing Pawdler auto-gather logic (lines 175-209 of production_system.rs) should also trigger for `UnitKind::Scrounger`. Extract the worker check to a helper function:
   ```
   fn is_worker(kind: UnitKind) -> bool {
       matches!(kind, UnitKind::Pawdler | UnitKind::Scrounger | ...)
   }
   ```

4. **Building-completion specialization** for Murder buildings (see 3F).

### 3B. Aerial Pathing System

**New system or modification to existing movement system.**

Currently the movement system uses A* pathfinding with terrain costs. Aerial units need to:
- Skip terrain passability checks entirely
- Move in straight lines (or simplified paths) ignoring obstacles
- Still respect map boundaries
- Still have GridCell updated for spatial queries

**Approach:** Modify the existing movement/pathfinding system rather than creating a separate one. When computing a path for a unit, check for `Has<Aerial>` (and not `Has<Grounded>`). If aerial and not grounded:
- Use direct line-of-sight movement (no A*)
- Ignore terrain impassability
- Still compute GridCell from position

**Files to modify:**
- `crates/cc_sim/src/systems/movement.rs` (or wherever pathfinding is invoked)
- May need to add `Aerial` and `Grounded` to the movement system query

### 3C. Aerial Combat Rules

Modifications to the combat/target_acquisition systems:

1. **Melee immunity for Aerial:** In `target_acquisition`, when a melee attacker (`AttackType::Melee`) selects a target, skip targets that have `Aerial` and do not have `Grounded`. This means melee units simply cannot target flying units.

2. **Anti-air bonus:** When a ranged unit deals damage to an `Aerial` target, apply a +50% damage multiplier. This can be implemented in the `combat` system damage calculation alongside the existing cover/elevation multipliers.

3. **Grounded tick-down:** New system (or added to cleanup/tick system) that decrements `Grounded.remaining_ticks` each tick and removes the component when it reaches 0.

**Files to modify:**
- `crates/cc_sim/src/systems/target_acquisition.rs` -- melee vs aerial filter
- `crates/cc_sim/src/systems/combat.rs` -- anti-air damage multiplier
- `crates/cc_sim/src/systems/cleanup.rs` or new tick-down system -- Grounded timer

### 3D. Fabrication System

**New system** that runs once per tick for players using Murder faction:

1. **Check Panopticon status:** Query all buildings for Murder player. If a completed Panopticon exists, set `panopticon_reduction = 10` in `FabricationState`.
2. **Check Antenna Array count:** Count destroyed/missing arrays vs expected, apply `+5` per destroyed.
3. **Corvid Network cross-referencing:** If CorvusRex is alive and units are within its aura, set `network_reduction_active = true` for those units' scouting abilities.

The fabrication check itself (`is_fabricated()`) is called from individual ability implementations (Glintwatch, Phantom Flock, All-Seeing Lie, Watchtower attacks) -- not from the system directly. The system just maintains the state.

**Files to create:**
- `crates/cc_sim/src/systems/fabrication_system.rs`

### 3E. Exposed Debuff System

**New tick-down system** (or extend existing status effect tick-down):

Each tick, decrement `Exposed.remaining_ticks` on all entities. Remove the component when it reaches 0. The fog-of-war system (when implemented) queries for `Exposed` to determine visibility.

**Files to modify:**
- `crates/cc_sim/src/systems/cleanup.rs` or new file -- tick down `Exposed`, `MurdersMarkDebuff`, `Grounded`

### 3F. Building-Completion Specialization

In `production_system.rs`, when a building finishes construction, add Murder-specific completion logic (matching the existing `BuildingKind::ScratchingPost` and `BuildingKind::LaserPointer` patterns):

- `BuildingKind::Panopticon` => insert `Researcher` + `ResearchQueue::default()` + `UniqueBuildingLimit`
- `BuildingKind::Watchtower` => insert `AttackStats { damage: 12, range: 7, attack_speed: 18, cooldown_remaining: 0 }` + `AttackTypeMarker { Ranged }` (longer range but slower fire than LaserPointer, matching Sentinel-class scope flavor)
- `BuildingKind::AntennaArray` => (no special components beyond Producer -- but destroying one triggers fabrication penalty via the fabrication system)
- `BuildingKind::ThornHedge` => insert pathfinding blocker component (blocks ground, slows aerial)

**Files to modify:**
- `crates/cc_sim/src/systems/production_system.rs` -- add match arms for Murder buildings

---

## Phase 4: AI FSM Updates (cc_sim)

### 4A. Personality Profile

Update `faction_personality()` in `crates/cc_sim/src/ai/fsm.rs` to use actual Murder units instead of cat proxies:

```rust
Faction::TheMurder => AiPersonalityProfile {
    name: "Gemineye".into(),
    attack_threshold: 7,
    unit_preferences: vec![
        (UnitKind::Rookclaw, 4),    // was FlyingFox -- dive strikers are Murder's main DPS
        (UnitKind::Sentinel, 3),    // was Mouser -- scouts/ranged
        (UnitKind::Magpike, 2),     // was Nuisance -- disruption
        (UnitKind::Jaycaller, 1),   // support
    ],
    target_workers: 4,
    economy_priority: false,
    retreat_threshold: 40,
    eval_speed_mult: 0.7,      // fast decisions (intel faction)
    chaos_factor: 20,          // Gemineye fabricates -- sometimes makes bad calls
    leak_chance: 0,
},
```

### 4B. Building Census

The `BuildingCensus` struct and `take_building_census()` function are currently hardcoded to cat building kinds. Two approaches:

**Option A (recommended): Faction-generic census.** Replace specific `has_cat_tree`, `cat_tree_entity` fields with a generic mapping:

```rust
struct BuildingCensus {
    has_hq: bool,
    has_barracks: bool,
    has_depot: bool,
    has_tech: bool,
    has_research: bool,
    has_defense_tower: bool,
    hq_entity: Option<Entity>,
    hq_pos: Option<GridPos>,
    barracks_entity: Option<Entity>,
    tech_entity: Option<Entity>,
    research_entity: Option<Entity>,
    // ... etc
    building_positions: Vec<(GridPos, BuildingKind)>,
    hq_queue_len: usize,
    barracks_queue_len: usize,
    tech_queue_len: usize,
    pending_supply_count: u32,
    tech_building_count: u32,  // for AI tier (replaces server_rack_count)
}
```

Then map Murder buildings to these roles:
- `TheParliament` => HQ
- `Rookery` => Barracks
- `CarrionCache` => Depot
- `AntennaArray` => Tech
- `Panopticon` => Research
- `NestBox` => Supply
- `Watchtower` => Defense Tower
- `ThornHedge` => (no AI census role -- built reactively)

**Option B: Duplicate census with Murder variants.** Less clean but lower risk of breaking existing code.

**Files to modify:**
- `crates/cc_sim/src/ai/fsm.rs` -- restructure `BuildingCensus` and `take_building_census()`

### 4C. Build Order Logic

The FSM build order logic in `run_ai_fsm()` references specific `BuildingKind` values. With a generic census (Option A), the logic becomes:

```
BuildUp phase:
  if !has_depot => Build depot (FishMarket or CarrionCache depending on faction)
  if !has_barracks => Build barracks (CatTree or Rookery)
MidGame phase:
  if !has_tech => Build tech (ServerRack or AntennaArray)
  if !has_research => Build research (ScratchingPost or Panopticon)
  if !has_defense_tower => Build tower (LaserPointer or Watchtower)
```

This requires the AI to know which faction it is. Add a `faction: Faction` field to `AiState`:

```rust
pub struct AiState {
    pub player_id: u8,
    pub faction: Faction,  // NEW
    pub phase: AiPhase,
    // ...
}
```

Then use a helper to map role to BuildingKind:

```rust
fn faction_building(faction: Faction, role: BuildingRole) -> BuildingKind {
    match (faction, role) {
        (Faction::CatGpt, BuildingRole::HQ) => BuildingKind::TheBox,
        (Faction::TheMurder, BuildingRole::HQ) => BuildingKind::TheParliament,
        // ...
    }
}
```

**Files to modify:**
- `crates/cc_sim/src/ai/fsm.rs` -- add `faction` to `AiState`, refactor build logic

### 4D. Unit Production Logic

The `take_unit_census()` function classifies workers vs army using `UnitKind::Pawdler`. Extend:

```rust
match unit_type.kind {
    UnitKind::Pawdler | UnitKind::Scrounger => {
        census.worker_count += 1;
        // ... same logic
    }
    _ => {
        census.army_count += 1;
        census.army_entities.push(entity);
    }
}
```

The `discover_enemy_spawn()` function looks for `BuildingKind::TheBox`. Extend to also check `BuildingKind::TheParliament` (and other faction HQs):

```rust
fn is_hq(kind: BuildingKind) -> bool {
    matches!(kind, BuildingKind::TheBox | BuildingKind::TheParliament | ...)
}
```

### 4E. Worker Type Mapping

The AI trains `UnitKind::Pawdler` at `TheBox`. For Murder, it trains `UnitKind::Scrounger` at `TheParliament`. The EarlyGame phase hardcodes `UnitKind::Pawdler` -- needs faction awareness:

```rust
fn worker_kind(faction: Faction) -> UnitKind {
    match faction {
        Faction::TheMurder => UnitKind::Scrounger,
        _ => UnitKind::Pawdler,
    }
}
```

**Files to modify:**
- `crates/cc_sim/src/ai/fsm.rs` -- refactor all hardcoded `UnitKind::Pawdler` references

---

## Phase 5: Client / Rendering (cc_client)

The client needs sprites for all Murder units and buildings. This is a content task, not a code architecture task, but some code changes are needed:

1. **Sprite mapping:** The client maps `UnitKind` to sprite sheet indices/asset paths. Add 10 new unit mappings and 8 new building mappings.

2. **Strategic icons (LOD):** Add strategic icon entities for all Murder units in the zoom LOD system.

3. **Aerial visual indicator:** Flying units should render at a slight visual offset (shadow below, unit above) to indicate elevation. This is a rendering-only concern -- the sim position stays on the ground tile.

4. **Exposed UI indicator:** When the player's units have `Exposed`, show a visual indicator (eye icon or highlight) on those units.

5. **Fabrication confidence UI:** When playing as Murder, show Gemineye's confidence level on scouted contacts (e.g., "87% confident" overlay). This is a UI element driven by `FabricationState`.

**Files to modify:**
- `crates/cc_client/src/setup.rs` -- sprite loading for Murder assets
- `crates/cc_client/src/units.rs` -- unit sprite mapping
- `crates/cc_client/src/selection.rs` -- strategic icon mapping
- Potentially new UI module for fabrication confidence display

---

## Phase 6: Agent / Harness (cc_agent, cc_harness)

### MCP Tool Updates (cc_harness)

The MCP harness exposes 35 tools. Several reference unit/building kinds by name or enum. Updates needed:

1. **Query tools** (`list_units`, `get_unit_info`, etc.) -- already work generically via `UnitKind` Display/FromStr, but the `FromStr` impl needs the new variants added (done in Phase 1).

2. **Command tools** (`train_unit`, `build`) -- same, need the new kind names parseable.

3. **New query tools** for Murder-specific data:
   - `get_fabrication_state` -- returns current fabrication chance, panopticon status
   - `get_exposed_units` -- returns list of enemy units currently Exposed
   - `get_aerial_status` -- returns whether a unit is Aerial, Grounded, etc.

### Agent Script Context (cc_agent)

The `ScriptContext` (25+ methods) may reference specific unit kinds in documentation or examples. Update documentation strings. The Lua sandbox should work generically with the new kinds since it operates on entity IDs and kind strings.

**Files to modify:**
- `crates/cc_harness/src/mcp_server.rs` -- add new query tools
- `crates/cc_agent/src/lib.rs` -- update ScriptContext documentation

---

## Phase 7: Tests

Following the project rule "every fix requires a test", all new code needs test coverage:

### cc_core tests (unit_stats.rs pattern):
- `all_murder_kinds_have_stats` -- 10 Murder UnitKinds return valid stats from `base_stats()`
- `murder_melee_units_have_range_one` -- Scrounger, Rookclaw, Dusktalon are melee with range 1
- `murder_ranged_units_have_range_gt_one` -- Sentinel, Magpike, etc.
- `corvus_rex_is_murder_strongest` -- CorvusRex has highest HP among Murder units
- `murder_units_are_fragile` -- Average Murder HP < average cat HP (faction identity validation)
- `all_murder_buildings_have_stats` -- 8 Murder BuildingKinds return valid stats
- `the_parliament_is_pre_built` -- build_time=0, cost=0
- `rookery_produces_basic_combat` -- can_produce contains Sentinel, Rookclaw, Magpike, Jaycaller
- `antenna_array_produces_advanced` -- can_produce contains Magpyre, Jayflicker, etc.
- `all_murder_ability_defs_valid` -- 30 Murder AbilityIds have valid AbilityDefs
- `murder_unit_abilities_returns_three` -- each Murder UnitKind returns 3 distinct abilities
- `murder_passive_abilities_no_cooldown` -- Glintwatch, MurdersMark, etc.
- `murder_toggle_abilities_have_cooldown` -- Overwatch, PanopticGaze
- `unitkind_display_from_str_round_trip` -- all 10 new variants survive serialization round-trip
- `buildingkind_display_from_str_round_trip` -- all 8 new variants

### cc_sim tests:
- `aerial_units_skip_terrain` -- Aerial unit pathfinds through impassable terrain
- `melee_cannot_target_aerial` -- Melee attacker skips Aerial target
- `melee_can_target_grounded_aerial` -- Melee attacker can hit Grounded Aerial
- `anti_air_bonus_damage` -- Ranged attack on Aerial deals +50% damage
- `grounded_timer_expires` -- Grounded component removed after tick countdown
- `murder_ai_trains_scrounger` -- AI with Murder faction trains Scrounger (not Pawdler)
- `murder_ai_builds_rookery` -- AI builds Rookery (not CatTree) in BuildUp phase
- `watchtower_gets_attack_stats` -- Watchtower completion inserts AttackStats
- `panopticon_gets_researcher` -- Panopticon completion inserts Researcher+ResearchQueue
- `production_spawns_aerial` -- Units produced from Rookery have Aerial component
- `fabrication_chance_default` -- FabricationState default is 20%
- `fabrication_chance_with_panopticon` -- With Panopticon, chance drops to 10%
- `exposed_ticks_down` -- Exposed component decrements and is removed at 0

### Integration tests:
- `murder_vs_catgpt_ai_match` -- Two AI players (Murder vs Cat) run for 2000 ticks without panic
- `murder_full_tech_tree` -- Build all 8 buildings, train all 10 units, no crashes

---

## System Modification Matrix

| System | File | Modification Type |
|--------|------|-------------------|
| `components.rs` | `cc_core` | Add 10 UnitKind, 8 BuildingKind, 5 UpgradeType, 3 AuraType variants, 5+ new components |
| `unit_stats.rs` | `cc_core` | Add 10 match arms |
| `building_stats.rs` | `cc_core` | Add 8 match arms |
| `abilities.rs` | `cc_core` | Add 30 AbilityId variants, 30 ability_def arms, 10 unit_abilities arms |
| `upgrade_stats.rs` | `cc_core` | Add 5 match arms |
| `production_system.rs` | `cc_sim` | Add Murder spawn-time components, building completion logic |
| `target_acquisition.rs` | `cc_sim` | Add Aerial melee immunity filter |
| `combat.rs` | `cc_sim` | Add anti-air damage multiplier |
| `movement.rs` | `cc_sim` | Add Aerial pathfinding bypass |
| `cleanup.rs` | `cc_sim` | Add Grounded/Exposed/MurdersMarkDebuff tick-down |
| `fsm.rs` | `cc_sim/ai` | Replace cat proxies, faction-aware building/worker logic |
| `fabrication_system.rs` | `cc_sim` | **NEW** -- fabrication state management |
| `setup.rs` | `cc_client` | Murder sprite loading |
| `units.rs` | `cc_client` | Murder unit rendering |
| `mcp_server.rs` | `cc_harness` | New Murder-specific query tools |

---

## Dependency Graph

```
Phase 1 (Data Layer) ──── no dependencies, pure data
  |
  v
Phase 2 (Components) ──── depends on Phase 1 for UnitKind/BuildingKind variants
  |
  v
Phase 3 (Systems) ─────── depends on Phase 2 for new components
  |
  v
Phase 4 (AI FSM) ──────── depends on Phase 1 (unit/building kinds) + Phase 3 (systems working)
  |
  v
Phase 5 (Client) ──────── depends on Phase 1 (sprite mapping) + Phase 2 (Aerial visual)
  |
  v
Phase 6 (Agent/Harness) ─ depends on Phase 1 (FromStr) + Phase 2 (new query data)
  |
  v
Phase 7 (Tests) ────────── depends on all above
```

Phases 5 and 6 can be parallelized. Phase 4 and 5 can also run in parallel since AI doesn't depend on rendering.

---

## Open Questions

1. **Faction-generic refactor scope:** Should the Building Census and AI FSM be fully genericized now (supporting all 6 factions) or should we do a Murder-specific fork and genericize later? A generic approach is cleaner but increases the scope of this task significantly. **Recommendation:** Do a minimal generic refactor -- introduce `BuildingRole` enum and `faction_building()` mapper, but don't refactor every line. Leave faction-specific fallback logic.

2. **Panopticon unique limit enforcement:** Where should the "only 1 Panopticon" rule be enforced? Options:
   - In the `GameCommand::Build` handler (reject the command)
   - In the AI FSM (don't issue the command)
   - Both (belt and suspenders)
   **Recommendation:** Both. The command handler validates it for human players; the AI FSM avoids it for bots.

3. **Fabrication and lockstep:** The fabrication system uses `simple_hash(tick, entity_id)` for determinism. This must be a pure function with no external state beyond the seed -- no `rand`, no system clock. Verify that `EntityId` bits are deterministic across clients in networked play. **Note:** Entity allocation order must be deterministic, which is already required by the lockstep architecture.

4. **Aerial visual representation:** Do aerial units need a separate "shadow" sprite on the ground tile? This affects the asset pipeline. The Into the Breach style already handles elevation visually with shadow offsets. **Recommendation:** Yes, add a small circular shadow sprite at the unit's grid position, with the unit sprite rendered at a +Y visual offset.

5. **ThornHedge pathing interaction:** ThornHedge blocks ground but slows aerial. The current pathfinding grid is binary (passable/impassable). Aerial units bypass it entirely, so the "slow aerial" effect needs a different mechanism -- perhaps a status effect applied when an aerial unit's GridCell overlaps a ThornHedge tile. **Recommendation:** Add a `terrain_effects_system` that checks aerial unit grid cells against ThornHedge positions and applies a temporary speed debuff.

6. **Ability implementation phasing:** Implementing all 30 abilities is a massive task. Should this plan cover ability implementations or just the data/plumbing? **Recommendation:** This plan covers the data definitions and component plumbing. Each ability's behavioral implementation should be a separate task, prioritized by gameplay impact:
   - **Priority 1 (core gameplay):** TalonDive, Glintwatch, Nightcloak, Overwatch, RallyCry
   - **Priority 2 (faction identity):** PhantomFlock, MimicCall, SignalJam, CorvidNetwork, AllSeeingLie
   - **Priority 3 (polish):** Pilfer, GlitterBomb, Rewire, Omen, MirrorPosition, etc.

7. **Worker gather mechanics:** Scrounger has TrinketStash (hidden ground caches) and Scavenge (post-combat resource extraction). These are significantly different from Pawdler's gather loop. The existing `Gathering` component and `GatherState` FSM are Pawdler-specific. Options:
   - Extend `GatherState` with Scrounger states
   - Create a separate `ScroungerGathering` component
   **Recommendation:** Separate component. The gather loops are different enough that sharing state is more confusing than helpful.

8. **Upgrade applicability:** Murder upgrades (SharperTalons, HardenedPlumage, etc.) should only apply to Murder units. The existing `apply_upgrades_to_new_unit()` doesn't check faction -- it applies cat upgrades to all units. Need to either:
   - Check faction when applying upgrades
   - Separate upgrade sets per faction in `PlayerResources`
   **Recommendation:** Add `faction: Faction` to the `apply_upgrades_to_new_unit` signature and filter accordingly.
