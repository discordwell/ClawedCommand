-- Naive: launch convoy immediately, drones escort along shipping lane.
-- No bombing, no guard base, no patriot mode change.
if not ctx.strait then return end

local drones = ctx.strait:my_drones()
local tick = ctx.strait:mission_tick()
local map_w, _ = ctx.strait:map_size()

-- Launch immediately
ctx.strait:launch_all_boats()

-- Spread drones along shipping lane as escorts
local alive = {}
for _, d in ipairs(drones) do if d.alive then table.insert(alive, d) end end

for i, d in ipairs(alive) do
    local x = map_w * i / (#alive + 1)
    ctx.strait:set_patrol(d.id, {
        { x = math.floor(x), y = 28 },
        { x = math.floor(x + 20), y = 32 },
        { x = math.floor(x), y = 32 },
        { x = math.floor(x - 20), y = 28 },
    })
end

-- Default allocation
ctx.strait:allocate_compute({ drone_vision = 0.6, satellite = 0.2, zero_day = 0.2 })
