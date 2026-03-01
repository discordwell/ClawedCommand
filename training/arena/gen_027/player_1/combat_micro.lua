-- @name: combat_micro_v5
-- @events: on_tick
-- @interval: 3

-- Gen 27: Refined kite — only critically wounded ranged units flee.
-- Changes from Gen 26:
-- 1. Kite only at < 20% HP (was any HP when outnumbered) — ranged units keep fighting
-- 2. Kite move doesn't override focus fire for healthy ranged units
-- 3. All melee units included in focus fire regardless of HP

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- Classify units
local combat_units = {}
local attackers = {}
local all_combat_ids = {}
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        table.insert(combat_units, u)
        table.insert(all_combat_ids, u.id)
        if u.attacking then
            table.insert(attackers, u)
        end
    end
end

local my_combat_count = #combat_units
if my_combat_count == 0 then return end

-- Army centroid
local army_cx, army_cy = 0, 0
for _, u in ipairs(combat_units) do
    army_cx = army_cx + u.x
    army_cy = army_cy + u.y
end
army_cx = army_cx / my_combat_count
army_cy = army_cy / my_combat_count

-- Rally point: nearest own building
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

-- Enemies
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local outnumbered = my_combat_count < enemy_count
local strong_advantage = my_combat_count >= enemy_count + 3

-- === RETREAT: wounded melee + critically wounded ranged ===
-- Build a set of units that will retreat (so we exclude them from focus fire)
local retreat_set = {}
for _, u in ipairs(combat_units) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    local is_ranged = (u.kind == "Hisser" or u.kind == "Yowler"
        or u.kind == "FlyingFox" or u.kind == "Catnapper")

    if u.attacking and outnumbered then
        if is_ranged and hp_pct < 0.20 then
            -- Critically wounded ranged: kite away from nearest enemy
            retreat_set[u.id] = true
            if enemies then
                local closest_dist = 999999
                local closest_ex, closest_ey = 0, 0
                for _, e in ipairs(enemies) do
                    local dx = e.x - u.x
                    local dy = e.y - u.y
                    local d = dx * dx + dy * dy
                    if d < closest_dist then
                        closest_dist = d
                        closest_ex = e.x
                        closest_ey = e.y
                    end
                end
                local flee_x = u.x - (closest_ex - u.x)
                local flee_y = u.y - (closest_ey - u.y)
                flee_x = math.max(0, math.min(map_w - 1, flee_x))
                flee_y = math.max(0, math.min(map_h - 1, flee_y))
                ctx:move_units({u.id}, flee_x, flee_y)
            end
        elseif not is_ranged and hp_pct < 0.30 then
            -- Wounded melee: retreat to rally
            retreat_set[u.id] = true
        end
    end
end

-- Melee retreat (batch)
local melee_retreat_ids = {}
for _, u in ipairs(combat_units) do
    if retreat_set[u.id] and not (u.kind == "Hisser" or u.kind == "Yowler"
        or u.kind == "FlyingFox" or u.kind == "Catnapper") then
        table.insert(melee_retreat_ids, u.id)
    end
end
if #melee_retreat_ids > 0 then
    ctx:move_units(melee_retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- === FOCUS FIRE: redirect non-retreating attackers to weakest enemy ===
local focus_ids = {}
for _, u in ipairs(attackers) do
    if not retreat_set[u.id] then
        table.insert(focus_ids, u.id)
    end
end

if #focus_ids >= 2 then
    local cx, cy = 0, 0
    local count = 0
    for _, u in ipairs(attackers) do
        if not retreat_set[u.id] then
            cx = cx + u.x
            cy = cy + u.y
            count = count + 1
        end
    end
    if count > 0 then
        cx = cx / count
        cy = cy / count
    end

    local weak = ctx:weakest_enemy_in_range(cx, cy, 12)
    if weak then
        ctx:attack_units(focus_ids, weak.id)
    end
end

-- === PUSH: attack-move toward enemy HQ ===
local should_push = (enemy_count == 0 and my_combat_count >= 2)
    or strong_advantage

if should_push then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        local hq = nil
        local nearest = nil
        local nearest_dist = 999999
        for _, b in ipairs(enemy_buildings) do
            if b.kind == "TheBox" or b.kind == "TheDumpster"
                or b.kind == "TheGrotto" or b.kind == "TheBurrow"
                or b.kind == "TheNest" or b.kind == "TheMound" then
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
