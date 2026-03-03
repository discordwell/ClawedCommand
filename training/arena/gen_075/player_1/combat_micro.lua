-- @name: combat_micro_hold_ranged_momentum
-- @events: on_tick
-- @interval: 3

-- Gen 75: Gen 072 (hold ranged) + Gen 068 (momentum push).
-- Combines the two best improvements: disciplined ranged + aggressive push trigger.
-- Push when ANY army lead AND enemies visible, fallback tick 5000.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local current_tick = ctx:tick()

-- Classify units
local combat_units = {}
local attackers = {}
local ranged_attackers = {}
local all_combat_ids = {}
local idle_tanks = {}
local idle_ranged = {}
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        table.insert(combat_units, u)
        table.insert(all_combat_ids, u.id)
        if u.attacking then
            table.insert(attackers, u)
            if u.kind == "Hisser" or u.kind == "Yowler"
                or u.kind == "FlyingFox" or u.kind == "Catnapper" then
                table.insert(ranged_attackers, u)
            end
        else
            if u.kind == "Chonk" or u.kind == "MechCommander" then
                table.insert(idle_tanks, u)
            elseif u.kind == "Hisser" or u.kind == "Yowler"
                or u.kind == "FlyingFox" or u.kind == "Catnapper" then
                table.insert(idle_ranged, u)
            end
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

-- Rally point
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
local late_game = (my_combat_count > enemy_count and enemies and #enemies > 0) or current_tick >= 5000

-- === RETREAT (disabled in late game) ===
if not late_game then
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
end

-- === FOCUS FIRE: closest enemy to centroid ===
if #attackers >= 2 and enemies and #enemies > 0 then
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #attackers
    cy = cy / #attackers

    local best_target = nil
    local best_dist = 12 * 12
    for _, e in ipairs(enemies) do
        local dx = e.x - cx
        local dy = e.y - cy
        local d = dx * dx + dy * dy
        if d < best_dist then
            best_dist = d
            best_target = e
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

-- === INLINE FORMATION (from Gen 053) ===
if enemies and #enemies > 0 and not outnumbered and not late_game then
    local nearest_ex, nearest_ey = 0, 0
    local nearest_d2 = 999999
    for _, e in ipairs(enemies) do
        local dx = e.x - army_cx
        local dy = e.y - army_cy
        local d2 = dx * dx + dy * dy
        if d2 < nearest_d2 then
            nearest_d2 = d2
            nearest_ex = e.x
            nearest_ey = e.y
        end
    end

    local dx = nearest_ex - army_cx
    local dy = nearest_ey - army_cy
    local dist = math.sqrt(dx * dx + dy * dy)

    if dist >= 4 and dist <= 15 then
        local nx = dx / dist
        local ny = dy / dist

        for _, t in ipairs(idle_tanks) do
            local tx = math.floor(army_cx + nx * 3)
            local ty = math.floor(army_cy + ny * 3)
            tx = math.max(0, math.min(map_w - 1, tx))
            ty = math.max(0, math.min(map_h - 1, ty))
            ctx:attack_move({t.id}, tx, ty)
        end

        for _, r in ipairs(idle_ranged) do
            local rx = math.floor(army_cx - nx * 2)
            local ry = math.floor(army_cy - ny * 2)
            rx = math.max(0, math.min(map_w - 1, rx))
            ry = math.max(0, math.min(map_h - 1, ry))
            -- Move to position, then hold once close enough
            local dx_r = r.x - rx
            local dy_r = r.y - ry
            if dx_r * dx_r + dy_r * dy_r < 2 * 2 then
                ctx:hold({r.id})
            else
                ctx:move_units({r.id}, rx, ry)
            end
        end
    end
end

-- === KITE when outnumbered (disabled in late game) ===
if not late_game and outnumbered and enemies and #ranged_attackers > 0 then
    for _, r in ipairs(ranged_attackers) do
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

-- === PUSH (forced after tick 4000) ===
local should_push = (enemy_count == 0 and my_combat_count >= 2)
    or strong_advantage
    or late_game

if should_push then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        local prod_target = nil
        local prod_dist = 999999
        local hq = nil
        local nearest = nil
        local nearest_dist = 999999

        for _, b in ipairs(enemy_buildings) do
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy

            if b.kind == "CatTree" or b.kind == "ServerRack"
                or b.kind == "ScrapHeap" or b.kind == "JunkServer"
                or b.kind == "SpawningPools" or b.kind == "SunkenServer"
                or b.kind == "MoleHill" or b.kind == "DeepServer"
                or b.kind == "RookeryNest" or b.kind == "DataCrypt"
                or b.kind == "ChopShop" or b.kind == "TinkerBench" then
                if d < prod_dist then
                    prod_dist = d
                    prod_target = b
                end
            end

            if b.kind == "TheBox" or b.kind == "TheDumpster"
                or b.kind == "TheGrotto" or b.kind == "TheBurrow"
                or b.kind == "TheNest" or b.kind == "TheMound" then
                hq = b
            end

            if d < nearest_dist then
                nearest_dist = d
                nearest = b
            end
        end

        local target = prod_target or hq or nearest
        if target then
            ctx:attack_move(all_combat_ids, target.x, target.y)
        end
    end
end
