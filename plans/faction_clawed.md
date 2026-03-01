# Implementation Plan: The Clawed (Mice / Claudeus Maximus)

**Goal**: Make The Clawed a fully playable faction with 10 unique units, 8 buildings, 30 abilities, and faction-specific mechanics.

**Scope**: This plan covers all code changes needed to bring The Clawed from "Faction enum variant with cat unit proxies" to "fully playable with own roster, AI, and mechanics."

---

## Table of Contents

1. [UnitKind Variants](#1-unitkind-variants)
2. [Unit Base Stats](#2-unit-base-stats)
3. [BuildingKind Variants](#3-buildingkind-variants)
4. [Building Base Stats](#4-building-base-stats)
5. [AbilityId Variants](#5-abilityid-variants)
6. [Ability Definitions](#6-ability-definitions)
7. [Spawn-Time Components](#7-spawn-time-components)
8. [Faction-Specific Mechanics](#8-faction-specific-mechanics)
9. [AI FSM Updates](#9-ai-fsm-updates)
10. [System Modifications vs New Systems](#10-system-modifications-vs-new-systems)
11. [Upgrade / Research System](#11-upgrade--research-system)
12. [Test Plan](#12-test-plan)
13. [Implementation Order](#13-implementation-order)
14. [File Change Summary](#14-file-change-summary)

---

## 1. UnitKind Variants

Add 10 new variants to the `UnitKind` enum in `crates/cc_core/src/components.rs`:

| Variant | Role | Animal | catGPT Analog |
|---------|------|--------|---------------|
| `Nibblet` | Worker | Mouse | Pawdler |
| `Swarmer` | Light Infantry | Mouse | Nuisance |
| `Gnawer` | Anti-Structure | Mouse | Catnapper (siege role) |
| `Shrieker` | Ranged Harasser | Shrew | Hisser |
| `Tunneler` | Transport/Utility | Vole | FerretSapper (utility role) |
| `Sparks` | Saboteur | Mouse | Mouser (disruption role) |
| `Quillback` | Heavy Defender | Hedgehog | Chonk |
| `Whiskerwitch` | Caster/Support | Shrew | Yowler |
| `Plaguetail` | Area Denial | Mouse | (no direct analog) |
| `WarrenMarshal` | Hero/Commander | Mouse | MechCommander |

**Changes required in `components.rs`**:
- Add 10 variants to `UnitKind` enum
- Add 10 arms to `UnitKind::fmt` (Display)
- Add 10 arms to `UnitKind::from_str`
- Update `unit_kind_display_from_str_round_trip` test

---

## 2. Unit Base Stats

Add 10 match arms to `base_stats()` in `crates/cc_core/src/unit_stats.rs`.

Design philosophy: The Clawed units are **cheaper, weaker individually, faster to produce** than catGPT equivalents. Supply costs skew low (most cost 1 supply) to enable swarm counts of 20-30. Food costs are 60-70% of catGPT equivalents. GPU costs are lower individually but aggregate higher due to volume.

| Unit | HP | Speed | Damage | Range | Attack Speed (ticks) | Attack Type | Food | GPU | Supply | Train Time (ticks) |
|------|-----|-------|--------|-------|---------------------|-------------|------|-----|--------|-------------------|
| Nibblet | 40 | 0.15 | 3 | 1 | 15 | Melee | 30 | 0 | 1 | 35 |
| Swarmer | 45 | 0.16 | 5 | 1 | 8 | Melee | 40 | 0 | 1 | 30 |
| Gnawer | 55 | 0.10 | 6 (vs buildings: +Structural Weakness) | 1 | 12 | Melee | 50 | 0 | 1 | 45 |
| Shrieker | 35 | 0.14 | 8 | 3 | 10 | Ranged | 55 | 0 | 1 | 40 |
| Tunneler | 60 | 0.09 | 4 | 1 | 15 | Melee | 75 | 25 | 2 | 70 |
| Sparks | 40 | 0.17 | 7 | 2 | 12 | Ranged | 60 | 15 | 1 | 50 |
| Quillback | 200 | 0.06 | 10 | 1 | 18 | Melee | 100 | 15 | 2 | 80 |
| Whiskerwitch | 50 | 0.12 | 4 | 4 | 14 | Ranged | 70 | 30 | 2 | 65 |
| Plaguetail | 60 | 0.11 | 6 | 2 | 12 | Ranged | 45 | 0 | 1 | 40 |
| WarrenMarshal | 300 | 0.08 | 12 | 3 | 14 | Ranged | 250 | 125 | 4 | 200 |

**Design rationale**:
- **Nibblet** (worker): Faster and cheaper than Pawdler (30 food vs 50), less HP (40 vs 60). You want 6-8 of them.
- **Swarmer**: Very cheap (40 food, 1 supply), very fast attack speed (8 ticks vs Nuisance's 10), but 45 HP vs 80. Dies easily, overwhelms in groups.
- **Gnawer**: Slower than Swarmer, focused on buildings. Low base damage but Structural Weakness passive makes sustained building damage escalate.
- **Shrieker**: Cheaper than Hisser (55 vs 100 food), shorter range (3 vs 5), lower HP (35 vs 70), but cone attack hits multiple targets.
- **Tunneler**: Utility/transport, not a combat unit. Moderate cost with GPU requirement.
- **Sparks**: Glass cannon saboteur. Static Charge mechanic makes it burst-damage after movement.
- **Quillback**: The only "heavy" Clawed unit. 200 HP (vs Chonk's 300) but Spine Wall gives 50% DR. Costs 100 food (vs Chonk's 150).
- **Whiskerwitch**: Support caster. Fragile (50 HP), expensive GPU cost (30), but force multiplier for the swarm.
- **Plaguetail**: Area denial through death. Cheap (45 food, 1 supply). Designed to die productively.
- **WarrenMarshal**: Hero unit. Less tanky than MechCommander (300 vs 500 HP), cheaper (250/125 vs 400/200), but force multiplier through Whiskernet Relay.

**Changes required in `unit_stats.rs`**:
- Replace `other => unimplemented!()` with 10 new match arms
- Update `all_kinds_have_stats` test to include all 10 Clawed units
- Update `melee_units_have_range_one` test to include Nibblet, Swarmer, Gnawer, Tunneler, Quillback
- Update `ranged_units_have_range_greater_than_one` test to include Shrieker, Sparks, Whiskerwitch, Plaguetail, WarrenMarshal
- Add `swarmer_is_cheapest` test (lowest food cost among combat units)
- Add `quillback_is_clawed_tankiest` test (highest HP among non-hero Clawed)

---

## 3. BuildingKind Variants

Add 8 new variants to the `BuildingKind` enum in `crates/cc_core/src/components.rs`:

| Variant | Role | catGPT Analog | Notable Differences |
|---------|------|---------------|-------------------|
| `TheBurrow` | Command Center | TheBox | Auto-generates Nibblets (1/45s), garrisons 8 small units |
| `NestingBox` | Barracks | CatTree | Queue depth 8 (vs ~3-5), 20% slower per-unit training |
| `SeedVault` | Resource Depot | FishMarket | +50% food storage, +15% gather speed aura, scatter-on-death |
| `JunkTransmitter` | Tech Building | ServerRack | 25% less HP, cheaper |
| `GnawLab` | Research | ScratchingPost | Research speed +10% per additional GnawLab (max 3) |
| `WarrenExpansion` | Supply Depot | LitterBox | Extends Burrow garrison +4, well-rested HP bonus at 3+ |
| `Mousehole` | Defensive Gate | CatFlap | Size-restricted passage, garrisoned units get +20% range |
| `SqueakTower` | Defense Tower | LaserPointer | AoE pulse with Rattled debuff, lower damage |

**Changes required in `components.rs`**:
- Add 8 variants to `BuildingKind` enum
- Add 8 arms to `BuildingKind::fmt` (Display)
- Add 8 arms to `BuildingKind::from_str`
- Update `building_kind_display_from_str_round_trip` test

---

## 4. Building Base Stats

Add 8 match arms to `building_stats()` in `crates/cc_core/src/building_stats.rs`.

| Building | HP | Build Time (ticks) | Food Cost | GPU Cost | Supply Provided | Can Produce |
|----------|-----|-------------------|-----------|----------|----------------|-------------|
| TheBurrow | 400 | 0 (pre-built) | 0 | 0 | 10 | [Nibblet] |
| NestingBox | 250 | 120 | 100 | 0 | 0 | [Swarmer, Gnawer, Plaguetail, Sparks] |
| SeedVault | 150 | 80 | 70 | 0 | 0 | [] |
| JunkTransmitter | 190 | 100 | 75 | 50 | 0 | [Shrieker, Tunneler, Quillback, Whiskerwitch, WarrenMarshal] |
| GnawLab | 160 | 80 | 70 | 35 | 0 | [] |
| WarrenExpansion | 80 | 60 | 50 | 0 | 10 | [] |
| Mousehole | 300 | 80 | 100 | 0 | 0 | [] |
| SqueakTower | 120 | 70 | 50 | 15 | 0 | [] |

**Design rationale**:
- All Clawed buildings are cheaper and less durable than catGPT equivalents (designed to be rebuilt quickly).
- TheBurrow has 400 HP (vs TheBox's 500) but can garrison units for protection.
- NestingBox is cheaper (100 vs 150 food) but trains slower per-unit. The deep queue compensates.
- JunkTransmitter has 190 HP (vs ServerRack's 250) and costs less (75/50 vs 100/75). "Build more, lose some."
- WarrenExpansion is very cheap (50 food) -- you need many for the garrison/HP bonuses.

**Changes required in `building_stats.rs`**:
- Add 8 match arms (currently no `other` fallback -- need to add one or add all 8)
- Update `all_buildings_have_stats` test
- Add `the_burrow_is_pre_built` test
- Add `nesting_box_produces_basic_clawed` test
- Add `junk_transmitter_produces_advanced_clawed` test
- Add `warren_expansion_provides_supply` test

---

## 5. AbilityId Variants

Add 30 new variants to the `AbilityId` enum in `crates/cc_core/src/abilities.rs`:

### Nibblet (Worker)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `CrumbTrail` | Passive | Leave speed-boosting scent trail while gathering |
| `StashNetwork` | Activated | Create hidden food cache (max 3), nearby allies heal |
| `PanicProductivity` | Passive | +60% gather, +40% speed on nearby ally death |

### Swarmer (Light Infantry)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `SafetyInNumbers` | Passive | +3% DR per nearby ally (cap 24%), +15% AS at 5+ |
| `PileOn` | Activated | Leap-attach to enemy, continuous dmg, slow target |
| `Scatter` | Activated | Dash 4 tiles random direction, dust cloud if 3+ sync |

### Gnawer (Anti-Structure)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `StructuralWeakness` | Passive | +2% dmg to building per Gnawed stack (max 10) |
| `ChewThrough` | Activated | Create 1-tile breach in wall/building for 20s |
| `IncisorsNeverStop` | Passive | +1%/s building dmg while attacking same target (cap 40%) |

### Shrieker (Ranged Harasser)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `SonicSpit` | Passive | Cone attack, applies Rattled (-10% accuracy) |
| `EcholocationPing` | Activated | Reveal all in 5-tile radius, Mark for +8% dmg |
| `FuryOfTheSmall` | Passive | +15% dmg vs larger units, +25% vs CC'd |

### Tunneler (Transport/Utility)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `BurrowExpress` | Activated | Dig tunnel for up to 6 small units (12 tile range) |
| `Undermine` | Activated | Dig under building, heavy dmg + Destabilized |
| `TremorSense` | Passive | Detect ground units in 8 tiles while stationary |

### Sparks (Saboteur)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `StaticCharge` | Passive | Build charge while moving, discharge on attack |
| `ShortCircuit` | Activated | Disable enemy building 4s, suppress AI if tech building |
| `DaisyChain` | Activated | Link to ally, damage arcs to enemies near the other |

### Quillback (Heavy Defender)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `SpineWall` | Toggle | Curl up: 50% DR, reflect melee, block pathing |
| `QuillBurst` | Activated | AoE spine attack, applies Spooked (flee 1.5s) |
| `StubbornAdvance` | Passive | Cannot be slowed below 70% base speed, +5% dmg per debuff |

### Whiskerwitch (Caster/Support)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `HexOfMultiplication` | Activated | Create 3 illusion units (1 HP, count for swarm scaling) |
| `WhiskerWeave` | Activated | Invisible tripwire, Spooked + vision on trigger |
| `DatacromanticRitual` | Activated | Sacrifice 50% HP, +20% dmg/AS to nearby allies 8s |

### Plaguetail (Area Denial)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `ContagionCloud` | Passive | On death: toxic cloud (DoT + Weakened) |
| `MiasmaTrail` | Toggle | Leave poison trail, slows enemies |
| `SympathySickness` | Activated | Target shares 20% incoming dmg to nearby enemies |

### WarrenMarshal (Hero/Commander)
| AbilityId | Activation | Description |
|-----------|-----------|-------------|
| `RallyTheSwarm` | Passive | +1% dmg/speed per nearby ally (cap 12%), formation follow |
| `ExpendableHeroism` | Activated | Dying allies deal 30% max HP burst to target, +30% AS |
| `WhiskernetRelay` | Passive | 50% AI GPU discount, hallucination rate 8% -> 3% |

---

## 6. Ability Definitions

Add 30 match arms to `ability_def()` in `crates/cc_core/src/abilities.rs`. Also add a `unit_abilities()` arm for each of the 10 Clawed unit kinds.

### Proposed AbilityDef Values

All cooldowns/durations in ticks (10hz sim).

| AbilityId | Activation | Cooldown | GPU | Duration | Range | Charges |
|-----------|-----------|----------|-----|----------|-------|---------|
| CrumbTrail | Passive | 0 | 0 | 0 | 0 | 0 |
| StashNetwork | Activated | 100 | 0 | 0 | 0 | 3 |
| PanicProductivity | Passive | 0 | 0 | 0 | 0 | 0 |
| SafetyInNumbers | Passive | 0 | 0 | 0 | 0 | 0 |
| PileOn | Activated | 100 | 0 | 30 | 2 tiles | 0 |
| Scatter | Activated | 60 | 0 | 5 | 0 (self) | 0 |
| StructuralWeakness | Passive | 0 | 0 | 0 | 0 | 0 |
| ChewThrough | Activated | 150 | 0 | 40 | 1 tile | 0 |
| IncisorsNeverStop | Passive | 0 | 0 | 0 | 0 | 0 |
| SonicSpit | Passive | 0 | 0 | 0 | 3 tiles | 0 |
| EcholocationPing | Activated | 180 | 0 | 30 | 5 tiles | 0 |
| FuryOfTheSmall | Passive | 0 | 0 | 0 | 0 | 0 |
| BurrowExpress | Activated | 200 | 0 | 0 | 12 tiles | 2 |
| Undermine | Activated | 250 | 0 | 50 | 6 tiles | 0 |
| TremorSense | Passive | 0 | 0 | 0 | 8 tiles | 0 |
| StaticCharge | Passive | 0 | 0 | 0 | 0 | 0 |
| ShortCircuit | Activated | 300 | 3 | 40 | 4 tiles | 0 |
| DaisyChain | Activated | 150 | 0 | 80 | 3 tiles | 0 |
| SpineWall | Toggle | 10 | 0 | 0 | 0 (self) | 0 |
| QuillBurst | Activated | 160 | 0 | 0 | 3 tiles | 0 |
| StubbornAdvance | Passive | 0 | 0 | 0 | 0 | 0 |
| HexOfMultiplication | Activated | 200 | 4 | 100 | 6 tiles | 0 |
| WhiskerWeave | Activated | 120 | 0 | 150 | 5 tiles | 2 |
| DatacromanticRitual | Activated | 300 | 0 | 80 | 6 tiles | 0 |
| ContagionCloud | Passive | 0 | 0 | 0 | 0 | 0 |
| MiasmaTrail | Toggle | 10 | 0 | 0 | 0 (self) | 0 |
| SympathySickness | Activated | 180 | 0 | 60 | 4 tiles | 0 |
| RallyTheSwarm | Passive | 0 | 0 | 0 | 6 tiles | 0 |
| ExpendableHeroism | Activated | 350 | 5 | 100 | 4 tiles | 0 |
| WhiskernetRelay | Passive | 0 | 0 | 0 | 0 | 0 |

### unit_abilities() mapping

```
Nibblet       -> [CrumbTrail, StashNetwork, PanicProductivity]
Swarmer       -> [SafetyInNumbers, PileOn, Scatter]
Gnawer        -> [StructuralWeakness, ChewThrough, IncisorsNeverStop]
Shrieker      -> [SonicSpit, EcholocationPing, FuryOfTheSmall]
Tunneler      -> [BurrowExpress, Undermine, TremorSense]
Sparks        -> [StaticCharge, ShortCircuit, DaisyChain]
Quillback     -> [SpineWall, QuillBurst, StubbornAdvance]
Whiskerwitch  -> [HexOfMultiplication, WhiskerWeave, DatacromanticRitual]
Plaguetail    -> [ContagionCloud, MiasmaTrail, SympathySickness]
WarrenMarshal -> [RallyTheSwarm, ExpendableHeroism, WhiskernetRelay]
```

**Changes required in `abilities.rs`**:
- Add 30 variants to `AbilityId` enum
- Add 30 match arms to `ability_def()`
- Add 10 match arms to `unit_abilities()`
- Update `all_ability_defs_valid` test (60 total IDs)
- Update `unit_abilities_returns_three_per_kind` test (20 total kinds)
- Add `clawed_passive_abilities_no_cooldown` test
- Add `clawed_toggle_abilities_have_cooldown` test
- Add faction-specific helper: `gnawer_structural_weakness_multiplier(stacks: u32) -> Fixed`
- Add faction-specific helper: `incisors_damage_bonus(continuous_ticks: u32) -> Fixed`

---

## 7. Spawn-Time Components

New components and spawn-time additions needed in `production_system.rs`, modeled after the existing `DreamSiegeTimer` and `Aura` patterns.

### New Components (add to `components.rs`)

| Component | Used By | Purpose |
|-----------|---------|---------|
| `SwarmScaling` | All Clawed combat units | Tracks current nearby-ally count for Safety in Numbers and other swarm effects. Recomputed by a spatial query system each tick. |
| `StructuralWeaknessTimer` | Gnawer | Like DreamSiegeTimer but tracks continuous attack time on buildings for Incisors Never Stop Growing, and building-specific Gnawed stacks. |
| `StaticChargeStacks` | Sparks | Tracks movement-accumulated static charge (0-10 stacks). Decays while stationary. |
| `PanicState` | Nibblet | Tracks Panic Productivity activation (remaining ticks, gather/speed multiplier). |
| `MiasmaTrailActive` | Plaguetail | Marker for active Miasma Trail toggle. Stores trail tile history. |
| `ContagionCloudOnDeath` | Plaguetail | Marker indicating this unit spawns a cloud on death. |
| `PileOnAttachment` | Swarmer | Tracks which enemy entity this Swarmer is attached to and remaining duration. |
| `BurrowExpressTunnel` | (entity) | Standalone entity representing an active tunnel endpoint pair. |
| `GnawedStacks` | Target buildings | Applied to buildings being gnawed. Tracks stack count and decay timer. |
| `WhiskernetRelayAura` | WarrenMarshal | Like TacticalUplink aura but with the Clawed-specific 50% GPU discount + hallucination reduction. |
| `RallyTheSwarmAura` | WarrenMarshal | Passive aura that computes per-ally scaling bonus. |

### New AuraType Variants

Add to the `AuraType` enum in `components.rs`:

- `RallyTheSwarm` -- WarrenMarshal's passive formation/buff aura
- `WhiskernetRelay` -- WarrenMarshal's GPU discount aura
- `SqueakTowerPulse` -- SqueakTower defense building AoE debuff

### Spawn-Time Logic in `production_system.rs`

Following the existing pattern where Chonk gets `Aura` + `NineLivesTracker` and Catnapper gets `DreamSiegeTimer`:

```
// Gnawer spawn-time components
if kind == UnitKind::Gnawer {
    entity_cmds.insert(StructuralWeaknessTimer::default());
}

// Sparks spawn-time components
if kind == UnitKind::Sparks {
    entity_cmds.insert(StaticChargeStacks::default());
}

// Quillback spawn-time components (like Chonk pattern)
if kind == UnitKind::Quillback {
    // No initial aura -- SpineWall is a toggle, starts off
}

// Plaguetail spawn-time components
if kind == UnitKind::Plaguetail {
    entity_cmds.insert(ContagionCloudOnDeath);
}

// WarrenMarshal spawn-time components
if kind == UnitKind::WarrenMarshal {
    entity_cmds.insert((
        Aura {
            aura_type: AuraType::RallyTheSwarm,
            radius: Fixed::from_bits(6 << 16),
            active: true,
        },
        Aura {  // second aura -- may need Vec<Aura> or AuraSet component
            aura_type: AuraType::WhiskernetRelay,
            radius: Fixed::from_bits(6 << 16),
            active: true,
        },
    ));
}
```

**Architecture note**: The WarrenMarshal has two simultaneous passive auras (RallyTheSwarm + WhiskernetRelay), unlike any catGPT unit. The current `Aura` component is a single value. Options:
1. Change `Aura` to `AuraSet { auras: Vec<(AuraType, Fixed, bool)> }` (breaking change, must update all Chonk/Yowler/MechCommander spawn code)
2. Add a second component `SecondaryAura` specifically for dual-aura units
3. Store the passive (WhiskernetRelay) as a marker component instead of an Aura, since it affects the AI system, not unit stats

**Recommended approach**: Option 3 -- implement `WhiskernetRelay` as a standalone marker component (like `DreamSiegeTimer`), since its effect (GPU discount + hallucination reduction) is checked by the AI command system, not the aura spatial query system. `RallyTheSwarm` uses the existing `Aura` component.

### Building Spawn-Time Logic

When a building finishes construction (in `production_system.rs`):

```
// SqueakTower gets attack stats + AoE pulse
if building.kind == BuildingKind::SqueakTower {
    commands.entity(entity).insert((
        AttackStats {
            damage: Fixed::from_bits(5 << 16),  // 5 damage (lower than LaserPointer)
            range: Fixed::from_bits(4 << 16),   // 4 range
            attack_speed: 20,                    // 2s between pulses
            cooldown_remaining: 0,
        },
        AttackTypeMarker { attack_type: AttackType::Ranged },
    ));
}

// GnawLab gets Researcher + ResearchQueue
if building.kind == BuildingKind::GnawLab {
    commands.entity(entity).insert((Researcher, ResearchQueue::default()));
}
```

### TheBurrow Auto-Nibblet Generation

The Burrow auto-generates 1 Nibblet every 45 seconds (450 ticks), max 4 queued. This is a new mechanic not present in catGPT's TheBox. Options:
1. Pre-seed the Burrow's `ProductionQueue` with a repeating Nibblet entry and a system to re-queue on completion
2. Add a new component `AutoProduction { kind: UnitKind, interval_ticks: u32, max_queued: u32, ticks_remaining: u32 }` and a system that auto-queues units

**Recommended approach**: Option 2 -- new `AutoProduction` component + system. Cleaner than hijacking the production queue. The auto-produced Nibblets should still cost no food (they're free trickle units) but should consume supply.

### NestingBox Deep Queue

The NestingBox supports queue depth 8 (vs catGPT's implicit ~3-5). The current `ProductionQueue` uses a `VecDeque` with no hard cap. The queue depth limit is enforced by the AI and by a `can_queue_unit` validation function. Changes needed:
- Add a `max_queue_depth()` function to `building_stats.rs` (or add a field to `BuildingBaseStats`)
- Return 8 for NestingBox, a default (e.g. 5) for all others
- Validation in the command handler to enforce the cap

---

## 8. Faction-Specific Mechanics

### 8a. Swarm Scaling System

**New system**: `swarm_scaling_system` (runs after `grid_sync`, before `combat`)

Spatial query: for each Clawed unit, count allied Clawed units within 3 tiles. Store result in a `SwarmCount` component (or update `StatModifiers`).

Effects to apply per-tick based on nearby count (N = min(nearby, 8)):
- **Safety in Numbers (Swarmer)**: `damage_reduction += 0.03 * N`, if N >= 5 then `attack_speed_multiplier *= 1.15`
- **Rally the Swarm (WarrenMarshal aura)**: All Clawed in 6-tile radius get `+1% damage * N` and `+1% speed * N`, cap 12%

This is similar to the existing aura system but needs to scale with count. Consider extending the aura system or making this a standalone system.

**Performance note**: This requires per-unit spatial queries. Use the existing `SpatialIndex` from cc_agent, or build a simple grid-based hash map (same as the aura system will need). The 10hz sim tick rate makes O(n*k) acceptable for n < 100 units.

### 8b. Claudeus Maximus GPU Economy

Per the design doc:
- Base GPU command cost: **2** (vs 3-5 for other factions)
- Command issuance rate: **2x** (Claudeus Maximus issues commands twice as fast)
- Net GPU drain: comparable but burstier

Implementation:
- Add a `gpu_command_cost` field to `AiPersonalityProfile` (default 3, Claudeus Maximus = 2)
- Add a `command_rate_multiplier` field (default 1.0, Claudeus Maximus = 2.0)
- The FSM's `eval_speed_mult` already controls eval frequency; the command rate multiplier adjusts how many commands are issued per eval

### 8c. Claudeus Maximus Hallucination Mechanic

8% chance per strategic assessment that Claudeus Maximus misidentifies a unit or building type. Does NOT affect direct commands (move, attack).

Implementation:
- Add a `hallucination_rate` field to `AiPersonalityProfile` (default 0, Claudeus Maximus = 8)
- In the AI's intel-gathering functions (census, threat assessment), roll RNG against hallucination_rate
- When hallucination triggers: swap the `UnitKind` of a randomly selected enemy unit in the census data (not in the actual ECS -- just in the AI's local snapshot)
- WhiskernetRelay reduces rate to 3% -- check if any `WarrenMarshal` is alive with its aura covering the assessed area

### 8d. Scatter (No Retreat)

Clawed units have no coordinated "retreat" command via manual player input. Instead, they have **Scatter** -- all selected units flee in random directions.

Implementation:
- Add `GameCommand::Scatter { unit_ids: Vec<EntityId> }` variant
- When processed: each unit gets a `MoveTarget` to a random position 4-8 tiles away from their current position
- The AI (Claudeus Maximus) CAN issue coordinated retreats via normal move commands -- Scatter is only for manual player input
- The FSM's retreat logic for TheClawed should use targeted move commands (not Scatter)

### 8e. Contagion Cloud Merging (Death System Extension)

When a Plaguetail dies, it spawns a `ContagionCloud` entity at its death position. If another cloud exists within merge range, they merge (extend duration, use larger radius).

Implementation:
- New component: `ContagionCloud { radius: Fixed, remaining_ticks: u32, damage_per_tick: Fixed }`
- Extend the `cleanup` system (or add a post-cleanup system): when a Dead entity with `ContagionCloudOnDeath` is about to be despawned, spawn a cloud entity
- New system `contagion_cloud_system`: ticks cloud duration, applies DoT + Weakened to enemies in radius, handles merging

### 8f. Building-Specific: Seed Vault Scatter-on-Death

When a SeedVault is destroyed, 40% of stored food scatters as 4-6 Stash cache entities.

Implementation:
- Track stored food in a `StoredResources` component (or extend existing economy tracking)
- In the cleanup/death system: when a Dead building with `BuildingKind::SeedVault` is despawned, spawn `StashCache` entities at random nearby positions
- `StashCache` component: `{ food_amount: u32, visible_to: Option<Faction> }` (invisible to enemies)

### 8g. Building-Specific: GnawLab Research Speed Bonus

Research speed increases by 10% per additional GnawLab (max 3 labs = +30% for the 3rd).

Implementation:
- In the research system: count GnawLab buildings owned by the player
- Apply a `research_speed_multiplier = 1.0 + 0.1 * (gnaw_lab_count - 1).max(0)` (capped at 1.2 for 3rd lab)
- This is a per-player modifier, not per-building -- all GnawLabs benefit

### 8h. Building-Specific: WarrenExpansion Burrow Bonuses

- Each WarrenExpansion extends Burrow garrison by +4
- At 3+ WarrenExpansions, units trained at NestingBox get +5% HP for 60s

Implementation:
- Garrison capacity: add `garrison_capacity` field to building or track dynamically via census
- Well-rested bonus: add a temporary `WellRested { remaining_ticks: u32 }` component to newly spawned units from NestingBox when warren_expansion_count >= 3. The status effect system already handles stat modifiers.

---

## 9. AI FSM Updates

### 9a. Replace Cat Unit Proxies in `faction_personality()`

Current `Faction::TheClawed` personality uses cat unit types as proxies:
```rust
unit_preferences: vec![
    (UnitKind::Nuisance, 5),
    (UnitKind::Mouser, 2),
    (UnitKind::Hisser, 1),
],
```

Replace with actual Clawed units:
```rust
unit_preferences: vec![
    (UnitKind::Swarmer, 6),
    (UnitKind::Shrieker, 3),
    (UnitKind::Gnawer, 2),
    (UnitKind::Sparks, 2),
    (UnitKind::Quillback, 1),
],
```

Also update:
- `attack_threshold`: 12 (swarm needs more bodies before attacking)
- `target_workers`: 8 (Nibblets are cheap, want many)
- `economy_priority`: true (need food economy for swarm)
- `retreat_threshold`: 15 (expendable doctrine -- don't retreat easily)
- `eval_speed_mult`: 0.5 (Claudeus Maximus evaluates 2x as fast)
- `chaos_factor`: 12 (occasional hallucination-driven mistakes)

### 9b. Faction-Aware Building Census

The `BuildingCensus` struct currently has cat-building-specific fields:
```rust
has_box: bool,
has_cat_tree: bool,
has_fish_market: bool,
has_server_rack: bool,
// etc.
```

Two approaches:
1. **Add Clawed-specific fields**: `has_burrow`, `has_nesting_box`, `has_seed_vault`, etc. This is verbose but matches the existing pattern.
2. **Refactor to generic**: `HashMap<BuildingKind, BuildingCensusEntry>` where entry tracks entity, count, queue lengths, etc.

**Recommended approach**: Option 2 (generic). The current approach won't scale to 6 factions x 8 buildings = 48 boolean fields. Refactor `BuildingCensus` to:
```rust
struct BuildingCensus {
    buildings: HashMap<BuildingKind, Vec<BuildingInfo>>,
    // where BuildingInfo = { entity, pos, queue_len }
}
```
Then add helper methods: `has(kind)`, `first_entity(kind)`, `total_queue_len(kind)`, etc.

### 9c. Faction-Aware Build Order

The `ensure_economy_buildings()` and `ensure_military_buildings()` functions currently hardcode cat building types:
```rust
building_kind: BuildingKind::FishMarket,
building_kind: BuildingKind::CatTree,
```

These need to be parameterized by faction. Add a mapping:
```rust
fn hq_kind(faction: Faction) -> BuildingKind
fn barracks_kind(faction: Faction) -> BuildingKind
fn depot_kind(faction: Faction) -> BuildingKind
fn tech_kind(faction: Faction) -> BuildingKind
fn research_kind(faction: Faction) -> BuildingKind
fn supply_kind(faction: Faction) -> BuildingKind
fn defense_kind(faction: Faction) -> BuildingKind
fn tower_kind(faction: Faction) -> BuildingKind
```

Or a struct:
```rust
struct FactionBuildingMap {
    hq: BuildingKind,
    barracks: BuildingKind,
    depot: BuildingKind,
    tech: BuildingKind,
    research: BuildingKind,
    supply: BuildingKind,
    defense: BuildingKind,
    tower: BuildingKind,
}
```

### 9d. Faction-Aware Worker Detection

The FSM currently checks `unit_type.kind == UnitKind::Pawdler` to identify workers. This needs to check for the faction's worker type:
```rust
fn is_worker(kind: UnitKind) -> bool {
    matches!(kind, UnitKind::Pawdler | UnitKind::Nibblet)
}
```

Similarly, `is_ranged_unit()` needs updating:
```rust
fn is_ranged_unit(kind: UnitKind) -> bool {
    matches!(kind,
        UnitKind::Hisser | UnitKind::FlyingFox | UnitKind::Catnapper | UnitKind::Yowler |
        UnitKind::Shrieker | UnitKind::Sparks | UnitKind::Whiskerwitch | UnitKind::Plaguetail | UnitKind::WarrenMarshal
    )
}
```

### 9e. TheBurrow Auto-Gather Reroute

The production system auto-sends newly spawned Pawdlers to the nearest deposit. This same logic needs to apply to Nibblets. Change the check from:
```rust
} else if kind == UnitKind::Pawdler {
```
to:
```rust
} else if is_worker(kind) {
```

---

## 10. System Modifications vs New Systems

### Existing Systems to Modify

| System | File | Changes |
|--------|------|---------|
| `production_system` | `crates/cc_sim/src/systems/production_system.rs` | Add spawn-time components for Clawed units; add TheBurrow auto-production; NestingBox queue depth; auto-gather for Nibblets |
| `cleanup` / death system | `crates/cc_sim/src/systems/cleanup.rs` | Trigger ContagionCloud spawn on Plaguetail death; SeedVault scatter-on-death |
| `combat` system | `crates/cc_sim/src/systems/combat.rs` | Cone attack for SonicSpit; Pile On attachment damage; Static Charge discharge |
| `target_acquisition` | `crates/cc_sim/src/systems/target_acquisition.rs` | Respect Pile On attachment (Swarmers attached to target should not re-target) |
| `movement` system | `crates/cc_sim/src/systems/movement.rs` | SpineWall pathing block; Scatter random direction; Stubborn Advance slow floor |
| `grid_sync` | `crates/cc_sim/src/systems/grid_sync.rs` | Track Static Charge tile movement for Sparks |
| `ai/fsm.rs` | `crates/cc_sim/src/ai/fsm.rs` | Full refactor per Section 9 |
| `command_handler` | `crates/cc_sim/src/systems/command_handler.rs` | Add Scatter command; validate NestingBox queue depth |
| `research_system` | `crates/cc_sim/src/systems/research_system.rs` | GnawLab research speed bonus; Clawed-specific upgrades |

### New Systems Needed

| System | Purpose | Runs After | Priority |
|--------|---------|-----------|----------|
| `swarm_scaling_system` | Count nearby allies, apply Safety in Numbers / Rally the Swarm buffs to `StatModifiers` | `grid_sync` | High (core mechanic) |
| `auto_production_system` | Tick AutoProduction on TheBurrow, auto-queue Nibblets | after `production_system` | Medium |
| `contagion_cloud_system` | Tick cloud duration, DoT to enemies, merge nearby clouds | after `cleanup` | Medium |
| `static_charge_system` | Track Sparks movement, increment/decay charge stacks | after `movement` | Medium |
| `structural_weakness_system` | Apply/decay Gnawed stacks on buildings | after `combat` | Medium |
| `pile_on_system` | Tick Pile On attachments, apply damage + slow, detach on expiry | after `combat` | Medium |
| `miasma_trail_system` | Spawn/expire trail tiles for Plaguetails with active MiasmaTrail | after `movement` | Low (visual + minor mechanic) |
| `tremor_sense_system` | Reveal nearby ground units to Tunneler's owner while stationary | after `grid_sync` | Low (info only) |
| `scatter_dust_cloud_system` | Track synchronized Scatters, spawn dust clouds | after `movement` | Low |
| `seed_vault_gather_bonus_system` | Apply +15% gather speed to Nibblets near SeedVaults | after `grid_sync` | Low |
| `warren_well_rested_system` | Apply +5% HP buff to NestingBox-trained units when 3+ WarrenExpansions exist | in `production_system` | Low |

### System Chain Update

Current chain: `tick -> commands -> target_acquisition -> combat -> projectile -> movement -> grid_sync -> cleanup`

Updated chain with Clawed systems inserted:
```
tick
  -> commands (+ Scatter handler)
  -> swarm_scaling_system (spatial count query)
  -> target_acquisition (+ Pile On attachment check)
  -> combat (+ cone attack, Static Charge discharge, Gnawed stack application)
  -> pile_on_system (attachment tick)
  -> structural_weakness_system (stack decay)
  -> projectile
  -> movement (+ SpineWall block, Stubborn Advance floor, Scatter)
  -> static_charge_system (movement-based charge accrual)
  -> grid_sync
  -> contagion_cloud_system (DoT + merge)
  -> miasma_trail_system (trail spawn/expire)
  -> tremor_sense_system (detection)
  -> cleanup (+ Plaguetail cloud spawn, SeedVault scatter)
  -> auto_production_system (Burrow Nibblet queue)
```

---

## 11. Upgrade / Research System

### New UpgradeType Variants

Add Clawed-specific upgrades to the `UpgradeType` enum in `components.rs`:

| UpgradeType | Cost (Food/GPU) | Research Time | Effect |
|-------------|----------------|---------------|--------|
| `SharperTeeth` | 100/50 | 200 ticks | +2 damage for all Clawed combat units |
| `ThickerHide` | 100/50 | 200 ticks | +15 HP for all Clawed combat units |
| `QuickPaws` | 75/25 | 150 ticks | +10% speed for all Clawed units |
| `AdvancedGnawing` | 150/75 | 250 ticks | Unlocks Gnawer Chew Through cooldown -30% |
| `WarrenProtocol` | 200/100 | 300 ticks | Unlocks WarrenMarshal training at JunkTransmitter |

These mirror the catGPT upgrades (SharperClaws, ThickerFur, NimblePaws, SiegeTraining, MechPrototype) with faction-appropriate names and identical mechanical effects where possible.

**Changes required**:
- Add 5 variants to `UpgradeType` enum in `components.rs`
- Update `UpgradeType::fmt` and `from_str`
- Update `apply_upgrades_to_new_unit` in research_system.rs to handle new upgrade types
- The GnawLab research speed bonus (Section 8g) modifies research tick rates

---

## 12. Test Plan

### Unit Tests (cc_core)

| Test | File | What it validates |
|------|------|-------------------|
| `all_clawed_kinds_have_stats` | `unit_stats.rs` | All 10 Clawed UnitKind variants return valid UnitBaseStats |
| `clawed_melee_range_one` | `unit_stats.rs` | Nibblet, Swarmer, Gnawer, Tunneler, Quillback have range 1 |
| `clawed_ranged_range_gt_one` | `unit_stats.rs` | Shrieker, Sparks, Whiskerwitch, Plaguetail, WarrenMarshal have range > 1 |
| `swarmer_cheaper_than_nuisance` | `unit_stats.rs` | Swarmer food cost < Nuisance food cost |
| `quillback_clawed_tankiest_non_hero` | `unit_stats.rs` | Quillback HP > all other non-hero Clawed |
| `warren_marshal_is_clawed_hero` | `unit_stats.rs` | WarrenMarshal has highest HP + cost among Clawed |
| `all_clawed_building_stats` | `building_stats.rs` | All 8 Clawed BuildingKind variants return valid stats |
| `burrow_is_pre_built` | `building_stats.rs` | TheBurrow build_time == 0 |
| `nesting_box_produces_basic_clawed` | `building_stats.rs` | NestingBox can_produce includes Swarmer, Gnawer, Plaguetail, Sparks |
| `junk_transmitter_produces_advanced` | `building_stats.rs` | JunkTransmitter can_produce includes Shrieker, Tunneler, Quillback, Whiskerwitch, WarrenMarshal |
| `all_clawed_ability_defs_valid` | `abilities.rs` | All 30 Clawed AbilityId variants have valid defs |
| `clawed_unit_abilities_three_per_kind` | `abilities.rs` | All 10 Clawed UnitKind variants have 3 distinct abilities |
| `clawed_passives_no_cooldown` | `abilities.rs` | All Clawed passive abilities have 0 cooldown |
| `gnawer_structural_weakness_scales` | `abilities.rs` | Multiplier scales correctly with stacks |
| `incisors_damage_caps_at_40pct` | `abilities.rs` | Bonus caps at +40% |
| `unit_kind_round_trip_clawed` | `components.rs` | All 10 Clawed kinds Display + FromStr |
| `building_kind_round_trip_clawed` | `components.rs` | All 8 Clawed building kinds Display + FromStr |
| `upgrade_type_round_trip_clawed` | `components.rs` | All 5 Clawed upgrade types Display + FromStr |

### Integration Tests (cc_sim)

| Test | What it validates |
|------|-------------------|
| `spawn_swarmer_has_correct_components` | Swarmer spawns with AbilitySlots, StatModifiers, correct stats |
| `spawn_gnawer_has_structural_weakness_timer` | Gnawer gets StructuralWeaknessTimer on spawn |
| `spawn_sparks_has_static_charge` | Sparks gets StaticChargeStacks on spawn |
| `spawn_plaguetail_has_contagion_marker` | Plaguetail gets ContagionCloudOnDeath marker |
| `spawn_warren_marshal_has_aura` | WarrenMarshal gets RallyTheSwarm Aura |
| `burrow_auto_produces_nibblets` | TheBurrow generates Nibblets on timer |
| `nesting_box_queue_depth_8` | NestingBox can queue up to 8 units |
| `plaguetail_death_spawns_cloud` | ContagionCloud entity created on Plaguetail death |
| `contagion_clouds_merge` | Two clouds in range merge, duration resets |
| `swarm_scaling_applies_bonuses` | Safety in Numbers grants correct DR at various ally counts |
| `spine_wall_blocks_pathing` | Quillback in SpineWall blocks movement |
| `scatter_command_moves_randomly` | Scatter causes units to move in different directions |
| `clawed_ai_uses_clawed_units` | TheClawed AI trains Swarmers/Shriekers, not Nuisances/Hissers |
| `clawed_ai_builds_clawed_buildings` | TheClawed AI builds NestingBox, not CatTree |
| `gnawed_stacks_increase_building_damage` | Building with Gnawed stacks takes amplified damage |
| `static_charge_accumulates_on_move` | Sparks gains stacks while moving, decays while stationary |

---

## 13. Implementation Order

Phased implementation to maintain a working build at each step.

### Phase A: Data Layer (no behavior changes)
- [ ] A1. Add 10 UnitKind variants + Display/FromStr
- [ ] A2. Add 8 BuildingKind variants + Display/FromStr
- [ ] A3. Add 30 AbilityId variants
- [ ] A4. Add 5 UpgradeType variants
- [ ] A5. Implement `base_stats()` for all 10 Clawed units
- [ ] A6. Implement `building_stats()` for all 8 Clawed buildings
- [ ] A7. Implement `ability_def()` for all 30 Clawed abilities
- [ ] A8. Implement `unit_abilities()` for all 10 Clawed units
- [ ] A9. Add helper functions: `gnawer_structural_weakness_multiplier()`, `incisors_damage_bonus()`
- [ ] A10. Unit tests for all data layer changes (must pass: `cargo test -p cc_core`)

### Phase B: Component Layer
- [ ] B1. Add new components: `SwarmScaling`, `StructuralWeaknessTimer`, `StaticChargeStacks`, `PanicState`, `ContagionCloudOnDeath`, `PileOnAttachment`, `GnawedStacks`, `WhiskernetRelayMarker`, `AutoProduction`, `StashCache`, `ContagionCloud`
- [ ] B2. Add new AuraType variants: `RallyTheSwarm`, `WhiskernetRelay`, `SqueakTowerPulse`
- [ ] B3. Add `GameCommand::Scatter` variant
- [ ] B4. Add `max_queue_depth` to `BuildingBaseStats` or as separate function

### Phase C: Production & Spawn
- [ ] C1. Add spawn-time components for Clawed units in `production_system.rs`
- [ ] C2. Implement `AutoProduction` component + auto_production_system for TheBurrow
- [ ] C3. Implement SqueakTower attack stats on construction completion
- [ ] C4. Implement GnawLab Researcher + ResearchQueue on construction completion
- [ ] C5. Generalize worker auto-gather to include Nibblets
- [ ] C6. Integration tests for spawn correctness

### Phase D: Core Mechanics
- [ ] D1. `swarm_scaling_system` -- nearby ally counting + stat modifier application
- [ ] D2. `contagion_cloud_system` -- death cloud spawning, DoT, merging
- [ ] D3. `static_charge_system` -- movement tracking, stack accrual/decay
- [ ] D4. `structural_weakness_system` -- Gnawed stack application and decay
- [ ] D5. `pile_on_system` -- attachment, continuous damage, slow, detach
- [ ] D6. Scatter command handler in command_handler.rs
- [ ] D7. SpineWall toggle handler (pathing block, DR, melee reflect)
- [ ] D8. Integration tests for core mechanics

### Phase E: Secondary Mechanics
- [ ] E1. `miasma_trail_system` -- trail spawn/expire, slow + damage
- [ ] E2. `tremor_sense_system` -- ground unit detection while stationary
- [ ] E3. `scatter_dust_cloud_system` -- synchronized Scatter detection, cloud spawn
- [ ] E4. SeedVault gather speed bonus + scatter-on-death
- [ ] E5. GnawLab research speed bonus
- [ ] E6. WarrenExpansion garrison extension + well-rested bonus
- [ ] E7. NestingBox queue depth enforcement
- [ ] E8. ShortCircuit building disable + AI suppression

### Phase F: AI FSM
- [ ] F1. Refactor `BuildingCensus` to generic `HashMap<BuildingKind, Vec<BuildingInfo>>`
- [ ] F2. Add `FactionBuildingMap` and faction-aware building selection
- [ ] F3. Generalize `is_worker()` and `is_ranged_unit()` checks
- [ ] F4. Update `faction_personality(Faction::TheClawed)` with actual Clawed units
- [ ] F5. Add hallucination mechanic to AI census/threat assessment
- [ ] F6. Add GPU economy fields to `AiPersonalityProfile` (base cost, rate multiplier)
- [ ] F7. AI integration tests: Clawed AI builds correct buildings, trains correct units

### Phase G: Upgrades & Research
- [ ] G1. Implement `apply_upgrades_to_new_unit` for Clawed upgrade types
- [ ] G2. AI research priorities for Clawed faction
- [ ] G3. Research tests

---

## 14. File Change Summary

### Files to Modify

| File | Nature of Changes |
|------|-------------------|
| `crates/cc_core/src/components.rs` | +10 UnitKind, +8 BuildingKind, +5 UpgradeType, +3 AuraType, new components (SwarmScaling, StructuralWeaknessTimer, StaticChargeStacks, PanicState, ContagionCloudOnDeath, PileOnAttachment, GnawedStacks, WhiskernetRelayMarker, AutoProduction, StashCache, ContagionCloud), new GameCommand::Scatter |
| `crates/cc_core/src/unit_stats.rs` | +10 base_stats() arms, updated tests |
| `crates/cc_core/src/building_stats.rs` | +8 building_stats() arms, possibly +max_queue_depth(), updated tests |
| `crates/cc_core/src/abilities.rs` | +30 AbilityId variants, +30 ability_def() arms, +10 unit_abilities() arms, +2 helper functions, updated tests |
| `crates/cc_core/src/commands.rs` | +Scatter variant to GameCommand |
| `crates/cc_sim/src/systems/production_system.rs` | Clawed spawn-time components, auto-production, worker generalization |
| `crates/cc_sim/src/systems/cleanup.rs` | Plaguetail death cloud, SeedVault scatter |
| `crates/cc_sim/src/systems/combat.rs` | Cone attack, Static Charge discharge, Gnawed stacks |
| `crates/cc_sim/src/systems/target_acquisition.rs` | Pile On attachment check |
| `crates/cc_sim/src/systems/movement.rs` | SpineWall block, Stubborn Advance, Scatter |
| `crates/cc_sim/src/systems/command_handler.rs` | Scatter handler, NestingBox queue validation |
| `crates/cc_sim/src/systems/research_system.rs` | Clawed upgrades, GnawLab speed bonus |
| `crates/cc_sim/src/ai/fsm.rs` | Full faction-awareness refactor (census, build order, unit preferences, worker detection, hallucination) |
| `crates/cc_sim/src/lib.rs` | Register new systems in FixedUpdate chain |
| `crates/cc_agent/src/lib.rs` | Update ScriptContext / behavior methods if they reference UnitKind or BuildingKind by name |
| `crates/cc_harness/src/lib.rs` | Update MCP tools to expose Clawed unit/building types |
| `crates/cc_client/src/setup.rs` | Sprite loading for Clawed units/buildings |
| `crates/cc_client/src/units.rs` | Sprite rendering for Clawed units |

### New Files

| File | Purpose |
|------|---------|
| `crates/cc_sim/src/systems/swarm_scaling.rs` | Swarm count spatial query + stat modifier system |
| `crates/cc_sim/src/systems/contagion_cloud.rs` | Cloud entity tick, DoT, merging system |
| `crates/cc_sim/src/systems/static_charge.rs` | Sparks charge accrual/decay system |
| `crates/cc_sim/src/systems/structural_weakness.rs` | Gnawed stack application/decay system |
| `crates/cc_sim/src/systems/pile_on.rs` | Swarmer attachment tick system |
| `crates/cc_sim/src/systems/miasma_trail.rs` | Trail spawn/expire system |
| `crates/cc_sim/src/systems/tremor_sense.rs` | Ground detection system |
| `crates/cc_sim/src/systems/auto_production.rs` | TheBurrow auto-Nibblet system |
| `crates/cc_sim/src/systems/faction_buildings.rs` | FactionBuildingMap, building role resolution |

### Estimated Scope

- **Enum/data additions**: ~400 lines across cc_core
- **New components**: ~200 lines in components.rs
- **New systems**: ~1200 lines across 8 new system files
- **FSM refactor**: ~300 lines of changes to fsm.rs
- **Production system**: ~100 lines of additions
- **Tests**: ~500 lines of new tests
- **Total estimate**: ~2700 lines of new/modified code

---

## Open Questions

1. **Multi-aura units**: Should we refactor `Aura` to support multiple auras per entity now (for WarrenMarshal), or use the marker-component workaround? Refactoring now is cleaner but touches existing Chonk/MechCommander code.

2. **Faction-building coupling**: Should `BuildingKind` variants encode their faction (`BuildingKind::Clawed(ClawedBuilding::NestingBox)`) or remain flat? Flat is simpler for now but won't scale cleanly to 6 factions x 8 buildings = 48 variants.

3. **Cone attack geometry**: SonicSpit uses a cone (1-tile wide at origin, 2 at max range). The current combat system does single-target damage. Do we extend AttackType with a `Cone { angle, width }` variant, or handle this entirely in the ability system?

4. **Building garrison system**: Both TheBurrow and Mousehole need garrison mechanics (units enter building, heal or fire from inside). This is listed as "deferred" for CatFlap. Should we implement basic garrison now for Clawed or defer for both factions?

5. **Illusion entities**: Hex of Multiplication creates fake units (1 HP, no damage, count for swarm scaling). These need to be real ECS entities with a `Illusion` marker component. Do they need full combat components (AttackStats, etc.) or minimal?
