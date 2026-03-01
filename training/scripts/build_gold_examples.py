#!/usr/bin/env python3
"""Build the 50 gold Lua training examples for Devstral Small 2 fine-tuning.

Categories (10 each):
1. Economy — gather, build, expand, rally workers
2. Combat Basic — attack-move, focus fire, retreat, hold position
3. Combat Advanced — kiting, flanking, ability combos, focus healers
4. Macro — build order, tech path, supply management, scouting
5. Adaptive — react to enemy comp, defensive fallback, counter-strategy

Run: python training/scripts/build_gold_examples.py
Output: training/data/gold_lua_examples.jsonl
"""

import json
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent / "data"
SYSTEM_PROMPT_PATH = DATA_DIR / "system_prompt.txt"
OUTPUT_PATH = DATA_DIR / "gold_lua_examples.jsonl"


def load_system_prompt() -> str:
    return SYSTEM_PROMPT_PATH.read_text().strip()


def make_example(system: str, user: str, assistant: str) -> dict:
    return {
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
            {"role": "assistant", "content": assistant},
        ]
    }


# ============================================================================
# CATEGORY 1: ECONOMY (10 examples)
# ============================================================================

ECONOMY_EXAMPLES = [
    # 1. Send idle workers to nearest food
    (
        "Send idle workers to gather food",
        """\
-- Intent: gather
-- Description: Send all idle Pawdlers to the nearest food deposit

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

for _, w in ipairs(workers) do
    local deposit = ctx:nearest_deposit(w.x, w.y, "Food")
    if deposit then
        ctx:gather({w.id}, deposit.id)
    end
end""",
    ),
    # 2. Build a Cat Tree near base
    (
        "Build a Cat Tree near my base",
        """\
-- Intent: build
-- Description: Order an idle Pawdler to build a Cat Tree near TheBox

local res = ctx:get_resources()
if res.food < 150 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

ctx:build(workers[1].id, "CatTree", base.x + 3, base.y)""",
    ),
    # 3. Auto-assign all idle workers
    (
        "Put all my idle workers to work",
        """\
-- Intent: gather
-- Description: Auto-assign all idle Pawdlers to nearest available resources

ctx.behaviors:assign_idle_workers()""",
    ),
    # 4. Build a Fish Market for economy
    (
        "I need more food income, build a Fish Market",
        """\
-- Intent: build
-- Description: Build a Fish Market near a food deposit for better gathering

local res = ctx:get_resources()
if res.food < 200 then return end

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then
    workers = ctx:my_units("Pawdler")
    if #workers == 0 then return end
end

local builder = workers[1]
local deposit = ctx:nearest_deposit(builder.x, builder.y, "Food")
if deposit then
    ctx:build(builder.id, "FishMarket", deposit.x + 2, deposit.y)
else
    local boxes = ctx:my_buildings("TheBox")
    if #boxes > 0 then
        ctx:build(builder.id, "FishMarket", boxes[1].x - 3, boxes[1].y)
    end
end""",
    ),
    # 5. Rally new units to a forward position
    (
        "Set my Cat Tree rally point to the center of the map",
        """\
-- Intent: rally
-- Description: Set rally point for all Cat Trees to map center

local trees = ctx:my_buildings("CatTree")
if #trees == 0 then return end

local w, h = ctx:map_size()
local cx = math.floor(w / 2)
local cy = math.floor(h / 2)

for _, t in ipairs(trees) do
    ctx:rally(t.id, cx, cy)
end""",
    ),
    # 6. Expand to GPU cores
    (
        "Send workers to mine GPU cores",
        """\
-- Intent: gather
-- Description: Send idle Pawdlers to nearest GPU core deposit

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

for _, w in ipairs(workers) do
    local gpu = ctx:nearest_deposit(w.x, w.y, "GpuCores")
    if gpu and gpu.remaining > 0 then
        ctx:gather({w.id}, gpu.id)
        break
    end
end""",
    ),
    # 7. Train more workers
    (
        "Train 3 more Pawdlers",
        """\
-- Intent: train
-- Description: Queue 3 Pawdlers from TheBox

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end

local res = ctx:get_resources()
local trained = 0

for _, box in ipairs(boxes) do
    while trained < 3 and res.food >= 50 do
        ctx:train(box.id, "Pawdler")
        trained = trained + 1
        res.food = res.food - 50
    end
    if trained >= 3 then break end
end""",
    ),
    # 8. Build supply (Litter Box)
    (
        "I'm supply blocked, build a Litter Box",
        """\
-- Intent: build
-- Description: Build a Litter Box to increase supply cap

local res = ctx:get_resources()
if res.food < 100 then return end

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then
    workers = ctx:my_units("Pawdler")
    if #workers == 0 then return end
end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local litters = ctx:my_buildings("LitterBox")
local offset = #litters * 2

ctx:build(workers[1].id, "LitterBox", base.x + 5 + offset, base.y + 3)""",
    ),
    # 9. Distribute workers evenly across deposits
    (
        "Spread my workers across all food deposits evenly",
        """\
-- Intent: gather
-- Description: Distribute idle workers evenly across all food deposits

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

local deposits = ctx:resource_deposits()
local food_deps = {}
for _, d in ipairs(deposits) do
    if d.resource_type == "Food" and d.remaining > 0 then
        table.insert(food_deps, d)
    end
end

if #food_deps == 0 then return end

for i, w in ipairs(workers) do
    local dep_idx = ((i - 1) % #food_deps) + 1
    ctx:gather({w.id}, food_deps[dep_idx].id)
end""",
    ),
    # 10. Economy expand behavior
    (
        "Expand my economy automatically",
        """\
-- Intent: expand
-- Description: Use the expand economy behavior to auto-build economy buildings

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

ctx.behaviors:expand_economy(workers[1].id)

-- Also put remaining idle workers to work
ctx.behaviors:assign_idle_workers()""",
    ),
]


# ============================================================================
# CATEGORY 2: COMBAT BASIC (10 examples)
# ============================================================================

COMBAT_BASIC_EXAMPLES = [
    # 1. Attack-move all combat units toward enemies
    (
        "Attack! Send all combat units at the enemy",
        """\
-- Intent: attack
-- Description: Attack-move all non-worker units toward enemy centroid

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

local cx, cy = 0, 0
for _, e in ipairs(enemies) do
    cx = cx + e.x
    cy = cy + e.y
end
cx = math.floor(cx / #enemies)
cy = math.floor(cy / #enemies)

local units = ctx:my_units()
local attackers = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(attackers, u.id)
    end
end

if #attackers > 0 then
    ctx:attack_move(attackers, cx, cy)
end""",
    ),
    # 2. Focus fire weakest enemy
    (
        "Focus fire the weakest enemy near my army",
        """\
-- Intent: focus_fire
-- Description: All combat units focus on the weakest visible enemy

local units = ctx:my_units()
local combat_ids = {}
local army_x, army_y = 0, 0
local count = 0

for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
        army_x = army_x + u.x
        army_y = army_y + u.y
        count = count + 1
    end
end

if count == 0 then return end
army_x = math.floor(army_x / count)
army_y = math.floor(army_y / count)

local target = ctx:weakest_enemy_in_range(army_x, army_y, 15)
if target then
    ctx.behaviors:focus_fire(combat_ids, target.id)
end""",
    ),
    # 3. Retreat all units to base
    (
        "Retreat! Fall back to base",
        """\
-- Intent: retreat
-- Description: Move all units back toward TheBox

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local units = ctx:my_units()
local ids = {}
for _, u in ipairs(units) do
    table.insert(ids, u.id)
end

if #ids > 0 then
    ctx:move_units(ids, base.x, base.y)
end""",
    ),
    # 4. Hold position
    (
        "Hold position with all units",
        """\
-- Intent: hold
-- Description: All combat units hold position and engage in range

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids > 0 then
    ctx:hold(combat_ids)
end""",
    ),
    # 5. Attack a specific enemy building
    (
        "Destroy the enemy base",
        """\
-- Intent: attack
-- Description: Send all combat units to attack the nearest enemy building

local enemy_buildings = ctx:enemy_buildings()
if #enemy_buildings == 0 then return end

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids == 0 then return end

-- Find the enemy HQ (TheBox) or nearest building
local target = enemy_buildings[1]
for _, b in ipairs(enemy_buildings) do
    if b.kind == "TheBox" then
        target = b
        break
    end
end

ctx:attack_move(combat_ids, target.x, target.y)""",
    ),
    # 6. Retreat wounded units
    (
        "Pull back any units below half health",
        """\
-- Intent: retreat
-- Description: Retreat all units below 50% HP to base

ctx.behaviors:retreat_wounded(0.5)""",
    ),
    # 7. Stop all units
    (
        "Stop everything, halt all units",
        """\
-- Intent: stop
-- Description: Immediately stop all own units

local units = ctx:my_units()
local ids = {}
for _, u in ipairs(units) do
    table.insert(ids, u.id)
end

if #ids > 0 then
    ctx:stop(ids)
end""",
    ),
    # 8. Attack-move to a position
    (
        "Attack-move my army to the top right corner of the map",
        """\
-- Intent: attack
-- Description: Attack-move all combat units to top-right corner

local w, h = ctx:map_size()

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids > 0 then
    ctx:attack_move(combat_ids, w - 5, h - 5)
end""",
    ),
    # 9. Defend the base
    (
        "Defend my base area",
        """\
-- Intent: defend
-- Description: Set up defense around TheBox using defend_area behavior

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids > 0 then
    ctx.behaviors:defend_area(combat_ids, base.x, base.y, 10)
end""",
    ),
    # 10. Send Nuisances to harass
    (
        "Send my Nuisances to harass the enemy workers",
        """\
-- Intent: harass
-- Description: Use Nuisance units to harass enemy economy

local nuisances = ctx:my_units("Nuisance")
if #nuisances == 0 then return end

local ids = {}
for _, u in ipairs(nuisances) do
    table.insert(ids, u.id)
end

ctx.behaviors:harass_economy(ids)""",
    ),
]


# ============================================================================
# CATEGORY 3: COMBAT ADVANCED (10 examples)
# ============================================================================

COMBAT_ADVANCED_EXAMPLES = [
    # 1. Kite ranged units away from melee
    (
        "Make my Hissers kite away from melee enemies",
        """\
-- Intent: kite
-- Description: Kite all Hisser units away from melee threats

local hissers = ctx:my_units("Hisser")
if #hissers == 0 then return end

local ids = {}
for _, h in ipairs(hissers) do
    table.insert(ids, h.id)
end

ctx.behaviors:kite_squad(ids)""",
    ),
    # 2. Split army and flank
    (
        "Split my army in two and attack from both sides",
        """\
-- Intent: flank
-- Description: Split combat units into two groups and attack from different angles

local units = ctx:my_units()
local combat = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat, u)
    end
end

if #combat < 4 then return end

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Find enemy centroid
local ex, ey = 0, 0
for _, e in ipairs(enemies) do
    ex = ex + e.x
    ey = ey + e.y
end
ex = math.floor(ex / #enemies)
ey = math.floor(ey / #enemies)

-- Split into two groups
local group1, group2 = {}, {}
for i, u in ipairs(combat) do
    if i <= #combat / 2 then
        table.insert(group1, u.id)
    else
        table.insert(group2, u.id)
    end
end

-- Attack from offset positions (flanking)
ctx:attack_move(group1, ex - 5, ey - 5)
ctx:attack_move(group2, ex + 5, ey + 5)""",
    ),
    # 3. Focus fire healers/support first
    (
        "Focus down enemy support units first, then ranged",
        """\
-- Intent: focus_fire
-- Description: Prioritize killing enemy Yowlers (support) then Hissers (ranged)

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids == 0 then return end

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Priority: Yowler > Hisser > anything else
local target = nil
for _, e in ipairs(enemies) do
    if e.kind == "Yowler" then
        target = e
        break
    end
end

if not target then
    for _, e in ipairs(enemies) do
        if e.kind == "Hisser" then
            target = e
            break
        end
    end
end

if not target then
    target = enemies[1]
end

ctx.behaviors:focus_fire(combat_ids, target.id)""",
    ),
    # 4. Surround and destroy
    (
        "Surround that enemy Chonk and take it down",
        """\
-- Intent: surround
-- Description: Surround the nearest enemy Chonk and focus fire it

local enemies = ctx:enemy_units()
local target = nil
for _, e in ipairs(enemies) do
    if e.kind == "Chonk" then
        target = e
        break
    end
end

if not target then return end

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids == 0 then return end

ctx.behaviors:surround_target(combat_ids, target.id, 3)""",
    ),
    # 5. Protect the MechCommander
    (
        "Keep my MechCommander alive, escort it with Chonks",
        """\
-- Intent: protect
-- Description: Assign Chonk units to escort the MechCommander

local mechs = ctx:my_units("MechCommander")
if #mechs == 0 then return end
local vip = mechs[1]

local chonks = ctx:my_units("Chonk")
if #chonks == 0 then return end

local escort_ids = {}
for _, c in ipairs(chonks) do
    table.insert(escort_ids, c.id)
end

ctx.behaviors:protect_unit(escort_ids, vip.id, 4)""",
    ),
    # 6. Split squads by role
    (
        "Organize my army: tanks forward, ranged behind, support in back",
        """\
-- Intent: formation
-- Description: Split army into role groups and position them tactically

local units = ctx:my_units()
local all_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(all_ids, u.id)
    end
end

if #all_ids == 0 then return end

local squads = ctx.behaviors:split_squads(all_ids)

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

local ex, ey = enemies[1].x, enemies[1].y

-- Melee forward, ranged behind, support furthest back
if #squads.melee > 0 then
    ctx:attack_move(squads.melee, ex, ey)
end
if #squads.ranged > 0 then
    local boxes = ctx:my_buildings("TheBox")
    local bx = boxes[1] and boxes[1].x or 5
    local by = boxes[1] and boxes[1].y or 5
    local mid_x = math.floor((bx + ex) / 2)
    local mid_y = math.floor((by + ey) / 2)
    ctx:attack_move(squads.ranged, mid_x, mid_y)
end
if #squads.support > 0 then
    ctx:hold(squads.support)
end""",
    ),
    # 7. Kite with position_at_range
    (
        "Make my Hissers keep exactly 6 tiles from the nearest enemy",
        """\
-- Intent: kite
-- Description: Manual kiting using position_at_range for precise distance control

local hissers = ctx:my_units("Hisser")
if #hissers == 0 then return end

for _, h in ipairs(hissers) do
    local enemy = ctx:nearest_enemy(h.x, h.y)
    if enemy then
        local kx, ky = ctx:position_at_range(h.x, h.y, enemy.x, enemy.y, 6)
        if kx then
            ctx:move_units({h.id}, kx, ky)
        end
    end
end""",
    ),
    # 8. Focus weakest in range
    (
        "Have my army focus the lowest HP enemy nearby",
        """\
-- Intent: focus_fire
-- Description: Focus fire on weakest enemy within range using behavior

local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids > 0 then
    ctx.behaviors:focus_weakest(combat_ids, 12)
end""",
    ),
    # 9. Use ability on enemies
    (
        "Use my MechCommander's first ability on the nearest enemy",
        """\
-- Intent: ability
-- Description: Activate MechCommander slot-0 ability targeting nearest enemy

local mechs = ctx:my_units("MechCommander")
if #mechs == 0 then return end
local mech = mechs[1]

local enemy = ctx:nearest_enemy(mech.x, mech.y)
if not enemy then return end

ctx.behaviors:use_ability(mech.id, 0, "entity", nil, nil, enemy.id)""",
    ),
    # 10. Retreat low HP then re-engage healthy
    (
        "Pull back wounded units but keep healthy ones fighting",
        """\
-- Intent: retreat_selective
-- Description: Retreat units below 30% HP, keep healthy units attacking

local wounded = ctx:wounded_units(0.3)
if #wounded > 0 then
    local boxes = ctx:my_buildings("TheBox")
    local rx, ry = 5, 5
    if #boxes > 0 then
        rx = boxes[1].x
        ry = boxes[1].y
    end

    local wound_ids = {}
    for _, w in ipairs(wounded) do
        table.insert(wound_ids, w.id)
    end
    ctx:move_units(wound_ids, rx, ry)
end

-- Healthy units keep fighting
local healthy = ctx:my_units()
local fight_ids = {}
for _, u in ipairs(healthy) do
    if u.kind ~= "Pawdler" and ctx:hp_pct(u.id) >= 0.3 and u.idle then
        table.insert(fight_ids, u.id)
    end
end

if #fight_ids > 0 then
    local enemies = ctx:enemy_units()
    if #enemies > 0 then
        ctx:attack_move(fight_ids, enemies[1].x, enemies[1].y)
    end
end""",
    ),
]


# ============================================================================
# CATEGORY 4: MACRO (10 examples)
# ============================================================================

MACRO_EXAMPLES = [
    # 1. Opening build order
    (
        "Execute a standard opening: train 3 workers, build a Cat Tree, then a Fish Market",
        """\
-- Intent: build_order
-- Description: Standard opening build order: workers first, then production, then economy

local res = ctx:get_resources()
local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

-- Step 1: Train workers if we have fewer than 6
local worker_count = ctx:count_units("Pawdler")
if worker_count < 6 then
    local needed = math.min(3, 6 - worker_count)
    for i = 1, needed do
        if res.food >= 50 then
            ctx:train(base.id, "Pawdler")
            res.food = res.food - 50
        end
    end
end

-- Step 2: Build Cat Tree if we don't have one
local trees = ctx:my_buildings("CatTree")
if #trees == 0 and res.food >= 150 then
    local builder = ctx:idle_units("Pawdler")
    if #builder > 0 then
        ctx:build(builder[1].id, "CatTree", base.x + 3, base.y + 2)
    end
    return
end

-- Step 3: Build Fish Market if we don't have one
local markets = ctx:my_buildings("FishMarket")
if #markets == 0 and res.food >= 200 then
    local builder = ctx:idle_units("Pawdler")
    if #builder > 0 then
        local food_dep = ctx:nearest_deposit(base.x, base.y, "Food")
        if food_dep then
            ctx:build(builder[1].id, "FishMarket", food_dep.x + 2, food_dep.y)
        end
    end
end""",
    ),
    # 2. Tech up to advanced units
    (
        "Tech up to Scratching Post for advanced units",
        """\
-- Intent: tech
-- Description: Build a Scratching Post if we have a Cat Tree and enough resources

local res = ctx:get_resources()
if res.food < 300 or res.gpu_cores < 100 then return end

local trees = ctx:my_buildings("CatTree")
if #trees == 0 then return end

local posts = ctx:my_buildings("ScratchingPost")
if #posts > 0 then return end

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

local base_tree = trees[1]
ctx:build(workers[1].id, "ScratchingPost", base_tree.x + 3, base_tree.y + 3)""",
    ),
    # 3. Supply management
    (
        "Check if I'm near supply cap and build Litter Boxes if needed",
        """\
-- Intent: supply
-- Description: Auto-build Litter Boxes when supply is close to cap

local res = ctx:get_resources()
local supply_headroom = res.supply_cap - res.supply

if supply_headroom > 4 then return end
if res.food < 100 then return end

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local litters = ctx:my_buildings("LitterBox")
local offset = #litters * 2

ctx:build(workers[1].id, "LitterBox", base.x - 3, base.y + offset)""",
    ),
    # 4. Scout the map
    (
        "Send a fast unit to scout the map corners",
        """\
-- Intent: scout
-- Description: Send a Nuisance on a scouting patrol of all map corners

local scouts = ctx:my_units("Nuisance")
if #scouts == 0 then
    scouts = ctx:my_units("Mouser")
    if #scouts == 0 then return end
end

local scout = scouts[1]
local w, h = ctx:map_size()

local waypoints = {
    {x = 5, y = 5},
    {x = w - 5, y = 5},
    {x = w - 5, y = h - 5},
    {x = 5, y = h - 5},
}

ctx.behaviors:scout_pattern(scout.id, waypoints)""",
    ),
    # 5. Balanced production
    (
        "Keep producing a balanced mix of units",
        """\
-- Intent: produce
-- Description: Auto-produce a balanced unit mix from all Cat Trees

local trees = ctx:my_buildings("CatTree")
if #trees == 0 then return end

for _, tree in ipairs(trees) do
    if not tree.under_construction then
        ctx.behaviors:balanced_production(tree.id)
    end
end""",
    ),
    # 6. Build Server Rack for tech
    (
        "Build a Server Rack so I can upgrade my AI",
        """\
-- Intent: build
-- Description: Build a Server Rack near base for tech upgrades

local res = ctx:get_resources()
if res.food < 200 or res.gpu_cores < 50 then return end

local racks = ctx:my_buildings("ServerRack")
if #racks > 0 then return end

local workers = ctx:idle_units("Pawdler")
if #workers == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

ctx:build(workers[1].id, "ServerRack", base.x - 3, base.y + 3)""",
    ),
    # 7. Research upgrades
    (
        "Research Sharper Claws at the Server Rack",
        """\
-- Intent: research
-- Description: Start Sharper Claws research if a Server Rack is available

local racks = ctx:my_buildings("ServerRack")
if #racks == 0 then return end

for _, rack in ipairs(racks) do
    if not rack.under_construction and not rack.producing then
        ctx:research(rack.id, "SharperClaws")
        return
    end
end""",
    ),
    # 8. Auto-research priority
    (
        "Research whatever upgrade is most important right now",
        """\
-- Intent: research
-- Description: Use the research_priority behavior to auto-pick best research

local racks = ctx:my_buildings("ServerRack")
if #racks == 0 then return end

for _, rack in ipairs(racks) do
    if not rack.under_construction then
        ctx.behaviors:research_priority(rack.id)
    end
end""",
    ),
    # 9. Mass produce specific unit
    (
        "Spam Hissers from all production buildings",
        """\
-- Intent: produce
-- Description: Train Hissers from every available Cat Tree

local trees = ctx:my_buildings("CatTree")
if #trees == 0 then return end

local res = ctx:get_resources()

for _, tree in ipairs(trees) do
    if not tree.under_construction and not tree.producing then
        ctx.behaviors:auto_produce(tree.id, "Hisser")
    end
end""",
    ),
    # 10. Control groups
    (
        "Put my Chonks in group 1 and Hissers in group 2",
        """\
-- Intent: control_group
-- Description: Assign Chonks to control group 1 and Hissers to control group 2

local chonks = ctx:my_units("Chonk")
local hissers = ctx:my_units("Hisser")

if #chonks > 0 then
    local ids = {}
    for _, c in ipairs(chonks) do
        table.insert(ids, c.id)
    end
    ctx:set_control_group(1, ids)
end

if #hissers > 0 then
    local ids = {}
    for _, h in ipairs(hissers) do
        table.insert(ids, h.id)
    end
    ctx:set_control_group(2, ids)
end""",
    ),
]


# ============================================================================
# CATEGORY 5: ADAPTIVE (10 examples)
# ============================================================================

ADAPTIVE_EXAMPLES = [
    # 1. React to enemy rush
    (
        "I'm being rushed! Emergency defense",
        """\
-- Intent: defend
-- Description: Emergency defense: pull workers, train units, set up defense

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

-- Pull all units to defend
local all_units = ctx:my_units()
local all_ids = {}
for _, u in ipairs(all_units) do
    table.insert(all_ids, u.id)
end

ctx.behaviors:defend_area(all_ids, base.x, base.y, 8)

-- Emergency unit production
local res = ctx:get_resources()
local trees = ctx:my_buildings("CatTree")
for _, tree in ipairs(trees) do
    if not tree.under_construction and res.food >= 50 then
        ctx:train(tree.id, "Nuisance")
        res.food = res.food - 50
    end
end""",
    ),
    # 2. Counter ranged heavy composition
    (
        "The enemy has a lot of ranged units, what should I do?",
        """\
-- Intent: counter
-- Description: Counter ranged-heavy enemy by closing distance with melee units

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Count enemy ranged
local ranged_count = 0
for _, e in ipairs(enemies) do
    if e.attack_type == "Ranged" then
        ranged_count = ranged_count + 1
    end
end

-- If enemy is ranged-heavy, rush with melee
if ranged_count > #enemies / 2 then
    local melee = {}
    local units = ctx:my_units()
    for _, u in ipairs(units) do
        if u.kind == "Chonk" or u.kind == "Nuisance" or u.kind == "Mouser" then
            table.insert(melee, u.id)
        end
    end

    if #melee > 0 then
        local target = enemies[1]
        ctx:attack_move(melee, target.x, target.y)
    end

    -- Also train more melee
    local trees = ctx:my_buildings("CatTree")
    local res = ctx:get_resources()
    for _, tree in ipairs(trees) do
        if not tree.under_construction and res.food >= 75 then
            ctx:train(tree.id, "Chonk")
        end
    end
end""",
    ),
    # 3. Defensive fallback when outnumbered
    (
        "We're outnumbered, play defensive until we build up",
        """\
-- Intent: defend
-- Description: Defensive stance when outnumbered — hold near base, keep producing

local my_count = ctx:count_units(nil) - ctx:count_units("Pawdler")
local enemies = ctx:enemy_units()

-- Only go defensive if outnumbered
if #enemies <= my_count then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

-- Pull all combat units to defensive perimeter
local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids > 0 then
    ctx.behaviors:adaptive_defense(combat_ids, base.x, base.y, 8)
end

-- Ramp up production
local trees = ctx:my_buildings("CatTree")
local res = ctx:get_resources()
for _, tree in ipairs(trees) do
    if not tree.under_construction then
        ctx.behaviors:balanced_production(tree.id)
    end
end""",
    ),
    # 4. Exploit enemy without buildings
    (
        "The enemy has no production buildings visible, push now!",
        """\
-- Intent: attack
-- Description: All-in push when enemy has no visible production

local enemy_buildings = ctx:enemy_buildings()
local has_production = false
for _, b in ipairs(enemy_buildings) do
    if b.kind == "CatTree" or b.kind == "ScratchingPost" then
        has_production = true
        break
    end
end

if has_production then return end

-- All-in attack
local units = ctx:my_units()
local combat_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat_ids, u.id)
    end
end

if #combat_ids == 0 then return end

-- Target enemy HQ
local target = nil
for _, b in ipairs(enemy_buildings) do
    if b.kind == "TheBox" then
        target = b
        break
    end
end

if target then
    ctx.behaviors:coordinate_assault(combat_ids, target.x, target.y)
else
    local enemies = ctx:enemy_units()
    if #enemies > 0 then
        ctx.behaviors:coordinate_assault(combat_ids, enemies[1].x, enemies[1].y)
    end
end""",
    ),
    # 5. Respond to harassment
    (
        "Enemy units are attacking my workers! Protect them",
        """\
-- Intent: defend
-- Description: Send nearby combat units to protect workers under attack

local workers = ctx:my_units("Pawdler")
local threatened_workers = {}

for _, w in ipairs(workers) do
    local threats = ctx:threats_to(w.id)
    if #threats > 0 then
        table.insert(threatened_workers, w)
    end
end

if #threatened_workers == 0 then return end

-- Find combat units near threatened workers
local target_w = threatened_workers[1]
local nearby = ctx:enemies_in_range(target_w.x, target_w.y, 10)

if #nearby == 0 then return end

local units = ctx:my_units()
local defenders = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(defenders, u.id)
    end
end

if #defenders > 0 then
    ctx.behaviors:focus_fire(defenders, nearby[1].id)
end

-- Also move threatened workers away
for _, w in ipairs(threatened_workers) do
    local safe = ctx:safe_positions(w.id, 5)
    if #safe > 0 then
        ctx:move_units({w.id}, safe[1].x, safe[1].y)
    end
end""",
    ),
    # 6. Adapt production based on what we see
    (
        "Build units that counter what the enemy has",
        """\
-- Intent: counter_produce
-- Description: Analyze enemy composition and produce counter units

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

local melee_count, ranged_count, air_count = 0, 0, 0
for _, e in ipairs(enemies) do
    if e.kind == "FlyingFox" then
        air_count = air_count + 1
    elseif e.attack_type == "Ranged" then
        ranged_count = ranged_count + 1
    else
        melee_count = melee_count + 1
    end
end

local trees = ctx:my_buildings("CatTree")
if #trees == 0 then return end
local res = ctx:get_resources()

-- Counter logic: melee-heavy → train Hissers, ranged-heavy → train Chonks
local counter_unit = "Nuisance"
if melee_count > ranged_count and melee_count > air_count then
    counter_unit = "Hisser"
elseif ranged_count > melee_count then
    counter_unit = "Chonk"
elseif air_count > 2 then
    counter_unit = "Hisser"
end

for _, tree in ipairs(trees) do
    if not tree.under_construction and not tree.producing and res.food >= 75 then
        ctx:train(tree.id, counter_unit)
        res.food = res.food - 75
    end
end""",
    ),
    # 7. Late game push with siege
    (
        "I have a big army, do a coordinated push with siege units",
        """\
-- Intent: assault
-- Description: Coordinated late-game assault with Catnappers protected by the army

local catnappers = ctx:my_units("Catnapper")
if #catnappers == 0 then return end

local enemy_buildings = ctx:enemy_buildings()
if #enemy_buildings == 0 then
    local enemies = ctx:enemy_units()
    if #enemies == 0 then return end
end

-- Find target
local target_x, target_y
if #enemy_buildings > 0 then
    local eb = enemy_buildings[1]
    target_x, target_y = eb.x, eb.y
else
    local enemies = ctx:enemy_units()
    target_x, target_y = enemies[1].x, enemies[1].y
end

-- Protect siege units with combat escort
local capper_ids = {}
for _, c in ipairs(catnappers) do
    table.insert(capper_ids, c.id)
end

local units = ctx:my_units()
local escort_ids = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" and u.kind ~= "Catnapper" then
        table.insert(escort_ids, u.id)
    end
end

-- Escort siege, then assault
if #escort_ids > 0 then
    ctx.behaviors:protect_unit(escort_ids, catnappers[1].id, 5)
end

ctx.behaviors:coordinate_assault(capper_ids, target_x, target_y)""",
    ),
    # 8. Retreat and regroup if losing
    (
        "We lost that fight, retreat and rebuild",
        """\
-- Intent: retreat_rebuild
-- Description: Full retreat, heal up, resume production

-- Retreat all units
local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base = boxes[1]

local units = ctx:my_units()
local all_ids = {}
for _, u in ipairs(units) do
    table.insert(all_ids, u.id)
end

if #all_ids > 0 then
    ctx:move_units(all_ids, base.x, base.y)
end

-- Resume production
local trees = ctx:my_buildings("CatTree")
local res = ctx:get_resources()
for _, tree in ipairs(trees) do
    if not tree.under_construction then
        ctx.behaviors:balanced_production(tree.id)
    end
end

-- Assign idle workers
ctx.behaviors:assign_idle_workers()""",
    ),
    # 9. Use terrain advantage
    (
        "Position my ranged units on high ground",
        """\
-- Intent: position
-- Description: Move Hissers to the highest elevation nearby for damage bonus

local hissers = ctx:my_units("Hisser")
if #hissers == 0 then return end

for _, h in ipairs(hissers) do
    local best_x, best_y = h.x, h.y
    local best_elev = ctx:elevation_at(h.x, h.y)

    -- Search nearby tiles for higher ground
    for dy = -5, 5 do
        for dx = -5, 5 do
            local tx, ty = h.x + dx, h.y + dy
            if ctx:is_passable(tx, ty) then
                local elev = ctx:elevation_at(tx, ty)
                if elev > best_elev then
                    local cover = ctx:cover_at(tx, ty)
                    best_elev = elev
                    best_x = tx
                    best_y = ty
                end
            end
        end
    end

    if best_x ~= h.x or best_y ~= h.y then
        ctx:move_units({h.id}, best_x, best_y)
    end
end""",
    ),
    # 10. NFT mine contest
    (
        "Capture and hold the NFT mines on the map",
        """\
-- Intent: objective
-- Description: Send units to control all NFT resource deposits

local deposits = ctx:resource_deposits()
local nft_mines = {}
for _, d in ipairs(deposits) do
    if d.resource_type == "Nfts" then
        table.insert(nft_mines, d)
    end
end

if #nft_mines == 0 then return end

local units = ctx:my_units()
local combat = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(combat, u)
    end
end

if #combat == 0 then return end

-- Distribute combat units across NFT mines
local per_mine = math.max(1, math.floor(#combat / #nft_mines))
local assigned = 0

for _, mine in ipairs(nft_mines) do
    local group = {}
    for i = assigned + 1, math.min(assigned + per_mine, #combat) do
        table.insert(group, combat[i].id)
    end
    assigned = assigned + #group

    if #group > 0 then
        ctx:attack_move(group, mine.x, mine.y)
    end
end""",
    ),
]


def main():
    system = load_system_prompt()

    all_examples = []
    categories = [
        ("Economy", ECONOMY_EXAMPLES),
        ("Combat Basic", COMBAT_BASIC_EXAMPLES),
        ("Combat Advanced", COMBAT_ADVANCED_EXAMPLES),
        ("Macro", MACRO_EXAMPLES),
        ("Adaptive", ADAPTIVE_EXAMPLES),
    ]

    for cat_name, examples in categories:
        for user_msg, lua_script in examples:
            all_examples.append(make_example(system, user_msg, lua_script))

    with open(OUTPUT_PATH, "w") as f:
        for ex in all_examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")

    print(f"Wrote {len(all_examples)} examples to {OUTPUT_PATH}")

    # Print category breakdown
    for cat_name, examples in categories:
        print(f"  {cat_name}: {len(examples)}")


if __name__ == "__main__":
    main()
