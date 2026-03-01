# ClawedCommand Lua Script API Reference

Scripts run inside a Luau sandbox with a global `ctx` object. All methods use colon syntax: `ctx:method(args)`. Scripts execute at 10Hz simulation rate, triggered by events.

## Script Annotations

Place in comment header:
```lua
-- @name: my_script
-- @events: on_tick, on_enemy_spotted, on_unit_attacked
-- @interval: 5
```

- `@name` — Script identifier (default: filename without .lua)
- `@events` — Comma-separated triggers (default: `on_tick`)
- `@interval` — Ticks between on_tick runs (default: 5 = 0.5s)

### Events
| Event | Fires when |
|-------|------------|
| `on_tick` | Every `@interval` ticks |
| `on_enemy_spotted` | New enemy unit becomes visible |
| `on_unit_attacked` | Own unit takes damage |
| `on_unit_idle` | Own unit finishes its command |
| `on_unit_died` | Own unit dies or is removed |

## Unit Queries

### ctx:my_units(kind?)
Returns table of own living units. Optional `kind` filter string.
```lua
local all = ctx:my_units()
local workers = ctx:my_units("Pawdler")
```

### ctx:enemy_units()
Returns table of visible enemy units.

### ctx:enemies_in_range(x, y, range)
Returns enemies within Euclidean distance of (x, y).

### ctx:nearest_enemy(x, y)
Returns single closest enemy unit, or nil.

### ctx:threats_to(unit_id)
Returns enemies whose attack range reaches the given unit.

### ctx:targets_for(unit_id)
Returns enemies within the given unit's attack range.

### ctx:idle_units(kind?)
Returns own idle units (not moving, attacking, or gathering). Optional kind filter.

### ctx:wounded_units(threshold)
Returns own units below HP percentage threshold (0.0-1.0).
```lua
local hurt = ctx:wounded_units(0.5) -- units below 50% HP
```

### ctx:units_by_state(state)
Returns units in a specific state. States: `"Moving"`, `"Attacking"`, `"Idle"`, `"Gathering"`.

### ctx:count_units(kind?)
Returns integer count of own living units. Optional kind filter.

### ctx:army_supply()
Returns current army supply usage (integer).

### ctx:weakest_enemy_in_range(x, y, range)
Returns the lowest-HP enemy within range, or nil.

### ctx:strongest_enemy_in_range(x, y, range)
Returns the highest-HP enemy within range, or nil.

### ctx:hp_pct(unit_id)
Returns HP fraction 0.0-1.0 for a unit, or nil if not found.

### ctx:distance_squared_between(id_a, id_b)
Returns squared distance between two entities, or nil.

### ctx:distance_squared_to_nearest_enemy(unit_id)
Returns squared distance to nearest enemy from this unit, or nil.

## Unit Table Fields

Every unit returned from queries has these fields:
```lua
unit = {
    id = 12345,         -- u64 entity ID (pass to commands)
    kind = "Hisser",    -- UnitKind string
    x = 10, y = 15,     -- grid position
    owner = 0,          -- player_id (0 or 1)
    hp = 70,            -- current health
    hp_max = 100,       -- max health
    speed = 0.12,       -- movement speed
    damage = 14,        -- attack damage
    range = 5,          -- attack range
    attack_speed = 12,  -- ticks between attacks
    attack_type = "Ranged", -- "Melee" or "Ranged"
    moving = false,
    attacking = false,
    idle = true,
    dead = false,
    gathering = false,
}
```

## Building Queries

### ctx:my_buildings(kind?)
Returns own buildings. Optional kind filter.
```lua
local boxes = ctx:my_buildings("TheBox")
```

### ctx:enemy_buildings()
Returns visible enemy buildings.

### Building Table Fields
```lua
building = {
    id = 67890,
    kind = "CatTree",
    x = 5, y = 5,
    owner = 0,
    hp = 300,
    hp_max = 300,
    producing = false,  -- true if training a unit
    under_construction = false,
}
```

## Economy Queries

### ctx:resources()
Returns own resource state.
```lua
local res = ctx:resources()
-- res.food, res.gpu_cores, res.nfts, res.supply, res.supply_cap
```

### ctx:nearest_deposit(x, y, type?)
Returns closest resource deposit. Optional type filter: `"Food"`, `"GpuCores"`, `"Nft"`.

### Deposit Table Fields
```lua
deposit = {
    id = 11111,
    x = 20, y = 20,
    resource_type = "Food",
    remaining = 1200,
}
```

## Terrain Queries

### ctx:terrain_at(x, y)
Returns terrain type string: `"Grass"`, `"Forest"`, `"Water"`, `"Mountain"`, `"Road"`, `"Bridge"`, `"Ford"`, `"Sand"`, `"TechRuins"`, `"Swamp"`.

### ctx:elevation_at(x, y)
Returns elevation integer at position.

### ctx:cover_at(x, y)
Returns cover level string: `"None"`, `"Light"`, `"Heavy"`.

### ctx:is_passable(x, y)
Returns boolean — whether a unit can walk on this tile.

### ctx:can_reach(from_x, from_y, to_x, to_y)
Returns boolean — whether a path exists between two points.

### ctx:path_length(from_x, from_y, to_x, to_y)
Returns integer path length, or nil if unreachable.

## Tactical Queries

### ctx:position_at_range(from_x, from_y, target_x, target_y, desired_range)
Returns (x, y) position that maintains desired range from target. For kiting.
```lua
local kx, ky = ctx:position_at_range(me.x, me.y, enemy.x, enemy.y, 5)
if kx then ctx:move_units({me.id}, kx, ky) end
```

### ctx:safe_positions(unit_id, search_radius)
Returns table of `{x, y}` positions outside all enemy attack ranges.

## Game State

### ctx:tick()
Returns current simulation tick (u64).

### ctx:map_size()
Returns width, height as two values.
```lua
local w, h = ctx:map_size()
```

## Commands

### ctx:move_units(unit_ids, x, y)
Move units to grid position.
```lua
ctx:move_units({unit1.id, unit2.id}, 30, 30)
```

### ctx:attack_units(unit_ids, target_id)
Direct attack a specific target.

### ctx:attack_move(unit_ids, x, y)
Attack-move toward position (attack anything encountered).

### ctx:stop(unit_ids)
Halt units immediately.

### ctx:hold(unit_ids)
Hold position — attack in range but don't chase.

### ctx:gather(unit_ids, deposit_id)
Send units to gather from a resource deposit.

### ctx:build(builder_id, building_type, x, y)
Order a Pawdler to construct a building.
Building types: `"TheBox"`, `"CatTree"`, `"FishMarket"`, `"LitterBox"`, `"ServerRack"`, `"ScratchingPost"`, `"CatFlap"`, `"LaserPointer"`.

### ctx:train(building_id, unit_type)
Queue unit training at a building.
Unit types: `"Pawdler"`, `"Nuisance"`, `"Chonk"`, `"FlyingFox"`, `"Hisser"`, `"Yowler"`, `"Mouser"`, `"Catnapper"`, `"FerretSapper"`, `"MechCommander"`.

### ctx:ability(unit_id, slot, target_type, x?, y?, entity_id?)
Activate a unit ability. target_type: `"self"`, `"position"`, `"entity"`.

### ctx:research(building_id, upgrade_type)
Start researching an upgrade at a building.

### ctx:cancel_queue(building_id)
Cancel production queue.

### ctx:cancel_research(building_id)
Cancel ongoing research.

### ctx:set_control_group(group, unit_ids)
Assign units to a control group (0-9).

### ctx:rally(building_id, x, y)
Set rally point for a production building.

## Compatibility Aliases

These legacy names also work:
- `ctx:get_units()` = `ctx:my_units()`
- `ctx:get_visible_enemies()` = `ctx:enemy_units()`
- `ctx:get_buildings()` = `ctx:my_buildings()`
- `ctx:get_resources()` = `ctx:resources()`
- `ctx:attack(ids, target)` = `ctx:attack_units(ids, target)`
- `ctx:gather_resource(ids, dep)` = `ctx:gather(ids, dep)`
- `ctx:train_unit(bld, kind)` = `ctx:train(bld, kind)`

## Behavior Composites (ctx.behaviors)

Higher-level behaviors built on primitives. All return command count (integer). Tier-gated by ServerRack count (arena gives Advanced tier by default).

### Basic (always available)
- `ctx.behaviors:assign_idle_workers()` — Send idle Pawdlers to nearest deposits
- `ctx.behaviors:attack_move_group(unit_ids, x, y)` — Smart group movement (melee front, ranged back)

### Tactical (1+ ServerRack)
- `ctx.behaviors:focus_fire(attacker_ids, target_id)` — All units attack one target
- `ctx.behaviors:kite_squad(unit_ids)` — Ranged units maintain attack range distance
- `ctx.behaviors:retreat_wounded(threshold)` — Move units below HP% to safe positions
- `ctx.behaviors:defend_area(unit_ids, cx, cy, radius)` — Hold + attack within radius
- `ctx.behaviors:harass_economy(raider_ids)` — Attack enemy workers or economy buildings
- `ctx.behaviors:scout_pattern(scout_id, waypoints)` — Move scout through waypoint list (`{{x=10,y=10},{x=20,y=20}}`)
- `ctx.behaviors:focus_weakest(unit_ids, range)` — Focus-fire lowest HP enemy in range
- `ctx.behaviors:use_ability(unit_id, slot, target_type, x?, y?, entity_id?)` — Smart ability activation
- `ctx.behaviors:split_squads(unit_ids)` — Returns `{melee={ids}, ranged={ids}, support={ids}}`
- `ctx.behaviors:protect_unit(escort_ids, vip_id, guard_radius?)` — Escort formation (default radius 5)
- `ctx.behaviors:surround_target(unit_ids, target_id, ring_radius?)` — Ring formation (default radius 3)

### Strategic (2+ ServerRacks)
- `ctx.behaviors:auto_produce(building_id, unit_type)` — Train if affordable
- `ctx.behaviors:balanced_production(building_id)` — Train least-represented unit type
- `ctx.behaviors:expand_economy(builder_id)` — Build FishMarkets near deposits + LitterBoxes
- `ctx.behaviors:coordinate_assault(unit_ids, x, y)` — 70/30 main/flank split attack

### Advanced (3+ ServerRacks)
- `ctx.behaviors:research_priority(building_id)` — Auto-queue best available research
- `ctx.behaviors:adaptive_defense(unit_ids, cx, cy, radius)` — Dynamic defense positioning

## Sandbox Limits
- **Instruction limit**: 10,000 Lua instructions per execution
- **No I/O**: `os` and `debug` libraries removed
- **No globals mutation**: Luau sandbox freezes global environment
