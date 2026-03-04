-- @name: cat_siege_formation
-- @events: on_tick
-- @interval: 3

-- Cat siege formation for Rat King's Maze.
-- Corridor-aware tank screen, ranged echelon, scouts ahead,
-- support rear, building push, focus fire on weakest in range.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- === CLASSIFY UNITS ===
local tanks = {}       -- Chonk
local ranged = {}      -- Hisser
local scouts = {}      -- Nuisance
local support = {}     -- Yowler
local all_combat = {}
local all_combat_ids = {}

local TANK_KINDS = { Chonk = true, MechCommander = true }
local RANGED_KINDS = { Hisser = true, FlyingFox = true, Catnapper = true }
local SCOUT_KINDS = { Nuisance = true, Mouser = true }
local SUPPORT_KINDS = { Yowler = true }
local WORKER_KINDS = { Pawdler = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if TANK_KINDS[u.kind] then
            table.insert(tanks, u)
        elseif RANGED_KINDS[u.kind] then
            table.insert(ranged, u)
        elseif SCOUT_KINDS[u.kind] then
            table.insert(scouts, u)
        elseif SUPPORT_KINDS[u.kind] then
            table.insert(support, u)
        end
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
local strong_advantage = my_count >= enemy_count + 3

-- === ENEMY CENTROID ===
local enemy_cx, enemy_cy = army_cx, army_cy - 10  -- default: push north
if enemies and enemy_count > 0 then
    enemy_cx, enemy_cy = 0, 0
    for _, e in ipairs(enemies) do
        enemy_cx = enemy_cx + e.x
        enemy_cy = enemy_cy + e.y
    end
    enemy_cx = enemy_cx / enemy_count
    enemy_cy = enemy_cy / enemy_count
end

-- Direction toward enemies (normalized)
local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

-- Perpendicular for lateral spread
local perp_x = -dir_y
local perp_y = dir_x

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

-- === CORRIDOR WIDTH DETECTION ===
-- Probe movement_cost left/right from centroid to detect corridor width.
-- Tiles with infinite cost (Rock/Water) mark corridor edges.
local corridor_half_width = 4  -- default: open field
local probe_y = math.floor(army_cy)
for offset = 1, 6 do
    local px = math.floor(army_cx) + offset
    if px < map_w then
        local cost = ctx:movement_cost(px, probe_y)
        if cost == nil or cost <= 0 then
            corridor_half_width = offset - 1
            break
        end
    end
end

-- === RETREAT WOUNDED ===
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.30 and u.attacking and outnumbered then
        table.insert(retreat_ids, u.id)
    end
end
if #retreat_ids > 0 then
    local rx, ry = clamp(rally_x, rally_y)
    ctx:move_units(retreat_ids, rx, ry)
end

-- === FORMATION POSITIONING ===
if enemies and enemy_count > 0 then

    -- CORRIDOR-AWARE TANK SCREEN
    -- Chonks in front, but don't spread wider than corridor
    local tank_spacing = math.min(3, corridor_half_width)
    for i, u in ipairs(tanks) do
        if not u.in_combat then
            local lateral = (i - 1 - (#tanks - 1) / 2) * tank_spacing
            local tx = army_cx + dir_x * 5 + perp_x * lateral
            local ty = army_cy + dir_y * 5 + perp_y * lateral
            local cx, cy = clamp(tx, ty)
            ctx:attack_move({u.id}, cx, cy)
        end
    end

    -- RANGED ECHELON: Hissers 3-4 tiles behind tank line
    for i, u in ipairs(ranged) do
        if not u.in_combat then
            local lateral = (i - 1 - (#ranged - 1) / 2) * 2
            local rx = army_cx - dir_x * 3 + perp_x * lateral
            local ry = army_cy - dir_y * 3 + perp_y * lateral
            local cx, cy = clamp(rx, ry)
            ctx:attack_move({u.id}, cx, cy)
        end
    end

    -- SUPPORT REAR: Yowlers near army centroid, slightly behind
    for _, u in ipairs(support) do
        if not u.in_combat then
            local sx, sy = clamp(army_cx - dir_x * 2, army_cy - dir_y * 2)
            ctx:attack_move({u.id}, sx, sy)
        end
    end
end

-- === SCOUT AHEAD ===
-- Nuisances 6-8 tiles ahead toward nearest enemy building
local scout_target_x, scout_target_y = army_cx + dir_x * 8, army_cy + dir_y * 8
local enemy_buildings = ctx:enemy_buildings()
if enemy_buildings and #enemy_buildings > 0 then
    -- Prioritize NestingBox, then nearest
    local best_b = nil
    local best_dist = 999999
    for _, b in ipairs(enemy_buildings) do
        local dx = b.x - army_cx
        local dy = b.y - army_cy
        local d = dx * dx + dy * dy
        -- Prefer NestingBox targets
        if b.kind == "NestingBox" then d = d * 0.5 end
        if d < best_dist then
            best_dist = d
            best_b = b
        end
    end
    if best_b then
        scout_target_x = best_b.x
        scout_target_y = best_b.y
    end
end

for _, u in ipairs(scouts) do
    if not u.in_combat then
        local sx, sy = clamp(scout_target_x, scout_target_y)
        ctx:attack_move({u.id}, sx, sy)
    end
end

-- === FOCUS FIRE: weakest enemy in range of attacker centroid ===
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

    local weak = ctx:weakest_enemy_in_range(cx, cy, 12)
    if weak then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, weak.id)
    end
end

-- === KITE: ranged flee close enemies when outnumbered ===
if outnumbered and enemies then
    for _, r in ipairs(ranged) do
        if r.attacking then
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
                local fx, fy = clamp(flee_x, flee_y)
                ctx:move_units({r.id}, fx, fy)
            end
        end
    end
end

-- === BUILDING PUSH ===
-- When area clear or strong advantage, push toward NestingBox (priority) or TheBurrow
local should_push = (enemy_count == 0 and my_count >= 2) or strong_advantage

if should_push and enemy_buildings and #enemy_buildings > 0 then
    local target = nil
    local target_dist = 999999

    local HQ_KINDS = { TheBurrow = true }
    local hq = nil

    for _, b in ipairs(enemy_buildings) do
        -- Prioritize NestingBox
        if b.kind == "NestingBox" then
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < target_dist then
                target_dist = d
                target = b
            end
        end
        if HQ_KINDS[b.kind] then hq = b end
    end

    -- Fall back to HQ if no NestingBox left
    if not target then target = hq end

    -- Fall back to nearest building
    if not target then
        local nd = 999999
        for _, b in ipairs(enemy_buildings) do
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < nd then
                nd = d
                target = b
            end
        end
    end

    if target then
        ctx:attack_move(all_combat_ids, target.x, target.y)
    end
end
