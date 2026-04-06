-- @name Coastline Coverage
-- @events on_tick
-- @interval 10
--
-- Sector-based drone patrol for a 300-wide strait (flow economy).
--
-- STRATEGY:
-- Compute is a continuous budget: at every moment the three slices
-- (drone_vision / satellite / zero_day) sum to 1.0. We dial the mix
-- based on how many drones are still alive and whether a zero-day
-- is currently building. Drones divide the strait into sectors and
-- patrol a diamond biased toward the hostile shallows and shipping
-- lane. A continuous satellite focal sits over the biggest coverage
-- gap. Airstrikes and drone rebuilds ride on time-based charges.
--
-- This is your code sword. Modify it. Break it. Make it better.

-- ===== CONFIGURATION =====

-- Patrol vertical bounds (grid y-coordinates)
local HOSTILE_ZONE_Y = 12   -- hostile shallows / launcher staging area
local SHIPPING_LANE_Y = 30  -- center of the shipping lane
local FRIENDLY_ZONE_Y = 45  -- southern patrol limit

-- Zero-day build priority (first available type that isn't deployed yet)
local ZERO_DAY_PRIORITY = { "brick", "blind", "hijack", "spoof" }

-- ===== MAIN LOGIC =====

-- Guard: only run when strait bindings are available
if not ctx.strait then return end

-- Get current state
local drones = ctx.strait:my_drones()
local tankers = ctx.strait:tanker_status()
local compute = ctx.strait:compute_status()
local zd = ctx.strait:zero_day_status()
local map_w, _ = ctx.strait:map_size()

-- Filter to alive drones only
local alive_drones = {}
for _, d in ipairs(drones) do
    if d.alive then
        table.insert(alive_drones, d)
    end
end

local drone_count = #alive_drones

-- ===== SECTOR ASSIGNMENT =====
-- Divide the map width evenly among alive drones.

if drone_count > 0 then
    table.sort(alive_drones, function(a, b) return a.x < b.x end)

    local sector_width = map_w / drone_count

    for i, drone in ipairs(alive_drones) do
        local sector_left = math.floor((i - 1) * sector_width)
        local sector_right = math.floor(i * sector_width) - 1
        local sector_center = math.floor((sector_left + sector_right) / 2)
        local diamond_hw = math.floor(sector_width * 0.33)

        local waypoints = {
            { x = sector_center, y = HOSTILE_ZONE_Y },
            { x = math.min(sector_center + diamond_hw, map_w - 1), y = SHIPPING_LANE_Y },
            { x = sector_center, y = FRIENDLY_ZONE_Y },
            { x = math.max(sector_center - diamond_hw, 0), y = SHIPPING_LANE_Y },
        }

        -- Bias toward active tankers in this sector
        for _, tanker in ipairs(tankers) do
            if not tanker.arrived and not tanker.destroyed then
                if tanker.x >= sector_left and tanker.x <= sector_right then
                    waypoints[1] = { x = math.floor(tanker.x), y = HOSTILE_ZONE_Y }
                    break
                end
            end
        end

        ctx.strait:set_patrol(drone.id, waypoints)
    end
end

-- ===== SATELLITE FOCAL =====
-- Place the continuous satellite focal over the biggest coverage gap
-- in the hostile zone. At low satellite allocation this covers little,
-- at high allocation it's a wide umbrella.

if drone_count >= 2 then
    local biggest_gap = 0
    local gap_x = map_w / 2
    for i = 1, drone_count - 1 do
        local gap = alive_drones[i + 1].x - alive_drones[i].x
        if gap > biggest_gap then
            biggest_gap = gap
            gap_x = (alive_drones[i].x + alive_drones[i + 1].x) / 2
        end
    end
    ctx.strait:set_satellite_focal(math.floor(gap_x), HOSTILE_ZONE_Y)
elseif drone_count == 1 then
    -- Lone drone: satellite covers the opposite end of the strait
    local drone_x = alive_drones[1].x
    local focal_x = drone_x < map_w / 2 and (map_w * 0.75) or (map_w * 0.25)
    ctx.strait:set_satellite_focal(math.floor(focal_x), HOSTILE_ZONE_Y)
else
    -- All drones dead: keep satellite on the shipping lane midpoint
    ctx.strait:set_satellite_focal(math.floor(map_w / 2), HOSTILE_ZONE_Y)
end

-- ===== ZERO-DAY PIPELINE (sequential builds) =====
-- Build state is one-at-a-time: pick a target, feed the channel, deploy,
-- then pick next. Queue a new build whenever the slot is idle.

if zd.state == "idle" then
    for _, zd_type in ipairs(ZERO_DAY_PRIORITY) do
        ctx.strait:build_zero_day(zd_type)
        break
    end
end

-- ===== COMPUTE ALLOCATION (continuous dial) =====
-- Fewer drones → we lean on satellite. If a zero-day is building and
-- the situation is stable (many drones), push the zero_day slice up.
-- Starve no channel completely — keep every slice ≥ 0.1.

local dv, sat, zdc
if drone_count >= 12 then
    dv, sat, zdc = 0.6, 0.15, 0.25
elseif drone_count >= 6 then
    dv, sat, zdc = 0.45, 0.25, 0.30
elseif drone_count >= 2 then
    dv, sat, zdc = 0.30, 0.40, 0.30
else
    -- Crisis: satellite carries coverage, drones get what's left
    dv, sat, zdc = 0.15, 0.55, 0.30
end

-- If no zero-day is building (or already ready), redirect that slice
-- into vision/satellite where it still does work.
if zd.state == "idle" or zd.state == "ready" then
    local rerouted = zdc * 0.5
    dv = dv + rerouted
    sat = sat + (zdc - rerouted)
    zdc = 0.0
end

-- Normalize to guarantee sum = 1.0
local sum = dv + sat + zdc
if sum > 0 then
    dv, sat, zdc = dv / sum, sat / sum, zdc / sum
end

ctx.strait:allocate_compute({ drone_vision = dv, satellite = sat, zero_day = zdc })
