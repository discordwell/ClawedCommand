-- @name: combat_micro
-- @events: on_tick
-- @interval: 3

-- Gen 21: Combat micro + post-fight push.
-- 1. Focus fire weakest enemy
-- 2. Retreat badly wounded when outnumbered
-- 3. After clearing nearby enemies, push all combat units to enemy buildings

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- Determine home position from first unit's quadrant
local home_x, home_y = 6, 6
for _, u in ipairs(my_units) do
    if u.x > map_w / 2 then
        home_x, home_y = map_w - 6, map_h - 6
    end
    break
end

-- Classify our units
local combat_units = {}
local attackers = {}
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        table.insert(combat_units, u)
        if u.attacking then
            table.insert(attackers, u)
        end
    end
end

local my_combat_count = #combat_units
if my_combat_count == 0 then return end

-- Count visible enemies
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local army_advantage = my_combat_count > enemy_count

-- === RETREAT wounded when outnumbered ===
local retreat_ids = {}
for _, u in ipairs(combat_units) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.25 and u.attacking and not army_advantage then
        table.insert(retreat_ids, u.id)
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, home_x, home_y)
end

-- === FOCUS FIRE: redirect attackers to weakest enemy ===
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

-- === PUSH: when no enemies nearby, attack-move toward their buildings ===
-- This prevents the FSM's retreat-after-attack cycle.
-- Only push when we have enough units and the coast is clear.
if enemy_count == 0 and my_combat_count >= 3 then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        -- Find our army centroid
        local cx, cy = 0, 0
        for _, u in ipairs(combat_units) do
            cx = cx + u.x
            cy = cy + u.y
        end
        cx = cx / my_combat_count
        cy = cy / my_combat_count

        -- Find nearest enemy building
        local best = nil
        local best_dist = 999999
        for _, b in ipairs(enemy_buildings) do
            local dx = b.x - cx
            local dy = b.y - cy
            local d = dx * dx + dy * dy
            if d < best_dist then
                best_dist = d
                best = b
            end
        end

        if best then
            local ids = {}
            for _, u in ipairs(combat_units) do
                table.insert(ids, u.id)
            end
            ctx:attack_move(ids, best.x, best.y)
        end
    end
end
