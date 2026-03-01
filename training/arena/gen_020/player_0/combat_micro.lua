-- @name: combat_micro
-- @events: on_tick
-- @interval: 3

-- Gen 18: Pure combat micro — no production, no economy interference.
-- 1. Focus fire: all attackers target the weakest nearby enemy
-- 2. Retreat: pull badly wounded units back toward base
-- 3. Kite: keep ranged units at max range from nearest melee enemy

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local map_cx = map_w / 2
local map_cy = map_h / 2

-- Determine our spawn quadrant (P0 = top-left, P1 = bottom-right)
local home_x, home_y = 6, 6 -- default P0
for _, u in ipairs(my_units) do
    if u.x > map_cx then
        home_x, home_y = map_w - 6, map_h - 6
    end
    break
end

-- Collect units by role
local attackers = {}  -- currently fighting
local wounded = {}    -- low HP, should retreat
local ranged_idle = {} -- ranged units that could kite

for _, u in ipairs(my_units) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)

    if hp_pct < 0.25 and u.attacking then
        -- Badly wounded and in combat — retreat
        table.insert(wounded, u)
    elseif u.attacking then
        table.insert(attackers, u)
    end
end

-- 1. RETREAT wounded units toward home
if #wounded > 0 then
    local retreat_ids = {}
    for _, u in ipairs(wounded) do
        table.insert(retreat_ids, u.id)
    end
    ctx:move_units(retreat_ids, home_x, home_y)
end

-- 2. FOCUS FIRE: redirect attackers to weakest enemy nearby
if #attackers >= 2 then
    -- Find centroid of our attackers
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #attackers
    cy = cy / #attackers

    local weak = ctx:weakest_enemy_in_range(cx, cy, 10)
    if weak then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, weak.id)
    end
end
