-- @name: combat_micro_threat_count
-- @events: on_tick
-- @interval: 3

-- Gen 57: Gen 34 base + threat-count targeting.
-- Target the enemy currently attacking the most of our units.
-- Killing the biggest threat first reduces total incoming damage fastest.
-- Uses per-unit threat tracking with enemy IDs.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- Classify units
local combat_units = {}
local attackers = {}
local ranged_attackers = {}
local all_combat_ids = {}
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

-- === RETREAT ===
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

-- === THREAT-COUNT TARGETING ===
-- Count how many of our units each enemy is threatening
if #attackers >= 2 and enemies and #enemies > 0 then
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #attackers
    cy = cy / #attackers

    -- Build threat count for enemies near our army
    local enemy_threat_count = {}
    local enemy_by_id = {}
    for _, e in ipairs(enemies) do
        enemy_by_id[e.id] = e
        enemy_threat_count[e.id] = 0
    end

    -- For each of our combat units, check what's threatening them
    for _, u in ipairs(combat_units) do
        local threats = ctx:threats_to(u.id)
        if threats then
            for _, t in ipairs(threats) do
                if enemy_threat_count[t.id] then
                    enemy_threat_count[t.id] = enemy_threat_count[t.id] + 1
                end
            end
        end
    end

    -- Find enemy with highest threat count within range 12 of centroid
    local best_target = nil
    local best_threat = 0
    for _, e in ipairs(enemies) do
        local dx = e.x - cx
        local dy = e.y - cy
        local d = dx * dx + dy * dy
        if d < 12 * 12 then
            local tc = enemy_threat_count[e.id] or 0
            if tc > best_threat then
                best_threat = tc
                best_target = e
            elseif tc == best_threat and best_target then
                -- Tiebreak: closer to centroid
                local bd = (best_target.x - cx) * (best_target.x - cx)
                    + (best_target.y - cy) * (best_target.y - cy)
                if d < bd then
                    best_target = e
                end
            end
        end
    end

    -- Fallback to closest if no threats detected
    if not best_target then
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
    end

    if best_target then
        local ids = {}
        for _, u in ipairs(attackers) do
            table.insert(ids, u.id)
        end
        ctx:attack_units(ids, best_target.id)
    end
end

-- === KITE when outnumbered ===
if outnumbered and enemies and #ranged_attackers > 0 then
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

-- === PUSH ===
local should_push = (enemy_count == 0 and my_combat_count >= 2)
    or strong_advantage

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
