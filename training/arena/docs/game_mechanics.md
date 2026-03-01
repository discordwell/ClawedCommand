# ClawedCommand Game Mechanics Reference

## Simulation

- **Tick rate**: 10Hz (10 ticks per second)
- **Match length**: 6000 ticks default (10 minutes)
- **Map size**: 64x64 grid default
- **Players**: 2 (player 0 and player 1)
- **Starting resources**: 200 food per player
- **Starting units**: 1 TheBox + 2 Pawdlers per player

## Unit Stats

| Unit | HP | Speed | Dmg | Range | AtkSpd | Type | Food | GPU | Supply | Train |
|------|-----|-------|-----|-------|--------|------|------|-----|--------|-------|
| Pawdler | 60 | 0.12 | 4 | 1 | 15 | Melee | 50 | 0 | 1 | 50 |
| Nuisance | 80 | 0.18 | 8 | 1 | 10 | Melee | 75 | 0 | 1 | 60 |
| Chonk | 300 | 0.08 | 12 | 1 | 20 | Melee | 150 | 25 | 3 | 120 |
| FlyingFox | 50 | 0.225 | 6 | 2 | 12 | Ranged | 100 | 25 | 2 | 80 |
| Hisser | 70 | 0.12 | 14 | 5 | 12 | Ranged | 100 | 0 | 2 | 80 |
| Yowler | 90 | 0.14 | 5 | 4 | 15 | Ranged | 100 | 50 | 2 | 100 |
| Mouser | 55 | 0.20 | 10 | 1 | 8 | Melee | 75 | 25 | 1 | 60 |
| Catnapper | 120 | 0.06 | 25 | 2 | 30 | Ranged | 200 | 50 | 3 | 150 |
| FerretSapper | 65 | 0.17 | 20 | 1 | 25 | Melee | 125 | 50 | 2 | 100 |
| MechCommander | 500 | 0.10 | 18 | 3 | 15 | Ranged | 400 | 200 | 6 | 250 |

- **Speed**: Grid cells per tick. Nuisance (0.18) is fast; Chonk (0.08) is slow.
- **AtkSpd**: Ticks between attacks. Lower = faster. Mouser (8) attacks fastest.
- **Train**: Ticks to produce. At 10Hz: 50 ticks = 5 seconds.

### Unit Roles
- **Pawdler**: Worker. Gathers resources, builds. Weak in combat.
- **Nuisance**: Basic melee fighter. Fast, cheap, decent damage.
- **Chonk**: Tank. Massive HP, slow, high damage. Absorbs punishment.
- **FlyingFox**: Fast air harasser. Low HP, ranged, best speed.
- **Hisser**: Ranged DPS. Best range (5), high damage (14), fragile.
- **Yowler**: Support. Medium range, low damage. (Abilities TBD)
- **Mouser**: Stealth assassin. Fast attack speed (8 ticks), fragile.
- **Catnapper**: Siege. Highest single-hit damage (25), very slow fire/move.
- **FerretSapper**: Demolition. High melee damage (20), moderate HP.
- **MechCommander**: Hero unit. 500 HP, ranged, very expensive.

### Key DPS Values (damage / attack_speed * 10)
| Unit | DPS |
|------|-----|
| Mouser | 12.5 |
| Hisser | 11.7 |
| Nuisance | 8.0 |
| Catnapper | 8.3 |
| FerretSapper | 8.0 |
| MechCommander | 12.0 |
| Chonk | 6.0 |
| FlyingFox | 5.0 |
| Yowler | 3.3 |
| Pawdler | 2.7 |

## Buildings

| Building | HP | BuildTime | Food | GPU | Supply | Produces |
|----------|-----|-----------|------|-----|--------|----------|
| TheBox | 500 | 0 (start) | 0 | 0 | +10 | Pawdler |
| CatTree | 300 | 150 | 150 | 0 | 0 | Nuisance, Hisser, Chonk, Yowler |
| FishMarket | 200 | 100 | 100 | 0 | 0 | (economy building) |
| LitterBox | 100 | 75 | 75 | 0 | +10 | (supply building) |
| ServerRack | 250 | 120 | 100 | 75 | 0 | FlyingFox, Mouser, Catnapper, FerretSapper, MechCommander |
| ScratchingPost | 200 | 100 | 100 | 50 | 0 | (upgrade building) |
| CatFlap | 400 | 100 | 150 | 0 | 0 | (defensive building) |
| LaserPointer | 150 | 80 | 75 | 25 | 0 | (utility building) |

## Combat

- **Melee**: Direct damage on attack, range 1 tile
- **Ranged**: Spawns projectile that travels to target
- **Cover multiplier**: Light cover = 0.75x damage taken, Heavy cover = 0.5x
- **Elevation multiplier**: +15% damage per elevation level advantage, -15% per disadvantage
- **Target acquisition**: Units auto-acquire enemies within weapon range
- **Death**: Two-phase — marked Dead, then despawned

## FSM AI Behavior (what scripts augment)

The FSM AI handles macro decisions automatically. Scripts add tactical micro on top.

### FSM Phases
1. **EarlyGame**: Train Pawdlers to target count (default 4), assign to gather
2. **BuildUp**: Build FishMarket + CatTree, train Nuisances, build LitterBox for supply
3. **MidGame**: Build ServerRack, ScratchingPost, LaserPointer; research; train mixed army
4. **Attack**: When army >= threshold (8 for balanced), attack-move to enemy base. Re-issue every 50 ticks.
5. **Defend**: When enemies near base (within 8 tiles of buildings), rally army home

### FSM Weaknesses (script opportunities)
- **No micro**: Army blob attack-moves without focus fire or kiting
- **No retreat**: Wounded units fight to death
- **No flanking**: Single attack vector toward enemy base
- **No scouting**: No information gathering before committing
- **No ability usage**: Unit abilities not activated
- **Fixed build order**: Always same sequence regardless of enemy composition
- **No adaptation**: Doesn't counter enemy unit composition

## Economy

- **Food**: Primary resource. Gathered from FishPonds (1500) and BerryBushes (800).
- **GPU Cores**: Secondary resource. From GpuDeposit (500). Required for advanced units/buildings.
- **NFTs**: Tertiary resource. From MonkeyMine (200). Victory condition resource.
- **Supply**: Unit population cap. TheBox provides 10, LitterBox provides 10.
- **Gathering**: Pawdlers auto-gather when assigned to deposits.

## Map

- 64x64 grid with procedurally generated terrain
- Two spawn points (opposing corners typically)
- Resource deposits scattered around map
- Terrain affects pathfinding cost, cover, and passability
