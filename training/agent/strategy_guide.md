# ClawedCommand AI Strategy Guide

Guide for writing competitive Lua micro scripts. The FSM handles macro (economy, building, training). Your script handles micro (combat positioning, target selection, ability usage).

## Core Principles

1. **Scripts override FSM for the same unit.** Your commands run after the FSM, so the last command wins. Focus on combat micro — the FSM already handles build orders and economy.

2. **Budget is limited.** You get 500 compute budget per invocation. Spatial queries cost 2, pathfinding costs 10. Plan queries carefully.

3. **Tick interval matters.** Default `@interval: 5` = runs every 0.5s. Lower intervals give more responsiveness but burn budget faster across the match.

4. **Position wins fights.** Units in cover take less damage. Ranged units kiting melee units deal free damage. Elevation advantage gives +15% damage per level.

## Effective Micro Patterns

### 1. Focus Fire

The single highest-impact micro technique. Concentrating damage kills units faster, reducing enemy DPS.

```lua
-- @name: focus_fire
-- @events: on_tick
-- @interval: 3

local army = ctx:my_units()
local attackers = {}
for _, u in ipairs(army) do
  if u.kind ~= "Pawdler" and not u.is_dead then
    table.insert(attackers, u.id)
  end
end

if #attackers == 0 then return end

-- Find weakest enemy near army centroid
local cx, cy = 0, 0
for _, u in ipairs(army) do cx = cx + u.x; cy = cy + u.y end
cx = math.floor(cx / #army)
cy = math.floor(cy / #army)

local target = ctx:weakest_enemy_in_range(cx, cy, 8)
if target then
  ctx:attack_units(attackers, target.id)
end
```

### 2. Kiting (Ranged Units)

Ranged units should attack then move away before melee units close the gap. The Hisser (range 5, speed 0.12) can kite the Chonk (range 1, speed 0.08) indefinitely.

```lua
-- @name: kite_ranged
-- @events: on_tick
-- @interval: 2

local ranged = ctx:my_units("Hisser")
for _, u in ipairs(ranged) do
  local threats = ctx:threats_to(u.id)
  if #threats > 0 then
    -- Enemy melee in our range — kite back
    local kx, ky = ctx:position_at_range(u.x, u.y, threats[1].x, threats[1].y, 5)
    if kx then
      ctx:move_units({u.id}, kx, ky)
    end
  else
    -- No immediate threat — attack nearest
    local target = ctx:nearest_enemy(u.x, u.y)
    if target then
      ctx:attack_units({u.id}, target.id)
    end
  end
end
```

### 3. Wounded Retreat

Pull damaged units out of combat to preserve army value. Dead units contribute zero DPS; wounded units still fight.

```lua
-- @name: retreat_wounded
-- @events: on_tick
-- @interval: 5

local wounded = ctx:wounded_units(0.3)  -- below 30% HP
if #wounded == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end

local base_x, base_y = boxes[1].x, boxes[1].y
local ids = {}
for _, u in ipairs(wounded) do
  table.insert(ids, u.id)
end
ctx:move_units(ids, base_x, base_y)
```

### 4. Cover Seeking

Position ranged units in Forest (-15% damage) or TechRuins (-30% damage) for defense.

```lua
-- @name: seek_cover
-- @events: on_enemy_spotted
-- @interval: 10

local hissers = ctx:my_units("Hisser")
for _, u in ipairs(hissers) do
  local cover = ctx:cover_at(u.x, u.y)
  if cover == "None" then
    -- Search nearby for cover
    for dy = -3, 3 do
      for dx = -3, 3 do
        local tx, ty = u.x + dx, u.y + dy
        local tc = ctx:cover_at(tx, ty)
        if tc ~= "None" and ctx:is_passable(tx, ty) then
          ctx:move_units({u.id}, tx, ty)
          goto next_unit
        end
      end
    end
  end
  ::next_unit::
end
```

### 5. Split Army Tactics

Separate melee and ranged roles. Melee absorbs damage while ranged deals it from behind.

```lua
-- @name: army_split
-- @events: on_tick
-- @interval: 5

local army = ctx:my_units()
local melee, ranged = {}, {}
for _, u in ipairs(army) do
  if u.kind == "Pawdler" then goto continue end
  if u.attack_type == "Melee" then
    table.insert(melee, u)
  else
    table.insert(ranged, u)
  end
  ::continue::
end

local target = ctx:nearest_enemy(32, 32)
if not target then return end

-- Melee: charge target
local melee_ids = {}
for _, u in ipairs(melee) do table.insert(melee_ids, u.id) end
if #melee_ids > 0 then
  ctx:attack_move(melee_ids, target.x, target.y)
end

-- Ranged: hold at range 4 from target
for _, u in ipairs(ranged) do
  local kx, ky = ctx:position_at_range(u.x, u.y, target.x, target.y, 4)
  if kx then
    ctx:move_units({u.id}, kx, ky)
  end
end
```

## Economy Tips for Scripts

The FSM handles economy, but scripts can optimize:

- **Don't command Pawdlers** unless you're doing something specific. The FSM assigns them to gather.
- **Watch supply**: `ctx:resources().supply` vs `ctx:resources().supply_cap`. If nearing cap, let the FSM build LitterBoxes.
- **GPU awareness**: Abilities cost GPU. Don't spam abilities if GPU is low.

## Matchup Knowledge

### Strong Compositions

| Comp | Strength | Weakness |
|------|----------|----------|
| Nuisance spam | Fast, cheap, high DPS | Dies to Chonk + Hisser deathball |
| Hisser + Chonk | Tanky frontline + ranged DPS | Slow, vulnerable to flanking |
| FlyingFox raids | Fast harassment, map control | Low HP, dies to ranged fire |
| Mouser assassins | Highest DPS/supply, fast | Fragile, needs surprise |
| Catnapper siege | Devastating sustained damage | Very slow, needs escorts |

### Counter Triangle
- **Melee** beats **Ranged** when they close the gap (Nuisance > Hisser in melee range)
- **Ranged** beats **Melee** with kiting (Hisser > Nuisance with micro)
- **Tank** beats **Harasser** (Chonk absorbs Nuisance damage)
- **Air** counters **Ground melee** (FlyingFox kites Nuisance/Chonk)
- **Massed ranged** counters **Air** (FlyingFox has only 50 HP)

## Ability Usage Priorities

### High-Value Abilities
1. **NineLives** (Chonk, slot 2): Passive auto-resurrect. 600 tick CD, 25 GPU. Keep Chonks alive.
2. **Zoomies** (Nuisance, slot 2): Speed boost for 30 ticks. Use for engages/escapes. 10 GPU.
3. **TacticalUplink** (MechCommander, slot 0): Toggle team buff. Keep active in combat.
4. **LoafMode** (Chonk, slot 1): Toggle damage reduction while stationary. Use for defense.
5. **DreamSiege** (Catnapper, slot 0): Passive damage ramp. Don't retarget — let damage build.

### When NOT to Use Abilities
- When GPU < 50 (save for emergencies)
- NineLives on a Chonk that's far from combat
- Zoomies on units already in melee range

## Script Design Tips

1. **Start simple.** A focus-fire script alone beats no-micro by a wide margin.
2. **Combine scripts.** Use multiple scripts with different `@events`. One for combat micro (`on_tick`), one for retreat (`on_unit_attacked`), one for scouting (`on_tick` with high interval).
3. **Budget wisely.** Use `count_units` (cost 1) before iterating `my_units` (cost 1). Don't call `safe_positions` (cost 4) every tick.
4. **Test with `--seeds`**. Run arena matches with multiple seeds to ensure scripts are robust:
   ```
   cargo run -p cc_agent --bin arena --features harness -- --seeds 1,2,3,4,5
   ```
5. **Check the report.** Arena JSON reports include `script_errors` — fix any runtime errors.

## Common Mistakes

- **Moving units every tick**: Cancels attacks. Only reposition when needed.
- **Commanding workers**: Breaks FSM economy. Only touch workers for specific strategies.
- **Ignoring budget**: Running out of budget mid-tick means losing the rest of your queries.
- **Too many pathfinding calls**: `can_reach`/`path_length` cost 10 each. Use sparingly.
- **Not checking nil returns**: `nearest_enemy`, `weakest_enemy_in_range` return nil when no enemies visible.

## Lessons from Training Iterations

### What Works
1. **Focus fire with priority targeting**: Kill high-DPS squishy targets (Hissers, Mousers) first. This is the single highest-impact script. Only redirect units already `is_attacking` to avoid breaking FSM attack-move orders.
2. **Building push**: Redirect idle units to attack enemy buildings. The FSM does attack-move toward the enemy base, but units get stuck fighting and never reach it. Explicit building targeting converts kill advantages into base destruction.
3. **Conservative retreat**: Only retreat units at very low HP (10%). Higher thresholds (15-25%) reduce army DPS too much. Never retreat the whole army — that gives enemies free damage.

### What Hurts
1. **Kiting scripts**: Moving ranged units cancels their attacks, reducing DPS. Only kite when the unit is about to die (HP < 15%).
2. **Formation scripts**: Pre-positioning melee/ranged before combat interrupts the FSM's attack-move, causing units to stall and take free damage.
3. **Aggressive army-wide retreat**: Full army retreat when outnumbered sounds logical but in practice the disengaging army takes free damage without dealing any.
4. **Macro scripts (production/gathering)**: The FSM already handles economy well. Scripts that queue extra training or reassign workers provide zero measurable improvement.

### What Works (Gen 18b — NEW BEST)
4. **Ability activation**: The single highest-impact addition since focus fire. LoafMode (free tank buff), Zoomies (speed burst for engage/retreat), and DissonantScreech (AoE stun on 3+ clustered enemies) provide massive combat advantage. P0 outscores P1 in kills on 10/10 seeds.

### Key Insights
- **Only focus fire is safe micro.** It changes targets without moving units or canceling attacks.
- **Don't redirect units near enemy base.** If units are within 6 tiles of enemy buildings, let them hit buildings instead of chasing units.
- **Abilities > extra units.** Gen 18 showed that smart_fill (extra unit production) HURTS when combined with abilities. The extra units dilute army quality and feed enemy kills. Focused ability usage on existing units beats quantity.
- **Fewer scripts is often better.** Gen 18 with 5 scripts (10% P0 win rate) was worse than Gen 18b with 4 scripts (80% effective P0 dominance). Script interactions can be counterproductive.
- **FSM symmetry fix applied.** defense_pos, fallback positions, and forward_pos now mirror based on map center. enemy_spawn pre-seeded from tick 0. Remaining P1 advantage (~50% FSM-only decisive wins) is map terrain generation.
- **Track cooldowns in `_G`**. Ability cooldowns aren't exposed in unit snapshots. Use `_G.ability_cooldowns` table with tick-based tracking.
