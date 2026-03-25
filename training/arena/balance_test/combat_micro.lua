-- @name: balance_test_total_hold
-- @events: on_tick
-- @interval: 3

-- Faction-generic Total Hold (adapted from Gen 107).
-- Uses stat-based role detection instead of unit-kind checks.
-- Hold all combat units, focus fire closest, push when advantage >= 3 or tick >= 5000.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local current_tick = ctx:tick()

-- Classify units by stats (faction-agnostic)
local combat_units = {}
local ranged_units = {}
local melee_units = {}
local attackers = {}
local non_attackers = {}

for _, u in ipairs(my_units) do
    if not u.is_worker then
        table.insert(combat_units, u)
        if u.range and u.range > 1 then
            table.insert(ranged_units, u)
        else
            table.insert(melee_units, u)
        end
        if u.attacking then
            table.insert(attackers, u)
        else
            table.insert(non_attackers, u)
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

-- Enemies
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local advantage = my_combat_count - enemy_count
local strong_advantage = advantage >= 3
local should_push = strong_advantage or current_tick >= 5000

-- === FOCUS FIRE: closest enemy to attacker centroid ===
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

-- === HOLD all non-attacking units (pre-push) ===
if not should_push then
    for _, u in ipairs(non_attackers) do
        ctx:hold({u.id})
    end
end

-- === PUSH when strong advantage or very late ===
if should_push then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        -- Find nearest enemy building to army centroid
        local nearest = nil
        local nearest_dist = 999999

        for _, b in ipairs(enemy_buildings) do
            local bx = b.x - army_cx
            local by = b.y - army_cy
            local d = bx * bx + by * by
            if d < nearest_dist then
                nearest_dist = d
                nearest = b
            end
        end

        if nearest then
            local idle_ids = {}
            for _, u in ipairs(combat_units) do
                if not u.attacking then
                    table.insert(idle_ids, u.id)
                end
            end
            if #idle_ids > 0 then
                ctx:attack_move(idle_ids, nearest.x, nearest.y)
            end
        end
    end
end
