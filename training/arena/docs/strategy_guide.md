# Strategy Guide for Script Writers

This guide evolves across iterations. Scripts augment the FSM AI's macro decisions with tactical micro.

## What the FSM Already Handles

Your scripts DON'T need to handle:
- Worker training (FSM trains Pawdlers in EarlyGame)
- Building construction (FSM builds in order: FishMarket, CatTree, LitterBox, ServerRack, etc.)
- Basic army movement (FSM attack-moves army toward enemy in Attack phase)
- Worker assignment to resources (FSM sends idle workers to gather)
- Phase transitions (FSM auto-transitions based on army/building counts)

## What Scripts Should Handle

Focus on tactical micro that the FSM can't do:

### Priority 1: Combat Micro
- **Focus fire**: Concentrate damage on one target to get kills faster
- **Kiting**: Ranged units maintain distance while attacking (Hissers have range 5)
- **Retreat wounded**: Pull back low-HP units instead of letting them die
- **Target priority**: Kill high-DPS targets first (Hissers, Mousers)

### Priority 2: Army Composition Reactions
- **Counter-building**: If enemy has many Chonks, train more Hissers (ranged vs melee)
- **Squad splitting**: Keep melee units in front, ranged behind

### Priority 3: Map Control
- **Scouting**: Send fast unit to reveal enemy base early
- **Terrain advantage**: Position units on high ground or in cover before fights

## Script Design Patterns

### Event-Driven Reactive
```lua
-- @events: on_unit_attacked
-- React when our units take damage
local wounded = ctx:wounded_units(0.3)
if #wounded > 0 then
    ctx.behaviors:retreat_wounded(0.3)
end
```

### Periodic Tactical Poll
```lua
-- @events: on_tick
-- @interval: 3
-- Every 0.3 seconds, check for micro opportunities
local enemies = ctx:enemy_units()
local my_ranged = ctx:my_units("Hisser")
if #enemies > 0 and #my_ranged > 0 then
    ctx.behaviors:kite_squad(collect_ids(my_ranged))
end
```

### Helper: Collect IDs
```lua
local function collect_ids(units)
    local ids = {}
    for _, u in ipairs(units) do
        table.insert(ids, u.id)
    end
    return ids
end
```

## Critical Technical Findings

### Command Override Chain
Schedule order: FSM → Scripts → process_commands → target_acquisition → combat → movement

- `GameCommand::Attack` (from `ctx:attack_units`) sets `AttackTarget` and removes all movement. The `combat_system` then re-adds chasing if target is out of range. This works correctly.
- `GameCommand::AttackMove` **removes `AttackTarget`**. This means the FSM's periodic `AttackMove` reissue (every ~50 ticks) will clear script-assigned targets.
- `target_acquisition_system` **respects existing `AttackTarget`** — it skips units that already have a valid target. So between FSM reissues, script redirects persist.
- **Net effect**: Script `attack_units` redirects last ~50 ticks (5 seconds) before the FSM clears them. This is enough for a Nuisance to cross 2-3 tiles and attack.

### Lua Field Names (Fixed in Gen 4)
Unit tables have both short and canonical names:
- `u.damage` and `u.atk_dmg` both work
- `u.attack_type` and `u.atk_type` both work (returns "Melee" or "Ranged")
- `b.producing` now correctly returns true/false (was always nil before Gen 4)

### FSM Timing
- FSM evaluates every 5 ticks (Hard difficulty, default in arena)
- Attack phase reissues AttackMove every 50 ticks (ATTACK_REISSUE_INTERVAL)
- Phase transitions happen at specific army/building thresholds

## Known FSM Baseline Win Rates

5-seed test (42, 123, 7777, 9999, 31415):

| Seed | Winner | Ticks | P0 K/D | P1 K/D |
|------|--------|-------|--------|--------|
| 42 | P0 | 3181 | 14/9 | 9/14 |
| 123 | P1 | 4187 | 12/17 | 17/12 |
| 7777 | P1 | 3339 | 6/13 | 13/6 |
| 9999 | P1 | 3456 | 11/16 | 16/11 |
| 31415 | Timeout | 6000 | 13/16 | 16/13 |

**P0 baseline: 20% win rate (1W/3L/1T)**

## Iteration History

### Generation 0 (Baseline)
- FSM vs FSM, no scripts
- P0: 1W / 3L / 1T (20%)
- Average ticks: 4033

### Generation 1 (CATASTROPHIC — 0% win rate)
- Scripts: combat_micro, retreat_logic, scout_and_adapt, formation_control
- **Failure**: All 4 scripts aggressively overrode FSM commands every tick
- formation_control split the army blob (worst offender)
- retreat_logic pulled units out of winnable fights
- combat_micro retargeted every 3 ticks preventing units from reaching targets
- **Lesson**: NEVER issue movement commands that split the army. NEVER override units that are actively fighting.

### Generation 2 (Too conservative — identical to baseline)
- Scripts: tactical_focus, worker_helper
- tactical_focus required 3+ distinct targets to trigger (never happened in blob fights)
- worker_helper caught no idle worker gaps
- **Lesson**: Need scripts that actually trigger. The blob fight IS the correct behavior; don't try to split it.

### Generation 3 (No effect — identical to baseline)
- Scripts: smart_focus, economy_boost
- Focus fire targeted weakest enemy = same as FSM's nearest-enemy auto-target
- **Lesson**: Targeting the weakest/nearest enemy has zero marginal value because the FSM already does this via target_acquisition_system (nearest enemy in range).

### Generation 4 (Measurable effects but inconsistent)
- Scripts: priority_kill, production_boost
- Fixed lua_runtime bugs: producing field, field name aliases
- priority_kill targets non-nearest backline threats (Hissers/Mousers)
- production_boost fills idle building production gaps
- Results: Same 20% win rate overall, but combat outcomes shifted:
  - Seed 31415: P0 K/D improved from 13/16 to **15/9** (massive swing)
  - Seed 42: P0 K/D worsened from 14/9 to 14/12 (script hurt)
  - Seeds 123, 9999: P0 killed fewer units
- **Analysis**: priority_kill sometimes pulls units off good fights to chase backline targets that are too far away. production_boost seems positive (31415 improvement likely from more units).
- **Lesson**: Don't redirect units that are about to finish a kill. Production/economy scripts have more consistent impact than combat micro.

## What DOESN'T Work (Proven)

1. **Army splitting**: Any attempt to separate melee from ranged reduces the blob's effectiveness. The blob IS the correct formation for the FSM's fight style.
2. **Aggressive retreat**: Pulling wounded units disrupts concentration of force.
3. **High-frequency retargeting**: Redirecting units every 3-5 ticks prevents them from reaching and killing targets.
4. **Focus fire on weakest/nearest**: Zero marginal value — FSM already targets nearest.
5. **Focus fire on non-nearest backline**: Marginal value at best, negative at worst. Pulling melee units off their current fight to chase a Hisser 3+ tiles away gives the enemy free damage on our blob.

## What MIGHT Work (Needs Testing)

1. **More aggressive production filling**: Lower resource thresholds in production_boost. The conservative thresholds (100/150/200) may prevent the script from ever training units.
2. **Hisser-heavy composition**: Hissers have the best DPS (11.7). Training mostly Hissers from CatTree could outperform the FSM's default mixed composition.
3. **Extra worker early**: Getting a 4th worker before tick 500 speeds up economy considerably.
4. **Building placement optimization**: If the FSM places buildings suboptimally, scripts could suggest better positions (near resources, away from enemy approach).
5. **Supply building earlier**: If supply cap is reached before buildings are queued, production stalls.
