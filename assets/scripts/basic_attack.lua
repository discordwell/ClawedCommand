-- basic_attack: Attack-move all selected units toward the nearest enemy
-- Intents: attack, fight, charge, go get them
local units = ctx:get_units()
local enemies = ctx:get_visible_enemies()

if #units == 0 or #enemies == 0 then return end

-- Find nearest enemy to first unit
local best_dist = math.huge
local best_enemy = nil
local u = units[1]
for _, e in ipairs(enemies) do
    local dx = e.x - u.x
    local dy = e.y - u.y
    local dist = dx * dx + dy * dy
    if dist < best_dist then
        best_dist = dist
        best_enemy = e
    end
end

if best_enemy then
    local ids = {}
    for _, unit in ipairs(units) do
        table.insert(ids, unit.id)
    end
    ctx:attack_move(ids, best_enemy.x, best_enemy.y)
end
