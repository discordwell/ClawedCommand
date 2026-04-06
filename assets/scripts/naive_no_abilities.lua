-- Naive: patrol drones in hostile zone, launch convoy at tick 2000.
-- Never use 0days, satellite, airstrikes, bombing, or guard base.
-- Just vision and hoping for the best.
if not ctx.strait then return end

local drones = ctx.strait:my_drones()
local tick = ctx.strait:mission_tick()
local map_w, _ = ctx.strait:map_size()

-- Launch at tick 2000
if tick > 2000 then ctx.strait:launch_all_boats() end

-- Patrol hostile zone (vision only, no combat)
local alive = {}
for _, d in ipairs(drones) do if d.alive then table.insert(alive, d) end end

for i, d in ipairs(alive) do
    local x = map_w * i / (#alive + 1)
    ctx.strait:set_patrol(d.id, {
        { x = math.floor(x), y = 10 },
        { x = math.floor(x + 15), y = 20 },
        { x = math.floor(x), y = 30 },
        { x = math.floor(x - 15), y = 20 },
    })
end

-- All vision, no zero-day
ctx.strait:allocate_compute({ drone_vision = 0.9, satellite = 0.05, zero_day = 0.05 })
