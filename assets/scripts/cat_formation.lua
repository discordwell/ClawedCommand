-- @name: cat_formation
-- @events: on_tick
-- @interval: 3

-- Cat formation AI: tanks in a front screen line, ranged spread behind,
-- melee split into two flank wings. Each unit gets its OWN position.
-- Gen 26 patterns: group focus fire, conditional kite, retreat wounded, push.

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

-- Direction toward enemies (normalized)
local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

-- Perpendicular vector (for spreading units in a line)
local perp_x = -dir_y
local perp_y = dir_x

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

-- Helper: clamp to map bounds
local function clamp_pos(x, y)
    return math.max(0, math.min(map_w - 1, math.floor(x))),
           math.max(0, math.min(map_h - 1, math.floor(y)))
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
    local rx, ry = clamp_pos(rally_x, rally_y)
    ctx:move_units(retreat_ids, rx, ry)
end

-- === FORMATION POSITIONING ===
-- Each unit gets its OWN position — spread along the perpendicular axis.
-- Front line (tanks): 7 tiles ahead, spaced 3 apart
-- Back line (ranged): 3 tiles behind centroid, spaced 3 apart
-- Flanks (melee): split into two wings, 5 tiles out on each side
if enemies and enemy_count > 0 then

    -- --- TANK LINE (front screen) ---
    for i, u in ipairs(tanks) do
        if not u.in_combat then
            -- Center the line: offset = (i - 1 - (count-1)/2) * spacing
            local lateral = (i - 1 - (#tanks - 1) / 2) * 3
            local tx = army_cx + dir_x * 7 + perp_x * lateral
            local ty = army_cy + dir_y * 7 + perp_y * lateral
            local cx, cy = clamp_pos(tx, ty)
            ctx:attack_move({u.id}, cx, cy)
        end
    end

    -- --- RANGED LINE (behind tanks, spread out) ---
    for i, u in ipairs(ranged) do
        if not u.in_combat then
            local lateral = (i - 1 - (#ranged - 1) / 2) * 3
            local rx = army_cx - dir_x * 3 + perp_x * lateral
            local ry = army_cy - dir_y * 3 + perp_y * lateral
            local cx, cy = clamp_pos(rx, ry)
            ctx:attack_move({u.id}, cx, cy)
        end
    end

    -- --- MELEE FLANKS (two wings) ---
    -- Split melee into left and right groups
    local left_wing = {}
    local right_wing = {}
    for i, u in ipairs(melee) do
        if i % 2 == 1 then
            table.insert(left_wing, u)
        else
            table.insert(right_wing, u)
        end
    end

    -- Left wing: forward + left
    for i, u in ipairs(left_wing) do
        if not u.in_combat then
            local forward = 5
            local lateral = -4 - (i - 1) * 2  -- spread further left
            local fx = army_cx + dir_x * forward + perp_x * lateral
            local fy = army_cy + dir_y * forward + perp_y * lateral
            local cx, cy = clamp_pos(fx, fy)
            ctx:attack_move({u.id}, cx, cy)
        end
    end

    -- Right wing: forward + right
    for i, u in ipairs(right_wing) do
        if not u.in_combat then
            local forward = 5
            local lateral = 4 + (i - 1) * 2  -- spread further right
            local fx = army_cx + dir_x * forward + perp_x * lateral
            local fy = army_cy + dir_y * forward + perp_y * lateral
            local cx, cy = clamp_pos(fx, fy)
            ctx:attack_move({u.id}, cx, cy)
        end
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
            if closest_dist < 9 then
                local flee_x = r.x - (closest_ex - r.x)
                local flee_y = r.y - (closest_ey - r.y)
                local fx, fy = clamp_pos(flee_x, flee_y)
                ctx:move_units({r.id}, fx, fy)
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
            if HQ_KINDS[b.kind] then hq = b end
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
