-- Naive: only use satellite, 0days, and airstrikes. Keep drones at base.
-- Never move drones toward enemies. Never bomb. Never guard base.
if not ctx.strait then return end

local tick = ctx.strait:mission_tick()
local zd = ctx.strait:zero_day_status()
local enemies = ctx.strait:visible_enemies()
local map_w, _ = ctx.strait:map_size()

-- Launch at tick 3000
if tick > 3000 then ctx.strait:launch_all_boats() end

-- Heavy satellite + zero-day allocation, minimal drone vision
ctx.strait:allocate_compute({ drone_vision = 0.1, satellite = 0.4, zero_day = 0.5 })
ctx.strait:set_satellite_focal(math.floor(map_w / 2), 8)

-- Build zero-days
if zd.state == "idle" then ctx.strait:build_zero_day("brick") end

-- Airstrike any visible launcher
for _, e in ipairs(enemies) do
    if e.kind == "launcher" then
        ctx.strait:call_airstrike(math.floor(e.x), math.floor(e.y))
        break
    end
end
