-- @name Hold and Clear V2
-- @events on_tick
-- @interval 10

if not ctx.strait then return end

local drones = ctx.strait:my_drones()
local compute = ctx.strait:compute_status()
local zd = ctx.strait:zero_day_status()
local patriot = ctx.strait:patriot_status()
local convoy = ctx.strait:convoy_status()
local threats = ctx.strait:incoming_threats()
local enemies = ctx.strait:visible_enemies()
local tick = ctx.strait:mission_tick()
local map_w, map_h = ctx.strait:map_size()

-- Filter alive drones
local alive = {}
for _, d in ipairs(drones) do
    if d.alive then table.insert(alive, d) end
end

-- Categorize enemies
local launchers, aa_units, soldiers = {}, {}, {}
for _, e in ipairs(enemies) do
    if e.kind == "launcher" then table.insert(launchers, e)
    elseif e.kind == "aa" then table.insert(aa_units, e)
    elseif e.kind == "soldier" then table.insert(soldiers, e) end
end

-- ===== PATRIOT MODE: missiles only (always) =====
ctx.strait:set_patriot_mode(true)

-- ===== DRONE DEPLOYMENT =====
-- Assign 2 drones to guard base, rest patrol the hostile zone.
-- Drones on bomb runs continue until they reload.

local guard_target = 2
if #threats.shaheeds > 5 then guard_target = 3 end

local guard_count = 0
local patrol_drones = {}

for _, d in ipairs(alive) do
    local ab = ctx.strait:drone_abilities(d.id)

    -- Already guarding? Count it.
    if ab.mode == "guard_base" then
        guard_count = guard_count + 1
    -- Already on a bomb run? Let it finish.
    elseif ab.mode == "bomb_target" then
        -- Don't reassign
    -- Need more guards?
    elseif guard_count < guard_target then
        ctx.strait:drone_guard_base(d.id)
        guard_count = guard_count + 1
    else
        table.insert(patrol_drones, d)
    end
end

-- ===== PATROL: spread drones across the hostile zone =====
-- Drones that aren't bombing or guarding should patrol y=8-15 area
-- evenly spaced across the map width. This positions them to
-- quickly bomb launchers when they emerge.

for i, d in ipairs(patrol_drones) do
    local ab = ctx.strait:drone_abilities(d.id)

    -- Look for a nearby bombable target
    local target = nil
    local best_dist = 999

    if ab.bomb_ready then
        -- Priority: launchers > aa > soldiers
        for _, l in ipairs(launchers) do
            local dist = math.abs(d.x - l.x) + math.abs(d.y - l.y)
            if dist < best_dist then
                best_dist = dist
                target = l
            end
        end
        -- Only bomb AA if 3+ drones are near it (swarm requirement)
        -- For now, skip AA bombing until we have better coordination
        if not target then
            for _, s in ipairs(soldiers) do
                local dist = math.abs(d.x - s.x) + math.abs(d.y - s.y)
                if dist < best_dist then
                    best_dist = dist
                    target = s
                end
            end
        end
    end

    if target and best_dist < 20 then
        -- Bomb nearby target
        ctx.strait:drone_bomb(d.id, math.floor(target.x), math.floor(target.y))
    elseif ab.mode ~= "patrol" or d.y > 20 then
        -- Deploy to hostile zone patrol
        local sector_x = map_w * (i) / (#patrol_drones + 1)
        ctx.strait:set_patrol(d.id, {
            { x = math.floor(sector_x), y = 8 },
            { x = math.floor(sector_x + 15), y = 15 },
            { x = math.floor(sector_x), y = 12 },
            { x = math.floor(sector_x - 15), y = 15 },
        })
    end
end

-- ===== AIRSTRIKE: use on visible launcher clusters =====
if #launchers >= 1 then
    -- Airstrike the first visible launcher we see
    local l = launchers[1]
    ctx.strait:call_airstrike(math.floor(l.x), math.floor(l.y))
end

-- ===== REBUILD DRONES =====
if #alive < 14 then ctx.strait:rebuild_drone() end

-- ===== ZERO-DAY PIPELINE =====
if zd.state == "idle" then
    ctx.strait:build_zero_day("brick")
end

-- ===== COMPUTE ALLOCATION =====
-- Heavy vision to spot launchers emerging. Switch to zero-day when building.
local dv, sat, zdc
if zd.state == "building" then
    dv, sat, zdc = 0.3, 0.1, 0.6
else
    dv, sat, zdc = 0.8, 0.1, 0.1
end
ctx.strait:allocate_compute({ drone_vision = dv, satellite = sat, zero_day = zdc })

-- ===== SATELLITE: watch the hidden launcher zone =====
ctx.strait:set_satellite_focal(math.floor(map_w / 2), 5)

-- ===== CONVOY LAUNCH DECISION =====
-- Launch when launchers are mostly dead and base is healthy
if convoy.hold then
    if #launchers == 0 and tick > 1000 and patriot.base_hp >= 6 then
        ctx.strait:launch_all_boats()
    elseif tick > 3000 and #launchers <= 1 then
        ctx.strait:launch_all_boats()
    elseif patriot.base_hp <= 3 then
        -- Desperate — launch now before base dies
        ctx.strait:launch_all_boats()
    end
end
