-- @name: clawed_advanced
-- @events: on_tick
-- @interval: 3

-- Clawed formation + abilities script for demo scenario 3.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.
-- Units that use an ability are tracked in ability_used and skipped for attacks.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()

-- Track which units used abilities this tick (skip them for attack commands)
local ability_used = {}

-- === CLASSIFY UNITS ===
local tanks = {}      -- Quillback
local ranged = {}     -- Shrieker, Sparks, Whiskerwitch
local swarm = {}      -- Swarmer
local siege = {}      -- Gnawer
local all_combat = {}
local all_combat_ids = {}

local TANK_KINDS = { Quillback = true }
local RANGED_KINDS = { Shrieker = true, Sparks = true, Whiskerwitch = true }
local SWARM_KINDS = { Swarmer = true }
local SIEGE_KINDS = { Gnawer = true }
local WORKER_KINDS = { Nibblet = true }

-- Also build kind-specific lists for abilities
local shriekers = {}
local sparks_units = {}
local witches = {}

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if TANK_KINDS[u.kind] then
            table.insert(tanks, u)
        elseif u.kind == "Shrieker" then
            table.insert(ranged, u)
            table.insert(shriekers, u)
        elseif u.kind == "Sparks" then
            table.insert(ranged, u)
            table.insert(sparks_units, u)
        elseif u.kind == "Whiskerwitch" then
            table.insert(ranged, u)
            table.insert(witches, u)
        elseif SWARM_KINDS[u.kind] then
            table.insert(swarm, u)
        elseif SIEGE_KINDS[u.kind] then
            table.insert(siege, u)
        end
    end
end

local my_count = #all_combat
if my_count == 0 then return end

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
local strong_advantage = my_count >= enemy_count + 3

-- === ENEMY CLUSTER CENTROID ===
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

local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

-- === RALLY POINT ===
local my_buildings = ctx:my_buildings()
local rally_x, rally_y = army_cx, army_cy
if my_buildings and #my_buildings > 0 then
    local best_dist = 999999
    for _, b in ipairs(my_buildings) do
        local dx = b.x - army_cx
        local dy = b.y - army_cy
        local d = dx * dx + dy * dy
        if d < best_dist then
            best_dist = d
            rally_x = b.x
            rally_y = b.y
        end
    end
end

-- === GPU RESOURCES ===
local resources = ctx:get_resources()
local gpu = resources.gpu_cores

-- =====================================================================
-- ABILITY LAYER (runs first — marks units in ability_used)
-- =====================================================================

-- Helper: count enemies within range of a position
local function enemies_near(px, py, range)
    local count = 0
    local sq = range * range
    if enemies then
        for _, e in ipairs(enemies) do
            local dx = e.x - px
            local dy = e.y - py
            if dx * dx + dy * dy <= sq then
                count = count + 1
            end
        end
    end
    return count
end

-- Helper: find closest enemy to a unit
local function closest_enemy(u)
    if not enemies or enemy_count == 0 then return nil, 999999 end
    local best = nil
    local best_d = 999999
    for _, e in ipairs(enemies) do
        local dx = e.x - u.x
        local dy = e.y - u.y
        local d = dx * dx + dy * dy
        if d < best_d then
            best_d = d
            best = e
        end
    end
    return best, best_d
end

-- --- Quillback abilities ---
for _, u in ipairs(tanks) do
    -- SpineWall (slot 0, toggle, free) — activate once early
    if tick < 4 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end

    -- StubbornAdvance (slot 2, 10 GPU, self) when HP < 50%
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.50 and gpu >= 10 and not ability_used[u.id] then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
        gpu = gpu - 10
    end
end

-- --- Swarmer abilities ---
-- Count swarmers near each other for PileOn trigger
if #swarm >= 3 and enemies and enemy_count > 0 then
    for _, u in ipairs(swarm) do
        if ability_used[u.id] then
            -- skip
        else
            -- PileOn (slot 1, 5 GPU, range 1) when >=3 swarmers clustered near enemies
            local nearby_swarm = 0
            for _, s in ipairs(swarm) do
                if s.id ~= u.id then
                    local dx = s.x - u.x
                    local dy = s.y - u.y
                    if dx * dx + dy * dy <= 9 then -- within 3 tiles
                        nearby_swarm = nearby_swarm + 1
                    end
                end
            end
            local ce, cd = closest_enemy(u)
            if nearby_swarm >= 2 and ce and cd <= 4 and gpu >= 5 then
                ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
                ability_used[u.id] = true
                gpu = gpu - 5
            end
        end
    end
end

-- Scatter (slot 2, 5 GPU, self) when HP < 25%
for _, u in ipairs(swarm) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.25 and gpu >= 5 then
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
            gpu = gpu - 5
        end
    end
end

-- --- Shrieker abilities ---
for _, u in ipairs(shriekers) do
    if not ability_used[u.id] and gpu >= 5 then
        -- SonicSpit (slot 0, 5 GPU, 3-range) on clusters of >=2 enemies
        local near_count = enemies_near(u.x, u.y, 3)
        if near_count >= 2 then
            -- Target the enemy cluster centroid within range
            local ce, cd = closest_enemy(u)
            if ce and cd <= 9 then
                ctx:ability(u.id, 0, "position", ce.x, ce.y)
                ability_used[u.id] = true
                gpu = gpu - 5
            end
        end
    end
end

-- --- Sparks abilities ---
for _, u in ipairs(sparks_units) do
    if not ability_used[u.id] then
        -- ShortCircuit (slot 1, 10 GPU, 2-range) on nearby high-value targets
        local ce, cd = closest_enemy(u)
        if ce and cd <= 4 and gpu >= 10 then
            ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
            gpu = gpu - 10
        -- DaisyChain (slot 2, 15 GPU, 3-range) on enemy groups
        elseif gpu >= 15 then
            local near_count = enemies_near(u.x, u.y, 3)
            if near_count >= 2 and ce and cd <= 9 then
                ctx:ability(u.id, 2, "position", ce.x, ce.y)
                ability_used[u.id] = true
                gpu = gpu - 15
            end
        end
    end
end

-- --- Whiskerwitch abilities ---
for _, u in ipairs(witches) do
    -- WhiskerWeave (slot 1, toggle, free, 3-range) — activate once early for ally buff
    if tick < 4 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end

    -- HexOfMultiplication (slot 0, 25 GPU, 4-range) on largest enemy cluster
    if not ability_used[u.id] and gpu >= 25 then
        local near_count = enemies_near(u.x, u.y, 4)
        if near_count >= 2 then
            ctx:ability(u.id, 0, "position", math.floor(enemy_cx), math.floor(enemy_cy))
            ability_used[u.id] = true
            gpu = gpu - 25
        end
    end
end

-- --- Gnawer abilities ---
for _, u in ipairs(siege) do
    -- ChewThrough (slot 1, toggle, free) — activate once when near buildings
    if not ability_used[u.id] then
        local enemy_buildings = ctx:enemy_buildings()
        if enemy_buildings and #enemy_buildings > 0 then
            local nearest_bld_dist = 999999
            for _, b in ipairs(enemy_buildings) do
                local dx = b.x - u.x
                local dy = b.y - u.y
                local d = dx * dx + dy * dy
                if d < nearest_bld_dist then
                    nearest_bld_dist = d
                end
            end
            if nearest_bld_dist <= 25 then -- within 5 tiles
                ctx:ability(u.id, 1, "self")
                ability_used[u.id] = true
            end
        end
    end
end

-- =====================================================================
-- FORMATION LAYER (same as clawed_formation.lua)
-- =====================================================================

-- === RETREAT WOUNDED ===
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.30 and u.attacking and outnumbered then
            table.insert(retreat_ids, u.id)
        end
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- === FORMATION POSITIONING ===
if enemies and enemy_count > 0 then
    -- Quillbacks: advance as frontline
    local tank_idle_ids = {}
    for _, u in ipairs(tanks) do
        if u.idle and not ability_used[u.id] then
            table.insert(tank_idle_ids, u.id)
        end
    end
    if #tank_idle_ids > 0 then
        local tx = math.floor(army_cx + dir_x * 4)
        local ty = math.floor(army_cy + dir_y * 4)
        tx = math.max(0, math.min(map_w - 1, tx))
        ty = math.max(0, math.min(map_h - 1, ty))
        ctx:attack_move(tank_idle_ids, tx, ty)
    end

    -- Ranged: stay behind
    local ranged_idle_ids = {}
    for _, u in ipairs(ranged) do
        if u.idle and not ability_used[u.id] then
            table.insert(ranged_idle_ids, u.id)
        end
    end
    if #ranged_idle_ids > 0 then
        local rx = math.floor(army_cx - dir_x * 1)
        local ry = math.floor(army_cy - dir_y * 1)
        rx = math.max(0, math.min(map_w - 1, rx))
        ry = math.max(0, math.min(map_h - 1, ry))
        ctx:attack_move(ranged_idle_ids, rx, ry)
    end

    -- Swarmers: cluster and attack
    if #swarm > 0 then
        local swarm_idle_ids = {}
        for _, u in ipairs(swarm) do
            if u.idle and not ability_used[u.id] then
                table.insert(swarm_idle_ids, u.id)
            end
        end
        if #swarm_idle_ids > 0 then
            local sx = math.floor(army_cx + dir_x * 3)
            local sy = math.floor(army_cy + dir_y * 3)
            sx = math.max(0, math.min(map_w - 1, sx))
            sy = math.max(0, math.min(map_h - 1, sy))
            ctx:attack_move(swarm_idle_ids, sx, sy)
        end
    end

    -- Gnawer: follows main army
    local siege_idle_ids = {}
    for _, u in ipairs(siege) do
        if u.idle and not ability_used[u.id] then
            table.insert(siege_idle_ids, u.id)
        end
    end
    if #siege_idle_ids > 0 then
        local gx = math.floor(army_cx + dir_x * 2)
        local gy = math.floor(army_cy + dir_y * 2)
        gx = math.max(0, math.min(map_w - 1, gx))
        gy = math.max(0, math.min(map_h - 1, gy))
        ctx:attack_move(siege_idle_ids, gx, gy)
    end
end

-- === FOCUS FIRE (skip ability_used units) ===
local attackers = {}
for _, u in ipairs(all_combat) do
    if u.attacking and not ability_used[u.id] then
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

    local weak = ctx:weakest_enemy_in_range(cx, cy, 12)
    if weak then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, weak.id)
    end
end

-- === KITE ===
if outnumbered and enemies then
    for _, r in ipairs(ranged) do
        if r.attacking and not ability_used[r.id] then
            local closest_dist = 999999
            local closest_ex, closest_ey = 0, 0
            for _, e in ipairs(enemies) do
                local dx = e.x - r.x
                local dy = e.y - r.y
                local d = dx * dx + dy * dy
                if d < closest_dist then
                    closest_dist = d
                    closest_ex = e.x
                    closest_ey = e.y
                end
            end
            if closest_dist < 9 then
                local flee_x = r.x - (closest_ex - r.x)
                local flee_y = r.y - (closest_ey - r.y)
                flee_x = math.max(0, math.min(map_w - 1, flee_x))
                flee_y = math.max(0, math.min(map_h - 1, flee_y))
                ctx:move_units({r.id}, flee_x, flee_y)
            end
        end
    end
end

-- === PUSH ===
local should_push = (enemy_count == 0 and my_count >= 2) or strong_advantage

if should_push then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        local hq = nil
        local nearest = nil
        local nearest_dist = 999999

        local HQ_KINDS = {
            TheBox = true, TheDumpster = true, TheGrotto = true,
            TheBurrow = true, TheNest = true, TheMound = true,
        }

        for _, b in ipairs(enemy_buildings) do
            if HQ_KINDS[b.kind] then hq = b end
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < nearest_dist then
                nearest_dist = d
                nearest = b
            end
        end

        -- Only push units not using abilities
        local push_ids = {}
        for _, id in ipairs(all_combat_ids) do
            if not ability_used[id] then
                table.insert(push_ids, id)
            end
        end

        local target = hq or nearest
        if target and #push_ids > 0 then
            ctx:attack_move(push_ids, target.x, target.y)
        end
    end
end
