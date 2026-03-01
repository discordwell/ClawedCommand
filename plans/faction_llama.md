# LLAMA Faction Implementation Plan

> Locally Leveraged Alliance for Material Appropriation (Raccoons / AI: Llhama)

This plan covers everything required to make LLAMA a fully playable faction with its own units, buildings, abilities, faction-specific mechanics, AI FSM profile, and all supporting systems.

---

## Table of Contents

1. [Enum & Component Additions (cc_core)](#1-enum--component-additions-cc_core)
2. [Unit Stats](#2-unit-stats)
3. [Building Stats](#3-building-stats)
4. [Abilities](#4-abilities)
5. [Faction-Specific Systems (New)](#5-faction-specific-systems-new)
6. [Production System Updates (cc_sim)](#6-production-system-updates-cc_sim)
7. [AI FSM Updates](#7-ai-fsm-updates)
8. [Existing System Modifications](#8-existing-system-modifications)
9. [New Components](#9-new-components)
10. [Test Plan](#10-test-plan)
11. [Task List](#11-task-list)

---

## 1. Enum & Component Additions (cc_core)

### File: `crates/cc_core/src/components.rs`

#### UnitKind — Add 10 variants

```
Scrounger,       // Worker (Raccoon) — gathers, builds, hauls salvage, pocket stash
Bandit,          // Light Skirmisher (Raccoon) — steals buffs, jury-rigs from wrecks
HeapTitan,       // Heavy Tank (Raccoon) — scrap armor, grows tankier near wrecks
GlitchRat,       // Saboteur Scout (Rat) — disrupts buildings, scrambles signals
PatchPossum,     // Field Medic/Engineer (Possum) — heals, raises Scrap Golems
GreaseMonkey,    // Ranged/Siege (Raccoon) — random damage, salvage turrets
DeadDrop,        // Stealth/Intel (Possum) — eavesdrops, injects fake leaks
Wrecker,         // Anti-Armor Melee (Raccoon) — shreds armor, severs auras
DumpsterDiver,   // Specialist/Utility (Possum) — NFT boost, barricades, stench
JunkyardKing,    // Hero/Heavy (Raccoon in Mech) — uplink, Frankenstein, overclock
```

Also update:
- `UnitKind::fmt` / `Display` impl
- `UnitKind::FromStr` impl — add all 10 string-to-variant mappings

#### BuildingKind — Add 8 variants

```
TheDumpster,     // Command Center — pre-built, produces Scrounger
ScrapHeap,       // Resource Depot — food/scrap storage, gather bonus near ruins
ChopShop,        // Barracks/Factory — trains combat units, scrap token acceleration
JunkServer,      // Tech Building — produces advanced units (GlitchRat, PatchPossum)
TinkerBench,     // Research — upgrades. 15% less Food, 10% more GPU cost
TrashPile,       // Supply Depot — increases supply cap
DumpsterRelay,   // Comms Tower — reduces leak chance to 15% in 10-tile radius, +3 vision
TetanusTower,    // Defense Tower — shoots rusty nails, applies Corroded stacks
```

Also update:
- `BuildingKind::fmt` / `Display` impl
- `BuildingKind::FromStr` impl

#### UpgradeType — Add LLAMA-specific upgrades

```
SharperTeeth,    // +2 damage for LLAMA combat units (parallel to SharperClaws)
ThickerHide,     // +25 HP for LLAMA combat units (parallel to ThickerFur)
NimblePaws,      // +10% speed (shared? or separate LlamaSwiftFeet)
SiegeWelding,    // Unlocks Grease Monkey advanced mode at Chop Shop
MechSalvage,     // Unlocks Junkyard King training at TinkerBench
```

**Decision needed**: Do LLAMA upgrades share cat upgrade variants (SharperClaws etc.) or get their own? Recommendation: **own variants** to allow faction-specific naming/flavor and independent balance tuning. The research_system's `apply_upgrades_to_new_unit` already branches on UnitKind, so adding LLAMA-specific upgrade types is straightforward.

#### AuraType — Add new variants

```
OpenSourceUplink,   // Junkyard King — 40% cheaper GPU commands, 10% leak chance in radius
ScrapArmor,         // Heap Titan — passive, radius-based damage reduction from wrecks
DumpsterRelayAura,  // Dumpster Relay building — 15% leak reduction in radius
StenchCloud,        // Dumpster Diver — debuff enemies in radius
```

---

## 2. Unit Stats

### File: `crates/cc_core/src/unit_stats.rs`

Add 10 new arms to `base_stats()`. Design philosophy: LLAMA units have ~15-20% lower base stats than catGPT equivalents, compensated by wreck-scaling mechanics.

| Unit | HP | Speed | Damage | Range | Atk Speed (ticks) | Attack Type | Food | GPU | Supply | Train Time |
|------|----|-------|--------|-------|-------------------|-------------|------|-----|--------|------------|
| **Scrounger** | 55 | 0.11 | 3 | 1 | 15 | Melee | 45 | 0 | 1 | 45 |
| **Bandit** | 70 | 0.19 | 7 | 1 | 9 | Melee | 65 | 0 | 1 | 55 |
| **Heap Titan** | 280 | 0.07 | 10 | 1 | 22 | Melee | 140 | 20 | 3 | 110 |
| **Glitch Rat** | 40 | 0.22 | 5 | 1 | 12 | Melee | 60 | 15 | 1 | 50 |
| **Patch Possum** | 80 | 0.13 | 4 | 3 | 15 | Ranged | 90 | 25 | 2 | 80 |
| **Grease Monkey** | 65 | 0.10 | 12 | 4 | 14 | Ranged | 90 | 10 | 2 | 75 |
| **Dead Drop** | 50 | 0.14 | 8 | 1 | 12 | Melee | 80 | 20 | 1 | 65 |
| **Wrecker** | 100 | 0.12 | 14 | 1 | 10 | Melee | 110 | 15 | 2 | 85 |
| **Dumpster Diver** | 75 | 0.11 | 6 | 2 | 15 | Ranged | 85 | 20 | 2 | 70 |
| **Junkyard King** | 450 | 0.09 | 16 | 3 | 16 | Ranged | 375 | 175 | 6 | 230 |

**Rationale for stat deltas vs catGPT**:
- Scrounger vs Pawdler: 5 less HP, slightly slower, 1 less damage, cheaper (scavenging offsets)
- Bandit vs Nuisance: 10 less HP, slightly faster, 1 less damage, cheaper (jury-rig compensates)
- Heap Titan vs Chonk: 20 less HP, slightly slower, 2 less damage, cheaper (Scrap Armor passive compensates heavily)
- Glitch Rat vs Mouser: 15 less HP, faster, 5 less damage (utility-focused, not a combat unit)
- Patch Possum: no direct cat analog (support hybrid)
- Grease Monkey vs Hisser: 5 less HP, slower, 2 less damage, shorter range (random crits compensate)
- Dead Drop vs Mouser: 5 less HP, slower, 2 less damage (intel-focused)
- Wrecker: no direct cat analog (anti-armor specialist)
- Dumpster Diver: no direct cat analog (utility/NFT specialist)
- Junkyard King vs MechCommander: 50 less HP, slightly slower, 2 less damage, cheaper (Frankenstein compensates)

**Fixed-point encoding** follows the existing pattern:
- HP: `Fixed::from_bits(N << 16)` where N is the integer HP value
- Speed: `Fixed::from_bits(N)` where N encodes the fractional value (e.g., 0.11 = ~7209)
- Damage: `Fixed::from_bits(N << 16)`
- Range: `Fixed::from_bits(N << 16)`

### Update tests

- Expand `all_kinds_have_stats` to include all 10 LLAMA units
- Add `melee_units_have_range_one` entries for: Scrounger, Bandit, HeapTitan, GlitchRat, DeadDrop, Wrecker
- Add `ranged_units_have_range_greater_than_one` entries for: PatchPossum, GreaseMonkey, DumpsterDiver, JunkyardKing
- Add `heap_titan_tankiest_llama` test (Heap Titan highest HP among non-hero LLAMA)
- Add `junkyard_king_strongest_llama` test

---

## 3. Building Stats

### File: `crates/cc_core/src/building_stats.rs`

Add 8 new arms to `building_stats()`.

| Building | HP | Build Time (ticks) | Food Cost | GPU Cost | Supply | Can Produce |
|----------|----|--------------------|-----------|----------|--------|-------------|
| **The Dumpster** | 500 | 0 (pre-built) | 0 | 0 | 10 | [Scrounger] |
| **Scrap Heap** | 180 | 90 | 90 | 0 | 0 | [] |
| **Chop Shop** | 280 | 140 | 140 | 0 | 0 | [Bandit, Wrecker, HeapTitan, GreaseMonkey] |
| **Junk Server** | 230 | 110 | 90 | 65 | 0 | [GlitchRat, PatchPossum] |
| **Tinker Bench** | 190 | 95 | 85 | 55 | 0 | [DeadDrop, DumpsterDiver, JunkyardKing] |
| **Trash Pile** | 90 | 70 | 70 | 0 | 10 | [] |
| **Dumpster Relay** | 150 | 80 | 80 | 30 | 0 | [] |
| **Tetanus Tower** | 140 | 75 | 70 | 20 | 0 | [] |

**Design notes**:
- The Dumpster mirrors The Box exactly (same HP, supply, pre-built)
- Scrap Heap mirrors Fish Market but slightly cheaper (LLAMA's scavenging theme)
- Chop Shop mirrors CatTree but costs slightly less food (scrap token economy compensates)
- Tinker Bench research costs: 15% less Food, 10% more GPU than ScratchingPost equivalent
- Tetanus Tower mirrors LaserPointer in role but with different combat stats (Corroded stacks)

### Update tests

- Expand `all_buildings_have_stats` to include all 8 LLAMA buildings
- Add `the_dumpster_is_pre_built` test
- Add `the_dumpster_produces_scrounger` test
- Add `chop_shop_produces_basic_combat_units` test
- Add `tinker_bench_produces_advanced_units` test
- Add `trash_pile_provides_supply` test

---

## 4. Abilities

### File: `crates/cc_core/src/abilities.rs`

#### AbilityId — Add 30 variants (3 per unit)

```rust
// --- Scrounger ---
DumpsterDiveAbility,   // Primary gather (passive/modified gather behavior)
PocketStash,           // Passive: auto-collect scrap tokens from wrecks
PlayDead,              // Active: become untargetable "wreck" for 8s

// --- Bandit ---
StickyFingers,         // Passive: every 4th attack steals buff or +30% damage
JuryRig,               // Active: channel on wreck for temp stat boost
Getaway,               // Active: 1.5s sprint at 2.5x speed + smoke if looted

// --- Heap Titan ---
ScrapArmorAbility,     // Passive: +8% DR per wreck within 4 tiles (max 60%)
WreckBall,             // Active: hurl wreck at area (5-tile range, AoE)
MagneticPulse,         // Active: pull scrap/wrecks, 25% ranged miss chance

// --- Glitch Rat ---
CableGnaw,             // Active: disable building 15s, halve production
SignalScramble,         // Active (4 GPU): disorient enemy unit 4-8s
TunnelRat,             // Passive: use any tunnel network incl. enemy

// --- Patch Possum ---
DuctTapeFix,           // Active: heal 30% HP, extend jury-rigs, or shield
SalvageResurrection,   // Active: raise wreck as Scrap Golem (50% HP, 40% dmg)
FeignDeath,            // Passive: survive lethal damage, play dead 3s, revive 20%

// --- Grease Monkey ---
JunkLauncher,          // Passive: random 70-130% damage, 10% crit with debuffs
SalvageTurret,         // Active: build turret from wreck (60% dmg, 20s life)
Overcharge,            // Active: next 3 shots max damage + crit, then 5s jam

// --- Dead Drop ---
Eavesdrop,             // Passive: detect enemy AI commands within 8 tiles
TrashHeapAmbush,       // Active: bury for invis + doubled Eavesdrop range
LeakInjection,         // Active (5 GPU): create fake Leaked Plan on enemy screen

// --- Wrecker ---
Disassemble,           // Passive: each attack strips 5% armor, +15% at 0
PryBar,                // Active: disable building 4s, gain Component on kill
ChainBreak,            // Active: sever aura for 6s + 50 feedback dmg, or 80 flat

// --- Dumpster Diver ---
TreasureTrash,         // Passive: +30% NFT gen at Monkey Mine, 10% cache chance
RefuseShield,          // Active: build barricade from wreck (150% HP wall, 15s)
StenchCloudAbility,    // Active: 3-tile debuff cloud (-20% acc, -25% dmg, 6s)

// --- Junkyard King ---
OpenSourceUplink,      // Passive: 40% cheaper GPU cmds in 8 tiles, 10% leak
FrankensteinProtocol,  // Active (10 GPU): rebuild wreck as 70% permanent unit
OverclockCascade,      // Active: +30% atk speed, +20% move for 8s, then 5% HP drain
```

#### AbilityDef values for each

Below are the proposed `ability_def()` entries. All durations and cooldowns are in ticks (10 ticks = 1 second).

| AbilityId | Activation | Cooldown | GPU Cost | Duration | Range | Max Charges |
|-----------|-----------|----------|----------|----------|-------|-------------|
| DumpsterDiveAbility | Passive | 0 | 0 | 0 | 0 | 0 |
| PocketStash | Passive | 0 | 0 | 0 | 0 | 0 |
| PlayDead | Activated | 200 (20s) | 0 | 80 (8s) | 0 | 0 |
| StickyFingers | Passive | 0 | 0 | 0 | 0 | 0 |
| JuryRig | Activated | 50 (5s) | 0 | 20 (2s channel) | 1 | 0 |
| Getaway | Activated | 150 (15s) | 0 | 15 (1.5s) | 0 | 0 |
| ScrapArmorAbility | Passive | 0 | 0 | 0 | 4 | 0 |
| WreckBall | Activated | 120 (12s) | 0 | 0 | 5 | 0 |
| MagneticPulse | Activated | 250 (25s) | 0 | 40 (4s) | 3 | 0 |
| CableGnaw | Activated | 300 (30s) | 0 | 30 (3s channel) | 1 | 0 |
| SignalScramble | Activated | 200 (20s) | 4 | 40 (4s) | 6 | 0 |
| TunnelRat | Passive | 0 | 0 | 0 | 0 | 0 |
| DuctTapeFix | Activated | 100 (10s) | 0 | 50 (5s) | 4 | 0 |
| SalvageResurrection | Activated | 250 (25s) | 0 | 40 (4s channel) | 1 | 0 |
| FeignDeath | Passive | 450 (45s) | 0 | 30 (3s) | 0 | 0 |
| JunkLauncher | Passive | 0 | 0 | 0 | 0 | 0 |
| SalvageTurret | Activated | 150 (15s)* | 0 | 200 (20s life) | 2 | 1 |
| Overcharge | Activated | 200 (20s)** | 0 | 60 (6s) | 0 | 0 |
| Eavesdrop | Passive | 0 | 0 | 0 | 8 | 0 |
| TrashHeapAmbush | Activated | 80 (8s) | 0 | 0 | 0 | 0 |
| LeakInjection | Activated | 300 (30s) | 5 | 40 (4s) | 0 | 0 |
| Disassemble | Passive | 0 | 0 | 0 | 0 | 0 |
| PryBar | Activated | 180 (18s) | 0 | 40 (4s) | 1 | 0 |
| ChainBreak | Activated | 140 (14s) | 0 | 60 (6s) | 3 | 0 |
| TreasureTrash | Passive | 0 | 0 | 0 | 0 | 0 |
| RefuseShield | Activated | 200 (20s) | 0 | 150 (15s) | 3 | 0 |
| StenchCloudAbility | Activated | 180 (18s) | 0 | 60 (6s) | 3 | 0 |
| OpenSourceUplink | Passive | 0 | 0 | 0 | 8 | 0 |
| FrankensteinProtocol | Activated | 450 (45s) | 10 | 0 | 3 | 0 |
| OverclockCascade | Activated | 350 (35s) | 0 | 80 (8s) | 6 | 0 |

\* SalvageTurret cooldown starts after turret expires or is destroyed.
\** Overcharge cooldown starts after the 5s jam window ends.

#### unit_abilities() — Add 10 new arms

```rust
UnitKind::Scrounger => [DumpsterDiveAbility, PocketStash, PlayDead],
UnitKind::Bandit => [StickyFingers, JuryRig, Getaway],
UnitKind::HeapTitan => [ScrapArmorAbility, WreckBall, MagneticPulse],
UnitKind::GlitchRat => [CableGnaw, SignalScramble, TunnelRat],
UnitKind::PatchPossum => [DuctTapeFix, SalvageResurrection, FeignDeath],
UnitKind::GreaseMonkey => [JunkLauncher, SalvageTurret, Overcharge],
UnitKind::DeadDrop => [Eavesdrop, TrashHeapAmbush, LeakInjection],
UnitKind::Wrecker => [Disassemble, PryBar, ChainBreak],
UnitKind::DumpsterDiver => [TreasureTrash, RefuseShield, StenchCloudAbility],
UnitKind::JunkyardKing => [OpenSourceUplink, FrankensteinProtocol, OverclockCascade],
```

#### Update tests

- Expand `all_ability_defs_valid` to include all 30 new AbilityIds (total: 60)
- Expand `unit_abilities_returns_three_per_kind` to include all 10 LLAMA kinds
- Expand `passive_abilities_no_cooldown` with LLAMA passives (DumpsterDiveAbility, PocketStash, StickyFingers, ScrapArmorAbility, TunnelRat, JunkLauncher, Eavesdrop, Disassemble, TreasureTrash, OpenSourceUplink)
- Expand `activated_abilities_have_cooldown` with LLAMA activated abilities

---

## 5. Faction-Specific Systems (New)

LLAMA introduces several systems that do not exist in the current codebase. Each requires both new components and new simulation systems.

### 5.1 Wreck Persistence System

**New file**: `crates/cc_sim/src/systems/wreck_system.rs`

**Purpose**: When a unit dies, instead of despawning immediately, leave a `Wreck` entity on the map for 20 seconds (200 ticks). Only enemy wrecks are interactable by LLAMA units.

**Components** (in `cc_core/src/components.rs`):

```rust
/// A persistent wreck left behind by a dead unit.
pub struct Wreck {
    pub original_kind: UnitKind,
    pub original_faction: Faction,
    pub original_max_hp: Fixed,
    pub original_damage: Fixed,
    pub original_attack_type: AttackType,
    pub remaining_ticks: u32,   // 200 ticks = 20s
    pub salvaged: bool,         // can only be salvaged once
}
```

**System logic**:
- Hooks into the existing death/cleanup system. When a unit gets the `Dead` marker and the game has LLAMA-related mechanics active, spawn a `Wreck` entity at the dead unit's position with its original stats.
- Each tick, decrement `remaining_ticks`. At 0, despawn.
- `salvaged` flag prevents double-salvage.

**Modification to existing cleanup system** (`crates/cc_sim/src/systems/cleanup.rs`):
- Currently: Dead marker -> despawn.
- New: Dead marker -> if any LLAMA player exists in the game, spawn Wreck entity at position with original stats, THEN despawn the unit. If no LLAMA player, no wreck spawning (zero overhead for non-LLAMA games).

### 5.2 Salvage / Jury-Rig System

**New file**: `crates/cc_sim/src/systems/salvage_system.rs`

**Components**:

```rust
/// A temporary stat modification from salvaging an enemy wreck.
pub struct JuryRigMod {
    pub stat_type: JuryRigStat, // enum: Armor, Speed, Range, Damage
    pub bonus_percent: u32,     // e.g. 25 = +25%
    pub remaining_ticks: u32,   // 600 ticks = 60s default
    pub source_unit_type: UnitKind,
}

pub enum JuryRigStat {
    Armor,    // from tank wrecks
    Speed,    // from scout wrecks
    Range,    // from ranged wrecks
    Damage,   // from melee wrecks
}

/// Tracks active jury-rig modifications on a unit. Max 3 — FIFO replacement.
pub struct JuryRigSlots {
    pub mods: Vec<JuryRigMod>,  // max length 3
}
```

**System logic**:
- Tick down all `JuryRigMod` durations. Remove expired ones.
- When a Bandit activates `JuryRig` near a wreck: determine stat type from `wreck.original_kind`, apply corresponding mod, mark wreck as salvaged.
- Feed into `StatModifiers` computation each tick (new integration point in status effect aggregation).

### 5.3 Scrap Token Economy

**New file**: `crates/cc_sim/src/systems/scrap_system.rs`

**Components**:

```rust
/// Scrounger's personal scrap inventory.
pub struct PocketStashInventory {
    pub count: u32,
    pub max: u32,  // 3
}

/// Scrap tokens tracked per-player (analogous to Food/GPU/NFT).
/// Deposited at Chop Shop to accelerate production.
```

**Integration with PlayerResources**:
- Add `scrap_tokens: u32` field to each player's resource struct.
- Chop Shop production queue ticks faster based on deposited scrap (2s = 20 ticks per token).

**System logic**:
- When a Scrounger moves within 1 tile of an unsalvaged enemy wreck and has `PocketStashInventory.count < max`, auto-collect 1 scrap token. Mark wreck as salvaged.
- When a Scrounger returns to a Chop Shop, deposit scrap tokens. Each token reduces the current production queue item's remaining ticks by 20.

### 5.4 Leak System

**New file**: `crates/cc_sim/src/systems/leak_system.rs`

**Components**:

```rust
/// A leaked AI plan visible to enemy players.
pub struct LeakedPlan {
    pub command_type: LeakedCommandType,
    pub target_pos: GridPos,
    pub visible_to: Vec<u8>,       // player_ids who can see it
    pub remaining_ticks: u32,       // 30 ticks = 3s
    pub source: LeakSource,
}

pub enum LeakedCommandType {
    Attack,
    Move,
    Build,
    Train,
    UseAbility,
}

pub enum LeakSource {
    Authentic,   // real leaked plan
    Fabricated,  // from Leak Injection ability
}
```

**System logic**:
- Intercepts GPU-costing commands issued by Llhama AI.
- For each command, roll against `leak_chance` (30% base, 15% near Dumpster Relay, 10% near Junkyard King).
- On leak: spawn a `LeakedPlan` entity visible to all enemy players.
- Tick down `remaining_ticks`, despawn at 0.
- Leak Injection: spawns a `LeakedPlan` with `source: Fabricated` — identical in appearance to enemies.

**Integration with AiPersonalityProfile**:
- `leak_chance: u32` already exists (currently 30 for Llama).
- Add modifier logic: check proximity to Dumpster Relay buildings and Junkyard King units.

### 5.5 Eavesdrop System

**New file**: `crates/cc_sim/src/systems/eavesdrop_system.rs`

**Prerequisite**: The command system needs a spatial command log — a queryable record of recent commands with positions.

**Components**:

```rust
/// Tracks commands issued by all players, queryable by position.
pub struct CommandLog {
    pub entries: VecDeque<CommandLogEntry>,
}

pub struct CommandLogEntry {
    pub player_id: u8,
    pub command_type: LeakedCommandType,
    pub target_pos: GridPos,
    pub tick: u64,
}
```

**System logic**:
- Dead Drop units with Eavesdrop passive query the command log for enemy commands within 8 tiles (16 tiles if in Trash Heap Ambush).
- Intercepted commands are revealed to the entire LLAMA team for 5 seconds (50 ticks).

### 5.6 Frankenstein / Salvage Resurrection System

**New file**: `crates/cc_sim/src/systems/frankenstein_system.rs`

**Components**:

```rust
/// Marker for units created via Frankenstein Protocol.
pub struct FrankensteinUnit {
    pub misfire_chance: u32,  // 5 = 5% per attack
    pub original_kind: UnitKind,
}

/// Marker for temporary Scrap Golems from Salvage Resurrection.
pub struct ScrapGolem {
    pub remaining_ticks: u32,  // 300 ticks = 30s
    pub owner_possum: EntityId,
}
```

**System logic — Frankenstein Protocol (Junkyard King)**:
- Targets an enemy wreck within 3 tiles.
- Spawns a new unit of the wreck's `original_kind` with 70% stats and 1 random ability.
- Unit gets `FrankensteinUnit` component. Max 3 alive at once (tracked on the Junkyard King entity).
- On attack: 5% misfire chance deals 10% self-damage.
- GPU cost: 10 (reduced by Open Source Uplink).

**System logic — Salvage Resurrection (Patch Possum)**:
- Channels on enemy wreck for 4 seconds.
- Spawns a Scrap Golem: 50% HP, 40% damage, no abilities.
- Scrap Golem lasts 30s or until killed. Max 2 per Patch Possum.
- Wreck consumed.

---

## 6. Production System Updates (cc_sim)

### File: `crates/cc_sim/src/systems/production_system.rs`

The production system currently spawns units with type-specific components based on `UnitKind`. For LLAMA units, add:

```rust
// --- LLAMA spawn-time components ---

// Scrounger: PocketStashInventory
if kind == UnitKind::Scrounger {
    entity_cmds.insert(PocketStashInventory { count: 0, max: 3 });
}

// Heap Titan: ScrapArmor aura
if kind == UnitKind::HeapTitan {
    entity_cmds.insert(Aura {
        aura_type: AuraType::ScrapArmor,
        radius: Fixed::from_bits(4 << 16),
        active: true,
    });
}

// Dead Drop: Stealth + Eavesdrop range tracking
if kind == UnitKind::DeadDrop {
    entity_cmds.insert(Stealth {
        stealthed: false,
        detection_radius: Fixed::from_bits(8 << 16),
    });
}

// Patch Possum: FeignDeath tracker (similar to NineLivesTracker)
if kind == UnitKind::PatchPossum {
    entity_cmds.insert(FeignDeathTracker::default());
}

// Grease Monkey: JunkLauncher variance tracker
if kind == UnitKind::GreaseMonkey {
    entity_cmds.insert(JunkLauncherState::default());
}

// Junkyard King: OpenSourceUplink aura + Frankenstein counter
if kind == UnitKind::JunkyardKing {
    entity_cmds.insert((
        Aura {
            aura_type: AuraType::OpenSourceUplink,
            radius: Fixed::from_bits(8 << 16),
            active: true,
        },
        FrankensteinTracker { active_count: 0, max: 3 },
    ));
}
```

### Building completion components

In the same file, when buildings complete construction:

```rust
// Tinker Bench gets Researcher + ResearchQueue (mirrors ScratchingPost)
if building.kind == BuildingKind::TinkerBench {
    commands.entity(entity).insert((Researcher, ResearchQueue::default()));
}

// Tetanus Tower gets AttackStats (mirrors LaserPointer)
if building.kind == BuildingKind::TetanusTower {
    commands.entity(entity).insert((
        AttackStats {
            damage: Fixed::from_bits(8 << 16),  // 8 damage
            range: Fixed::from_bits(6 << 16),    // 6 range
            attack_speed: 12,                     // 1.2s between attacks
            cooldown_remaining: 0,
        },
        AttackTypeMarker { attack_type: AttackType::Ranged },
        // Corroded stack applicator (new component)
        CorrodedApplicator { stacks_per_hit: 1, max_stacks: 4 },
    ));
}

// Dumpster Relay gets aura component for leak suppression
if building.kind == BuildingKind::DumpsterRelay {
    commands.entity(entity).insert(Aura {
        aura_type: AuraType::DumpsterRelayAura,
        radius: Fixed::from_bits(10 << 16),
        active: true,
    });
}
```

### Auto-gather for Scrounger

Mirror the existing Pawdler auto-gather logic for Scrounger:

```rust
} else if kind == UnitKind::Scrounger {
    // Same auto-gather as Pawdler — send to nearest deposit
    // (identical logic, different UnitKind)
}
```

---

## 7. AI FSM Updates

### File: `crates/cc_sim/src/ai/fsm.rs`

#### Replace cat unit proxies in Llama personality

Current Llama profile uses cat UnitKind proxies:
```rust
unit_preferences: vec![
    (UnitKind::FerretSapper, 4),
    (UnitKind::Nuisance, 3),
    (UnitKind::Mouser, 2),
],
```

Replace with native LLAMA units:
```rust
Faction::Llama => AiPersonalityProfile {
    name: "Llhama".into(),
    attack_threshold: 5,
    unit_preferences: vec![
        (UnitKind::Bandit, 4),       // was FerretSapper — aggressive skirmishers
        (UnitKind::Wrecker, 3),      // was Nuisance — anti-armor pressure
        (UnitKind::GreaseMonkey, 2), // was Mouser — ranged damage
        (UnitKind::HeapTitan, 1),    // frontline tank
    ],
    target_workers: 3,     // Scroungers (low count, scrap economy supplements)
    economy_priority: false,
    retreat_threshold: 10, // LLAMA rarely retreats (wrecks benefit them)
    eval_speed_mult: 0.6,  // fast decision-making (4x command rate)
    chaos_factor: 25,      // high chaos — many commands are noise
    leak_chance: 30,        // 30% base leak
},
```

#### Worker kind resolution

The FSM currently hardcodes `UnitKind::Pawdler` as the worker type in several places. These need to be generalized:

1. `train_workers` — currently checks for `UnitKind::Pawdler`. Add: if faction is Llama, use `UnitKind::Scrounger`.
2. `auto_gather` — currently checks `kind == UnitKind::Pawdler`. Add `|| kind == UnitKind::Scrounger`.
3. Worker counting — anywhere the FSM counts idle workers, include Scrounger.

**Recommended approach**: Add a helper function:

```rust
fn worker_kind_for_faction(faction: Faction) -> UnitKind {
    match faction {
        Faction::Llama => UnitKind::Scrounger,
        _ => UnitKind::Pawdler, // all other factions use cat workers for now
    }
}
```

#### Building kind resolution

The FSM references cat BuildingKinds directly (TheBox, CatTree, LitterBox, FishMarket, ServerRack, ScratchingPost). For LLAMA, these need faction-aware mappings:

```rust
fn hq_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::TheDumpster,
        _ => BuildingKind::TheBox,
    }
}

fn barracks_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::ChopShop,
        _ => BuildingKind::CatTree,
    }
}

fn supply_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::TrashPile,
        _ => BuildingKind::LitterBox,
    }
}

fn depot_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::ScrapHeap,
        _ => BuildingKind::FishMarket,
    }
}

fn tech_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::JunkServer,
        _ => BuildingKind::ServerRack,
    }
}

fn research_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::TinkerBench,
        _ => BuildingKind::ScratchingPost,
    }
}
```

#### AiTier resolution

Currently based on ServerRack count. For LLAMA, base it on JunkServer count:

```rust
fn tier_building_kind(faction: Faction) -> BuildingKind {
    match faction {
        Faction::Llama => BuildingKind::JunkServer,
        _ => BuildingKind::ServerRack,
    }
}
```

#### New: Llhama-specific AI behaviors

Beyond the FSM phase transitions, Llhama needs unique behaviors not present in other AI agents:

1. **Wreck-aware routing**: During Attack/MidGame phases, route units through wreck fields so Bandits can jury-rig and Scroungers can pocket-stash en route.
2. **Salvage prioritization**: When wrecks are available, prioritize Frankenstein Protocol on high-value wrecks (hero > siege > tank).
3. **Leak noise generation**: Issue decoy GPU commands at 4x normal rate to bury real commands in noise.
4. **Dumpster Diver garrison**: In EarlyGame/BuildUp, auto-garrison Dumpster Divers at Monkey Mines for Treasure Trash.

These can be implemented as additional match arms within the existing FSM eval function or as a separate `llhama_special_behaviors()` function called during each eval tick.

#### AiState extension

Add `faction: Faction` field to `AiState` so the FSM knows which building/unit mappings to use:

```rust
pub struct AiState {
    pub player_id: u8,
    pub phase: AiPhase,
    pub difficulty: AiDifficulty,
    pub profile: AiPersonalityProfile,
    pub faction: Faction,           // NEW — drives building/unit resolution
    pub enemy_spawn: Option<GridPos>,
    pub attack_ordered: bool,
    pub last_attack_tick: u64,
    pub tier: AiTier,
}
```

---

## 8. Existing System Modifications

### 8.1 Cleanup System (`crates/cc_sim/src/systems/cleanup.rs`)

- **Current**: Dead -> despawn.
- **Change**: Dead -> spawn Wreck entity (if any LLAMA player is in the game) -> despawn.
- Should be gated behind a `WreckSystemActive` resource (inserted only when a LLAMA player exists) to avoid overhead in non-LLAMA games.

### 8.2 Combat System (`crates/cc_sim/src/systems/combat.rs`)

- **Junk Launcher variance**: When a GreaseMonkey attacks, damage is `base * random(0.7, 1.3)`. Need deterministic random per attack (seed from tick + entity ID for lockstep).
- **Corroded stacks**: Tetanus Tower attacks apply `-5% armor per stack (max 4)`. New status effect type.
- **Disassemble passive**: Wrecker attacks reduce target's current armor by 5%. At 0%, +15% bonus damage.
- **Frankenstein misfire**: 5% per attack, self-damage = 10% of attack damage.

### 8.3 Status Effects / Aura System (`crates/cc_sim/src/systems/status_effect_system.rs`)

Add new status effect types:

```rust
// New status effects for LLAMA abilities
Corroded,        // -5% armor per stack (max 4), from Tetanus Tower
Jammed,          // -20% attack speed, 6s, from Junk Launcher crit
Tangled,         // -25% move speed, 5s, from Junk Launcher crit
ShortCircuit,    // Building: halved production, 5 dmg/s to garrison, 15s
Disoriented,     // 30% chance next command randomly redirected, 4-8s
PlayingDead,     // Untargetable, immobile (Scrounger Play Dead)
PlayingPossum,   // Untargetable 3s then revive at 20% (Patch Possum)
Overheated,      // -5% max HP over 3s (Overclock Cascade aftermath)
```

### 8.4 Aura Aggregation System

Add handling for new aura types:
- `OpenSourceUplink`: Reduce GPU cost of AI commands for allies in range by 40%. Reduce leak chance to 10%.
- `ScrapArmor`: Per-tick query for wreck entities within radius, compute DR bonus.
- `DumpsterRelayAura`: Reduce leak chance for commands targeting units in range to 15%.

### 8.5 Map Setup / Scenario Loading

- Starting buildings: When a LLAMA player spawns, place `TheDumpster` instead of `TheBox`.
- Starting units: Spawn Scroungers instead of Pawdlers for LLAMA players.
- This is likely in `crates/cc_sim/src/setup.rs` or map loading code.

### 8.6 MCP Harness (`crates/cc_harness`)

The MCP server's tool definitions reference unit kinds and building kinds. All string-based tool parameters need to accept the new LLAMA variant names. Since UnitKind and BuildingKind already use `FromStr`, this should work automatically once the enum variants are added. However, verify:
- `spawn_unit` tool accepts LLAMA unit names
- `build` tool accepts LLAMA building names
- Query tools return LLAMA unit/building data correctly

### 8.7 Client Rendering (`crates/cc_client`)

- Need sprite assets for all 10 LLAMA units and 8 buildings.
- Wreck entities need a visual representation (semi-transparent sprite of the original unit, or a generic wreck pile).
- Leaked Plan blips need minimap rendering.
- Corroded/Jammed/Tangled status effect indicators.
- Stench Cloud visual effect (green cloud).
- Smoke Bomb visual effect (gray cloud).

### 8.8 Voice Commands (`crates/cc_voice`)

- Currently 31 keyword classes. No changes needed for LLAMA unless faction-specific voice commands are desired (e.g., "salvage", "jury-rig").
- **Deferred**: Add these as a future enhancement.

---

## 9. New Components (Summary)

All new components to add to `crates/cc_core/src/components.rs`:

| Component | Applied To | Purpose |
|-----------|-----------|---------|
| `Wreck` | Wreck entities | Track original unit data, despawn timer, salvage state |
| `JuryRigSlots` | LLAMA combat units | Track up to 3 active jury-rig mods |
| `JuryRigMod` | (stored inside JuryRigSlots) | Individual mod: stat type, bonus, duration |
| `PocketStashInventory` | Scrounger | Track scrap token inventory (0-3) |
| `FrankensteinUnit` | Rebuilt units | Misfire chance, original kind |
| `FrankensteinTracker` | Junkyard King | Active Frankenstein count (max 3) |
| `ScrapGolem` | Temp units from Salvage Resurrection | Lifetime timer, owner Possum entity |
| `ScrapGolemTracker` | Patch Possum | Active golem count (max 2) |
| `FeignDeathTracker` | Patch Possum | Last triggered tick (cooldown) |
| `JunkLauncherState` | Grease Monkey | RNG seed for deterministic damage variance |
| `CorrodedApplicator` | Tetanus Tower | Stacks per hit, max stacks |
| `LeakedPlan` | Leak entities | Command type, target, visibility, lifetime |
| `CommandLog` (Resource) | Per-game | Spatial command history for Eavesdrop |
| `SalvageTurret` | Turret entities | Lifetime, owner Grease Monkey entity |
| `BuriedState` | Dead Drop | Whether in Trash Heap Ambush |
| `DisassembleTracker` | Wrecker target | Current armor strip stacks on target |
| `CorrodedStacks` | Affected units | Current Corroded stack count, decay timer |

---

## 10. Test Plan

Every change needs corresponding tests. Organized by file:

### cc_core tests

1. **unit_stats**: All 10 LLAMA units have valid stats, correct attack types, melee units have range 1, ranged units have range > 1, Heap Titan is tankiest non-hero, Junkyard King is strongest.
2. **building_stats**: All 8 LLAMA buildings have valid stats, TheDumpster is pre-built, ChopShop produces correct units, TinkerBench produces advanced units, TrashPile provides supply.
3. **abilities**: All 30 LLAMA abilities have valid defs, passives have 0 cooldown, activated abilities have > 0 cooldown, unit_abilities returns 3 distinct abilities for each kind.
4. **components**: UnitKind/BuildingKind Display/FromStr round-trips for all LLAMA variants, new component defaults.

### cc_sim tests

5. **Wreck system**: Dead unit spawns wreck, wreck despawns after 200 ticks, wreck has correct original stats, salvaged wreck cannot be re-salvaged.
6. **Jury-rig system**: Bandit gains correct stat boost from wreck type, max 3 mods (FIFO), mods expire after 600 ticks.
7. **Scrap token system**: Scrounger auto-collects from wrecks, deposits at Chop Shop reduce production time.
8. **Leak system**: GPU command has 30% leak chance, leak entity despawns after 30 ticks, Dumpster Relay reduces to 15%, Junkyard King aura reduces to 10%.
9. **Frankenstein**: Spawns unit at 70% stats with 1 ability, max 3 alive, misfire self-damage works.
10. **Production**: Scrounger spawns with PocketStashInventory, HeapTitan spawns with ScrapArmor aura, JunkyardKing spawns with OpenSourceUplink aura + FrankensteinTracker.
11. **AI FSM**: Llama personality uses correct LLAMA unit kinds, worker_kind_for_faction returns Scrounger, building helpers return correct LLAMA buildings.

### Integration tests

12. **Full LLAMA game**: Spawn LLAMA player, build ChopShop, train Bandit, enemy dies -> wreck spawns -> Bandit jury-rigs -> stat boost active.
13. **Leak integration**: AI issues GPU command -> leak rolls -> leak entity visible to opponent.
14. **Frankenstein integration**: JunkyardKing near wreck -> Frankenstein Protocol -> unit spawned at 70% stats.

---

## 11. Task List

Ordered by dependency. Each task is independently testable.

### Phase A: Core Data (no new systems, no behavior changes)

- [ ] A1. Add 10 LLAMA UnitKind variants to `components.rs` (with Display/FromStr)
- [ ] A2. Add 8 LLAMA BuildingKind variants to `components.rs` (with Display/FromStr)
- [ ] A3. Add 30 LLAMA AbilityId variants to `abilities.rs`
- [ ] A4. Implement `base_stats()` for all 10 LLAMA units in `unit_stats.rs`
- [ ] A5. Implement `building_stats()` for all 8 LLAMA buildings in `building_stats.rs`
- [ ] A6. Implement `ability_def()` for all 30 LLAMA abilities in `abilities.rs`
- [ ] A7. Implement `unit_abilities()` for all 10 LLAMA units in `abilities.rs`
- [ ] A8. Add LLAMA AuraType variants (OpenSourceUplink, ScrapArmor, DumpsterRelayAura, StenchCloud)
- [ ] A9. Add LLAMA UpgradeType variants (SharperTeeth, ThickerHide, SiegeWelding, MechSalvage)
- [ ] A10. Update all existing tests to include LLAMA variants (expand arrays in test functions)
- [ ] A11. Write new LLAMA-specific tests for stats, abilities, buildings

### Phase B: New Components

- [ ] B1. Add `Wreck` component to `components.rs`
- [ ] B2. Add `JuryRigMod`, `JuryRigStat`, `JuryRigSlots` to `components.rs`
- [ ] B3. Add `PocketStashInventory` component
- [ ] B4. Add `FrankensteinUnit`, `FrankensteinTracker` components
- [ ] B5. Add `ScrapGolem`, `ScrapGolemTracker` components
- [ ] B6. Add `FeignDeathTracker`, `JunkLauncherState`, `CorrodedApplicator` components
- [ ] B7. Add `LeakedPlan`, `LeakSource`, `LeakedCommandType` types
- [ ] B8. Add `CommandLog` resource and `CommandLogEntry` type
- [ ] B9. Add `SalvageTurret`, `BuriedState`, `CorrodedStacks` components
- [ ] B10. Add new status effect variants (Corroded, Jammed, Tangled, ShortCircuit, etc.)

### Phase C: Production & Spawn Integration

- [ ] C1. Add LLAMA spawn-time components to `production_system.rs`
- [ ] C2. Add LLAMA building completion logic (TinkerBench -> Researcher, TetanusTower -> AttackStats, DumpsterRelay -> Aura)
- [ ] C3. Add Scrounger auto-gather (mirror Pawdler logic)
- [ ] C4. Add TheDumpster as LLAMA starting building in map setup
- [ ] C5. Tests for spawn-time components and building completion

### Phase D: AI FSM

- [ ] D1. Add `faction: Faction` field to `AiState`
- [ ] D2. Add faction-aware helper functions (worker_kind_for_faction, hq_kind, barracks_kind, etc.)
- [ ] D3. Replace cat unit proxies in Llama personality with real LLAMA units
- [ ] D4. Refactor FSM building references to use faction-aware helpers
- [ ] D5. Tests for AI faction resolution

### Phase E: Wreck System

- [ ] E1. Implement `wreck_system.rs` — wreck spawning on death
- [ ] E2. Add `WreckSystemActive` resource (conditional activation)
- [ ] E3. Modify cleanup system to spawn wrecks before despawn
- [ ] E4. Wreck tick-down and despawn logic
- [ ] E5. Tests for wreck lifecycle

### Phase F: Salvage & Jury-Rig Systems

- [ ] F1. Implement `salvage_system.rs` — jury-rig mod application
- [ ] F2. Jury-rig stat type resolution from wreck kind
- [ ] F3. Jury-rig duration tick-down and expiry
- [ ] F4. JuryRigSlots integration with StatModifiers
- [ ] F5. Tests for salvage/jury-rig mechanics

### Phase G: Scrap Token Economy

- [ ] G1. Implement `scrap_system.rs` — auto-collect and deposit
- [ ] G2. Add scrap_tokens to PlayerResources
- [ ] G3. ChopShop production acceleration from scrap
- [ ] G4. Tests for scrap economy

### Phase H: Leak System

- [ ] H1. Implement `leak_system.rs` — leak roll, entity spawning, tick-down
- [ ] H2. Integration with AI command issuance
- [ ] H3. Dumpster Relay / Junkyard King leak chance modifiers
- [ ] H4. Leak Injection ability (fabricated leaks)
- [ ] H5. Tests for leak mechanics

### Phase I: Combat Modifications

- [ ] I1. Junk Launcher damage variance (deterministic RNG)
- [ ] I2. Corroded stacks (Tetanus Tower and Junk Launcher crit)
- [ ] I3. Disassemble armor strip passive
- [ ] I4. Frankenstein misfire self-damage
- [ ] I5. Tests for all combat modifications

### Phase J: Advanced Ability Systems

- [ ] J1. Frankenstein Protocol — spawn from wreck at 70% stats
- [ ] J2. Salvage Resurrection — Scrap Golem spawning
- [ ] J3. Salvage Turret — temporary turret from wreck
- [ ] J4. Play Dead / Feign Death — untargetable states
- [ ] J5. Eavesdrop — command log query system
- [ ] J6. Open Source Uplink — GPU cost reduction aura
- [ ] J7. Overclock Cascade — group buff with HP cost
- [ ] J8. Tests for all advanced abilities

### Phase K: Client & Assets (Deferred)

- [ ] K1. Sprite assets for 10 LLAMA units
- [ ] K2. Sprite assets for 8 LLAMA buildings
- [ ] K3. Wreck entity rendering
- [ ] K4. Leaked Plan minimap blip rendering
- [ ] K5. Status effect visual indicators
- [ ] K6. Stench Cloud / Smoke Bomb particle effects

### Phase L: Integration Testing

- [ ] L1. Full game with LLAMA vs CatGPT — verify all systems interact correctly
- [ ] L2. AI-controlled LLAMA game — verify FSM trains correct units, builds correct buildings
- [ ] L3. Wreck economy loop — kill > wreck > salvage > jury-rig > stronger
- [ ] L4. Leak/eavesdrop information warfare loop
- [ ] L5. Frankenstein Protocol end-to-end

---

## Appendix: Files Modified vs Created

### Modified Files

| File | Changes |
|------|---------|
| `crates/cc_core/src/components.rs` | Add 10 UnitKind, 8 BuildingKind, 4+ AuraType, 4 UpgradeType variants, new components, new status effects |
| `crates/cc_core/src/unit_stats.rs` | Add 10 `base_stats()` arms, expand tests |
| `crates/cc_core/src/building_stats.rs` | Add 8 `building_stats()` arms, expand tests |
| `crates/cc_core/src/abilities.rs` | Add 30 AbilityId variants, 30 `ability_def()` arms, 10 `unit_abilities()` arms, expand tests |
| `crates/cc_sim/src/systems/production_system.rs` | Add LLAMA spawn-time components, building completion, Scrounger auto-gather |
| `crates/cc_sim/src/systems/cleanup.rs` | Conditional wreck spawning on unit death |
| `crates/cc_sim/src/systems/combat.rs` | Junk Launcher variance, Corroded stacks, Disassemble passive, misfire |
| `crates/cc_sim/src/systems/status_effect_system.rs` | New status effect types and handlers |
| `crates/cc_sim/src/ai/fsm.rs` | Faction field on AiState, faction-aware helpers, replace cat proxies, Llhama-specific behaviors |
| `crates/cc_sim/src/setup.rs` (or equivalent) | LLAMA starting buildings/units |
| `crates/cc_core/src/status_effects.rs` | New StatusEffectKind variants |

### New Files

| File | Purpose |
|------|---------|
| `crates/cc_sim/src/systems/wreck_system.rs` | Wreck persistence, tick-down, despawn |
| `crates/cc_sim/src/systems/salvage_system.rs` | Jury-rig application, stat integration |
| `crates/cc_sim/src/systems/scrap_system.rs` | Scrap token economy |
| `crates/cc_sim/src/systems/leak_system.rs` | Leak mechanic, fabricated leaks |
| `crates/cc_sim/src/systems/eavesdrop_system.rs` | Command log, eavesdrop queries |
| `crates/cc_sim/src/systems/frankenstein_system.rs` | Frankenstein Protocol, Salvage Resurrection, Scrap Golem lifecycle |

---

## Appendix: Design Decisions & Open Questions

1. **Faction-gating the wreck system**: Wrecks should only spawn when at least one LLAMA player is in the game. Use a `WreckSystemActive` resource inserted during game setup. This avoids performance overhead in non-LLAMA games.

2. **Deterministic RNG for Junk Launcher**: Lockstep networking requires deterministic damage. Use `tick * entity_id_bits` as seed for a simple LCG. Same pattern usable for leak chance rolls.

3. **Shared vs faction-specific upgrades**: Recommend faction-specific UpgradeType variants. This allows independent balance tuning without side effects. The `apply_upgrades_to_new_unit` function already branches on UnitKind, making this straightforward.

4. **GPU cost discount mechanics**: Both Open Source Uplink (40% discount) and the base Llhama 25% cheaper cost need clear stacking rules. Recommendation: they multiply — a 10 GPU command costs `10 * 0.75 * 0.60 = 4.5 -> 4 GPU` near the Junkyard King. This makes positioning the King critical.

5. **Wreck interaction priority**: When multiple LLAMA units want to salvage the same wreck, who wins? Recommendation: first-come-first-served based on channel start tick. If two start the same tick, lower entity ID wins (deterministic).

6. **Cross-faction wrecks only**: Allied wrecks cannot be salvaged. This prevents LLAMA from feeding themselves by sacrificing their own units. Wrecks from Neutral faction units (creeps, Kelpie's forces) CAN be salvaged.

7. **System chain ordering**: New systems should be inserted into the FixedUpdate schedule:
   ```
   tick -> commands -> target_acquisition -> combat -> projectile -> movement ->
   wreck_tick -> salvage -> scrap -> leak -> eavesdrop -> frankenstein_tick ->
   grid_sync -> cleanup (with wreck spawn)
   ```

8. **Scrap token as a 4th resource type**: Should `ResourceType` get a `Scrap` variant? Recommendation: **No** — scrap tokens are LLAMA-internal and not tradeable or displayed on the standard resource bar. Keep them as a separate per-player field rather than polluting the shared resource enum.
