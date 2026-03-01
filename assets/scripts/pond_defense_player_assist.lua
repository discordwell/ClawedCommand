-- @name: pond_defense_player_assist
-- @events: on_tick
-- @interval: 3

-- Defensive player assist for Pond Defense mission.
-- Detects which pond is under attack and rallies idle units.
-- Focus fires closest enemy to army centroid.
-- Retreats wounded units toward Kelpie.
-- Defensive only — no push toward enemy spawns.

local my_units = ctx:my_units()
if not my_units then return end
if #my_units == 0 then return end

local enemies = ctx:enemy_units()
if not enemies then return end

-- Pond center positions (passable tiles near each pond)
local NORTH_POND = {x = 10, y = 14}
local LILY_POND  = {x = 13, y = 21}
local SOUTH_POND = {x = 10, y = 33}
local KELPIE_BASE = {x = 5, y = 24}
local THREAT_RADIUS_SQ = 100  -- 10 tile radius squared

-- Count enemies near each pond
local function count_near(pos, enemy_list)
    local n = 0
    for _, e in ipairs(enemy_list) do
        local dx = e.x - pos.x
        local dy = e.y - pos.y
        if dx * dx + dy * dy < THREAT_RADIUS_SQ then
            n = n + 1
        end
    end
    return n
end

-- Step 1: Retreat wounded units (HP < 30%) toward Kelpie
local wounded_ids = {}
local wounded = ctx:wounded_units()
if wounded then
    for _, u in ipairs(wounded) do
        local hp = ctx:hp_pct(u.id)
        if hp and hp < 0.3 then
            table.insert(wounded_ids, u.id)
        end
    end
end

if #wounded_ids > 0 then
    ctx:move_units(wounded_ids, KELPIE_BASE.x, KELPIE_BASE.y)
end

-- Step 2: Find which pond is most threatened
if #enemies == 0 then return end

local north_threat = count_near(NORTH_POND, enemies)
local lily_threat  = count_near(LILY_POND, enemies)
local south_threat = count_near(SOUTH_POND, enemies)

local rally_target = nil
local max_threat = 0
if north_threat > max_threat then
    max_threat = north_threat
    rally_target = NORTH_POND
end
if lily_threat > max_threat then
    max_threat = lily_threat
    rally_target = LILY_POND
end
if south_threat > max_threat then
    max_threat = south_threat
    rally_target = SOUTH_POND
end

-- No threats detected? Nothing more to do.
if max_threat == 0 or not rally_target then return end

-- Step 3: Get idle combat units and rally toward threatened pond
local idle = ctx:idle_units()
if not idle then return end
if #idle == 0 then return end

-- Build set of wounded unit IDs to exclude
local wounded_set = {}
for _, wid in ipairs(wounded_ids) do
    wounded_set[wid] = true
end

local idle_combat_ids = {}
for _, u in ipairs(idle) do
    if not wounded_set[u.id] then
        table.insert(idle_combat_ids, u.id)
    end
end

if #idle_combat_ids == 0 then return end

-- Step 4: Focus fire on closest enemy to our army centroid
local cx, cy = 0, 0
for _, u in ipairs(my_units) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #my_units
cy = cy / #my_units

local best_target = nil
local best_dist = math.huge
for _, e in ipairs(enemies) do
    local dx = e.x - cx
    local dy = e.y - cy
    local dist = dx * dx + dy * dy
    if dist < best_dist then
        best_dist = dist
        best_target = e
    end
end

if best_target then
    ctx:attack_units(idle_combat_ids, best_target.id)
else
    ctx:attack_move(idle_combat_ids, rally_target.x, rally_target.y)
end
