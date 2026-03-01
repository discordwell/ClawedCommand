# ClawedCommand Game Mechanics Reference

Technical reference for AI script authors. All values are from source code and authoritative.

## Simulation

- **Tick rate**: 10 Hz (10 ticks/second)
- **Default map**: 64x64 tiles
- **Default match length**: 6000 ticks (10 minutes)
- **Math**: Fixed-point arithmetic (FixedI32<U16>)
- **Coordinate system**: Grid-based isometric, (0,0) is top-left

## System Execution Order (per tick)

1. `tick_system` — increment SimClock
2. `multi_ai_decision_system` — FSM macro AI (build orders, phase transitions)
3. `script_runner_system` — Lua scripts execute (**override FSM for same unit**)
4. `process_commands` — execute queued GameCommands
5. `ability_cooldown_system` — tick down ability cooldowns
6. `ability_effect_system` — apply ability effects
7. `status_effect_system` — tick status effects
8. `aura_system` — apply aura buffs/debuffs
9. `stat_modifier_system` — recalculate modified stats
10. `production_system` — advance build/train queues
11. `research_system` — advance research
12. `gathering_system` — workers gather resources
13. `target_acquisition_system` — auto-acquire targets in weapon range
14. `combat_system` — melee damage + ranged projectile spawn
15. `tower_combat_system` — tower attacks
16. `projectile_system` — move projectiles, apply ranged damage on hit
17. `movement_system` — move units along paths
18. `builder_system` — workers construct buildings
19. `grid_sync_system` — sync world positions to grid
20. `cleanup_system` — mark Dead entities
21. `headless_despawn_system` — remove Dead (arena only, no fade animation)
22. `victory_system` — check win conditions

**Key insight**: Scripts run at step 3, commands execute at step 4. The FSM runs at step 2. For the same unit, the **last command wins**, so scripts naturally override FSM micro decisions.

## Units

### Combat Stats

| Unit | HP | Speed | DPS | Range | Type | Food | GPU | Supply | Train |
|------|-----|-------|-----|-------|------|------|-----|--------|-------|
| Pawdler | 60 | 0.12 | 2.7 | 1 | Melee | 50 | 0 | 1 | 5.0s |
| Nuisance | 80 | 0.18 | 8.0 | 1 | Melee | 75 | 0 | 1 | 6.0s |
| Chonk | 300 | 0.08 | 6.0 | 1 | Melee | 150 | 25 | 3 | 12.0s |
| FlyingFox | 50 | 0.225 | 5.0 | 2 | Ranged | 100 | 25 | 2 | 8.0s |
| Hisser | 70 | 0.12 | 11.7 | 5 | Ranged | 100 | 0 | 2 | 8.0s |
| Yowler | 90 | 0.14 | 3.3 | 4 | Ranged | 100 | 50 | 2 | 10.0s |
| Mouser | 55 | 0.20 | 12.5 | 1 | Melee | 75 | 25 | 1 | 6.0s |
| Catnapper | 120 | 0.06 | 8.3 | 2 | Ranged | 200 | 50 | 3 | 15.0s |
| FerretSapper | 65 | 0.17 | 8.0 | 1 | Melee | 125 | 50 | 2 | 10.0s |
| MechCommander | 500 | 0.10 | 12.0 | 3 | Ranged | 400 | 200 | 6 | 25.0s |

DPS = damage / (attack_speed_ticks / 10). Train time in seconds (ticks / 10).

### Unit Roles

- **Pawdler**: Worker. Gathers resources, builds buildings. Low combat value.
- **Nuisance**: Harasser. Fast melee, good DPS, cheap. Core early/mid army unit.
- **Chonk**: Tank. Huge HP pool, slow, moderate damage. Frontline absorber.
- **FlyingFox**: Air scout/harasser. Fastest unit, low HP. Good for scouting and raiding.
- **Hisser**: Ranged DPS. Best range (5), high damage. Glass cannon — must be protected.
- **Yowler**: Support. Moderate range, low damage. Abilities provide team buffs.
- **Mouser**: Stealth assassin. Fast melee, highest DPS per supply. Fragile.
- **Catnapper**: Siege. Very slow, high damage. DreamSiege ability ramps damage over time.
- **FerretSapper**: Demolition. Anti-building specialist. ShapedCharge for burst damage.
- **MechCommander**: Hero. Highest HP, expensive. TacticalUplink buffs nearby allies.

### Production Buildings

| Building | Produces | Cost |
|----------|----------|------|
| TheBox | Pawdler | Pre-built (free) |
| CatTree | Nuisance, Hisser, Chonk, Yowler | 150 food |
| ServerRack | FlyingFox, Mouser, Catnapper, FerretSapper, MechCommander | 100 food + 75 GPU |

## Buildings

| Building | HP | Build Time | Food | GPU | Supply | Purpose |
|----------|-----|-----------|------|-----|--------|---------|
| TheBox | 500 | Pre-built | 0 | 0 | +10 | Base, trains Pawdlers |
| CatTree | 300 | 15.0s | 150 | 0 | 0 | Trains basic combat units |
| FishMarket | 200 | 10.0s | 100 | 0 | 0 | Economy (food production) |
| LitterBox | 100 | 7.5s | 75 | 0 | +10 | Supply building |
| ServerRack | 250 | 12.0s | 100 | 75 | 0 | Trains advanced units, sets AI tier |
| ScratchingPost | 200 | 10.0s | 100 | 50 | 0 | Research upgrades |
| CatFlap | 400 | 10.0s | 150 | 0 | 0 | Defensive structure |
| LaserPointer | 150 | 8.0s | 75 | 25 | 0 | Tower defense |

## AI Tier System

ServerRack count determines AI tier, which gates behavior complexity:

| Tier | ServerRacks | Unlocked Behaviors |
|------|------------|-------------------|
| Basic | 0 | assign_idle_workers, attack_move_group |
| Tactical | 1 | focus_fire, kite_squad, retreat_wounded, defend_area, harass_economy, scout, split_squads, protect, surround |
| Strategic | 2 | auto_produce, balanced_production, expand_economy, coordinate_assault |
| Advanced | 3+ | research_priority, adaptive_defense |

## Upgrades

Researched at ScratchingPost. Each can only be researched once.

| Upgrade | Time | Food | GPU | Effect |
|---------|------|------|-----|--------|
| SharperClaws | 20.0s | 150 | 50 | Increases melee damage |
| ThickerFur | 20.0s | 150 | 50 | Increases unit HP |
| NimblePaws | 15.0s | 100 | 25 | Increases movement speed |
| SiegeTraining | 25.0s | 200 | 100 | Enables Catnapper production |
| MechPrototype | 40.0s | 400 | 200 | Enables MechCommander production |

Research priority for the FSM AI: SharperClaws → ThickerFur → SiegeTraining.

## Terrain

| Type | Move Cost | Passable | Cover | Notes |
|------|-----------|----------|-------|-------|
| Grass | 1.0x | Yes | None | Default terrain |
| Dirt | 0.95x | Yes | None | Slightly faster |
| Sand | 1.2x | Yes | None | Slower movement |
| Forest | 1.3x | Yes | Light (-15%) | Concealment |
| Water | — | No* | None | *Croak faction only |
| Shallows | 1.5x | Yes | None | Slowest passable |
| Rock | — | No | None | Impassable wall |
| Ramp | 1.1x | Yes | None | Elevation transition |
| Road | 0.7x | Yes | None | Fastest movement |
| TechRuins | 1.15x | Yes | Heavy (-30%) | Best cover |

### Cover System

Cover reduces incoming damage based on the **defender's** terrain:
- **None**: 1.0x (full damage)
- **Light** (Forest): 0.85x (-15% damage taken)
- **Heavy** (TechRuins): 0.70x (-30% damage taken)

### Elevation

- Each elevation level difference applies ±15% damage modifier
- Attacking downhill: +15% per level
- Attacking uphill: -15% per level (floor at 0.55x)
- Ramp tiles connect different elevation levels

## Economy

### Resources

| Resource | Source | Primary Use |
|----------|--------|-------------|
| Food | Fish Ponds (1500), Berry Bushes (800) | Units, buildings, upgrades |
| GPU Cores | Tech Ruins deposits (500) | Advanced buildings, abilities, AI actions |
| NFTs | Monkey Mines (200) | Victory condition |

### Starting Resources
- Each player starts with: 200 food, 0 GPU, 0 NFTs
- Starting supply: 10 (from TheBox), 2 used (2 Pawdlers)
- Starting units: 1 TheBox + 2 Pawdlers

### Resource Gathering
- Workers (Pawdlers) gather by moving to a deposit entity
- Gathering rate is per-tick while adjacent to deposit
- FishMarket building enhances food production

## Combat

### Damage Calculation

```
final_damage = base_damage * cover_multiplier * elevation_multiplier
```

- **Melee**: Damage applied directly when attacker is in range (1 tile)
- **Ranged**: Spawns a projectile that travels to target, damage on hit
- **Attack speed**: Cooldown in ticks between attacks (lower = faster)

### Target Acquisition
- Units auto-acquire targets within weapon range (system step 13)
- Manual commands override auto-targeting
- Attack-move engages enemies encountered while moving

### Death
- Units at 0 HP get `Dead` marker
- Dead entities despawned next tick (arena) or after fade animation (client)

## Victory Conditions

1. **Domination**: Destroy all enemy TheBox buildings
2. **NFT Monopoly**: Hold all Monkey Mine deposits for 60 seconds
3. **Digital Ascension**: Build "The Cloud" wonder (not yet implemented)
4. **Timeout**: At max ticks, player with more living entities leads
5. **Elimination**: Player with no remaining entities loses

## FSM AI Behavior

The built-in FSM AI follows these phases:

1. **EarlyGame**: Train workers until target count (default 4)
2. **BuildUp**: Build FishMarket → CatTree → train first army units. Transition at 4+ army.
3. **MidGame**: Build ServerRack, ScratchingPost, LaserPointer. Research upgrades. Train mixed army. Transition to Attack at threshold (default 8).
4. **Attack**: Send army to enemy base. Re-issue orders periodically. Fall back to Defend if base threatened, or MidGame if army decimated.
5. **Defend**: Rally army to base. Train reinforcements. Return to MidGame when threat clears.

The FSM handles macro (building, training, economy). Scripts should focus on micro (unit positioning, target selection, ability usage, kiting).
