# Croak (Axolotls / Grok) - Full Faction Implementation Plan

> Faction identity: "You can't kill what won't die." Individually unimpressive, collectively unkillable. Attrition warfare through regeneration, water terrain manipulation, and the Limb Economy.

---

## Table of Contents

1. [Scope Summary](#1-scope-summary)
2. [UnitKind Variants & Stats](#2-unitkind-variants--stats)
3. [BuildingKind Variants & Stats](#3-buildingkind-variants--stats)
4. [AbilityId Variants & Definitions](#4-abilityid-variants--definitions)
5. [Faction-Specific Components](#5-faction-specific-components)
6. [Faction-Specific Systems](#6-faction-specific-systems)
7. [Existing System Modifications](#7-existing-system-modifications)
8. [AI FSM Updates](#8-ai-fsm-updates)
9. [Asset Pipeline Entries](#9-asset-pipeline-entries)
10. [Testing Strategy](#10-testing-strategy)
11. [Implementation Order](#11-implementation-order)

---

## 1. Scope Summary

### What already exists
- `Faction::Croak` enum variant in `components.rs`
- `FactionId::Croak` in `terrain.rs` with `can_traverse_water() -> true`
- `is_passable_for_faction(Water, Croak) -> true` (Croak water traversal in pathfinding)
- Croak pathfinding test in `pathfinding.rs`
- Grok AI personality profile in `fsm.rs` (using cat unit proxies: Chonk, Yowler, Hisser)

### What needs to be built
- 10 new UnitKind variants + base_stats entries
- 8 new BuildingKind variants + building_stats entries
- 30 new AbilityId variants + ability_def entries + unit_abilities mappings
- 6 new ECS components (LimbTracker, WaterAffinityBuff, Waterlogged, MucusTrail, SpawnlingParent, BogPatch)
- 4 new simulation systems (water_affinity_system, limb_system, mucus_trail_system, bog_patch_system)
- Modifications to 5 existing systems (command_system, production_system, combat_system, movement_system, cleanup_system)
- AI FSM update: replace cat unit proxies with Croak unit types
- Croak-specific upgrades in UpgradeType enum
- Asset catalog entries for 18 new sprites (10 units + 8 buildings)

---

## 2. UnitKind Variants & Stats

All 10 Croak units to add to `UnitKind` enum in `crates/cc_core/src/components.rs` and `base_stats()` in `crates/cc_core/src/unit_stats.rs`.

### Design Principles for Stat Balance
- Croak units are individually weaker than cat equivalents (lower damage, slightly lower HP on non-tanks)
- They compensate through regeneration stacking, Water Affinity bonuses, and synergies
- Speed is moderate across the board -- they don't run, they endure
- Higher food costs than cats on average (regeneration is "free" value that must be paid for upfront)

### Unit Stat Table

| # | UnitKind Variant | Role | HP | Speed | Damage | Range | AtkSpd (ticks) | AtkType | Food | GPU | Supply | Train (ticks) |
|---|-----------------|------|-----|-------|--------|-------|----------------|---------|------|-----|--------|---------------|
| 1 | `Ponderer` | Worker | 55 | 0.10 | 3 | 1 | 18 | Melee | 50 | 0 | 1 | 50 |
| 2 | `Regeneron` | Light Skirmisher | 75 | 0.16 | 7 | 1 | 10 | Melee | 75 | 0 | 1 | 60 |
| 3 | `Broodmother` | Healer/Support | 100 | 0.10 | 4 | 3 | 15 | Ranged | 125 | 25 | 2 | 100 |
| 4 | `Gulper` | Heavy Bruiser | 280 | 0.07 | 10 | 1 | 18 | Melee | 175 | 25 | 3 | 130 |
| 5 | `Eftsaber` | Assassin/Flanker | 60 | 0.18 | 12 | 1 | 9 | Melee | 100 | 25 | 2 | 80 |
| 6 | `Croaker` | Ranged Artillery | 65 | 0.10 | 16 | 6 | 40 | Ranged | 125 | 0 | 2 | 90 |
| 7 | `Leapfrog` | Mobile Harasser | 70 | 0.17 | 8 | 1 | 10 | Melee | 75 | 0 | 1 | 60 |
| 8 | `Shellwarden` | Tank/Defender | 350 | 0.06 | 6 | 1 | 22 | Melee | 175 | 50 | 4 | 140 |
| 9 | `Bogwhisper` | Support/Caster | 80 | 0.11 | 5 | 5 | 15 | Ranged | 125 | 50 | 2 | 110 |
| 10 | `MurkCommander` | Hero/Heavy | 450 | 0.09 | 15 | 3 | 15 | Ranged | 400 | 200 | 6 | 250 |

### Stat Rationale

**Ponderer (worker)**: Slower and weaker than Pawdler (0.10 vs 0.12 speed, 55 vs 60 HP, 3 vs 4 damage). Compensated by Ambient Gathering's zero-trip economy on water. Same cost.

**Regeneron (skirmisher)**: Comparable to Nuisance but slightly slower (0.16 vs 0.18) with less HP (75 vs 80). Limb Economy provides unique utility (pathing denial, Waterlogged) that Nuisance lacks. Same cost tier.

**Broodmother (support)**: Higher HP than Yowler (100 vs 90) but slower (0.10 vs 0.14) and lower damage. More expensive (125+25 vs 100+50 GPU). Justification: Spawnling generation provides free bodies and Transfusion is potent burst healing.

**Gulper (heavy)**: Slightly less HP than Chonk (280 vs 300) but with massive regen (Bottomless). Slower (0.07 vs 0.08). Higher cost (175+25 vs 150+25). Devour is extremely high-value -- removing a unit from the fight for 8s.

**Eftsaber (assassin)**: Fragile (60 HP) but fast (0.18) with high single-target damage (12). Toxic Skin passive discourages melee trades. More expensive than Mouser (100+25 vs 75+25) because Waterway provides invulnerability + speed on water.

**Croaker (artillery)**: Low HP (65), very slow attack speed (40 ticks = 4s between shots), but 6-tile range with splash. Compensated by Bog Mortar creating water terrain. Moderate cost.

**Leapfrog (harasser)**: Moderate stats across the board. Fast (0.17), decent damage (8). True power is in Hop chain mobility on water tiles. Same cost tier as Nuisance.

**Shellwarden (tank)**: Tankiest non-hero at 350 HP. Extremely slow (0.06). Very low damage output. High cost (175+50). Hunker mode makes it a terrain obstacle. Ancient Moss aura is the real value.

**Bogwhisper (caster)**: Fragile caster (80 HP). Moderate range (5). Low direct damage. High GPU cost (50) because Prophecy and Mire Curse are extremely high-value abilities.

**MurkCommander (hero)**: Lower HP than MechCommander (450 vs 500). Same cost tier. Undying Presence aura + Stasis + Murk Uplink GPU discount make it the faction's lynchpin.

### Code Changes: `crates/cc_core/src/components.rs`

Add to `UnitKind` enum:
```
Ponderer,       // Worker (Croak) -- gathers via ambient gathering on water
Regeneron,      // Light Skirmisher (Croak) -- Limb Economy, self-regen
Broodmother,    // Healer/Support (Croak) -- spawns Spawnlings, burst heals
Gulper,         // Heavy Bruiser (Croak) -- Devour mechanic, massive regen
Eftsaber,       // Assassin/Flanker (Croak) -- poison, Waterway stealth
Croaker,        // Ranged Artillery (Croak) -- Bog Mortar, terrain creation
Leapfrog,       // Mobile Harasser (Croak) -- Hop chains on water
Shellwarden,    // Tank/Defender (Croak) -- Hunker, Ancient Moss aura
Bogwhisper,     // Support/Caster (Croak) -- Mire Curse, Prophecy
MurkCommander,  // Hero/Heavy (Croak) -- Grok Protocol, Murk Uplink
```

Also add `FromStr` and `Display` impl arms for each.

### Code Changes: `crates/cc_core/src/unit_stats.rs`

Add 10 new arms to the `base_stats()` match block following the exact same pattern as the existing cat units (Fixed::from_bits for HP/speed/damage/range, plain u32 for attack_speed/costs/train_time).

Update the `all_kinds_have_stats` test to include all 10 Croak units. Add equivalent `melee_units_have_range_one` and `ranged_units_have_range_greater_than_one` test entries.

---

## 3. BuildingKind Variants & Stats

All 8 Croak buildings to add to `BuildingKind` enum in `crates/cc_core/src/components.rs` and `building_stats()` in `crates/cc_core/src/building_stats.rs`.

### Building Stat Table

| # | BuildingKind Variant | Cat Equivalent | HP | Build Time (ticks) | Food | GPU | Supply | Produces |
|---|---------------------|----------------|-----|-------------------|------|-----|--------|----------|
| 1 | `TheGrotto` | TheBox (HQ) | 500 | 0 (pre-built) | 0 | 0 | 10 | `[Ponderer]` |
| 2 | `SpawningPools` | CatTree (Barracks) | 300 | 150 | 150 | 0 | 0 | `[Regeneron, Croaker, Leapfrog, Gulper]` |
| 3 | `LilyMarket` | FishMarket (Resource Depot) | 200 | 100 | 100 | 0 | 0 | `[]` |
| 4 | `SunkenServer` | ServerRack (Tech) | 250 | 120 | 100 | 75 | 0 | `[Eftsaber, Broodmother, Shellwarden, Bogwhisper, MurkCommander]` |
| 5 | `FossilStones` | ScratchingPost (Research) | 200 | 100 | 100 | 50 | 0 | `[]` |
| 6 | `ReedBed` | LitterBox (Supply) | 100 | 75 | 75 | 0 | 10 | `[]` |
| 7 | `TidalGate` | CatFlap (Garrison) | 400 | 100 | 150 | 0 | 0 | `[]` |
| 8 | `SporeTower` | LaserPointer (Defense) | 150 | 80 | 75 | 25 | 0 | `[]` |

### Building-Specific Mechanics

**TheGrotto**: Functions identically to TheBox but can be placed on water tiles. Pre-built at game start.

**SpawningPools**: Trains basic combat units. Adjacent water tiles increase training speed by 15% -- requires a new component `AdjacentWaterBonus` checked during production_system tick. Produces: Regeneron (light skirmisher), Croaker (ranged), Leapfrog (harasser), Gulper (heavy).

**LilyMarket**: Resource drop-off. If built on water, Ponderers within 3 tiles use Ambient Gathering passively (no return trips). This is handled by the gather system checking for nearby LilyMarket-on-water.

**SunkenServer**: Tech building. Generates 10% more GPU Cores than standard -- implemented as a modifier in the resource tick system. Produces: Eftsaber, Broodmother, Shellwarden, Bogwhisper, MurkCommander.

**FossilStones**: Research building. Functions like ScratchingPost with Croak-specific upgrades. One-time 5% max HP bonus to all living units of researched type on completion -- requires a one-shot system event on research complete.

**ReedBed**: Supply depot. Provides concealment -- enemy units lose vision within 2 tiles. This is a fog-of-war interaction (Phase 2+ feature, stub for now).

**TidalGate**: Garrison building. When garrisoned units > 3, floods adjacent tiles (WaterConvert overlay). Ungarrisoning reverses after 5s. Uses existing `OverlayEffect::WaterConvert`.

**SporeTower**: Defense tower. Like LaserPointer but applies Waterlogged debuff and deals DoT (2% max HP/s for 6s). Prioritizes enemies on water tiles.

### Code Changes: `crates/cc_core/src/components.rs`

Add to `BuildingKind` enum:
```
TheGrotto,      // HQ (Croak)
SpawningPools,  // Barracks (Croak)
LilyMarket,     // Resource Depot (Croak)
SunkenServer,   // Tech Building (Croak)
FossilStones,   // Research (Croak)
ReedBed,        // Supply Depot (Croak)
TidalGate,      // Garrison/Gate (Croak)
SporeTower,     // Defense Tower (Croak)
```

Also add `FromStr` / `Display` arms.

### Code Changes: `crates/cc_core/src/building_stats.rs`

Add 8 new arms to `building_stats()` match. Update all tests.

---

## 4. AbilityId Variants & Definitions

30 new abilities (3 per unit) to add to `AbilityId` enum in `crates/cc_core/src/abilities.rs`.

### Ability Table

#### Ponderer (Worker)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `AmbientGathering` | Passive | 0 | 0 | 0 | 0 | 0 | 40% slower on land, 120% on water; passive gather near LilyMarket on water |
| `MucusTrail` | Passive | 0 | 0 | 0 | 0 | 0 | Leaves trail tiles; +10% ally speed, -15% enemy speed; 30s duration |
| `ExistentialDread` | Passive | 150 | 0 | 80 | 3 | 0 | Auto-triggers on ally death within 4 tiles; -20% enemy attack speed in 3-tile AoE |

#### Regeneron (Light Skirmisher)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `LimbToss` | Activated | 30 | 0 | 0 | 5 | 0 | Costs 1 Limb; moderate damage + Waterlogged 6s; creates pathing block 8s |
| `RegrowthBurst` | Activated | 250 | 0 | 0 | 0 | 0 | Regrows all Limbs; costs 30% current HP (15% on water) |
| `PhantomLimb` | Passive | 0 | 0 | 0 | 0 | 0 | +8% attack speed per missing Limb (max +32% at 0 Limbs) |

#### Broodmother (Healer/Support)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `SpawnPool` | Passive | 300 (180 on water) | 0 | 0 | 0 | 0 | Auto-spawns Spawnling every 30s/18s; max 4 active per Broodmother |
| `Transfusion` | Activated | 0 | 0 | 50 | 3 | 0 | Sacrifices a Spawnling to heal ally 25% max HP over 5s; limited by Spawnling count |
| `PrimordialSoup` | Activated | 350 | 0 | 120 | 0 | 0 | 3x3 regen pool (3% HP/s); counts as water; enemies Waterlogged |

#### Gulper (Heavy Bruiser)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `Devour` | Activated | 300 | 0 | 80 | 1 | 0 | Swallows enemy <30% HP; digests for 10% maxHP/s true dmg; gains temp shields |
| `Regurgitate` | Activated | 100 | 0 | 0 | 4 | 0 | Ends Devour early; spits unit + Waterlogged AoE; or bile glob if empty |
| `Bottomless` | Passive | 0 | 0 | 0 | 0 | 0 | +100% HP regen (+200% on water); below 25% HP: doubles again for 5s (once/60s) |

#### Eftsaber (Assassin/Flanker)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `ToxicSkin` | Passive | 0 | 0 | 0 | 0 | 0 | Melee attackers take 3% their maxHP as poison/hit; stacks 5x |
| `Waterway` | Activated | 50 | 0 | 0 | 0 | 0 | Submerge in water tile; untargetable, invisible, 150% speed; water-only movement |
| `Venomstrike` | Activated | 120 | 0 | 0 | 3 | 0 | Lunge + heavy dmg + 50% of active poison stacks; free Waterway on kill if water nearby |

#### Croaker (Ranged Artillery)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `BogMortar` | Passive | 0 | 0 | 0 | 6 | 0 | Primary attack mod: 2-tile splash, leaves Bog Patch (water terrain 15s), -20% enemy speed |
| `ResonanceChain` | Passive | 0 | 0 | 0 | 0 | 0 | Bog Mortars near existing patches link; 3+ chain triggers Bog Eruption (AoE burst) |
| `Inflate` | Activated | 180 | 0 | 30 | 0 | 0 | Immobile 3s; next Bog Mortar +50% range, +75% splash; hitbox 50% larger |

#### Leapfrog (Mobile Harasser)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `Hop` | Activated | 60 | 0 | 0 | 4 | 0 | Leap to tile; light landing damage; cooldown reset on landing on water/Bog Patch |
| `TongueLash` | Activated | 100 | 0 | 0 | 5 | 0 | Pulls enemy 2 tiles toward Leapfrog; Waterlogged if lands on water; no pull on heavy class |
| `Slipstream` | Passive | 0 | 0 | 30 | 0 | 0 | After Hop, next attack within 3s: +40% damage + 1s micro-stun (Drowsed) |

#### Shellwarden (Tank/Defender)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `Hunker` | Toggle | 10 | 0 | 0 | 0 | 0 | 75% damage reduction, 15% reflect; immobile, no attack; counts as impassable terrain |
| `AncientMoss` | Passive | 0 | 0 | 0 | 3 (5 on water) | 0 | Aura: 1.5% maxHP/s regen to allies; self 0.5% HP/s; diminishing returns on overlap |
| `TidalMemory` | Activated | 600 | 6 | 200 | 0 | 0 | 5x5 flood zone centered on self; allies get Water Affinity; enemies Waterlogged + -25% dmg |

#### Bogwhisper (Support/Caster)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `MireCurse` | Activated | 200 | 0 | 80 | 6 | 0 | Target generates Bog Patch every 2s while moving (4 max); 2% HP/s if standing still |
| `Prophecy` | Activated | 300 | 4 | 60 | 8 | 0 | Reveals fog 8-tile radius; shows enemy ability cooldowns 15s; benefits from Murk Uplink |
| `BogSong` | Passive | 0 | 0 | 0 | 5 | 0 | Aura: below 50% HP = 2x regen rate; above 50% = +5% move speed; water raises threshold to 65% |

#### MurkCommander (Hero/Heavy)

| AbilityId Variant | Activation | Cooldown (ticks) | GPU Cost | Duration (ticks) | Range (tiles) | Max Charges | Notes |
|-------------------|-----------|------------------|----------|------------------|---------------|-------------|-------|
| `UndyingPresence` | Passive | 0 | 0 | 0 | 8 | 0 | Aura: +30% regen, CC immunity 1s at <10% HP (once/30s); self cannot die in 1 hit |
| `GrokProtocol` | Activated | 450 | 8 | 120 | 0 | 0 | Target ally: +25% all stats, 4% HP/s regen 12s; self regen suppressed; Murk Uplink discount |
| `MurkUplink` | Passive | 0 | 0 | 0 | 0 | 0 | GPU commands in aura cost 50% less; units that die in aura enter Stasis (2s invuln, revive 15% HP, 90s per-unit CD) |

### Code Changes: `crates/cc_core/src/abilities.rs`

1. Add 30 new variants to `AbilityId` enum (grouped by unit with comments, matching cat pattern)
2. Add 30 new arms to `ability_def()` match
3. Add 10 new arms to `unit_abilities()` match (each returning `[AbilityId; 3]`)
4. Update `all_ability_defs_valid` test to include all 60 abilities (30 cat + 30 Croak)
5. Update `unit_abilities_returns_three_per_kind` test to include all 20 unit kinds
6. Add Croak-specific passive/toggle/activated tests mirroring existing cat tests

---

## 5. Faction-Specific Components

New ECS components needed in `crates/cc_core/src/components.rs`.

### 5.1 LimbTracker

Tracks the Limb Economy for axolotl units that participate in it (Regeneron primarily, potentially extensible).

```
Component: LimbTracker {
    current_limbs: u8,       // 0-4
    max_limbs: u8,           // 4
    regen_ticks: u32,        // ticks until next limb regen (200 on land = 20s, 120 on water = 12s)
}
```

Attached at spawn time to: `Regeneron`

### 5.2 WaterAffinityBuff

Applied dynamically by the water_affinity_system when a Croak unit stands on a water tile. Removed when the unit leaves water.

```
Component: WaterAffinityBuff
```

This is a marker component. The actual stat modifications (+25% speed, +15% damage, +2 HP/s regen) are applied through `StatModifiers` in the water_affinity_system each tick.

### 5.3 Waterlogged (Status Effect)

A debuff applied by many Croak abilities. Could be implemented as a status effect variant in `StatusEffects` or as a dedicated component.

Recommendation: Add to the existing `StatusEffects` system as a new variant, since the codebase already has a `StatusEffects` component. Add a `Waterlogged` variant with:
- Duration: 60 ticks (6s)
- Effect: -10% move speed, -50% fire damage dealt (fire damage type TBD -- for now, just -10% speed)
- Refresh on reapplication

### 5.4 MucusTrailTile

Not an entity component but a map overlay. Use existing `TerrainOverlay` system with a new `OverlayEffect` variant:

```
OverlayEffect::MucusTrail {
    owner_faction: FactionId,
    ally_speed_bonus: Fixed,   // +10%
    enemy_speed_penalty: Fixed, // -15%
}
```

Add this variant to `OverlayEffect` enum in `crates/cc_core/src/terrain.rs`.

### 5.5 SpawnlingParent

Tracks the Broodmother that spawned a Spawnling, and the Spawnling count per Broodmother.

```
Component: SpawnlingParent {
    parent_entity: EntityId,
}

Component: SpawnlingCounter {
    count: u8,              // current active Spawnlings (max 4)
    spawn_cooldown: u32,    // ticks until next spawn
}
```

### 5.6 BogPatchTracker

Tracks Bog Patches for Resonance Chain logic. Each Bog Patch is a terrain overlay. The tracker lives on the Croaker to count its chain state.

```
Component: BogPatchCounter {
    active_patches: Vec<(i32, i32)>,  // grid positions of patches this Croaker created
}
```

### 5.7 DevourState

Tracks the Gulper's Devour ability state.

```
Component: DevourState {
    swallowed_entity: EntityId,
    swallowed_max_hp: Fixed,
    digest_ticks_remaining: u32,
    digest_damage_per_tick: Fixed,
    temp_shields: Fixed,
}
```

### 5.8 HunkerState

Marker for Shellwarden in Hunker mode.

```
Component: Hunkered    // marker -- unit is in shell
```

When Hunkered:
- 75% damage_reduction applied via StatModifiers
- `immobilized: true` in StatModifiers
- `cannot_attack: true` in StatModifiers
- Reflects 15% incoming damage (handled in combat_system)
- Unit's GridCell position becomes impassable for pathfinding

### 5.9 SubmergedState

Marker for Eftsaber in Waterway mode.

```
Component: Submerged   // marker -- unit is underwater
```

When Submerged:
- Untargetable (skip in target_acquisition_system)
- Invisible (no vision, no rendering)
- +50% speed via StatModifiers
- Can only traverse water tiles (pathfinding constraint)

### 5.10 StasisState

Tracks units saved by Murk Uplink's Stasis mechanic.

```
Component: Stasis {
    remaining_ticks: u32,    // 20 ticks (2s)
    revive_hp_fraction: Fixed, // 0.15 (15% max HP)
    per_unit_cooldown_tick: u64, // tick when Stasis was last triggered for this unit
}
```

### 5.11 AuraType Extensions

Add new variants to `AuraType` enum in `components.rs`:

```
AuraType::AncientMoss,
AuraType::BogSong,
AuraType::UndyingPresence,
AuraType::MurkUplink,
```

---

## 6. Faction-Specific Systems

New systems to add in `crates/cc_sim/src/systems/`.

### 6.1 `water_affinity_system` (NEW)

**File**: `crates/cc_sim/src/systems/water_affinity_system.rs`

**Schedule position**: After movement_system (needs updated GridCell), before combat_system (needs updated StatModifiers).

**Logic**:
1. Query all units with `(UnitType, Owner, GridCell, &mut StatModifiers, &mut Health)`
2. For each unit, check if its faction is Croak (`FactionId::from_u8(owner.player_id)`)
3. Check if the unit's GridCell terrain is water (`map.terrain_at(grid_cell.pos).is_water()`)
4. If on water:
   - Apply `speed_multiplier *= 1.25`
   - Apply `damage_multiplier *= 1.15`
   - Apply +2 HP/s regeneration (0.2 HP per tick at 10hz) capped by regen stacking rules (8% max HP/s total)
   - Insert `WaterAffinityBuff` marker if not present
5. If not on water:
   - Remove `WaterAffinityBuff` marker if present
   - Reset water-specific modifiers

**Regeneration stacking**: Track total regen rate per unit per tick. Sum from all sources (Water Affinity, Ancient Moss, Bog Song, Bottomless, Grok Protocol). Cap at 8% max HP per second (0.8% per tick). This requires a `RegenAccumulator` transient computed each tick.

### 6.2 `limb_system` (NEW)

**File**: `crates/cc_sim/src/systems/limb_system.rs`

**Schedule position**: After water_affinity_system (needs water state), before combat_system.

**Logic**:
1. Query all units with `LimbTracker`
2. Each tick, if `current_limbs < max_limbs`, decrement `regen_ticks`
3. If `regen_ticks == 0`, increment `current_limbs`, reset `regen_ticks` (200 on land, 120 on water)
4. Apply Phantom Limb passive: set `attack_speed_multiplier` based on `(max_limbs - current_limbs) * 0.08`

### 6.3 `mucus_trail_system` (NEW)

**File**: `crates/cc_sim/src/systems/mucus_trail_system.rs`

**Schedule position**: After movement_system.

**Logic**:
1. Query all Ponderer units each tick
2. Record their current GridCell position
3. If the position changed since last tick, add a `TerrainOverlay` with `OverlayEffect::MucusTrail` at the old position, duration 300 ticks (30s)
4. The movement_system already reads overlays for speed modifiers -- extend it to check for MucusTrail overlays and apply speed bonus/penalty based on faction

### 6.4 `bog_patch_system` (NEW)

**File**: `crates/cc_sim/src/systems/bog_patch_system.rs`

**Schedule position**: After combat_system (Bog Mortar projectiles resolve there).

**Logic**:
1. When a Croaker's Bog Mortar projectile lands, create a `TerrainOverlay` with `OverlayEffect::WaterConvert` at the impact location, duration 150 ticks (15s)
2. Check adjacency to existing Bog Patches for Resonance Chain:
   - If within 2 tiles of another patch, mark both as "linked"
   - If 3+ patches are linked, trigger Bog Eruption (AoE damage at all linked patches, then consume them)
3. Track patch ownership per Croaker via `BogPatchCounter`

### 6.5 `spawnling_system` (NEW)

**File**: `crates/cc_sim/src/systems/spawnling_system.rs`

**Schedule position**: After production_system.

**Logic**:
1. Query all Broodmothers with `SpawnlingCounter`
2. Each tick, decrement `spawn_cooldown`
3. If cooldown hits 0 and `count < 4`, spawn a Spawnling entity (15 HP, minimal damage, 45s lifetime timer)
4. Track Spawnling death/expiry and decrement parent's counter

### 6.6 `devour_system` (NEW)

**File**: `crates/cc_sim/src/systems/devour_system.rs`

**Schedule position**: After combat_system.

**Logic**:
1. Query all Gulpers with `DevourState`
2. Each tick, apply digest damage to swallowed entity (10% max HP per second = 1% per tick)
3. Decrement `digest_ticks_remaining`
4. If ticks expire or Gulper dies, release swallowed unit at remaining HP
5. While digesting: set `cannot_attack: true`, `speed_multiplier *= 0.5` in StatModifiers
6. Manage temp shields on Gulper (absorb damage before HP, equal to 50% of swallowed unit's max HP)

### 6.7 `stasis_system` (NEW)

**File**: `crates/cc_sim/src/systems/stasis_system.rs`

**Schedule position**: After cleanup_system's Dead marking phase, before despawn phase.

**Logic**:
1. Query units marked `Dead` that have an active Murk Commander's `UndyingPresence` aura in range
2. Check per-unit Stasis cooldown (90s)
3. If eligible, remove `Dead` marker, insert `Stasis` component (20 ticks)
4. While in Stasis: invulnerable, untargetable
5. When Stasis expires: set HP to 15% max, remove Stasis component

---

## 7. Existing System Modifications

### 7.1 `command_system.rs` -- Water Building Placement

**File**: `crates/cc_sim/src/systems/command_system.rs`

**Current behavior** (line 306):
```rust
if !cc_core::terrain::is_passable_for_faction(terrain, cc_core::terrain::FactionId::CatGPT) {
    continue; // Can't build on impassable terrain
}
```

**Required change**: The build validation currently hardcodes `FactionId::CatGPT`. It needs to:
1. Resolve the builder's faction from `Owner.player_id` via `FactionId::from_u8()`
2. For Croak factions, additionally check: is this a Croak building kind (`TheGrotto`, `SpawningPools`, etc.)? If so, water tiles are valid build sites.
3. For non-Croak factions, behavior remains unchanged (water = impassable for building).

New logic:
```
let faction = FactionId::from_u8(owner.player_id).unwrap_or(FactionId::CatGPT);
let buildable = match terrain {
    TerrainType::Water => {
        faction == FactionId::Croak && is_croak_building(building_kind)
    },
    other => other.base_passable(),
};
if !buildable { continue; }
```

Add a helper `fn is_croak_building(kind: BuildingKind) -> bool` that returns true for all 8 Croak building kinds. Alternatively, add a `faction()` method to `BuildingKind` that returns the owning faction.

### 7.2 `production_system.rs` -- Croak Spawn-Time Components

**File**: `crates/cc_sim/src/systems/production_system.rs`

**Current pattern**: After spawning a unit, the system adds special components based on `UnitKind`:
- `Catnapper` -> `DreamSiegeTimer`
- `Chonk` -> `Aura(GravitationalChonk)` + `NineLivesTracker`

**New spawn-time additions for Croak units**:

```rust
// Regeneron: LimbTracker
if kind == UnitKind::Regeneron {
    entity_cmds.insert(LimbTracker {
        current_limbs: 4,
        max_limbs: 4,
        regen_ticks: 200,
    });
}

// Broodmother: SpawnlingCounter
if kind == UnitKind::Broodmother {
    entity_cmds.insert(SpawnlingCounter {
        count: 0,
        spawn_cooldown: 300, // 30s
    });
}

// Gulper: (no spawn-time component -- DevourState added when Devour is activated)

// Shellwarden: Aura(AncientMoss)
if kind == UnitKind::Shellwarden {
    entity_cmds.insert(Aura {
        aura_type: AuraType::AncientMoss,
        radius: Fixed::from_bits(3 << 16), // 3 tiles
        active: true,
    });
}

// Bogwhisper: Aura(BogSong)
if kind == UnitKind::Bogwhisper {
    entity_cmds.insert(Aura {
        aura_type: AuraType::BogSong,
        radius: Fixed::from_bits(5 << 16), // 5 tiles
        active: true,
    });
}

// MurkCommander: Aura(UndyingPresence) + Aura(MurkUplink)
// Note: may need multi-aura support or combine into single component with multiple effects
if kind == UnitKind::MurkCommander {
    entity_cmds.insert(Aura {
        aura_type: AuraType::UndyingPresence,
        radius: Fixed::from_bits(8 << 16), // 8 tiles
        active: true,
    });
}

// Croaker: BogPatchCounter
if kind == UnitKind::Croaker {
    entity_cmds.insert(BogPatchCounter {
        active_patches: Vec::new(),
    });
}
```

**Also**: The auto-gather logic for Ponderer needs the same Ambient Gathering behavior as Pawdler (auto-assign to nearest deposit on spawn), with the modification that Ponderer prefers water-adjacent deposits. Change the `kind == UnitKind::Pawdler` check to `kind == UnitKind::Pawdler || kind == UnitKind::Ponderer`.

### 7.3 `production_system.rs` -- SpawningPools Water Training Speed Bonus

When ticking the production queue for a SpawningPools building, check if any adjacent tile is water. If so, reduce remaining ticks by an extra 15% per tick (multiply tick decrement by 1.15, or equivalently reduce initial train_time by 15% at enqueue time).

Implementation: At enqueue time in `command_system.rs` (where `TrainUnit` command is processed), check if the building is `SpawningPools` and has adjacent water. If so, multiply `train_time` by 0.85 (as Fixed).

### 7.4 `production_system.rs` -- SporeTower Attack Stats

Following the `LaserPointer` pattern, add SporeTower's attack stats on construction completion:

```rust
if building.kind == BuildingKind::SporeTower {
    commands.entity(entity).insert((
        AttackStats {
            damage: Fixed::from_bits(5 << 16),  // 5 base damage (lower than LaserPointer)
            range: Fixed::from_bits(5 << 16),    // 5 range
            attack_speed: 20,                     // 2s between attacks
            cooldown_remaining: 0,
        },
        AttackTypeMarker {
            attack_type: AttackType::Ranged,
        },
    ));
}
```

SporeTower's Waterlogged application and DoT (2% max HP/s for 6s) is handled as a status effect applied on hit, integrated into the combat_system.

### 7.5 `production_system.rs` -- FossilStones Research Support

Following the `ScratchingPost` pattern:

```rust
if building.kind == BuildingKind::FossilStones {
    commands.entity(entity).insert((Researcher, ResearchQueue::default()));
}
```

### 7.6 `combat_system.rs` -- Hunker Damage Reflection

When a unit with `Hunkered` marker takes damage:
1. Apply 75% damage reduction (already handled via `StatModifiers.damage_reduction`)
2. Reflect 15% of the incoming damage back to the attacker

This requires adding a check in the damage application phase: if defender has `Hunkered`, queue an `ApplyDamageCommand` back to the attacker for `incoming_damage * 0.15`.

### 7.7 `combat_system.rs` -- Toxic Skin Passive

When a melee attacker hits an Eftsaber:
1. Apply 3% of the attacker's max HP as poison damage over 3s (stacks 5x)
2. Implemented as a status effect check after melee damage resolution

### 7.8 `target_acquisition_system.rs` -- Submerged/Stasis Filtering

Units with `Submerged` or `Stasis` components should be excluded from target acquisition queries (untargetable).

### 7.9 `movement_system.rs` -- Mucus Trail Speed Modification

When computing movement speed for a unit on a tile with a MucusTrail overlay:
- If ally of the trail owner: multiply speed by 1.10
- If enemy: multiply speed by 0.85

### 7.10 `movement_system.rs` -- Submerged Movement Constraints

Units with `Submerged` marker can only move on water tiles. Pathfinding for submerged units must use a water-only constraint (invert normal passability: only `is_water()` tiles are passable).

### 7.11 `cleanup_system.rs` -- Spawnling Lifetime

Spawnlings have a 450-tick (45s) lifetime. Decrement each tick. Mark `Dead` on expiry.

### 7.12 `resource_system.rs` -- Ambient Gathering Modification

Ponderer gathering speed modification:
- On land: `gather_speed_multiplier *= 0.60` (40% slower)
- On water: `gather_speed_multiplier *= 1.20` (20% faster than standard)
- Near LilyMarket on water: skip return trips (passive gather -- no `GatherState::ReturningToBase`)

### 7.13 `resource_system.rs` -- SunkenServer GPU Bonus

When calculating GPU income per tick, if a completed SunkenServer is present, multiply GPU generation by 1.10 (10% bonus).

### 7.14 `terrain.rs` -- New OverlayEffect Variant

Add to `OverlayEffect` enum:
```
MucusTrail {
    owner_player_id: u8,
}
```

Also add a corresponding dynamic flag bit:
```
pub const FLAG_MUCUS_TRAIL: u8 = 0b0000_0100;
```

---

## 8. AI FSM Updates

### 8.1 Replace Cat Unit Proxies with Croak Units

**File**: `crates/cc_sim/src/ai/fsm.rs`

**Current** (line 202-216):
```rust
Faction::Croak => AiPersonalityProfile {
    name: "Grok".into(),
    attack_threshold: 10,
    unit_preferences: vec![
        (UnitKind::Chonk, 5),     // proxy
        (UnitKind::Yowler, 3),    // proxy
        (UnitKind::Hisser, 2),    // proxy
    ],
    target_workers: 5,
    economy_priority: true,
    retreat_threshold: 60,
    eval_speed_mult: 1.2,
    chaos_factor: 10,
    leak_chance: 0,
},
```

**New**:
```rust
Faction::Croak => AiPersonalityProfile {
    name: "Grok".into(),
    attack_threshold: 10,
    unit_preferences: vec![
        (UnitKind::Shellwarden, 4),  // tank/anchor
        (UnitKind::Regeneron, 3),    // skirmisher
        (UnitKind::Croaker, 3),      // ranged/terrain creation
        (UnitKind::Broodmother, 2),  // support/healing
        (UnitKind::Gulper, 2),       // heavy bruiser
        (UnitKind::Leapfrog, 1),     // harasser
    ],
    target_workers: 5,
    economy_priority: true,
    retreat_threshold: 60,
    eval_speed_mult: 1.2,
    chaos_factor: 10,
    leak_chance: 0,
},
```

### 8.2 Building Kind Resolution

The AI FSM currently hardcodes cat building kinds (TheBox, CatTree, ServerRack, etc.). The FSM needs to resolve faction-appropriate buildings.

**Approach**: Add a `fn faction_buildings(faction: Faction) -> FactionBuildingSet` that maps generic building roles to faction-specific BuildingKinds:

```rust
pub struct FactionBuildingSet {
    pub hq: BuildingKind,
    pub barracks: BuildingKind,
    pub resource_depot: BuildingKind,
    pub tech: BuildingKind,
    pub research: BuildingKind,
    pub supply: BuildingKind,
    pub garrison: BuildingKind,
    pub defense: BuildingKind,
}
```

CatGpt returns `{TheBox, CatTree, FishMarket, ServerRack, ScratchingPost, LitterBox, CatFlap, LaserPointer}`.
Croak returns `{TheGrotto, SpawningPools, LilyMarket, SunkenServer, FossilStones, ReedBed, TidalGate, SporeTower}`.

All existing FSM references to `BuildingKind::TheBox`, `BuildingKind::CatTree`, etc. are replaced with `faction_buildings.hq`, `faction_buildings.barracks`, etc.

### 8.3 Worker Kind Resolution

The FSM references `UnitKind::Pawdler` directly for worker-related logic (counting workers, training workers, auto-gather). Add:

```rust
pub fn worker_kind(faction: Faction) -> UnitKind {
    match faction {
        Faction::CatGpt | Faction::Neutral => UnitKind::Pawdler,
        Faction::Croak => UnitKind::Ponderer,
        // Other factions TBD
        _ => UnitKind::Pawdler, // fallback
    }
}
```

### 8.4 Water-Aware Build Placement

The FSM's `find_build_position()` currently avoids water tiles. For Croak, it should:
1. Prefer water-adjacent positions for SpawningPools (training speed bonus)
2. Allow water tiles for all Croak buildings
3. Prefer water tiles for LilyMarket (Ambient Gathering synergy)

### 8.5 Grok-Specific Tactical Behaviors

Beyond basic FSM changes, Grok's personality should include:
- **Water retreat**: When retreating, prefer paths through water tiles (speed bonus + inaccessible to most factions)
- **Regen patience**: Higher retreat threshold (60%) reflects willingness to take damage knowing units will heal
- **Terrain creation priority**: In MidGame, prioritize having at least 1 Croaker for Bog Mortar terrain creation

---

## 9. Asset Pipeline Entries

Add to `tools/asset_pipeline/asset_catalog.yaml`:

### Unit Sprites (10)

| Asset ID | Description |
|----------|-------------|
| `croak_ponderer` | Axolotl worker, pink/purple, carrying supplies, gills visible |
| `croak_regeneron` | Axolotl skirmisher, missing limbs growing back, combat stance |
| `croak_broodmother` | Large axolotl, maternal, surrounded by tiny spawnlings |
| `croak_gulper` | Oversized axolotl, massive mouth, belly bulging |
| `croak_eftsaber` | Sleek newt, dark colors, toxic green accents |
| `croak_croaker` | Frog with inflatable vocal sac, artillery stance |
| `croak_leapfrog` | Small frog mid-leap, dynamic pose |
| `croak_shellwarden` | Ancient turtle, moss-covered shell, stoic |
| `croak_bogwhisper` | Frog on lily pad, mystical wisps, prophet-like |
| `croak_murkcommander` | Axolotl in diving suit, glowing visor, cables trailing |

### Building Sprites (8)

| Asset ID | Description |
|----------|-------------|
| `croak_grotto` | Mossy cave half-submerged, server humming inside |
| `croak_spawning_pools` | Shallow glowing pools with regenerative slime |
| `croak_lily_market` | Floating lily pads with stored food |
| `croak_sunken_server` | Submerged server rack, water cooling, cables |
| `croak_fossil_stones` | Ancient moss-covered standing stones |
| `croak_reed_bed` | Dense reeds, partially concealing |
| `croak_tidal_gate` | Stone gate structure with water channels |
| `croak_spore_tower` | Organic tower releasing spore clouds |

---

## 10. Testing Strategy

### 10.1 Unit Tests (`cc_core`)

| Test | File | Description |
|------|------|-------------|
| `croak_units_have_stats` | `unit_stats.rs` | All 10 Croak UnitKinds return valid UnitBaseStats |
| `croak_melee_range_one` | `unit_stats.rs` | Ponderer, Regeneron, Gulper, Eftsaber, Leapfrog, Shellwarden have range 1 |
| `croak_ranged_gt_one` | `unit_stats.rs` | Broodmother, Croaker, Bogwhisper, MurkCommander have range > 1 |
| `shellwarden_tankiest_croak` | `unit_stats.rs` | Shellwarden has most HP among non-hero Croak units |
| `murk_commander_strongest_croak` | `unit_stats.rs` | MurkCommander has most HP among all Croak units |
| `croak_buildings_have_stats` | `building_stats.rs` | All 8 Croak buildings return valid stats |
| `grotto_is_pre_built` | `building_stats.rs` | TheGrotto.build_time == 0, food_cost == 0, gpu_cost == 0 |
| `grotto_produces_ponderer` | `building_stats.rs` | TheGrotto.can_produce contains Ponderer |
| `spawning_pools_produces_basic` | `building_stats.rs` | SpawningPools produces Regeneron, Croaker, Leapfrog, Gulper |
| `sunken_server_produces_advanced` | `building_stats.rs` | SunkenServer produces Eftsaber, Broodmother, Shellwarden, Bogwhisper, MurkCommander |
| `reed_bed_provides_supply` | `building_stats.rs` | ReedBed.supply_provided == 10 |
| `croak_abilities_all_valid` | `abilities.rs` | All 30 Croak AbilityIds have valid AbilityDefs |
| `croak_unit_abilities_three_each` | `abilities.rs` | All 10 Croak units return exactly 3 distinct abilities |
| `croak_passive_no_cooldown` | `abilities.rs` | AmbientGathering, MucusTrail, PhantomLimb, ToxicSkin, BogMortar, ResonanceChain, Slipstream, AncientMoss, BogSong, Bottomless, UndyingPresence, MurkUplink have 0 cooldown |
| `croak_toggle_have_cooldown` | `abilities.rs` | Hunker has nonzero cooldown |
| `croak_activated_have_cooldown` | `abilities.rs` | LimbToss, RegrowthBurst, SpawnPool, Transfusion, PrimordialSoup, Devour, Regurgitate, Waterway, Venomstrike, Inflate, Hop, TongueLash, TidalMemory, MireCurse, Prophecy, GrokProtocol have nonzero cooldown |
| `unit_kind_display_from_str_round_trip` (extend) | `components.rs` | All 20 unit kinds round-trip |
| `building_kind_display_from_str_round_trip` (extend) | `components.rs` | All 16 building kinds round-trip |
| `croak_water_passability` | `terrain.rs` | All Croak-related water checks still pass |

### 10.2 Integration Tests (`cc_sim`)

| Test | Description |
|------|-------------|
| `croak_ponderer_spawns_from_grotto` | TheGrotto produces Ponderer with correct stats |
| `croak_ponderer_has_ambient_gathering` | Ponderer gathers 40% slower on land, 120% faster on water |
| `croak_water_affinity_speed_bonus` | Croak unit on water tile gets +25% speed |
| `croak_water_affinity_damage_bonus` | Croak unit on water tile gets +15% damage |
| `croak_water_affinity_regen` | Croak unit on water tile regenerates +2 HP/s |
| `croak_water_affinity_removed_on_land` | Water bonuses removed when unit leaves water |
| `croak_can_build_on_water` | Croak Ponderer can place buildings on water tiles |
| `non_croak_cannot_build_on_water` | Cat Pawdler still cannot build on water tiles |
| `limb_tracker_regeneration` | Regeneron limbs regen at 1/20s on land, 1/12s on water |
| `limb_toss_costs_limb` | LimbToss activation reduces current_limbs by 1 |
| `phantom_limb_attack_speed` | Regeneron at 0 limbs has +32% attack speed |
| `shellwarden_hunker_damage_reduction` | Hunkered Shellwarden takes 75% less damage |
| `shellwarden_hunker_reflects_damage` | Hunkered Shellwarden reflects 15% damage to attacker |
| `gulper_devour_removes_unit` | Devoured unit is removed from battlefield |
| `gulper_devour_releases_on_death` | If Gulper dies during Devour, swallowed unit is released |
| `eftsaber_submerge_untargetable` | Submerged Eftsaber cannot be targeted |
| `eftsaber_submerge_water_only` | Submerged Eftsaber can only move on water tiles |
| `mucus_trail_speed_bonuses` | Allied units on trail get +10% speed, enemies get -15% |
| `bog_mortar_creates_water_terrain` | Croaker's projectile impact creates Bog Patch (water terrain) |
| `resonance_chain_detonation` | 3+ connected Bog Patches trigger Bog Eruption AoE |
| `broodmother_spawns_spawnlings` | Broodmother creates Spawnling every 30s (max 4) |
| `transfusion_heals_ally` | Sacrificing Spawnling heals target 25% max HP over 5s |
| `stasis_prevents_death` | Unit in MurkCommander aura enters Stasis instead of dying |
| `stasis_revives_at_15_percent` | After Stasis expires, unit has 15% max HP |
| `stasis_90s_cooldown` | Stasis cannot trigger again on same unit for 90s |
| `regen_cap_8_percent` | Total regen from all sources capped at 8% max HP/s |
| `ai_grok_trains_croak_units` | Grok AI trains Croak units (not cat proxies) |
| `ai_grok_builds_croak_buildings` | Grok AI builds Croak buildings (not cat buildings) |
| `spawning_pools_water_adjacency_bonus` | SpawningPools adjacent to water trains 15% faster |
| `sunken_server_gpu_bonus` | SunkenServer generates 10% more GPU |
| `spore_tower_attacks` | SporeTower auto-attacks and applies Waterlogged |

### 10.3 Existing Test Updates

- `all_kinds_have_stats` (unit_stats.rs) -- extend array to include 10 Croak units
- `all_buildings_have_stats` (building_stats.rs) -- extend array to include 8 Croak buildings
- `all_ability_defs_valid` (abilities.rs) -- extend array to include 30 Croak abilities (60 total)
- `unit_abilities_returns_three_per_kind` (abilities.rs) -- extend to 20 kinds
- `unit_kind_display_from_str_round_trip` (components.rs) -- extend to 20 kinds
- `building_kind_display_from_str_round_trip` (components.rs) -- extend to 16 kinds
- `test_build_on_water_rejected` (integration.rs) -- should still pass for non-Croak factions; add a companion test for Croak success

---

## 11. Implementation Order

Implementation should proceed in layers, with each layer fully testable before moving to the next.

### Phase 1: Core Data (Estimated: 1-2 sessions)
Enums, stats, and ability definitions. No new systems.

- [ ] 1.1 Add 10 UnitKind variants to `components.rs` (enum + Display + FromStr)
- [ ] 1.2 Add 8 BuildingKind variants to `components.rs` (enum + Display + FromStr)
- [ ] 1.3 Add 30 AbilityId variants to `abilities.rs` (enum only)
- [ ] 1.4 Add 4 AuraType variants to `components.rs`
- [ ] 1.5 Implement `base_stats()` for all 10 Croak units in `unit_stats.rs`
- [ ] 1.6 Implement `building_stats()` for all 8 Croak buildings in `building_stats.rs`
- [ ] 1.7 Implement `ability_def()` for all 30 Croak abilities in `abilities.rs`
- [ ] 1.8 Implement `unit_abilities()` for all 10 Croak units in `abilities.rs`
- [ ] 1.9 Add Croak-specific UpgradeType variants (TougherHide, SlickerMucus, AmphibianAgility, SiegeEvolution, MurkPrototype)
- [ ] 1.10 Run all cc_core unit tests -- all 30+ new tests must pass
- [ ] 1.11 Update existing tests that enumerate unit/building/ability lists

### Phase 2: New Components (Estimated: 1 session)
ECS components needed by Croak systems.

- [ ] 2.1 Add `LimbTracker` component
- [ ] 2.2 Add `WaterAffinityBuff` marker component
- [ ] 2.3 Add `Waterlogged` status effect variant to StatusEffects
- [ ] 2.4 Add `MucusTrail` overlay variant to OverlayEffect + dynamic flag
- [ ] 2.5 Add `SpawnlingParent`, `SpawnlingCounter` components
- [ ] 2.6 Add `BogPatchCounter` component
- [ ] 2.7 Add `DevourState` component
- [ ] 2.8 Add `Hunkered` marker component
- [ ] 2.9 Add `Submerged` marker component
- [ ] 2.10 Add `Stasis` component

### Phase 3: System Modifications (Estimated: 2-3 sessions)
Modify existing systems to be faction-aware.

- [ ] 3.1 Fix command_system.rs build validation: faction-aware water building placement
- [ ] 3.2 Update production_system.rs: Croak spawn-time components (LimbTracker, SpawnlingCounter, Auras, BogPatchCounter)
- [ ] 3.3 Update production_system.rs: SporeTower attack stats, FossilStones researcher
- [ ] 3.4 Update target_acquisition_system.rs: filter Submerged and Stasis units
- [ ] 3.5 Update resource_system.rs: Ponderer gather speed modification (Ambient Gathering)
- [ ] 3.6 Update resource_system.rs: SunkenServer GPU bonus
- [ ] 3.7 Update combat_system.rs: Hunker damage reflection
- [ ] 3.8 Update combat_system.rs: Toxic Skin melee retaliation
- [ ] 3.9 Update cleanup_system.rs: Spawnling lifetime expiry
- [ ] 3.10 Run all integration tests

### Phase 4: New Systems (Estimated: 3-4 sessions)
New simulation systems for Croak mechanics.

- [ ] 4.1 Implement `water_affinity_system` (+25% speed, +15% damage, +2 HP/s on water)
- [ ] 4.2 Implement `limb_system` (limb regen + Phantom Limb passive)
- [ ] 4.3 Implement `mucus_trail_system` (trail creation + speed modifier overlays)
- [ ] 4.4 Implement `bog_patch_system` (Bog Mortar terrain creation + Resonance Chain)
- [ ] 4.5 Implement `spawnling_system` (Broodmother auto-spawn + lifetime management)
- [ ] 4.6 Implement `devour_system` (Gulper Devour/Regurgitate state machine)
- [ ] 4.7 Implement `stasis_system` (Murk Uplink death-save mechanic)
- [ ] 4.8 Wire all new systems into FixedUpdate schedule (correct ordering)
- [ ] 4.9 Implement regen accumulator with 8% max HP/s cap
- [ ] 4.10 Integration tests for all new systems

### Phase 5: AI FSM (Estimated: 1-2 sessions)
Make Grok use Croak units and buildings.

- [ ] 5.1 Replace cat unit proxies in Grok's AiPersonalityProfile
- [ ] 5.2 Add `FactionBuildingSet` abstraction + `faction_buildings()` function
- [ ] 5.3 Add `worker_kind()` function for faction-aware worker detection
- [ ] 5.4 Refactor FSM building references to use FactionBuildingSet
- [ ] 5.5 Add water-aware build position logic for Croak
- [ ] 5.6 Add water retreat preference for Grok
- [ ] 5.7 Test: AI Grok trains Croak units, builds Croak buildings

### Phase 6: Assets & Polish (Estimated: 1-2 sessions)

- [ ] 6.1 Add 18 entries to asset_catalog.yaml (10 units + 8 buildings)
- [ ] 6.2 Generate asset prompts using existing prompt templates
- [ ] 6.3 Verify all 211+ existing tests still pass (no regressions)
- [ ] 6.4 Run a full skirmish: Croak vs CatGpt AI, verify no panics
- [ ] 6.5 Update ARCHITECTURE.md with Croak faction details

### Phase 7: Advanced Ability Systems (Estimated: 3-5 sessions)
Complex ability implementations that build on Phase 4 systems.

- [ ] 7.1 Implement Eftsaber Waterway (submerge/surface state machine)
- [ ] 7.2 Implement Leapfrog Hop (tile leap + water cooldown reset)
- [ ] 7.3 Implement Leapfrog Tongue Lash (pull displacement)
- [ ] 7.4 Implement Croaker Inflate (temporary stat modification)
- [ ] 7.5 Implement Shellwarden Tidal Memory (5x5 flood zone overlay)
- [ ] 7.6 Implement Bogwhisper Mire Curse (cursor debuff + Bog Patch trail)
- [ ] 7.7 Implement Bogwhisper Prophecy (fog reveal + cooldown display)
- [ ] 7.8 Implement Broodmother Transfusion (Spawnling sacrifice heal)
- [ ] 7.9 Implement Broodmother Primordial Soup (3x3 water pool)
- [ ] 7.10 Implement Gulper Regurgitate (early Devour end + displacement)
- [ ] 7.11 Implement MurkCommander Grok Protocol (stat buff + regen)
- [ ] 7.12 Full integration tests for all ability interactions

---

## Appendix A: Croak Upgrade Types

New variants for `UpgradeType` enum:

| UpgradeType | Cat Equivalent | Effect | Food | GPU | Research Time |
|-------------|---------------|--------|------|-----|---------------|
| `TougherHide` | SharperClaws | +2 damage for all Croak combat units | 100 | 50 | 200 ticks |
| `ThickerMucus` | ThickerFur | +25 HP for all Croak combat units | 100 | 50 | 200 ticks |
| `AmphibianAgility` | NimblePaws | +10% speed for all Croak units | 75 | 75 | 150 ticks |
| `SiegeEvolution` | SiegeTraining | Unlocks Shellwarden training at SunkenServer | 150 | 100 | 300 ticks |
| `MurkPrototype` | MechPrototype | Unlocks MurkCommander training at SunkenServer | 200 | 200 | 400 ticks |

## Appendix B: Spawnling (Sub-Unit) Stats

Spawnlings are mini-units spawned by Broodmother. They are NOT full UnitKind variants -- they are lightweight entities with fixed stats, no abilities, and a lifetime timer.

| Stat | Value |
|------|-------|
| HP | 15 |
| Speed | 0.08 |
| Damage | 1 |
| Range | 1 |
| Attack Speed | 20 ticks |
| Attack Type | Melee |
| Lifetime | 450 ticks (45s) |
| Supply Cost | 0 (does not count against supply) |

Implementation: Spawn as regular entity with `UnitType { kind: UnitKind::Spawnling }` or as a separate `Spawnling` marker component without UnitKind. Recommendation: Add `Spawnling` as a UnitKind variant for consistency with queries and stat lookups, but with zero supply cost.

## Appendix C: System Schedule Order (Updated)

Current FixedUpdate chain:
```
tick -> commands -> target_acquisition -> combat -> projectile -> movement -> grid_sync -> cleanup
```

Updated with Croak systems:
```
tick -> commands -> water_affinity -> limb -> mucus_trail -> target_acquisition -> combat -> projectile -> devour -> movement -> grid_sync -> bog_patch -> spawnling -> stasis -> cleanup
```

Rationale:
- `water_affinity` before `target_acquisition`: stat modifiers must be current before combat decisions
- `limb` after `water_affinity`: limb regen rate depends on water state
- `mucus_trail` before `movement`: trail effects apply to this tick's movement
- `devour` after `combat`: Devour activation is triggered by combat damage bringing target below 30%
- `bog_patch` after `grid_sync`: needs finalized positions to check patch adjacency
- `spawnling` after `grid_sync`: new spawnlings need grid positions
- `stasis` before `cleanup`: intercepts Dead markers before despawn

## Appendix D: Determinism Considerations

All Croak systems must maintain deterministic lockstep simulation:
- All HP values, damage, speed modifiers use `Fixed` (FixedI32<U16>) -- no f32
- Regen accumulation uses Fixed math with explicit capping
- Spawnling spawn timing uses tick counts, not real time
- Bog Patch adjacency checks use grid coordinates (integer comparison)
- Limb regen uses tick counters, not timers
- Stasis revive uses Fixed HP calculations
