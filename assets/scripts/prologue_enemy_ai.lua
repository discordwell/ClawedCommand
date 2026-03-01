-- @name: prologue_enemy_ai
-- @events: on_tick
-- @interval: 5

-- Simple feral monkey AI for the prologue tutorial.
-- No retreat, no formation — they're feral animals, not a coordinated army.
-- Pack Leader (Chonk) charges straight in.

local my_units = ctx:my_units()
if not my_units then return end
if #my_units == 0 then return end

local enemies = ctx:enemy_units()
if not enemies then return end
if #enemies == 0 then return end

-- All feral units attack-move toward the nearest player unit
local all_ids = {}
for _, u in ipairs(my_units) do
    table.insert(all_ids, u.id)
end

-- Find centroid of enemy (player) army as the attack target
local cx, cy = 0, 0
for _, e in ipairs(enemies) do
    cx = cx + e.x
    cy = cy + e.y
end
cx = cx / #enemies
cy = cy / #enemies

-- Attack-move all units toward the player centroid
ctx:attack_move(all_ids, math.floor(cx), math.floor(cy))
