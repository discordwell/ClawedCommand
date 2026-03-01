# ClawedCommand Lua Script API Reference

This document describes the complete Lua scripting API available to AI-controlled scripts in arena matches. Scripts run inside a sandboxed Luau VM with a per-invocation instruction limit of 10,000.

## Script Structure

Scripts are `.lua` files with optional annotation headers:

```lua
-- @name: tactical_micro
-- @events: on_tick, on_enemy_spotted, on_unit_attacked
-- @interval: 3

local units = ctx:my_units()
-- script body here
```

### Annotations

| Annotation | Default | Description |
|-----------|---------|-------------|
| `@name` | filename stem | Script display name |
| `@events` | `on_tick` | Comma-separated event list |
| `@interval` | `5` | Ticks between `on_tick` runs (at 10Hz sim = 2Hz default) |

### Events

| Event | Fires when |
|-------|-----------|
| `on_tick` | Every `@interval` ticks |
| `on_enemy_spotted` | New enemy unit becomes visible |
| `on_unit_attacked` | Own unit takes damage |
| `on_unit_idle` | Own unit transitions to idle |
| `on_unit_died` | Own unit dies |

## Compute Budget

Each script invocation gets a budget of **500 points** (configurable). Queries cost budget:

| Cost | Query type |
|------|-----------|
| 1 | Simple queries (my_units, enemy_units, count, resources, terrain) |
| 2 | Spatial queries (enemies_in_range, nearest_enemy, threats_to, safe_positions) |
| 10 | Pathfinding queries (can_reach, path_length) |

When budget is exhausted, queries return empty results. Commands are free.

## The `ctx` Object

All API calls go through the `ctx` object, available as a global. Use colon syntax: `ctx:method(args)`.

---

## Unit Queries

### ctx:my_units(kind?)
Returns a table of own living units. Optional string filter by UnitKind.

```lua
local all = ctx:my_units()
local hissers = ctx:my_units("Hisser")
```

### ctx:enemy_units()
Returns a table of all visible living enemy units.

### ctx:enemies_in_range(x, y, range)
Returns enemies within Euclidean `range` tiles of position (x,y).

```lua
local nearby = ctx:enemies_in_range(10, 10, 5.0)
```

### ctx:nearest_enemy(x, y)
Returns the closest living enemy to (x,y), or nil.

### ctx:idle_units(kind?)
Returns own idle units, optionally filtered by kind string.

### ctx:wounded_units(threshold)
Returns own units below HP percentage threshold (0.0-1.0).

```lua
local hurt = ctx:wounded_units(0.5) -- units below 50% HP
```

### ctx:units_by_state(state)
Returns own units matching a state string: `"Moving"`, `"Attacking"`, `"Idle"`, `"Gathering"`.

### ctx:count_units(kind?)
Returns count of alive own units. Optional kind filter.

### ctx:army_supply()
Returns total supply cost of all alive own units.

### ctx:weakest_enemy_in_range(x, y, range)
Returns the lowest-HP enemy within range, or nil.

### ctx:strongest_enemy_in_range(x, y, range)
Returns the highest-HP enemy within range, or nil.

### ctx:hp_pct(unit_id)
Returns HP as fraction 0.0-1.0 for a unit, or nil.

### ctx:distance_squared_between(a_id, b_id)
Returns squared distance between two units (by entity ID), or nil.

### ctx:distance_squared_to_nearest_enemy(unit_id)
Returns squared distance from unit to its closest visible enemy, or nil.

### ctx:threats_to(unit_id)
Returns enemies whose attack range reaches the given unit.

### ctx:targets_for(unit_id)
Returns enemies within the given unit's attack range.

---

## Building Queries

### ctx:my_buildings(kind?)
Returns own buildings, optionally filtered by kind string.

```lua
local boxes = ctx:my_buildings("TheBox")
```

### ctx:enemy_buildings()
Returns all visible enemy buildings.

---

## Tactical Queries

### ctx:position_at_range(from_x, from_y, target_x, target_y, desired_range)
Finds a passable position exactly `desired_range` tiles (Chebyshev) from target, closest to `from`. Core kiting primitive. Returns `(x, y)` or `(nil, nil)`.

```lua
local kx, ky = ctx:position_at_range(my.x, my.y, enemy.x, enemy.y, 5)
if kx then ctx:move_units({my.id}, kx, ky) end
```

### ctx:safe_positions(unit_id, search_radius)
Returns passable positions within `search_radius` that are outside all enemy attack ranges. Returns table of `{x, y}` tables.

---

## Terrain Queries

### ctx:terrain_at(x, y)
Returns terrain type string or nil: `"Grass"`, `"Dirt"`, `"Sand"`, `"Forest"`, `"Water"`, `"Shallows"`, `"Rock"`, `"Ramp"`, `"Road"`, `"TechRuins"`.

### ctx:elevation_at(x, y)
Returns elevation level (integer) at position.

### ctx:cover_at(x, y)
Returns cover level string: `"None"`, `"Light"`, `"Heavy"`.

### ctx:is_passable(x, y)
Returns true if the tile is passable for the script's faction.

### ctx:can_reach(from_x, from_y, to_x, to_y)
Returns true if a path exists (costs 10 budget).

### ctx:path_length(from_x, from_y, to_x, to_y)
Returns path length in tiles, or nil if unreachable (costs 10 budget).

---

## Economy Queries

### ctx:resources()
Returns a table (free, no budget cost):
```lua
local res = ctx:resources()
-- res.food, res.gpu_cores, res.nfts, res.supply, res.supply_cap
```

### ctx:nearest_deposit(x, y, kind?)
Returns nearest non-depleted resource deposit, optionally filtered by kind (`"Food"`, `"GpuCores"`, `"Nft"`). Returns deposit table or nil.

---

## Game State Queries

### ctx:tick()
Returns current simulation tick (free).

### ctx:map_size()
Returns `(width, height)` of the map.

---

## Command Methods

Commands are free (no budget cost). They emit GameCommands into the command queue. For the same unit, the **last command wins** — scripts run after the FSM, so script commands override FSM commands.

### ctx:move_units(unit_ids, x, y)
Move units to position.
```lua
ctx:move_units({unit.id}, 20, 30)
```

### ctx:attack_units(unit_ids, target_id)
Attack a specific enemy unit.

### ctx:attack_move(unit_ids, x, y)
Attack-move toward position (engage enemies along the way).

### ctx:stop(unit_ids)
Stop units immediately.

### ctx:hold(unit_ids)
Hold position (attack in range, don't chase).

### ctx:gather(unit_ids, deposit_id)
Send workers to gather from a resource deposit.

### ctx:build(builder_id, building_type, x, y)
Order a Pawdler to build. `building_type` is a string: `"TheBox"`, `"CatTree"`, `"FishMarket"`, `"LitterBox"`, `"ServerRack"`, `"ScratchingPost"`, `"CatFlap"`, `"LaserPointer"`.

### ctx:train(building_id, unit_type)
Queue a unit for training. `unit_type` is a string matching UnitKind.

### ctx:ability(unit_id, slot, target_type, x?, y?, entity_id?)
Activate an ability. `slot` is 0-2. `target_type`: `"self"`, `"position"`, `"entity"`.

```lua
ctx:ability(unit.id, 0, "self")                    -- self-cast
ctx:ability(unit.id, 1, "position", 10, 15)        -- ground-target
ctx:ability(unit.id, 2, "entity", nil, nil, enemy.id) -- unit-target
```

### ctx:research(building_id, upgrade_type)
Start researching an upgrade at a ScratchingPost.

### ctx:cancel_queue(building_id)
Cancel the production queue at a building.

### ctx:cancel_research(building_id)
Cancel ongoing research at a building.

### ctx:set_control_group(group, unit_ids)
Assign units to control group (0-9).

### ctx:rally(building_id, x, y)
Set rally point for a production building.

---

## Behavior Methods (Tier-Gated)

Higher-level composable behaviors available via `ctx.behaviors`. Each returns the number of commands issued. Availability depends on AI tier (determined by ServerRack count).

### Always Available (Tier 0 — Basic)

#### ctx.behaviors:assign_idle_workers()
Auto-assigns idle Pawdlers to nearest resource deposits.

#### ctx.behaviors:attack_move_group(unit_ids, x, y)
Attack-move a group of units to target position.

### Tier 1 — Tactical (1+ ServerRack)

#### ctx.behaviors:focus_fire(attacker_ids, target_id)
Concentrate fire on a single target.

#### ctx.behaviors:kite_squad(unit_ids)
Automatically kite ranged units away from approaching melee.

#### ctx.behaviors:retreat_wounded(threshold)
Move units below HP threshold back toward base.

#### ctx.behaviors:defend_area(unit_ids, cx, cy, radius)
Position units defensively around a center point.

#### ctx.behaviors:harass_economy(raider_ids)
Send raiders to attack enemy workers/economy buildings.

#### ctx.behaviors:scout_pattern(scout_id, waypoints)
Send a scout through a series of waypoints. `waypoints` is `{{x1,y1},{x2,y2},...}`.

#### ctx.behaviors:focus_weakest(unit_ids, range)
Auto-focus fire on the weakest enemy in range.

#### ctx.behaviors:use_ability(unit_id, slot, target_type, x?, y?, entity_id?)
Same as `ctx:ability()` but tier-gated.

#### ctx.behaviors:split_squads(unit_ids)
Split units into `{melee={...}, ranged={...}, support={...}}` tables by role.

#### ctx.behaviors:protect_unit(escort_ids, vip_id, guard_radius?)
Keep escort units within radius of VIP.

#### ctx.behaviors:surround_target(unit_ids, target_id, ring_radius?)
Position units in a ring around a target.

### Tier 2 — Strategic (2+ ServerRacks)

#### ctx.behaviors:auto_produce(building_id, unit_type_str)
Automatically queue unit production if resources allow.

#### ctx.behaviors:balanced_production(building_id)
Produce a balanced army composition from a building.

#### ctx.behaviors:expand_economy(builder_id)
Auto-build economy buildings near base.

#### ctx.behaviors:coordinate_assault(unit_ids, x, y)
70/30 split assault with main force + flank group.

### Tier 3 — Advanced (3+ ServerRacks)

#### ctx.behaviors:research_priority(building_id)
Auto-research upgrades in priority order.

#### ctx.behaviors:adaptive_defense(unit_ids, cx, cy, radius)
Dynamic defense: melee forward, ranged behind, repositions based on threats.

---

## Unit Table Schema

Unit tables returned by queries have these fields:

```lua
{
  id = 12345,           -- entity ID (number)
  kind = "Hisser",      -- UnitKind string
  x = 10, y = 15,       -- grid position
  hp = 70, hp_max = 70, -- current and max health
  speed = 0.12,         -- movement speed
  damage = 14,          -- attack damage
  range = 5.0,          -- attack range
  attack_type = "Ranged", -- "Melee" or "Ranged"
  is_idle = true,
  is_moving = false,
  is_attacking = false,
  is_gathering = false,
  is_dead = false,
}
```

## Building Table Schema

```lua
{
  id = 67890,
  kind = "CatTree",
  x = 30, y = 30,
  owner = 0,
  hp = 300, hp_max = 300,
  under_construction = false,
  construction_progress = 1.0,  -- 0.0-1.0
  production_queue = {},        -- list of queued unit kinds
}
```

## Deposit Table Schema

```lua
{
  id = 11111,
  resource_type = "Food",  -- "Food", "GpuCores", "Nft"
  x = 5, y = 10,
  remaining = 1500,
}
```
