-- @name: combat_micro
-- @events: on_tick
-- @interval: 3

-- Gen 19: Aggressive combat micro — focus fire + conditional retreat.
-- Key change from Gen 18: only retreat when outnumbered, push when ahead.

local my_units = ctx:my_units()
if not my_units then return end

local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

-- Count our combat units (non-worker)
local my_combat = {}
local my_workers = {}
for _, u in ipairs(my_units) do
    if u.kind == "Pawdler" or u.kind == "Scrounger" or u.kind == "Delver" or u.kind == "Ponderer" then
        table.insert(my_workers, u)
    else
        table.insert(my_combat, u)
    end
end

local my_combat_count = #my_combat
local army_advantage = my_combat_count > enemy_count

-- Determine home position
local map_w, map_h = ctx:map_size()
local home_x, home_y = 6, 6
for _, u in ipairs(my_units) do
    if u.x > map_w / 2 then
        home_x, home_y = map_w - 6, map_h - 6
    end
    break
end

local attackers = {}
local retreat_ids = {}

for _, u in ipairs(my_combat) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)

    -- Only retreat if badly wounded AND we don't have army advantage
    if hp_pct < 0.15 and u.attacking and not army_advantage then
        table.insert(retreat_ids, u.id)
    elseif u.attacking then
        table.insert(attackers, u)
    end
end

-- Retreat (only when outnumbered)
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, home_x, home_y)
end

-- Focus fire: all attackers target weakest enemy
if #attackers >= 2 then
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #attackers
    cy = cy / #attackers

    local weak = ctx:weakest_enemy_in_range(cx, cy, 12)
    if weak then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, weak.id)
    end
end
