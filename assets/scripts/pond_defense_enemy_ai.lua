-- @name: pond_defense_enemy_ai
-- @events: on_tick
-- @interval: 3

-- Clawed raider AI for Pond Defense mission.
-- Centroid-based focus fire (proven Gen 42 pattern).
-- Wave-aware targeting: north group -> north pond, east -> lily pond, south -> south pond.
-- Conditional kiting for ranged units when outnumbered.

local my_units = ctx:my_units()
if not my_units then return end
if #my_units == 0 then return end

local enemies = ctx:enemy_units()
if not enemies then return end
if #enemies == 0 then return end

-- Pond target positions
local NORTH_POND = {x = 10, y = 14}
local LILY_POND  = {x = 13, y = 21}
local SOUTH_POND = {x = 10, y = 33}

-- Classify units by approximate spawn region (y-position)
local north_group = {}
local east_group = {}
local south_group = {}

for _, u in ipairs(my_units) do
    if u.y < 18 then
        table.insert(north_group, u)
    elseif u.y > 28 then
        table.insert(south_group, u)
    else
        table.insert(east_group, u)
    end
end

-- Find closest enemy to army centroid for focus fire
local function find_focus_target(units, enemies_list)
    if #units == 0 or #enemies_list == 0 then return nil end

    -- Compute centroid of our group
    local cx, cy = 0, 0
    for _, u in ipairs(units) do
        cx = cx + u.x
        cy = cy + u.y
    end
    cx = cx / #units
    cy = cy / #units

    -- Find closest enemy to our centroid
    local best = nil
    local best_dist = math.huge
    for _, e in ipairs(enemies_list) do
        local dx = e.x - cx
        local dy = e.y - cy
        local dist = dx * dx + dy * dy
        if dist < best_dist then
            best_dist = dist
            best = e
        end
    end
    return best
end

-- Track kited unit IDs so attack_group skips them
local kited_set = {}

-- Conditional kiting for ranged units (Hisser) when outnumbered
local RANGED_KINDS = { Hisser = true }
local my_count = #my_units
local enemy_count = #enemies

if my_count < enemy_count then
    for _, u in ipairs(my_units) do
        if RANGED_KINDS[u.kind] then
            local nearest = ctx:nearest_enemy(u.id)
            if nearest then
                local dx = u.x - nearest.x
                local dy = u.y - nearest.y
                local dist_sq = dx * dx + dy * dy
                -- If enemy is too close, kite away
                if dist_sq < 9 then
                    local flee_x = u.x + dx
                    local flee_y = u.y + dy
                    -- Check terrain passability
                    local cost = ctx:movement_cost(flee_x, flee_y)
                    if cost then
                        ctx:move_units({u.id}, flee_x, flee_y)
                        kited_set[u.id] = true
                    end
                end
            end
        end
    end
end

-- Attack-move a group toward their pond target, focusing nearest enemy
-- Excludes units that are currently kiting
local function attack_group(group, pond_target)
    if #group == 0 then return end

    local ids = {}
    local non_kited = {}
    for _, u in ipairs(group) do
        if not kited_set[u.id] then
            table.insert(ids, u.id)
            table.insert(non_kited, u)
        end
    end

    if #ids == 0 then return end

    local target = find_focus_target(non_kited, enemies)
    if target then
        -- Focus fire on closest enemy to group centroid
        ctx:attack_units(ids, target.id)
    else
        -- Fall back to attack-moving toward the pond
        ctx:attack_move(ids, pond_target.x, pond_target.y)
    end
end

-- Direct each group toward its designated pond
attack_group(north_group, NORTH_POND)
attack_group(east_group, LILY_POND)
attack_group(south_group, SOUTH_POND)
