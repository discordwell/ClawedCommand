# Seekers of the Deep -- Full Faction Implementation Plan

> Faction identity: "Immovable object." Slow to deploy, terrifying once entrenched.
> AI agent: Deepseek -- 3x slower to respond, 30% more effective when it does.
> Coalition: Badgers, moles, wolverines.

This plan covers every code change required to make the Seekers of the Deep a fully playable faction, on par with the existing catGPT implementation. It is organized by subsystem, with each section listing the exact enum variants, stats, and components to add.

---

## Table of Contents

1. [UnitKind Variants](#1-unitkind-variants)
2. [Unit Base Stats](#2-unit-base-stats)
3. [BuildingKind Variants](#3-buildingkind-variants)
4. [Building Base Stats](#4-building-base-stats)
5. [AbilityId Variants](#5-abilityid-variants)
6. [Ability Definitions](#6-ability-definitions)
7. [unit_abilities() Mapping](#7-unit_abilities-mapping)
8. [New Components](#8-new-components)
9. [New Aura Types](#9-new-aura-types)
10. [Spawn-Time Components (production_system)](#10-spawn-time-components)
11. [Faction-Specific Mechanics -- New Systems](#11-faction-specific-mechanics--new-systems)
12. [Existing System Modifications](#12-existing-system-modifications)
13. [AI FSM Updates](#13-ai-fsm-updates)
14. [UpgradeType Variants](#14-upgradetype-variants)
15. [Terrain System Additions](#15-terrain-system-additions)
16. [Test Plan](#16-test-plan)
17. [Implementation Order](#17-implementation-order)
18. [Open Questions](#18-open-questions)

---

## 1. UnitKind Variants

Add 10 variants to `UnitKind` in `crates/cc_core/src/components.rs`:

```
Delver,         // Worker (Mole) -- gathers resources, builds, digs passages
Ironhide,       // Heavy Infantry (Badger) -- tanky melee, Shield Wall, Grudge Charge
Cragback,       // Siege Tank (Badger) -- long-range boulder mortar, Entrench toggle
Warden,         // Defensive Support (Badger) -- Vigilance Aura, Intercept, Rally Cry
Sapjaw,         // Anti-Armor Specialist (Badger) -- Armor Rend, Patient Strike, Lockjaw
Wardenmother,   // Hero/Heavy (Badger in Exosuit) -- faction hero, Deepseek Uplink
Tunneler,       // Utility/Transport (Mole) -- digs permanent tunnels, Undermine
Embermaw,       // Ranged Assault (Wolverine) -- incendiary launcher, area denial
Dustclaw,       // Skirmisher/Scout (Mole) -- Dust Cloud, Ambush Instinct, Sentry Burrow
Gutripper,      // Berserker/Shock (Wolverine) -- Frenzy, Bloodgreed, Reckless Lunge
```

Also update:
- `Display` impl (Debug-based, automatic)
- `FromStr` impl (add 10 match arms)
- Any exhaustive match in existing code (the `other => unimplemented!()` arms in `base_stats()` and `unit_abilities()` will catch these at runtime, but they must be replaced with real implementations)

---

## 2. Unit Base Stats

Add to `base_stats()` in `crates/cc_core/src/unit_stats.rs`. Design rationale follows GAME_DESIGN.md: Seekers are slower, tougher, and more expensive than catGPT equivalents. All values at 10hz tick rate.

### Stat Table

| UnitKind       | HP   | Speed | Damage | Range | AtkSpd (ticks) | AttackType | Food | GPU | Supply | Train (ticks) |
|----------------|------|-------|--------|-------|----------------|------------|------|-----|--------|---------------|
| Delver         | 50   | 0.10  | 3      | 1     | 18             | Melee      | 50   | 0   | 1      | 55            |
| Ironhide       | 250  | 0.08  | 16     | 1     | 18             | Melee      | 125  | 0   | 2      | 100           |
| Cragback       | 350  | 0.06  | 30     | 8     | 30             | Ranged     | 200  | 50  | 4      | 150           |
| Warden         | 150  | 0.10  | 8      | 3     | 15             | Ranged     | 100  | 25  | 2      | 80            |
| Sapjaw         | 120  | 0.12  | 18     | 1     | 12             | Melee      | 100  | 0   | 2      | 80            |
| Wardenmother   | 600  | 0.08  | 22     | 3     | 15             | Ranged     | 450  | 250 | 6      | 280           |
| Tunneler       | 80   | 0.14  | 6      | 1     | 20             | Melee      | 75   | 25  | 1      | 70            |
| Embermaw       | 90   | 0.10  | 16     | 6     | 15             | Ranged     | 125  | 25  | 2      | 90            |
| Dustclaw       | 70   | 0.16  | 12     | 1     | 10             | Melee      | 75   | 0   | 1      | 60            |
| Gutripper      | 160  | 0.12  | 20     | 1     | 8              | Melee      | 150  | 25  | 3      | 120           |

### Design Notes

- **Delver vs Pawdler**: Slightly less HP (50 vs 60), slightly slower (0.10 vs 0.12), less damage. Tradeoff: Subterranean Haul passive income makes fewer workers needed long-term.
- **Ironhide vs Chonk**: Less HP (250 vs 300) but faster damage (16@18 vs 12@20), cannot be displaced (Unbowed). More of an active frontliner than a pure absorber.
- **Cragback**: Slowest non-siege unit. Highest range in roster (8, or 12 entrenched). True artillery platform. 30 damage per boulder, 30 ticks between = 10 DPS, but AoE.
- **Wardenmother vs MechCommander**: More HP (600 vs 500), more expensive (450/250 vs 400/200). Reflects fortress doctrine -- the hero is a defensive anchor, not a mobile commander.
- **Gutripper**: The offensive anomaly. Fastest attack speed in roster (8 ticks) but moderate damage. Frenzy scales attack speed further. Glass cannon by Seekers standards.
- **Dustclaw**: Fastest Seekers unit (0.16). Still slower than catGPT's fastest (Nuisance 0.18, FlyingFox 0.225). Seekers' "fast" is other factions' "moderate."

### Fixed-Point Encoding

Speed values use `Fixed::from_bits(N)` where N = speed * 65536:
- 0.06 = 3932
- 0.08 = 5242
- 0.10 = 6553
- 0.12 = 7864
- 0.14 = 9175
- 0.16 = 10486

---

## 3. BuildingKind Variants

Add 8 variants to `BuildingKind` in `crates/cc_core/src/components.rs`:

```
TheSett,       // Command Center -- pre-built, produces Delver
WarHollow,     // Barracks -- trains Ironhide, Sapjaw, Warden, Gutripper
BurrowDepot,   // Resource Depot -- food storage, receives Subterranean Haul
CoreTap,       // Tech Building -- GPU processing, trains Tunneler, Embermaw, Dustclaw, Cragback, Wardenmother
ClawMarks,     // Research -- upgrades and tech unlocks
DeepWarren,    // Supply Depot -- increases supply cap
BulwarkGate,   // Defensive Gate -- garrison with attack, ejection on death
SlagThrower,   // Defense Tower -- AoE + Burning tile
```

Also update:
- `Display` impl
- `FromStr` impl (add 8 match arms)
- `building_stats()` match (currently panics on unknown kinds)

---

## 4. Building Base Stats

Add to `building_stats()` in `crates/cc_core/src/building_stats.rs`. Seekers buildings are generally tougher, slower to build, and more expensive than catGPT equivalents.

### Stat Table

| BuildingKind   | HP   | Build Time (ticks) | Food | GPU | Supply | Can Produce                                                  |
|----------------|------|--------------------|------|-----|--------|--------------------------------------------------------------|
| TheSett        | 600  | 0 (pre-built)      | 0    | 0   | 10     | [Delver]                                                     |
| WarHollow      | 400  | 180                | 175  | 0   | 0      | [Ironhide, Sapjaw, Warden, Gutripper]                       |
| BurrowDepot    | 250  | 120                | 100  | 0   | 0      | []                                                           |
| CoreTap        | 300  | 150                | 125  | 100 | 0      | [Tunneler, Embermaw, Dustclaw, Cragback, Wardenmother]       |
| ClawMarks      | 250  | 120                | 125  | 75  | 0      | []                                                           |
| DeepWarren     | 125  | 95                 | 80   | 0   | 12     | []                                                           |
| BulwarkGate    | 500  | 120                | 175  | 0   | 0      | []                                                           |
| SlagThrower    | 200  | 100                | 100  | 50  | 0      | []                                                           |

### Design Notes

- **TheSett**: +20% HP over The Box (600 vs 500). Same supply (10). Seekers' HQ is harder to kill.
- **WarHollow**: +100 HP over CatTree (400 vs 300), but 30 ticks slower to build (180 vs 150) and 25 Food more expensive.
- **BurrowDepot**: +50 HP over FishMarket, +20 ticks build time. Per GAME_DESIGN.md: stores 25% more Food (handled in economy system, not building_stats).
- **DeepWarren vs LitterBox**: +12 supply (vs 10), but +20 ticks build time (95 vs 75). Per GAME_DESIGN.md: 20% more supply, 25% longer to build.
- **BulwarkGate**: +100 HP over CatFlap (500 vs 400). Garrison mechanic has unique properties (attack from within, HP scaling, ejection on death).
- **SlagThrower vs LaserPointer**: +50 HP (200 vs 150), but slower fire rate. AoE + Burning tile creation on completion.

---

## 5. AbilityId Variants

Add 30 variants (3 per unit) to `AbilityId` in `crates/cc_core/src/abilities.rs`:

```
// Delver (worker)
SubterraneanHaul,
Earthsense,
EmergencyBurrow,

// Ironhide (heavy infantry)
Unbowed,
ShieldWall,
GrudgeCharge,

// Cragback (siege tank)
BoulderBarrage,
Entrench,
SeismicSlam,

// Warden (defensive support)
VigilanceAura,
Intercept,
RallyCry,

// Sapjaw (anti-armor)
ArmorRend,
PatientStrike,
Lockjaw,

// Wardenmother (hero)
DeepseekUplink,
FortressProtocol,
CalculatedCounterstrike,

// Tunneler (utility)
DeepBore,
Undermine,
TremorNetwork,

// Embermaw (ranged assault)
MoltenShot,
FuelReserve,
ScorchedEarth,

// Dustclaw (skirmisher)
DustCloud,
AmbushInstinct,
SentryBurrow,

// Gutripper (berserker)
Frenzy,
Bloodgreed,
RecklessLunge,
```

---

## 6. Ability Definitions

Add to `ability_def()` in `crates/cc_core/src/abilities.rs`. All cooldowns and durations in ticks (10hz = 10 ticks per second).

### Delver

| AbilityId          | Activation | Cooldown | GPU | Duration | Range | Charges |
|--------------------|------------|----------|-----|----------|-------|---------|
| SubterraneanHaul   | Activated  | 200      | 0   | 80       | 0     | 4       |
| Earthsense         | Passive    | 0        | 0   | 0        | 5     | 0       |
| EmergencyBurrow    | Activated  | 150      | 0   | 30       | 0     | 0       |

- SubterraneanHaul: 20s cooldown, 8s channel to dig passage. 4 max passages per depot.
- Earthsense: Always on, 5-tile tremorsense radius.
- EmergencyBurrow: 15s cooldown, 3s underground. Range 0 (self-only).

### Ironhide

| AbilityId    | Activation | Cooldown | GPU | Duration | Range | Charges |
|--------------|------------|----------|-----|----------|-------|---------|
| Unbowed      | Passive    | 0        | 0   | 0        | 0     | 0       |
| ShieldWall   | Activated  | 180      | 0   | 60       | 2     | 0       |
| GrudgeCharge | Activated  | 200      | 0   | 20       | 8     | 0       |

- Unbowed: Always on. No cooldown, no cost.
- ShieldWall: 18s cooldown, 6s duration. 2-tile radius for ally protection.
- GrudgeCharge: 20s cooldown, 2s windup (included in duration). 8-tile max charge range.

### Cragback

| AbilityId      | Activation | Cooldown | GPU | Duration | Range | Charges |
|----------------|------------|----------|-----|----------|-------|---------|
| BoulderBarrage | Passive    | 0        | 0   | 0        | 8     | 0       |
| Entrench       | Toggle     | 30       | 0   | 0        | 0     | 0       |
| SeismicSlam    | Activated  | 250      | 0   | 0        | 3     | 0       |

- BoulderBarrage: Primary attack modifier. Range 8 (or 12 when Entrenched -- handled in combat system).
- Entrench: Toggle with 3s cooldown (30 ticks) to prevent spam. 3s un-entrench animation.
- SeismicSlam: 25s cooldown. Range 3. Instant damage + Drowse + pushback. Cannot be used while Entrenched.

### Warden

| AbilityId     | Activation | Cooldown | GPU | Duration | Range | Charges |
|---------------|------------|----------|-----|----------|-------|---------|
| VigilanceAura | Passive    | 0        | 0   | 0        | 5     | 0       |
| Intercept     | Activated  | 160      | 4   | 30       | 6     | 0       |
| RallyCry      | Activated  | 220      | 0   | 50       | 6     | 0       |

- VigilanceAura: Always on. 5-tile radius. Tracks Intruder marks.
- Intercept: 16s cooldown, 4 GPU. 3s defensive stance (30 ticks). Max 6-tile sprint distance. Benefits from Deepseek Uplink delay reduction.
- RallyCry: 22s cooldown. 5s duration (50 ticks). 6-tile radius. CC immunity + speed buff.

### Sapjaw

| AbilityId     | Activation | Cooldown | GPU | Duration | Range | Charges |
|---------------|------------|----------|-----|----------|-------|---------|
| ArmorRend     | Passive    | 0        | 0   | 0        | 0     | 0       |
| PatientStrike | Passive    | 0        | 0   | 0        | 0     | 0       |
| Lockjaw       | Activated  | 200      | 0   | 30       | 1     | 0       |

- ArmorRend: Attack modifier, always on.
- PatientStrike: Conditional passive, always on. Triggers after 4s stationary.
- Lockjaw: 20s cooldown. 3s tether (30 ticks). Melee range.

### Wardenmother

| AbilityId               | Activation | Cooldown | GPU | Duration | Range | Charges |
|-------------------------|------------|----------|-----|----------|-------|---------|
| DeepseekUplink          | Passive    | 0        | 0   | 0        | 8     | 0       |
| FortressProtocol        | Activated  | 450      | 10  | 200      | 6     | 0       |
| CalculatedCounterstrike | Activated  | 300      | 6   | 80       | 4     | 0       |

- DeepseekUplink: Always on. 8-tile radius. Reduces Deepseek delay by 50%, GPU costs by 30%.
- FortressProtocol: 45s cooldown, 10 GPU. 20s duration (200 ticks). 6-tile radius zone.
- CalculatedCounterstrike: 30s cooldown, 6 GPU. 8s tracking window (80 ticks). 4-tile radius.

### Tunneler

| AbilityId     | Activation | Cooldown | GPU | Duration | Range | Charges |
|---------------|------------|----------|-----|----------|-------|---------|
| DeepBore      | Activated  | 250      | 5   | 0        | 15    | 3       |
| Undermine     | Activated  | 300      | 0   | 50       | 3     | 0       |
| TremorNetwork | Passive    | 0        | 0   | 0        | 8     | 0       |

- DeepBore: 25s cooldown, 5 GPU. Max 3 tunnel pairs. 15-tile max distance between endpoints.
- Undermine: 30s cooldown. 5s interruptible channel (50 ticks). 3-tile max range to target building.
- TremorNetwork: Always on. 8-tile range to extend Earthsense.

### Embermaw

| AbilityId    | Activation | Cooldown | GPU | Duration | Range | Charges |
|--------------|------------|----------|-----|----------|-------|---------|
| MoltenShot   | Passive    | 0        | 0   | 0        | 6     | 0       |
| FuelReserve  | Passive    | 0        | 0   | 0        | 0     | 3       |
| ScorchedEarth| Activated  | 250      | 0   | 0        | 4     | 0       |

- MoltenShot: Attack modifier. Creates Burning tile on hit.
- FuelReserve: Passive charge system. 3 max charges, 1 per 12s while stationary.
- ScorchedEarth: 25s cooldown. Cone attack (4 tiles long, 3 tiles wide). Consumes all Fuel Reserve charges.

### Dustclaw

| AbilityId      | Activation | Cooldown | GPU | Duration | Range | Charges |
|----------------|------------|----------|-----|----------|-------|---------|
| DustCloud      | Activated  | 140      | 0   | 50       | 3     | 0       |
| AmbushInstinct | Passive    | 0        | 0   | 0        | 0     | 0       |
| SentryBurrow   | Activated  | 80       | 0   | 0        | 0     | 0       |

- DustCloud: 14s cooldown. 5s duration (50 ticks). 3-tile radius.
- AmbushInstinct: Always on. 40% bonus damage from fog/stealth.
- SentryBurrow: 8s cooldown (after emerging). Toggle-like behavior but Activated.

### Gutripper

| AbilityId     | Activation | Cooldown | GPU | Duration | Range | Charges |
|---------------|------------|----------|-----|----------|-------|---------|
| Frenzy        | Passive    | 0        | 0   | 0        | 3     | 0       |
| Bloodgreed    | Passive    | 0        | 0   | 0        | 0     | 0       |
| RecklessLunge | Activated  | 150      | 0   | 30       | 4     | 0       |

- Frenzy: Always on. 3-tile detection radius. +5% atk speed per nearby enemy (max +40%).
- Bloodgreed: Always on. 20% lifesteal on damage dealt.
- RecklessLunge: 15s cooldown. 3s vulnerability window (30 ticks). 4-tile leap range.

---

## 7. unit_abilities() Mapping

Add to `unit_abilities()` in `crates/cc_core/src/abilities.rs`:

```
UnitKind::Delver => [AbilityId::SubterraneanHaul, AbilityId::Earthsense, AbilityId::EmergencyBurrow],
UnitKind::Ironhide => [AbilityId::Unbowed, AbilityId::ShieldWall, AbilityId::GrudgeCharge],
UnitKind::Cragback => [AbilityId::BoulderBarrage, AbilityId::Entrench, AbilityId::SeismicSlam],
UnitKind::Warden => [AbilityId::VigilanceAura, AbilityId::Intercept, AbilityId::RallyCry],
UnitKind::Sapjaw => [AbilityId::ArmorRend, AbilityId::PatientStrike, AbilityId::Lockjaw],
UnitKind::Wardenmother => [AbilityId::DeepseekUplink, AbilityId::FortressProtocol, AbilityId::CalculatedCounterstrike],
UnitKind::Tunneler => [AbilityId::DeepBore, AbilityId::Undermine, AbilityId::TremorNetwork],
UnitKind::Embermaw => [AbilityId::MoltenShot, AbilityId::FuelReserve, AbilityId::ScorchedEarth],
UnitKind::Dustclaw => [AbilityId::DustCloud, AbilityId::AmbushInstinct, AbilityId::SentryBurrow],
UnitKind::Gutripper => [AbilityId::Frenzy, AbilityId::Bloodgreed, AbilityId::RecklessLunge],
```

---

## 8. New Components

Add to `crates/cc_core/src/components.rs`. These are Seekers-specific runtime components.

### Dug In Passive

```
/// Tracks how long a unit has been stationary. All Seekers units gain Dug In after 50 ticks (5s).
pub struct StationaryTimer {
    pub ticks_stationary: u32,
    pub dug_in: bool,
}
```
- Cleared on any Move command.
- When `ticks_stationary >= 50`, set `dug_in = true`, apply +10% damage_reduction via StatModifiers.
- When unit moves, `dug_in` remains true for 20 ticks (2s grace period), then clears.

### Heavy Unit Tag

```
/// Marker: this unit crushes terrain obstacles when moving through them.
pub struct HeavyUnit;
```
- Applied at spawn to: Ironhide, Cragback, Wardenmother, Gutripper.
- Movement system checks for this tag and destroys TerrainObstacle entities on contact.

### Terrain Obstacle

```
/// Destructible terrain feature (bush, fence, light cover) that can be crushed by heavy units.
pub struct TerrainObstacle {
    pub hp: u32,
}
```
- Spawned by map generation for bushes, fences, etc.
- Heavy units destroy on contact. Other units pay normal terrain cost.

### Rubble Tile

```
/// A rubble pile left by Boulder Barrage. Costs 3x movement. Blocks LoS for ground units.
pub struct RubbleTile {
    pub remaining_ticks: u32,   // 120 ticks = 12s
    pub owner_player_id: u8,
}
```
- Similar pattern to existing HairballObstacle.
- Max 4 active per Cragback (tracked via entity count query).
- Destroyed by Seismic Slam.

### Burning Tile

```
/// A tile on fire from Embermaw or Slag Thrower. Deals %HP damage per tick to units on it.
pub struct BurningTile {
    pub remaining_ticks: u32,       // 60 = 6s, 150 = 15s (enhanced)
    pub owner_player_id: u8,
    pub damage_per_tick: Fixed,     // 1% max HP / 10 ticks = 0.001 per tick for standard
    pub grants_vision: bool,        // Seekers see through Burning tiles
}
```
- Created by MoltenShot (on projectile impact), ScorchedEarth (cone), and SlagThrower (tower shots).
- Does NOT stack burn damage on same tile; refresh duration only.

### Subterranean Haul Passage

```
/// A permanent underground resource delivery route.
pub struct ResourcePassage {
    pub source_deposit: EntityId,
    pub depot_entity: EntityId,
    pub delivery_rate: Fixed,       // resources per tick (0.08 = 80% of Delver gather rate / 10hz)
    pub resource_type: ResourceType,
}
```
- Resource entity (not a component on building). Ticked in resource_system.
- Max 4 per BurrowDepot.
- Building a passage costs 50 Food and takes 80 ticks (8s) of Delver channeling.

### Deep Bore Tunnel

```
/// A permanent one-way underground tunnel. Two entities linked.
pub struct DeepBoreTunnel {
    pub entrance: EntityId,
    pub exit: EntityId,
    pub reinforced: bool,           // takes 2 hits to destroy
    pub hidden: bool,               // invisible unless scouted
    pub owner_player_id: u8,
}
```
- Entrance and exit are separate entities with Position.
- Units entering travel for 2s (20 ticks) before emerging.
- Max 3 active per Tunneler (tracked via entity count).

### Earthsense Blip

```
/// A tremorsense detection event -- position known, unit type optionally known.
pub struct TremorsenseBlip {
    pub position: GridPos,
    pub unit_type: Option<UnitKind>,    // None if only 1 Delver sensing
    pub owner_player_id: u8,
    pub remaining_ticks: u32,
}
```
- Generated by Delver/Tunneler Earthsense each tick for nearby enemy units.
- If 2+ Delvers sense the same unit, `unit_type` is populated (triangulation).
- Displayed as minimap ripple, separate from fog of war.

### Intruder Mark

```
/// Marks an enemy unit as an Intruder by the Warden's Vigilance Aura.
pub struct IntruderMark {
    pub remaining_ticks: u32,       // 80 ticks = 8s
}
```
- Applied when an enemy enters any Warden's Vigilance Aura radius.
- Shared across all Wardens: once marked, every Warden's aura grants +15% damage to allies attacking this target.

### Armor Rend Stacks

```
/// Tracks Sapjaw Armor Rend debuff stacks on a target.
pub struct ArmorRendStacks {
    pub stacks: u32,                // max 5
    pub decay_timer: u32,           // ticks until next stack decays (100 = 10s)
}
```
- Each stack = -8% damage reduction on the target.
- Max 5 stacks = -40% damage reduction.
- Stacks decay one at a time every 10s. Timer resets on new stack application.

### Lockjaw Tether

```
/// Tether pair: Sapjaw is latched onto a target. Both are movement-locked.
pub struct LockjawTether {
    pub attacker: EntityId,
    pub target: EntityId,
    pub remaining_ticks: u32,       // 30 ticks = 3s
}
```
- Applied as components on BOTH the Sapjaw and the target.
- Movement system skips both entities while tether is active.
- Both take 50% reduced AoE damage.
- Applies 1 ArmorRend stack per 10 ticks (1/s).

### Frenzy Stacks

```
/// Tracks Gutripper's Frenzy passive -- gains attack speed per nearby enemy.
pub struct FrenzyStacks {
    pub current_stacks: u32,        // max 8
    pub frozen_until_tick: u64,     // stacks don't decay until this game tick (Bloodgreed kill proc)
}
```
- Updated by spatial query each tick.
- +5% attack speed per stack (multiplicative via StatModifiers).
- At 5+ stacks: +15% move speed.

### Bloodgreed Tracker

```
/// Tracks Gutripper's Bloodgreed lifesteal and kill proc state.
pub struct BloodgreedTracker {
    pub lifesteal_fraction: Fixed,  // 0.20 = 20% of damage dealt
}
```
- Lifesteal applied in combat system after damage calculation.
- Kill proc: heal 15% max HP + freeze Frenzy stacks for 50 ticks.

### Shield Wall State

```
/// Ironhide is in Shield Wall stance -- directional damage reduction.
pub struct ShieldWallActive {
    pub facing_angle: Fixed,        // radians, direction the shield faces
    pub remaining_ticks: u32,       // 60 ticks = 6s
}
```
- Damage from the front 180-degree arc: 50% reduction on Ironhide, 25% on allies behind (within 2 tiles).
- Ironhide cannot move or turn, but can attack melee targets in range.

### Entrench State

```
/// Cragback is entrenched -- boosted range and fire rate, immobile.
pub struct Entrenched;
```
- Simple marker component. When present:
  - BoulderBarrage range: 8 -> 12
  - Attack speed: 30 -> 20 ticks (2s between shots)
  - +30% damage reduction
  - Cannot move
  - Can fire over obstacles/units

### Fortress Protocol Zone

```
/// Active Fortress Protocol zone centered on Wardenmother.
pub struct FortressZone {
    pub center: WorldPos,
    pub radius: Fixed,              // 6 tiles
    pub remaining_ticks: u32,       // 200 ticks = 20s
    pub owner_player_id: u8,
}
```
- Spawned as a separate entity (zone marker).
- Grants: +20% DR, +10% attack speed to allies inside.
- Reveals stealth and fog for enemies inside.

### Counterstrike Accumulator

```
/// Tracks damage absorbed within Calculated Counterstrike area.
pub struct CounterstrikeAccumulator {
    pub center: WorldPos,
    pub radius: Fixed,              // 4 tiles
    pub damage_absorbed: Fixed,     // total damage taken by allies in area
    pub remaining_ticks: u32,       // 80 ticks = 8s
    pub owner_player_id: u8,
}
```
- On expiry (or manual early trigger): each allied unit in area deals a retaliatory strike to nearest enemy. Bonus damage = 30% of `damage_absorbed` / number of allied units in area.

### Dust Cloud Zone

```
/// Active Dust Cloud at a position -- reduces vision, causes ranged miss chance.
pub struct DustCloudZone {
    pub center: WorldPos,
    pub radius: Fixed,              // 3 tiles
    pub remaining_ticks: u32,       // 50 ticks = 5s
    pub owner_player_id: u8,
}
```
- Units inside: -50% vision range.
- Ranged attacks passing through: 40% miss chance.
- Dustclaw is immune to vision reduction (checked by unit kind).

### Sentry Burrow State

```
/// Dustclaw is burrowed as a sentry -- stealthed, immobile, has Earthsense.
pub struct SentryBurrowed;
```
- Marker component. When present: stealthed, immobile, Earthsense 5-tile radius.
- On emerge: first attack triggers Ambush Instinct.
- If enemy steps on tile: auto-reveal + free Ambush attack.

### Deepseek Processing Delay

```
/// Pending AI command that is being "processed" by Deepseek before execution.
pub struct PendingDeepseekCommand {
    pub command: GameCommand,
    pub delay_remaining: u32,       // ticks until command executes
}
```
- Resource-level storage (Vec of these), not a component.
- Deepseek commands take 3x the normal AI eval interval to process.
- Within Wardenmother's Deepseek Uplink radius: delay halved.

---

## 9. New Aura Types

Add to `AuraType` enum in `crates/cc_core/src/components.rs`:

```
VigilanceAura,          // Warden: +15% damage vs Intruders
DeepseekUplink,         // Wardenmother: AI speed/cost reduction
FortressProtocol,       // Wardenmother: zone DR + attack speed
Frenzy,                 // Gutripper: attack speed per nearby enemy (technically not an "aura" applied to allies, but uses the spatial query system)
```

Note: Some existing aura types (GeppityUplink) already have similar patterns. VigilanceAura and DeepseekUplink follow the same Aura component pattern with `aura_type`, `radius`, and `active` fields.

---

## 10. Spawn-Time Components

Update `production_system()` in `crates/cc_sim/src/systems/production_system.rs` to attach faction-specific components at unit spawn time, following the existing pattern for Catnapper/Chonk.

### Components by UnitKind

| UnitKind     | Spawn-Time Components                                                               |
|--------------|--------------------------------------------------------------------------------------|
| Delver       | StationaryTimer::default()                                                           |
| Ironhide     | HeavyUnit, StationaryTimer::default()                                                |
| Cragback     | HeavyUnit, StationaryTimer::default()                                                |
| Warden       | Aura { aura_type: VigilanceAura, radius: 5, active: true }, StationaryTimer::default() |
| Sapjaw       | StationaryTimer::default()                                                           |
| Wardenmother | HeavyUnit, Aura { aura_type: DeepseekUplink, radius: 8, active: true }, StationaryTimer::default() |
| Tunneler     | StationaryTimer::default()                                                           |
| Embermaw     | FuelReserveCharges { charges: 0, regen_timer: 0 }, StationaryTimer::default()        |
| Dustclaw     | StationaryTimer::default()                                                           |
| Gutripper    | HeavyUnit, FrenzyStacks::default(), BloodgreedTracker { lifesteal_fraction: 0.20 }, StationaryTimer::default() |

### Auto-Gather for Delver

The existing Pawdler auto-gather logic (send newly spawned workers to nearest deposit) should also apply to Delver. Check `kind == UnitKind::Delver` in the same branch as `kind == UnitKind::Pawdler`:

```
if kind == UnitKind::Pawdler || kind == UnitKind::Delver {
    // existing auto-gather code
}
```

### SlagThrower Completion

Add a completion branch for SlagThrower, parallel to LaserPointer:

```
if building.kind == BuildingKind::SlagThrower {
    // AoE ranged attack, slower than LaserPointer
    entity_cmds.insert((
        AttackStats {
            damage: Fixed::from_bits(15 << 16),  // 15 damage
            range: Fixed::from_bits(7 << 16),    // 7 range
            attack_speed: 30,                     // 3s between attacks
            cooldown_remaining: 0,
        },
        AttackTypeMarker { attack_type: AttackType::Ranged },
    ));
}
```

### ClawMarks Completion

Add Researcher + ResearchQueue on ClawMarks completion, parallel to ScratchingPost:

```
if building.kind == BuildingKind::ClawMarks {
    entity_cmds.insert((Researcher, ResearchQueue::default()));
}
```

---

## 11. Faction-Specific Mechanics -- New Systems

These require new Bevy systems added to the simulation schedule.

### 11.1 Dug In System

**File**: `crates/cc_sim/src/systems/dug_in_system.rs` (new)

**Purpose**: Track unit movement and apply/remove Dug In status.

**Logic**:
1. Query all entities with `StationaryTimer` + `Position`.
2. If position changed since last tick: reset timer, start 20-tick grace period if was dug_in.
3. If position unchanged: increment timer.
4. If timer >= 50 ticks: set dug_in = true.
5. Apply/remove +10% damage_reduction to StatModifiers.

**Schedule position**: After movement system, before combat system.

### 11.2 Burning Tile System

**File**: `crates/cc_sim/src/systems/burning_tile_system.rs` (new)

**Purpose**: Tick Burning tiles, apply damage to units standing on them, grant vision.

**Logic**:
1. Tick down `remaining_ticks` on all BurningTile entities.
2. Despawn expired tiles.
3. For each active tile: find all units on that tile via GridCell.
4. Apply damage (damage_per_tick * unit max HP) to each unit.
5. If `grants_vision`: add tile position to Seekers' vision overlay.

**Schedule position**: After movement system (units may have moved onto/off tiles).

### 11.3 Rubble Tile System

**File**: `crates/cc_sim/src/systems/rubble_tile_system.rs` (new)

**Purpose**: Tick Rubble tiles, apply movement cost penalty.

**Logic**:
1. Tick down `remaining_ticks` on all RubbleTile entities.
2. Despawn expired tiles.
3. Pathfinding integration: RubbleTile positions added to a dynamic cost overlay on the pathfinding grid with 3x multiplier.
4. LoS check: ground units cannot see through Rubble tiles (treated as elevation blocker for LoS ray).

**Schedule position**: Before movement system (affects pathfinding costs).

### 11.4 Terrain Crushing System

**File**: `crates/cc_sim/src/systems/terrain_crush_system.rs` (new)

**Purpose**: Heavy units destroy terrain obstacles when moving through them.

**Logic**:
1. Query all entities with `HeavyUnit` + `GridCell`.
2. Check if the grid cell contains a `TerrainObstacle`.
3. If yes: despawn the obstacle, clear the terrain to passable.
4. Log/event for enemy awareness (observant enemies see cleared paths).

**Schedule position**: After movement system, before grid_sync.

### 11.5 Earthsense System

**File**: `crates/cc_sim/src/systems/earthsense_system.rs` (new)

**Purpose**: Generate TremorsenseBlip events from Delver/Tunneler Earthsense.

**Logic**:
1. Query all Delver + Tunneler entities with Earthsense.
2. For each: find all enemy units within Earthsense radius (5 tiles, or 10 if in EmergencyBurrow).
3. Generate blip: position known, unit type unknown.
4. Cross-reference: if 2+ Delvers sense the same enemy, populate unit type (triangulation).
5. Blips are per-player -- only visible to the Seekers player.

**Schedule position**: After movement system (uses current positions).

### 11.6 Resource Passage System

**File**: Integration into existing `resource_system.rs`

**Purpose**: Tick Subterranean Haul passages for passive resource delivery.

**Logic**:
1. Query all ResourcePassage entities.
2. Each tick: add `delivery_rate` of the passage's resource type to the owning player's stockpile.
3. Deplete the source deposit by the same amount.
4. If deposit exhausted: passage stops delivering (but remains, can be reassigned if deposit replenishes -- probably never).

**Schedule position**: Within existing resource_system tick.

### 11.7 Lockjaw System

**File**: `crates/cc_sim/src/systems/lockjaw_system.rs` (new)

**Purpose**: Manage Lockjaw tethers between Sapjaw and target.

**Logic**:
1. Query all LockjawTether components.
2. Tick down `remaining_ticks`.
3. While active: override movement for both entities (force same position), apply 50% AoE damage reduction.
4. Each 10 ticks: apply 1 ArmorRend stack to target.
5. On expiry: remove tether from both entities, apply 1s CC immunity to target.

**Schedule position**: Before movement system (prevents movement), after target_acquisition.

### 11.8 Frenzy System

**File**: `crates/cc_sim/src/systems/frenzy_system.rs` (new)

**Purpose**: Update Gutripper Frenzy stacks based on nearby enemies.

**Logic**:
1. Query all entities with `FrenzyStacks` + `Position`.
2. Use spatial hash to count enemy units within 3 tiles.
3. Set `current_stacks = min(enemy_count, 8)`.
4. Check frozen_until_tick -- if frozen, don't decay stacks.
5. Apply +5% attack_speed_multiplier per stack, +15% speed_multiplier at 5+ stacks via StatModifiers.

**Schedule position**: Before combat system (affects attack speed).

### 11.9 Deepseek Command Delay System

**File**: `crates/cc_sim/src/systems/deepseek_delay_system.rs` (new)

**Purpose**: Buffer and delay-execute AI commands for Deepseek.

**Logic**:
1. When Deepseek AI issues a command, instead of immediate execution, push to `PendingDeepseekCommand` buffer.
2. Calculate delay: base = 3x normal eval interval. Check if target unit is within any Wardenmother's Deepseek Uplink radius -- if yes, delay halved.
3. Each tick: decrement `delay_remaining` on all pending commands.
4. When delay reaches 0: execute the command via normal CommandQueue.

**Schedule position**: Before command processing system.

### 11.10 Fortress Protocol System

**File**: `crates/cc_sim/src/systems/fortress_protocol_system.rs` (new)

**Purpose**: Manage active Fortress Zone entities.

**Logic**:
1. Query all FortressZone entities.
2. Tick down `remaining_ticks`. Despawn on expiry.
3. For each active zone: find all allied units within radius.
4. Apply +20% DR and +10% attack speed via StatModifiers.
5. Reveal stealth/fog for enemies within zone.

**Schedule position**: Before combat system (affects stats).

### 11.11 Counterstrike System

**File**: `crates/cc_sim/src/systems/counterstrike_system.rs` (new)

**Purpose**: Accumulate damage and trigger retaliatory burst.

**Logic**:
1. Query all CounterstrikeAccumulator entities.
2. When an allied unit within the accumulator's area takes damage: add to `damage_absorbed`.
3. Tick down `remaining_ticks`.
4. On expiry (or manual trigger): query all allied units in area. Each deals retaliatory strike to nearest enemy. Bonus damage = `damage_absorbed * 0.30 / allied_count`.
5. Despawn accumulator.

**Schedule position**: Hooks into damage application (needs to observe damage events).

---

## 12. Existing System Modifications

### 12.1 Combat System (`crates/cc_sim/src/systems/combat.rs`)

Changes needed:
- **BoulderBarrage AoE**: When Cragback's ranged attack hits, apply damage in 2-tile radius and spawn RubbleTile.
- **Entrench range modifier**: Check for `Entrenched` component on Cragback; if present, range = 12 instead of 8.
- **MoltenShot fire tile**: When Embermaw's ranged attack hits, spawn BurningTile at impact position.
- **ArmorRend application**: On Sapjaw attack hit, apply/increment ArmorRendStacks on target.
- **PatientStrike check**: On Sapjaw attack, check if `StationaryTimer.ticks_stationary >= 40` (4s). If yes, apply 2.5x damage and 2 ArmorRend stacks.
- **Bloodgreed lifesteal**: After Gutripper deals damage, heal for 20% of damage dealt.
- **Shield Wall directional DR**: On damage application, check if target has ShieldWallActive and if damage source is in the front arc.
- **Unbowed displacement immunity**: Skip any knockback/pull effects if target has Ironhide Unbowed passive.
- **Unbowed bonus damage**: Track attackers on Ironhide. +15% damage against units that hit it in last 50 ticks (5s).

### 12.2 Movement System (`crates/cc_sim/src/systems/movement.rs`)

Changes needed:
- **Rubble tile cost**: Check dynamic cost overlay for RubbleTile positions. Apply 3x movement cost.
- **Burning tile cost (AI only)**: AI-controlled units should path around Burning tiles when possible (treat as increased cost).
- **Lockjaw freeze**: Skip movement for entities with LockjawTether.
- **Entrench freeze**: Skip movement for entities with Entrenched.
- **ShieldWall freeze**: Skip movement for entities with ShieldWallActive.
- **StationaryTimer update**: Movement system should set a `moved_this_tick` flag or clear StationaryTimer when position changes.

### 12.3 Target Acquisition (`crates/cc_sim/src/systems/target_acquisition.rs`)

Changes needed:
- **Intruder mark priority**: When a Seekers unit is selecting targets, prioritize units with IntruderMark (+15% damage bonus makes them higher-value targets).
- **Dust Cloud miss chance**: Ranged attacks through DustCloudZone have 40% miss chance. Need LoS check against Dust Cloud positions.

### 12.4 Pathfinding (`crates/cc_core/src/pathfinding.rs` or similar)

Changes needed:
- **Dynamic cost overlay**: Support a HashMap<GridPos, Fixed> for temporary cost modifiers (Rubble, Burning tiles). Merge with static terrain costs during A*.
- **Tunnel portal edges**: Support zero-cost (or fixed 20-tick) portal edges for Deep Bore tunnels. A* graph needs optional portal edge list.
- **Terrain obstacle passability**: TerrainObstacle tiles are impassable for non-heavy units but passable (auto-destroyed) for heavy units.

### 12.5 Cleanup System (`crates/cc_sim/src/systems/cleanup.rs`)

Changes needed:
- **Despawn expired components**: RubbleTile, BurningTile, DustCloudZone, FortressZone, CounterstrikeAccumulator, ShieldWallActive, LockjawTether.
- **Kill proc for Bloodgreed**: On unit death, check if killer was a Gutripper. If yes, heal 15% max HP and freeze Frenzy stacks.

### 12.6 Production System (`crates/cc_sim/src/systems/production_system.rs`)

Changes covered in section 10 above (spawn-time components, auto-gather, SlagThrower, ClawMarks).

### 12.7 Fog of War (if exists)

Changes needed:
- **Burning tile vision**: Seekers player gains vision of all BurningTile positions.
- **Earthsense layer**: Separate from fog of war. Tremorsense blips appear on minimap without granting full tile vision.
- **Fortress Protocol reveal**: Enemies inside FortressZone are revealed through stealth/fog.
- **Sentry Burrow stealth**: Burrowed Dustclaws are stealthed.

---

## 13. AI FSM Updates

### 13.1 Replace Cat Unit Proxies

In `crates/cc_sim/src/ai/fsm.rs`, the `faction_personality(Faction::SeekersOfTheDeep)` profile currently uses cat unit proxies:

```rust
// CURRENT (proxy):
unit_preferences: vec![
    (UnitKind::Chonk, 4),       // proxy for Ironhide
    (UnitKind::Hisser, 3),      // proxy for Embermaw
    (UnitKind::Catnapper, 2),   // proxy for Cragback
],
```

Replace with actual Seekers units:

```rust
// UPDATED:
unit_preferences: vec![
    (UnitKind::Ironhide, 4),
    (UnitKind::Embermaw, 3),
    (UnitKind::Cragback, 2),
    (UnitKind::Warden, 2),
    (UnitKind::Sapjaw, 1),
],
```

### 13.2 Worker Detection

The FSM's `eval_early_game` and economy logic identifies workers by `UnitKind::Pawdler`. Add `UnitKind::Delver` as an equivalent check for Seekers:

```rust
fn is_worker(kind: UnitKind) -> bool {
    matches!(kind, UnitKind::Pawdler | UnitKind::Delver | ...)
}
```

### 13.3 Building Selection

The FSM selects buildings to construct by kind. For Seekers AI:
- Instead of `BuildingKind::CatTree`, use `BuildingKind::WarHollow`
- Instead of `BuildingKind::FishMarket`, use `BuildingKind::BurrowDepot`
- Instead of `BuildingKind::ServerRack`, use `BuildingKind::CoreTap`
- Instead of `BuildingKind::LitterBox`, use `BuildingKind::DeepWarren`
- Instead of `BuildingKind::ScratchingPost`, use `BuildingKind::ClawMarks`
- Instead of `BuildingKind::CatFlap`, use `BuildingKind::BulwarkGate`
- Instead of `BuildingKind::LaserPointer`, use `BuildingKind::SlagThrower`
- Instead of `BuildingKind::TheBox`, use `BuildingKind::TheSett`

This requires either:
1. A `faction_building_equivalent(BuildingKind, Faction) -> BuildingKind` lookup, or
2. The FSM querying buildings by role rather than by specific kind

Option 2 is cleaner long-term. Add a `BuildingRole` enum:
```
enum BuildingRole { HQ, Barracks, ResourceDepot, TechBuilding, Research, SupplyDepot, DefensiveGate, DefenseTower }
```
And a mapping function:
```
fn building_for_role(role: BuildingRole, faction: Faction) -> BuildingKind
```

### 13.4 Deepseek Personality Tweaks

Beyond unit preferences, update the Deepseek profile to reflect faction character:
- `eval_speed_mult: 3.0` (not 1.5 -- full 3x slowdown per design doc)
- `retreat_threshold: 50` (already correct -- Seekers hold positions)
- `chaos_factor: 0` (already correct -- Deepseek never makes random mistakes)
- Add `economy_priority: true` (Seekers need time to set up economy with Subterranean Haul)
- Consider new field: `command_effectiveness: 1.3` (30% more effective commands)

---

## 14. UpgradeType Variants

Add Seekers-equivalent upgrades to `UpgradeType` in `crates/cc_core/src/components.rs`:

```
// Seekers of the Deep upgrades (researched at ClawMarks)
SharperFangs,       // +2 damage for all Seekers combat units
ThickerHide,        // +30 HP for all Seekers combat units (more than cat's +25, Seekers are tougher)
SteadyStance,       // +10% speed for all Seekers units
SiegeEngineering,   // Unlocks Cragback training at CoreTap
ExosuitPrototype,   // Unlocks Wardenmother training at CoreTap
```

Update `research_system.rs` to recognize these and apply appropriate stat bonuses. The existing `apply_upgrades_to_new_unit()` function should handle them with the same pattern as SharperClaws/ThickerFur/NimblePaws.

---

## 15. Terrain System Additions

### 15.1 New Dynamic Terrain Types

The existing `TerrainType` enum (10 types) covers static terrain. The Seekers introduce dynamic terrain overlays:

- **Rubble**: Movement cost 3x. Blocks ground-level LoS. Created by BoulderBarrage, destroyed by SeismicSlam. 12s duration.
- **Burning**: Movement cost 1.5x (AI pathfinding only). Deals DoT. Grants vision to Seekers. Created by MoltenShot, ScorchedEarth, SlagThrower. 6s/15s duration.
- **Destroyed Obstacle**: Permanently cleared tile where a TerrainObstacle was crushed by a heavy unit. No movement cost penalty. Reveals that heavy units passed through.

These are handled as entity-based overlays (RubbleTile, BurningTile components), NOT as TerrainType enum variants. The pathfinding system queries these entities to build the dynamic cost overlay.

### 15.2 Terrain Obstacle Layer

Introduce a sparse set of destructible terrain features on the map:
- Bushes (Light cover tiles in Forest terrain)
- Fences (artificial obstacles near tech ruins or roads)
- Light barriers (on map edges)

These exist as entities with `TerrainObstacle { hp }` + `Position` + `GridCell`. Heavy units auto-destroy them. This requires map generation changes to place obstacle entities.

---

## 16. Test Plan

Following the existing test patterns (211+ passing tests), add:

### 16.1 cc_core Tests

- `unit_stats::all_seekers_kinds_have_stats` -- iterate all 10 Seekers UnitKind variants, assert positive HP/speed/damage/range/attack_speed.
- `unit_stats::seekers_melee_units_have_range_one` -- Delver, Ironhide, Sapjaw, Tunneler, Dustclaw, Gutripper.
- `unit_stats::seekers_ranged_units_have_range_gt_one` -- Cragback, Warden, Embermaw, Wardenmother.
- `unit_stats::ironhide_is_seekers_tankiest_non_hero` -- Ironhide HP > all non-Wardenmother Seekers.
- `unit_stats::wardenmother_is_seekers_strongest` -- Wardenmother HP > Ironhide HP.
- `unit_stats::cragback_has_longest_range` -- Cragback range >= all other Seekers.
- `building_stats::all_seekers_buildings_have_stats` -- iterate all 8 Seekers BuildingKind variants.
- `building_stats::the_sett_is_pre_built` -- build_time == 0, food_cost == 0, gpu_cost == 0.
- `building_stats::the_sett_produces_delver` -- can_produce contains Delver.
- `building_stats::war_hollow_produces_basic_combat` -- contains Ironhide, Sapjaw, Warden, Gutripper.
- `building_stats::core_tap_produces_advanced` -- contains Tunneler, Embermaw, Dustclaw, Cragback, Wardenmother.
- `building_stats::deep_warren_provides_supply` -- supply_provided == 12.
- `building_stats::deep_warren_more_supply_than_litter_box` -- 12 > 10.
- `building_stats::the_sett_tougher_than_the_box` -- 600 > 500 HP.
- `abilities::all_seekers_ability_defs_valid` -- iterate all 30 Seekers AbilityId variants.
- `abilities::seekers_unit_abilities_returns_three_per_kind` -- all 10 Seekers kinds.
- `abilities::seekers_passive_abilities_no_cooldown` -- Unbowed, ArmorRend, PatientStrike, BoulderBarrage, Earthsense, MoltenShot, FuelReserve, AmbushInstinct, Frenzy, Bloodgreed, DeepseekUplink, TremorNetwork, VigilanceAura.
- `abilities::seekers_toggle_abilities_have_cooldown` -- Entrench.
- `abilities::seekers_activated_abilities_have_cooldown` -- ShieldWall, GrudgeCharge, SeismicSlam, Intercept, RallyCry, Lockjaw, FortressProtocol, CalculatedCounterstrike, DeepBore, Undermine, ScorchedEarth, DustCloud, SentryBurrow, RecklessLunge, SubterraneanHaul, EmergencyBurrow.
- `components::seekers_unit_kind_display_from_str_round_trip` -- all 10 Seekers UnitKind variants.
- `components::seekers_building_kind_display_from_str_round_trip` -- all 8 Seekers BuildingKind variants.

### 16.2 cc_sim Unit Tests

- `dug_in_system::applies_after_50_ticks_stationary` -- unit gains +10% DR after 5s.
- `dug_in_system::clears_on_move_after_grace_period` -- Dug In removed 2s after movement starts.
- `dug_in_system::does_not_stack` -- Only +10% even if stationary for 100s.
- `burning_tile_system::damages_units_on_tile` -- unit on BurningTile takes damage.
- `burning_tile_system::grants_vision_to_seekers` -- vision check.
- `burning_tile_system::despawns_on_expiry` -- tile removed after duration.
- `rubble_tile_system::applies_movement_penalty` -- 3x cost verified in pathfinding.
- `rubble_tile_system::despawns_on_expiry` -- tile removed after 12s.
- `rubble_tile_system::destroyed_by_seismic_slam` -- slam in range removes rubble.
- `terrain_crush_system::heavy_unit_destroys_obstacle` -- obstacle despawned on contact.
- `terrain_crush_system::non_heavy_unit_blocked` -- obstacle remains.
- `lockjaw_system::prevents_movement_both_entities` -- neither moves during tether.
- `lockjaw_system::applies_armor_rend_per_second` -- 1 stack per 10 ticks.
- `lockjaw_system::cc_immunity_on_expiry` -- target gets 1s CC immunity.
- `frenzy_system::stacks_per_nearby_enemy` -- correct stack count.
- `frenzy_system::caps_at_8_stacks` -- 9+ enemies still = 8 stacks.
- `frenzy_system::speed_bonus_at_5_stacks` -- +15% move speed.
- `production_system::spawns_delver_with_stationary_timer` -- component present.
- `production_system::spawns_ironhide_with_heavy_unit` -- HeavyUnit marker present.
- `production_system::spawns_gutripper_with_frenzy_and_bloodgreed` -- both components present.
- `production_system::slag_thrower_gets_attack_stats` -- AoE ranged attack on completion.
- `production_system::delver_auto_gathers` -- auto-assigned to nearest deposit.

### 16.3 Integration Tests

- `seekers_full_game::build_and_train_basic_army` -- Delver builds WarHollow, trains Ironhide + Sapjaw.
- `seekers_full_game::ironhide_shield_wall_reduces_damage` -- damage from front arc reduced 50%.
- `seekers_full_game::cragback_entrench_increases_range` -- entrenched Cragback hits at 12-tile range.
- `seekers_full_game::dug_in_applies_to_all_seekers` -- any stationary Seekers unit gets DR.
- `seekers_full_game::heavy_units_crush_obstacles` -- Ironhide walks through bush, bush destroyed.
- `seekers_vs_catgpt::balanced_engagement` -- Seekers defensive position vs catGPT attack.

---

## 17. Implementation Order

Recommended phased implementation to minimize breakage and allow incremental testing.

### Phase 1: Enums and Static Data (no runtime behavior)

**Goal**: All UnitKind, BuildingKind, AbilityId variants compile. All static lookup functions return valid data. All round-trip tests pass.

- [ ] Add 10 UnitKind variants + Display/FromStr
- [ ] Add 8 BuildingKind variants + Display/FromStr
- [ ] Add 30 AbilityId variants
- [ ] Implement base_stats() for all 10 Seekers units
- [ ] Implement building_stats() for all 8 Seekers buildings
- [ ] Implement ability_def() for all 30 Seekers abilities
- [ ] Implement unit_abilities() for all 10 Seekers units
- [ ] Add UpgradeType variants for Seekers
- [ ] Write all cc_core unit tests (section 16.1)
- [ ] Verify: `cargo test -p cc_core` passes

### Phase 2: Spawn Pipeline (units appear in game)

**Goal**: Seekers units can be trained from Seekers buildings. They move, attack, and die correctly with basic stats.

- [ ] Add new component structs (StationaryTimer, HeavyUnit, FrenzyStacks, BloodgreedTracker, etc.)
- [ ] Update production_system.rs with spawn-time components
- [ ] Update production_system.rs with SlagThrower/ClawMarks completion logic
- [ ] Add Delver to auto-gather logic
- [ ] Add new AuraType variants
- [ ] Write production system tests (section 16.2 spawn tests)
- [ ] Verify: Seekers units spawn correctly in headless sim

### Phase 3: Core Faction Mechanics (Dug In, Heavy Pathing, Terrain)

**Goal**: The passive mechanics that define the faction identity work.

- [ ] Implement dug_in_system.rs
- [ ] Implement terrain_crush_system.rs
- [ ] Add TerrainObstacle entity spawning in map generation
- [ ] Add dynamic pathfinding cost overlay for Rubble/Burning
- [ ] Write Dug In and terrain crush tests
- [ ] Verify: Ironhide crushes bushes, stationary units gain DR

### Phase 4: Active Abilities -- Tier 1 (simple, self-contained)

**Goal**: Abilities that don't require complex inter-system coordination.

- [ ] ShieldWall (directional DR, movement lock)
- [ ] Entrench (toggle, stat modifier, movement lock)
- [ ] GrudgeCharge (targeted dash with windup)
- [ ] EmergencyBurrow (invulnerability + Earthsense boost)
- [ ] RecklessLunge (targeted leap + vulnerability)
- [ ] DustCloud (zone spawn, vision/miss effects)
- [ ] SentryBurrow (stealth + Earthsense)
- [ ] Write ability-specific unit tests

### Phase 5: Active Abilities -- Tier 2 (inter-system coordination)

**Goal**: Abilities requiring interaction with other systems.

- [ ] BoulderBarrage AoE + Rubble spawning (combat + terrain)
- [ ] SeismicSlam (damage + Rubble destruction + knockback)
- [ ] MoltenShot + BurningTile spawning (combat + terrain)
- [ ] FuelReserve charge system (passive regen while stationary)
- [ ] ScorchedEarth cone attack (consumes FuelReserve charges)
- [ ] ArmorRend stacks (combat modifier)
- [ ] PatientStrike conditional bonus (StationaryTimer check)
- [ ] Lockjaw tether system
- [ ] Bloodgreed lifesteal (combat hook)
- [ ] Frenzy system (spatial query)
- [ ] Write integration tests

### Phase 6: Strategic Abilities (economy, tunnels, AI integration)

**Goal**: The faction's strategic depth -- tunnels, passages, and AI coordination.

- [ ] SubterraneanHaul resource passages
- [ ] DeepBore tunnel system (portals in pathfinding)
- [ ] Undermine building debuff
- [ ] Earthsense/TremorNetwork blip system
- [ ] Vigilance Aura + Intruder mark tracking
- [ ] Intercept (AI-coordinated sprint)
- [ ] RallyCry (CC immunity + speed buff)
- [ ] Write strategic ability tests

### Phase 7: Hero Abilities + AI FSM

**Goal**: Wardenmother works, Deepseek AI uses Seekers units natively.

- [ ] DeepseekUplink (AI processing delay modifier)
- [ ] FortressProtocol zone system
- [ ] CalculatedCounterstrike damage tracking + burst
- [ ] Deepseek command delay system
- [ ] Update FSM: replace cat proxies with Seekers units
- [ ] Update FSM: building selection by faction
- [ ] Update FSM: worker detection for Delver
- [ ] Write FSM + hero tests

### Phase 8: BulwarkGate Garrison Mechanic

**Goal**: Unique defensive building mechanic.

- [ ] Garrison system (units enter building, attack from within at 50% damage)
- [ ] HP scaling (+10% per garrisoned unit)
- [ ] Ejection on death (garrisoned units emerge at 50% HP)
- [ ] Write garrison tests

Note: This is deferred because the cat faction's CatFlap garrison is also marked as deferred in the codebase. Implement both simultaneously.

---

## 18. Open Questions

1. **BuildingRole abstraction**: Should we introduce a BuildingRole enum now (cleaner for multi-faction FSM) or defer until a third faction is added? Adding it now means refactoring the FSM's building selection logic, but it pays off immediately.

2. **Shared component reuse**: Several Seekers components are conceptually similar to cat components (RubbleTile ~ HairballObstacle, DeepBoreTunnel ~ TunnelNetwork). Should we generalize into a single `TemporaryTerrainEffect` component with a `kind` field, or keep them separate for clarity?

3. **Dug In for non-Seekers units**: The design doc says Dug In is a Seekers passive. Should StationaryTimer be added only to Seekers units, or should the system exist globally and only grant the DR bonus to Seekers? The latter is more extensible if other factions gain similar mechanics.

4. **SlagThrower BurningTile creation**: The SlagThrower creates Burning tiles on each shot (like Embermaw). This means the tower combat system needs to spawn BurningTile entities on projectile impact. Should this be handled in a unified "on-hit effect" system, or as special-case logic per building/unit?

5. **Claw Marks visual intel**: GAME_DESIGN.md says enemies can read a Seekers player's tech level by scouting their Claw Marks (each completed research adds a visible mark). This is a client-side visual feature. Should it be tracked as a component (`CompletedResearchCount { count: u32 }` on the building) or derived from the player's upgrade list?

6. **Garrison system scope**: BulwarkGate garrison has unique properties (attack from within, HP scaling, ejection on death). The cat CatFlap garrison is listed as "deferred" in existing code. Implement both garrisons together, or implement BulwarkGate alone? Together is cleaner but increases scope.

7. **Deepseek processing delay -- competitive balance**: The 3x delay makes Deepseek dramatically slower than other AI agents. In the FSM, `eval_speed_mult` is currently 1.5 (already a proxy). The full 3x delay means Deepseek evaluates every 15 ticks * 3 = 45 ticks at medium difficulty (4.5s between decisions). This is intentional per design doc, but may need playtesting. Flag for balance pass.

8. **Asset pipeline**: This plan covers code only. Seekers need: 10 unit sprites, 8 building sprites, strategic icons, portraits, and UI elements. Coordinate with asset pipeline separately.

---

## File Change Summary

### New Files

| File | Purpose |
|------|---------|
| `crates/cc_sim/src/systems/dug_in_system.rs` | Dug In passive tracking |
| `crates/cc_sim/src/systems/burning_tile_system.rs` | Burning tile DoT + vision |
| `crates/cc_sim/src/systems/rubble_tile_system.rs` | Rubble tile movement cost |
| `crates/cc_sim/src/systems/terrain_crush_system.rs` | Heavy unit obstacle destruction |
| `crates/cc_sim/src/systems/earthsense_system.rs` | Tremorsense blip generation |
| `crates/cc_sim/src/systems/lockjaw_system.rs` | Sapjaw tether mechanic |
| `crates/cc_sim/src/systems/frenzy_system.rs` | Gutripper Frenzy stacks |
| `crates/cc_sim/src/systems/deepseek_delay_system.rs` | AI command buffering |
| `crates/cc_sim/src/systems/fortress_protocol_system.rs` | Fortress Zone management |
| `crates/cc_sim/src/systems/counterstrike_system.rs` | Counterstrike accumulation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/cc_core/src/components.rs` | +10 UnitKind, +8 BuildingKind, +4 AuraType, +5 UpgradeType, +18 new component structs, Display/FromStr updates |
| `crates/cc_core/src/unit_stats.rs` | +10 base_stats() arms |
| `crates/cc_core/src/building_stats.rs` | +8 building_stats() arms |
| `crates/cc_core/src/abilities.rs` | +30 AbilityId variants, +30 ability_def() arms, +10 unit_abilities() arms |
| `crates/cc_sim/src/systems/production_system.rs` | Spawn-time components for 10 Seekers units, SlagThrower/ClawMarks completion, Delver auto-gather |
| `crates/cc_sim/src/systems/combat.rs` | AoE, terrain effects, ArmorRend, lifesteal, directional DR, displacement immunity |
| `crates/cc_sim/src/systems/movement.rs` | Dynamic cost overlay, Lockjaw/Entrench/ShieldWall freeze, StationaryTimer |
| `crates/cc_sim/src/systems/target_acquisition.rs` | Intruder mark priority, Dust Cloud miss chance |
| `crates/cc_sim/src/systems/cleanup.rs` | Despawn expired zone/tile entities, Bloodgreed kill proc |
| `crates/cc_sim/src/systems/resource_system.rs` | ResourcePassage ticking |
| `crates/cc_sim/src/ai/fsm.rs` | Replace cat proxies, building selection, worker detection, Deepseek profile update |
| `crates/cc_sim/src/systems/research_system.rs` | Seekers upgrade application |
| `crates/cc_sim/src/lib.rs` | Register new systems in FixedUpdate schedule |
| Pathfinding module | Dynamic cost overlay, tunnel portal edges |

### Estimated Scope

- **New component/struct definitions**: ~18
- **New enum variants**: 53 (10 unit + 8 building + 30 ability + 5 upgrade)
- **New systems**: 10
- **Modified systems**: 7
- **New tests**: ~55-65
- **Total new lines (estimated)**: 2500-3500

---

*Plan created: 2026-03-01*
*Based on: GAME_DESIGN.md Seekers section (lines 495-722), existing cat implementation patterns in cc_core and cc_sim*
