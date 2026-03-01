-- @name: focus_fire
-- @events: on_tick
-- @interval: 3

-- Gen 17: Focus fire — redirect all attacking units to the weakest
-- enemy in range. Only affects units already in combat (attacking),
-- never pulls units away from FSM army movements.

local my_units = ctx:my_units()
if not my_units then return end

-- Collect units that are currently attacking (already in combat)
local attackers = {}
for _, u in ipairs(my_units) do
    if u.attacking then
        table.insert(attackers, u)
    end
end

if #attackers == 0 then return end

-- Find the centroid of our attacking units
local cx, cy = 0, 0
for _, u in ipairs(attackers) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #attackers
cy = cy / #attackers

-- Find weakest enemy near our attackers (range 12 covers most combats)
local weak = ctx:weakest_enemy_in_range(cx, cy, 12)
if not weak then return end

-- Only focus fire if the target is low-ish HP (worth finishing off)
-- This avoids constantly switching targets mid-fight
if weak.hp / math.max(weak.hp_max, 1) > 0.7 then return end

-- Redirect all attackers to the weakest enemy
local ids = {}
for _, u in ipairs(attackers) do
    table.insert(ids, u.id)
end
ctx:attack_units(ids, weak.id)
