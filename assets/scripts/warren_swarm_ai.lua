-- @name: warren_swarm_ai
-- @events: on_tick
-- @interval: 3

-- Clawed swarm defense AI for Rat King's Maze.
-- Corridor flooding, centroid focus fire, chokepoint formation,
-- flanking idle swarmers, progressive fallback when outnumbered.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- === CLASSIFY UNITS ===
local swarm = {}       -- Swarmer, Nibblet (cheap melee)
local tanks = {}       -- Quillback
local ranged = {}      -- Shrieker, Sparks, Whiskerwitch
local all_combat = {}
local all_combat_ids = {}

local SWARM_KINDS = { Swarmer = true, Nibblet = true }
local TANK_KINDS = { Quillback = true }
local RANGED_KINDS = { Shrieker = true, Sparks = true, Whiskerwitch = true }
local WORKER_KINDS = { Nibblet = true }
-- Nibblet is both worker and swarm — treat as swarm in combat scripts

for _, u in ipairs(my_units) do
    table.insert(all_combat, u)
    table.insert(all_combat_ids, u.id)
    if TANK_KINDS[u.kind] then
        table.insert(tanks, u)
    elseif RANGED_KINDS[u.kind] then
        table.insert(ranged, u)
    elseif SWARM_KINDS[u.kind] then
        table.insert(swarm, u)
    end
end

local my_count = #all_combat
if my_count == 0 then return end

-- === HELPER ===
local function clamp(x, y)
    return math.max(0, math.min(map_w - 1, math.floor(x))),
           math.max(0, math.min(map_h - 1, math.floor(y)))
end

-- === ARMY CENTROID ===
local army_cx, army_cy = 0, 0
for _, u in ipairs(all_combat) do
    army_cx = army_cx + u.x
    army_cy = army_cy + u.y
end
army_cx = army_cx / my_count
army_cy = army_cy / my_count

-- === ENEMY INFO ===
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local outnumbered = my_count < enemy_count
local badly_outnumbered = my_count * 2 < enemy_count

-- === ENEMY CENTROID ===
local enemy_cx, enemy_cy = army_cx, army_cy
if enemies and enemy_count > 0 then
    enemy_cx, enemy_cy = 0, 0
    for _, e in ipairs(enemies) do
        enemy_cx = enemy_cx + e.x
        enemy_cy = enemy_cy + e.y
    end
    enemy_cx = enemy_cx / enemy_count
    enemy_cy = enemy_cy / enemy_count
end

-- Direction from us to enemies
local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

-- TheBurrow fallback position (keep at map north)
local burrow_x, burrow_y = 32, 4
local my_buildings = ctx:my_buildings()
if my_buildings then
    for _, b in ipairs(my_buildings) do
        if b.kind == "TheBurrow" then
            burrow_x = b.x
            burrow_y = b.y
            break
        end
    end
end

-- === PROGRESSIVE FALLBACK ===
-- When badly outnumbered, retreat all survivors toward TheBurrow
if badly_outnumbered and #all_combat_ids > 0 then
    local fx, fy = clamp(burrow_x, burrow_y + 3)
    ctx:move_units(all_combat_ids, fx, fy)
    return
end

-- === WOUNDED RETREAT ===
-- <30% HP + outnumbered → fall back to TheBurrow
local retreat_ids = {}
local active_ids = {}
local active_units = {}
for _, u in ipairs(all_combat) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.30 and outnumbered then
        table.insert(retreat_ids, u.id)
    else
        table.insert(active_ids, u.id)
        table.insert(active_units, u)
    end
end
if #retreat_ids > 0 then
    local rx, ry = clamp(burrow_x, burrow_y + 3)
    ctx:move_units(retreat_ids, rx, ry)
end

-- === CHOKEPOINT FORMATION ===
-- Quillbacks hold forward, ranged 2-3 tiles behind
if enemies and enemy_count > 0 then
    -- Tanks: advance toward enemy as front line
    for _, u in ipairs(tanks) do
        if not u.in_combat then
            local tx, ty = clamp(army_cx + dir_x * 3, army_cy + dir_y * 3)
            ctx:attack_move({u.id}, tx, ty)
        end
    end

    -- Ranged: hold position behind tanks
    for _, u in ipairs(ranged) do
        if not u.in_combat then
            local rx, ry = clamp(army_cx - dir_x * 2, army_cy - dir_y * 2)
            ctx:attack_move({u.id}, rx, ry)
        end
    end
end

-- === CORRIDOR FLOODING ===
-- Split idle swarmers into packs by proximity, each attacks through different corridor
-- Corridor x-centers for the maze: 16, 32, 48 (outer), 20, 44 (middle/inner)
local CORRIDORS = {16, 20, 32, 44, 48}

local idle_swarm = {}
for _, u in ipairs(swarm) do
    if not u.in_combat then
        table.insert(idle_swarm, u)
    end
end

if #idle_swarm > 0 and enemies and enemy_count > 0 then
    -- Assign each idle swarmer to nearest corridor
    for _, u in ipairs(idle_swarm) do
        local best_col = CORRIDORS[1]
        local best_dist = math.abs(u.x - best_col)
        for _, col in ipairs(CORRIDORS) do
            local d = math.abs(u.x - col)
            if d < best_dist then
                best_dist = d
                best_col = col
            end
        end

        -- Check if player is concentrated elsewhere — flank through adjacent corridor
        local player_col = math.floor(enemy_cx)
        if math.abs(best_col - player_col) < 4 and #CORRIDORS > 1 then
            -- Player is in our corridor — pick an adjacent one
            local alt_col = best_col
            local max_dist = 0
            for _, col in ipairs(CORRIDORS) do
                local d = math.abs(col - player_col)
                if d > max_dist then
                    max_dist = d
                    alt_col = col
                end
            end
            best_col = alt_col
        end

        -- Attack-move south toward player through chosen corridor
        local target_y = math.max(u.y + 8, math.floor(enemy_cy))
        local tx, ty = clamp(best_col, target_y)
        ctx:attack_move({u.id}, tx, ty)
    end
end

-- === FOCUS FIRE ===
-- All attacking units target closest enemy to army centroid (proven #1 strategy)
local attackers = {}
for _, u in ipairs(all_combat) do
    if u.attacking then
        table.insert(attackers, u)
    end
end

if #attackers >= 2 then
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #attackers
    cy = cy / #attackers

    -- Closest enemy to attacker centroid
    local best_target = nil
    local best_dist = 999999
    if enemies then
        for _, e in ipairs(enemies) do
            local dx = e.x - cx
            local dy = e.y - cy
            local d = dx * dx + dy * dy
            if d < best_dist then
                best_dist = d
                best_target = e
            end
        end
    end

    if best_target then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, best_target.id)
    end
end
