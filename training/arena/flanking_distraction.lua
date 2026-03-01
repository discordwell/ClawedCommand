-- @name flanking_distraction
-- @events tick
-- @interval 5

-- Sends the fastest unit on a wide orbit around the enemy army,
-- staying outside attack range. Enemy units retarget toward the
-- flanker, splitting their orientation away from the main force.

local my = ctx:my_units()
local enemies = ctx:enemy_units()
if #my < 2 or #enemies == 0 then return end

-- Pick fastest unit as flanker
local flanker = nil
local top_speed = 0
for _, u in ipairs(my) do
    if u.speed > top_speed then
        top_speed = u.speed
        flanker = u
    end
end
if not flanker then return end

-- Enemy centroid
local ecx, ecy = 0, 0
for _, e in ipairs(enemies) do
    ecx = ecx + e.x
    ecy = ecy + e.y
end
ecx = ecx / #enemies
ecy = ecy / #enemies

-- Army centroid (excluding flanker so it doesn't skew)
local acx, acy, count = 0, 0, 0
for _, u in ipairs(my) do
    if u.id ~= flanker.id then
        acx = acx + u.x
        acy = acy + u.y
        count = count + 1
    end
end
if count == 0 then return end
acx = acx / count
acy = acy / count

-- Angle from enemy centroid back toward our army
local dx = acx - ecx
local dy = acy - ecy
local d = math.sqrt(dx * dx + dy * dy)
if d < 1 then return end
local base_angle = math.atan2(dy, dx)

-- Orbit radius: max enemy attack range + 3 tile safety buffer
local max_range = 0
for _, e in ipairs(enemies) do
    local r = e.range or 1
    if r > max_range then max_range = r end
end
local orbit_r = max_range + 3

-- Sweep from flank (90° off approach) around to the rear (270°)
-- Full orbit takes 200 ticks (~20s at 10hz), then repeats
local t = ctx:tick()
local phase = ((t / 8) % 25) / 25
local sweep_angle = base_angle + math.pi * 0.5 + phase * math.pi

local tx = ecx + math.cos(sweep_angle) * orbit_r
local ty = ecy + math.sin(sweep_angle) * orbit_r

-- Clamp to map bounds
local map = ctx:map_size()
tx = math.max(1, math.min(map.w - 2, tx))
ty = math.max(1, math.min(map.h - 2, ty))

-- Find passable ground, shrink orbit if terrain blocks
local gx, gy = math.floor(tx), math.floor(ty)
if not ctx:is_passable(gx, gy) then
    for shrink = 8, 4, -1 do
        local s = shrink / 10
        local sx = ecx + math.cos(sweep_angle) * orbit_r * s
        local sy = ecy + math.sin(sweep_angle) * orbit_r * s
        local sgx, sgy = math.floor(sx), math.floor(sy)
        if ctx:is_passable(sgx, sgy) then
            gx, gy = sgx, sgy
            break
        end
    end
end

-- Avoid slow terrain on the orbit path
local cost = ctx:movement_cost(gx, gy)
if cost and cost > 1.5 then
    -- Nudge forward along the sweep to find faster ground
    for nudge = 1, 3 do
        local na = sweep_angle + nudge * 0.15
        local nx = math.floor(ecx + math.cos(na) * orbit_r)
        local ny = math.floor(ecy + math.sin(na) * orbit_r)
        local nc = ctx:movement_cost(nx, ny)
        if nc and nc <= 1.5 and ctx:is_passable(nx, ny) then
            gx, gy = nx, ny
            break
        end
    end
end

ctx:move_units({flanker.id}, gx, gy)
