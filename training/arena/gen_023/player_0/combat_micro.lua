-- @name: combat_micro
-- @events: on_tick
-- @interval: 3

-- Gen 23: Refined combat micro — per-unit focus fire targeting.
-- Changes from Gen 21:
-- 1. Each attacker targets the weakest enemy within ITS OWN range (not centroid)
-- 2. Retreat toward army centroid (safety in numbers) instead of home base
-- 3. Push is more aggressive: requires only 3+ units and 0 visible enemies

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()

-- Classify our units
local combat_units = {}
local all_combat_ids = {}
for _, u in ipairs(my_units) do
    local is_worker = (u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer")
    if not is_worker then
        table.insert(combat_units, u)
        table.insert(all_combat_ids, u.id)
    end
end

local my_combat_count = #combat_units
if my_combat_count == 0 then return end

-- Army centroid (for retreat direction)
local army_cx, army_cy = 0, 0
for _, u in ipairs(combat_units) do
    army_cx = army_cx + u.x
    army_cy = army_cy + u.y
end
army_cx = army_cx / my_combat_count
army_cy = army_cy / my_combat_count

-- Count visible enemies
local enemies = ctx:enemy_units()
local enemy_count = 0
if enemies then enemy_count = #enemies end

local outnumbered = my_combat_count < enemy_count

-- === PER-UNIT FOCUS FIRE + RETREAT ===
for _, u in ipairs(combat_units) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)

    -- Retreat badly wounded units toward army centroid (not home)
    if hp_pct < 0.25 and u.attacking and outnumbered then
        ctx:move_units({u.id}, math.floor(army_cx), math.floor(army_cy))
    elseif u.attacking then
        -- Focus fire: target weakest enemy within this unit's attack range + 2
        local weak = ctx:weakest_enemy_in_range(u.x, u.y, 8)
        if weak then
            ctx:attack_units({u.id}, weak.id)
        end
    end
end

-- === PUSH: when no enemies visible, attack-move toward enemy buildings ===
if enemy_count == 0 and my_combat_count >= 3 then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        -- Find nearest enemy building to our army
        local best = nil
        local best_dist = 999999
        for _, b in ipairs(enemy_buildings) do
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < best_dist then
                best_dist = d
                best = b
            end
        end

        if best then
            ctx:attack_move(all_combat_ids, best.x, best.y)
        end
    end
end
