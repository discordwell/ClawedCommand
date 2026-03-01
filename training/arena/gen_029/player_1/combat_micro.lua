-- @name: combat_micro_v7
-- @events: on_tick
-- @interval: 3

-- Gen 29: Gen 26 base + worker harassment.
-- Detach one Nuisance (fast unit) to hunt enemy workers while main army fights.
-- Worker kills disrupt enemy economy and can swing close games.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- Classify units
local combat_units = {}
local attackers = {}
local ranged_attackers = {}
local all_combat_ids = {}
local harasser = nil  -- first available Nuisance for worker hunting
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        -- Reserve first healthy Nuisance as harasser
        if not harasser and u.kind == "Nuisance" then
            local hp_pct = u.hp / math.max(u.hp_max, 1)
            if hp_pct > 0.5 then
                harasser = u
                -- Don't add to combat pool — this unit harasses
            else
                table.insert(combat_units, u)
                table.insert(all_combat_ids, u.id)
            end
        else
            table.insert(combat_units, u)
            table.insert(all_combat_ids, u.id)
        end
        if u.attacking then
            table.insert(attackers, u)
            if u.kind == "Hisser" or u.kind == "Yowler"
                or u.kind == "FlyingFox" or u.kind == "Catnapper" then
                table.insert(ranged_attackers, u)
            end
        end
    end
end

local my_combat_count = #combat_units
if my_combat_count == 0 and not harasser then return end

-- === WORKER HARASSMENT ===
if harasser then
    local enemies = ctx:enemy_units()
    if enemies then
        -- Find nearest enemy worker
        local target_worker = nil
        local target_dist = 999999
        for _, e in ipairs(enemies) do
            if e.kind == "Pawdler" or e.kind == "Scrounger"
                or e.kind == "Delver" or e.kind == "Ponderer" then
                local dx = e.x - harasser.x
                local dy = e.y - harasser.y
                local d = dx * dx + dy * dy
                if d < target_dist then
                    target_dist = d
                    target_worker = e
                end
            end
        end
        if target_worker then
            ctx:attack_units({harasser.id}, target_worker.id)
        else
            -- No workers visible, harass toward enemy buildings
            local enemy_buildings = ctx:enemy_buildings()
            if enemy_buildings and #enemy_buildings > 0 then
                local nearest = enemy_buildings[1]
                ctx:attack_move({harasser.id}, nearest.x, nearest.y)
            end
        end
    end
end

if my_combat_count == 0 then return end

-- Army centroid (excluding harasser)
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

-- Enemies (recalculate for main army logic)
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local outnumbered = my_combat_count < enemy_count
local strong_advantage = my_combat_count >= enemy_count + 3

-- === RETREAT wounded when outnumbered ===
local retreat_ids = {}
for _, u in ipairs(combat_units) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.30 and u.attacking and outnumbered then
        table.insert(retreat_ids, u.id)
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- === FOCUS FIRE ===
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
            -- Don't redirect harasser
            if not harasser or u.id ~= harasser.id then
                table.insert(ids, u.id)
            end
        end
        if #ids > 0 then
            ctx:attack_units(ids, weak.id)
        end
    end
end

-- === KITE: ranged units when outnumbered ===
if outnumbered and enemies and #ranged_attackers > 0 then
    for _, r in ipairs(ranged_attackers) do
        if harasser and r.id == harasser.id then
            -- Don't kite the harasser
        else
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
            if closest_dist < 5 then
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
