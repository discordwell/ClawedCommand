-- @name: push_finish
-- @events: on_tick
-- @interval: 10

-- Gen 20: Push to finish — when we have army advantage, send idle
-- combat units toward enemy buildings to close out the game.
-- Only activates when we outnumber the enemy.

local my_units = ctx:my_units()
if not my_units then return end

local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local enemy_buildings = ctx:enemy_buildings()
if not enemy_buildings or #enemy_buildings == 0 then return end

-- Count our combat units
local combat_count = 0
local idle_combat = {}
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        combat_count = combat_count + 1
        -- Units that are idle or just standing around (not actively attacking)
        if not u.attacking and not u.moving then
            table.insert(idle_combat, u)
        end
    end
end

-- Only push when we have clear advantage (2:1 or better)
if combat_count < 4 or combat_count < enemy_count * 2 then return end

-- No idle combat units to send
if #idle_combat == 0 then return end

-- Find nearest enemy building to our army centroid
local cx, cy = 0, 0
for _, u in ipairs(idle_combat) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #idle_combat
cy = cy / #idle_combat

local best_building = nil
local best_dist = 999999
for _, b in ipairs(enemy_buildings) do
    local dx = b.x - cx
    local dy = b.y - cy
    local dist = dx * dx + dy * dy
    if dist < best_dist then
        best_dist = dist
        best_building = b
    end
end

if best_building then
    local ids = {}
    for _, u in ipairs(idle_combat) do
        table.insert(ids, u.id)
    end
    ctx:attack_move(ids, best_building.x, best_building.y)
end
