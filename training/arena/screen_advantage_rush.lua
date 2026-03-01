-- @name screen_advantage_rush
-- @events tick
-- @interval 5

-- When we outnumber nearby enemies 5:1 or more, all-in attack.
-- "Nearby" = within 20 tiles of army centroid (roughly screen-visible).

local SCREEN_RADIUS = 20
local ADVANTAGE_RATIO = 5

local my = ctx:my_units()
if #my == 0 then return end

-- Compute army centroid
local cx, cy = 0, 0
for _, u in ipairs(my) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #my
cy = cy / #my

-- Count enemies within screen radius of centroid
local enemies = ctx:enemy_units()
local nearby_enemies = {}
local r_sq = SCREEN_RADIUS * SCREEN_RADIUS

for _, e in ipairs(enemies) do
    local dx = e.x - cx
    local dy = e.y - cy
    if dx * dx + dy * dy <= r_sq then
        nearby_enemies[#nearby_enemies + 1] = e
    end
end

if #nearby_enemies == 0 then return end

-- Check 5:1 advantage
if #my < ADVANTAGE_RATIO * #nearby_enemies then return end

-- We have overwhelming advantage — focus fire nearest enemy to centroid
local best = nil
local best_dist = math.huge

for _, e in ipairs(nearby_enemies) do
    local dx = e.x - cx
    local dy = e.y - cy
    local d = dx * dx + dy * dy
    if d < best_dist then
        best_dist = d
        best = e
    end
end

if best then
    local ids = {}
    for _, u in ipairs(my) do
        ids[#ids + 1] = u.id
    end
    ctx:attack_units(ids, best.id)
end
