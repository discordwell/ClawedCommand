-- basic_attack: Attack-move combat units toward nearest enemy
-- Intents: attack, fight, charge

local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Find centroid of visible enemies
local cx, cy = 0, 0
for _, e in ipairs(enemies) do
    cx = cx + e.x
    cy = cy + e.y
end
cx = math.floor(cx / #enemies)
cy = math.floor(cy / #enemies)

-- Gather all non-worker combat units
local units = ctx:my_units()
local attackers = {}
for _, u in ipairs(units) do
    if u.kind ~= "Pawdler" then
        table.insert(attackers, u.id)
    end
end

if #attackers > 0 then
    ctx:attack_move(attackers, cx, cy)
end
