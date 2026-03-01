-- @name: cat_formation
-- @events: on_tick
-- @interval: 3

-- Cat formation AI: tanks screen, ranged stay behind, melee flank.
-- Incorporates Gen 26 proven patterns: group focus fire, conditional kite,
-- retreat wounded, aggressive push.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- === CLASSIFY UNITS ===
local tanks = {}
local ranged = {}
local melee = {}
local all_combat = {}
local all_combat_ids = {}

local TANK_KINDS = { Chonk = true }
local RANGED_KINDS = { Hisser = true, Yowler = true, FlyingFox = true, Catnapper = true }
local WORKER_KINDS = { Pawdler = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if TANK_KINDS[u.kind] then
            table.insert(tanks, u)
        elseif RANGED_KINDS[u.kind] then
            table.insert(ranged, u)
        else
            table.insert(melee, u)
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

-- Direction vector from army to enemies (normalized-ish)
local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

-- === RALLY POINT (nearest own building) ===
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

-- === RETREAT WOUNDED (<30% HP when outnumbered) ===
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.30 and u.attacking and outnumbered then
        table.insert(retreat_ids, u.id)
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- === FORMATION POSITIONING ===
-- Only reposition units that are idle (not already engaged)
if enemies and enemy_count > 0 then
    -- Tanks: advance toward enemy cluster as frontline screen
    local tank_ids = {}
    for _, u in ipairs(tanks) do
        if u.idle then
            table.insert(tank_ids, u.id)
        end
    end
    if #tank_ids > 0 then
        -- Tanks move toward enemies
        local tx = math.floor(army_cx + dir_x * 4)
        local ty = math.floor(army_cy + dir_y * 4)
        tx = math.max(0, math.min(map_w - 1, tx))
        ty = math.max(0, math.min(map_h - 1, ty))
        ctx:attack_move(tank_ids, tx, ty)
    end

    -- Ranged: stay 3-4 tiles behind the tank line
    local ranged_idle_ids = {}
    for _, u in ipairs(ranged) do
        if u.idle then
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

    -- Melee DPS: flank from the side, attack-move toward enemies
    local melee_idle_ids = {}
    for _, u in ipairs(melee) do
        if u.idle then
            table.insert(melee_idle_ids, u.id)
        end
    end
    if #melee_idle_ids > 0 then
        -- Perpendicular flank offset
        local flank_x = math.floor(army_cx + dir_x * 3 + dir_y * 3)
        local flank_y = math.floor(army_cy + dir_y * 3 - dir_x * 3)
        flank_x = math.max(0, math.min(map_w - 1, flank_x))
        flank_y = math.max(0, math.min(map_h - 1, flank_y))
        ctx:attack_move(melee_idle_ids, flank_x, flank_y)
    end
end

-- === FOCUS FIRE: all attackers target the weakest enemy ===
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

-- === KITE: ranged units flee close enemies ONLY when outnumbered ===
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
            if closest_dist < 9 then -- 3 tiles squared
                local flee_x = r.x - (closest_ex - r.x)
                local flee_y = r.y - (closest_ey - r.y)
                flee_x = math.max(0, math.min(map_w - 1, flee_x))
                flee_y = math.max(0, math.min(map_h - 1, flee_y))
                ctx:move_units({r.id}, flee_x, flee_y)
            end
        end
    end
end

-- === PUSH: attack-move toward enemy HQ when advantage ===
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
            if HQ_KINDS[b.kind] then
                hq = b
            end
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < nearest_dist then
                nearest_dist = d
                nearest = b
            end
        end

        local target = hq or nearest
        if target then
            ctx:attack_move(all_combat_ids, target.x, target.y)
        end
    end
end
