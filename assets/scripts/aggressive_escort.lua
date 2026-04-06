-- @name Aggressive Escort
-- @events on_tick
-- @interval 10
--
-- Launch convoy immediately. Win through active defense:
-- - Guard drones intercept shaheeds (save Patriots for missiles)
-- - Forward drones bomb launchers to reduce missile output
-- - Airstrikes on AA clusters
-- - Right tool for the right job

if not ctx.strait then return end

local drones = ctx.strait:my_drones()
local threats = ctx.strait:incoming_threats()
local enemies = ctx.strait:visible_enemies()
local zd = ctx.strait:zero_day_status()
local patriot = ctx.strait:patriot_status()
local tick = ctx.strait:mission_tick()
local map_w, _ = ctx.strait:map_size()

-- Always launch — we're going aggressive
ctx.strait:launch_all_boats()

-- Save Patriots for missiles
ctx.strait:set_patriot_mode(true)

-- Categorize
local alive = {}
for _, d in ipairs(drones) do if d.alive then table.insert(alive, d) end end

local launchers, aa_units, soldiers = {}, {}, {}
for _, e in ipairs(enemies) do
    if e.kind == "launcher" then table.insert(launchers, e)
    elseif e.kind == "aa" then table.insert(aa_units, e)
    elseif e.kind == "soldier" then table.insert(soldiers, e) end
end

local shaheed_count = threats.shaheeds and #threats.shaheeds or 0

-- === DRONE ALLOCATION ===
-- Dynamic split: more guards when more shaheeds incoming
local guard_target = 2
if shaheed_count > 3 then guard_target = 3 end
if shaheed_count > 8 then guard_target = 4 end

local guard_count = 0
local forward_drones = {}

for _, d in ipairs(alive) do
    local ab = ctx.strait:drone_abilities(d.id)

    if ab.mode == "guard_base" then
        guard_count = guard_count + 1
    elseif ab.mode == "bomb_target" then
        -- let it finish
    elseif guard_count < guard_target then
        ctx.strait:drone_guard_base(d.id)
        guard_count = guard_count + 1
    else
        table.insert(forward_drones, d)
    end
end

-- === FORWARD DRONES: bomb priority targets ===
-- Launchers are THE priority — every launcher killed = fewer missiles = more Patriots preserved
local targets = {}
for _, l in ipairs(launchers) do table.insert(targets, l) end
for _, s in ipairs(soldiers) do table.insert(targets, s) end

local tidx = 1
for _, d in ipairs(forward_drones) do
    local ab = ctx.strait:drone_abilities(d.id)

    if ab.bomb_ready and tidx <= #targets then
        local t = targets[tidx]
        -- Only bomb if reasonably close (within 30 tiles)
        local dist = math.abs(d.x - t.x) + math.abs(d.y - t.y)
        if dist < 30 then
            ctx.strait:drone_bomb(d.id, math.floor(t.x), math.floor(t.y))
            tidx = tidx + 1
        else
            -- Too far — patrol toward hostile zone instead
            local sx = map_w * ((d.id % 6) + 1) / 7
            ctx.strait:set_patrol(d.id, {
                { x = math.floor(sx), y = 8 },
                { x = math.floor(sx + 10), y = 15 },
                { x = math.floor(sx), y = 12 },
                { x = math.floor(sx - 10), y = 15 },
            })
        end
    elseif ab.mode ~= "patrol" or d.y > 20 then
        -- No target or reloading — patrol hostile zone to be ready
        local sx = map_w * ((d.id % 6) + 1) / 7
        ctx.strait:set_patrol(d.id, {
            { x = math.floor(sx), y = 8 },
            { x = math.floor(sx + 10), y = 15 },
            { x = math.floor(sx), y = 12 },
            { x = math.floor(sx - 10), y = 15 },
        })
    end
end

-- === AIRSTRIKES: use on AA clusters (they block our drones) ===
if #aa_units >= 2 then
    local cx, cy = 0, 0
    for _, a in ipairs(aa_units) do cx = cx + a.x; cy = cy + a.y end
    cx = cx / #aa_units; cy = cy / #aa_units
    local tight = true
    for _, a in ipairs(aa_units) do
        if math.abs(a.x - cx) > 8 or math.abs(a.y - cy) > 8 then tight = false; break end
    end
    if tight then ctx.strait:call_airstrike(math.floor(cx), math.floor(cy)) end
elseif #aa_units == 1 then
    -- Single AA — airstrike it so our bombers can operate freely
    ctx.strait:call_airstrike(math.floor(aa_units[1].x), math.floor(aa_units[1].y))
end

-- === ZERO-DAY: build Brick for hard launcher kills ===
if zd.state == "idle" then ctx.strait:build_zero_day("brick") end

-- === COMPUTE: vision-heavy for targeting, 0day when building ===
if zd.state == "building" then
    ctx.strait:allocate_compute({ drone_vision = 0.3, satellite = 0.1, zero_day = 0.6 })
else
    ctx.strait:allocate_compute({ drone_vision = 0.7, satellite = 0.2, zero_day = 0.1 })
end

-- Satellite on hostile shore
ctx.strait:set_satellite_focal(math.floor(map_w / 2), 6)

-- Rebuild lost drones
if #alive < 14 then ctx.strait:rebuild_drone() end
