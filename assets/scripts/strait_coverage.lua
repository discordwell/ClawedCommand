-- @name Coastline Coverage
-- @events on_tick
-- @interval 10
--
-- Sector-based drone patrol for a 300-wide strait.
--
-- STRATEGY:
-- The strait is 300 tiles wide. We divide it into sectors, one per drone.
-- Each drone patrols a diamond pattern covering its sector, biased toward
-- the hostile shallows (where launchers set up) and the shipping lane
-- (where tankers transit). When drones die, survivors expand their sectors.
-- If drone density drops below threshold, we fall back to satellite scans
-- for uncovered sectors. During lulls, we build zero-day exploits.
--
-- This is your code sword. Modify it. Break it. Make it better.

-- ===== CONFIGURATION =====

-- Patrol vertical bounds (grid y-coordinates)
local HOSTILE_ZONE_Y = 12   -- hostile shallows / launcher staging area
local SHIPPING_LANE_Y = 30  -- center of the shipping lane
local FRIENDLY_ZONE_Y = 45  -- southern patrol limit

-- When fewer than this many drones alive, start using satellite to fill gaps
local SATELLITE_THRESHOLD = 6

-- Build zero-days when compute is above this fraction of max
local ZERO_DAY_COMPUTE_THRESHOLD = 0.6

-- Zero-day build priority (first available type that isn't deployed yet)
local ZERO_DAY_PRIORITY = { "brick", "blind", "hijack", "spoof" }

-- ===== MAIN LOGIC =====

-- Guard: only run when strait bindings are available
if not ctx.strait then return end

-- Get current state
local drones = ctx.strait:my_drones()
local tankers = ctx.strait:tanker_status()
local compute = ctx.strait:compute_status()
local tick = ctx.strait:mission_tick()
local map_w, map_h = ctx.strait:map_size()

-- Filter to alive drones only
local alive_drones = {}
for _, d in ipairs(drones) do
    if d.alive then
        table.insert(alive_drones, d)
    end
end

local drone_count = #alive_drones
if drone_count == 0 then
    -- All drones dead. Switch fully to satellite coverage.
    -- Scan 5 evenly-spaced points along the shipping lane.
    for i = 1, 5 do
        local scan_x = math.floor(map_w * i / 6)
        ctx.strait:satellite_scan(scan_x, HOSTILE_ZONE_Y)
    end
    return
end

-- ===== SECTOR ASSIGNMENT =====
-- Divide the map width evenly among alive drones.
-- Sort drones by current x position to minimize reassignment churn.

table.sort(alive_drones, function(a, b) return a.x < b.x end)

local sector_width = map_w / drone_count

for i, drone in ipairs(alive_drones) do
    -- Sector boundaries
    local sector_left = math.floor((i - 1) * sector_width)
    local sector_right = math.floor(i * sector_width) - 1
    local sector_center = math.floor((sector_left + sector_right) / 2)

    -- Diamond patrol pattern within this sector:
    --   North point: sector center, hostile zone (watch for launchers)
    --   East point: sector right edge, shipping lane (protect tankers)
    --   South point: sector center, friendly zone (rear coverage)
    --   West point: sector left edge, shipping lane
    local diamond_hw = math.floor(sector_width * 0.33)

    local waypoints = {
        { x = sector_center, y = HOSTILE_ZONE_Y },
        { x = math.min(sector_center + diamond_hw, map_w - 1), y = SHIPPING_LANE_Y },
        { x = sector_center, y = FRIENDLY_ZONE_Y },
        { x = math.max(sector_center - diamond_hw, 0), y = SHIPPING_LANE_Y },
    }

    -- Bias toward active tankers: if a tanker is in this sector, tighten
    -- the patrol to focus on the hostile zone above it
    for _, tanker in ipairs(tankers) do
        if not tanker.arrived and not tanker.destroyed then
            if tanker.x >= sector_left and tanker.x <= sector_right then
                -- Tanker in our sector! Shift north point to directly above it
                waypoints[1] = { x = math.floor(tanker.x), y = HOSTILE_ZONE_Y }
                break
            end
        end
    end

    ctx.strait:set_patrol(drone.id, waypoints)
end

-- ===== SATELLITE FALLBACK =====
-- If we've lost enough drones, fill gaps with satellite scans.
-- Each scan costs compute but covers a wide area temporarily.

if drone_count < SATELLITE_THRESHOLD and compute.compute > 30 then
    -- Find sectors that are most stretched (widest per-drone coverage)
    -- and scan the hostile zone in the gaps between drones
    for i = 1, drone_count - 1 do
        local gap = alive_drones[i + 1].x - alive_drones[i].x
        if gap > sector_width * 1.5 then
            -- Big gap between these two drones — satellite the midpoint
            local mid_x = math.floor((alive_drones[i].x + alive_drones[i + 1].x) / 2)
            ctx.strait:satellite_scan(mid_x, HOSTILE_ZONE_Y)
        end
    end
end

-- ===== ZERO-DAY PIPELINE =====
-- During quiet periods (high compute), invest in building exploits.
-- Priority: brick (destroy launchers) > blind > hijack > spoof.

local zd = ctx.strait:zero_day_status()

if zd.state == "idle" and compute.compute > compute.max * ZERO_DAY_COMPUTE_THRESHOLD then
    -- Pick the highest-priority type we haven't deployed yet
    for _, zd_type in ipairs(ZERO_DAY_PRIORITY) do
        -- Try to build it (the runtime will reject if already deployed)
        ctx.strait:build_zero_day(zd_type)
        break
    end
end

-- ===== COMPUTE ALLOCATION =====
-- Adaptive allocation: more drones alive = more drone vision budget.
-- Fewer drones = more satellite + zero-day budget.

if drone_count >= 12 then
    ctx.strait:allocate_compute({ drone_vision = 0.6, satellite = 0.2, zero_day = 0.2 })
elseif drone_count >= 6 then
    ctx.strait:allocate_compute({ drone_vision = 0.4, satellite = 0.3, zero_day = 0.3 })
else
    ctx.strait:allocate_compute({ drone_vision = 0.2, satellite = 0.4, zero_day = 0.4 })
end
