-- @name: combat_micro
-- @events: on_tick
-- @interval: 3

-- Combat Micro: Focus fire on weakest enemy and kite with ranged units.
-- Runs every 3 ticks (0.3s) for responsive combat control.
-- Strategy: Concentrate all damage on one target to secure kills faster
-- than the enemy's distributed damage. Ranged units maintain distance.

local function collect_ids(units)
    local ids = {}
    for _, u in ipairs(units) do
        ids[#ids + 1] = u.id
    end
    return ids
end

-- Only act if there are visible enemies
local enemies = ctx:enemy_units()
if #enemies == 0 then
    return
end

-- Split our army into melee and ranged
local my_units = ctx:my_units()
local melee = {}
local ranged = {}
local ranged_ids = {}

for _, u in ipairs(my_units) do
    -- Skip workers and gathering units
    if u.kind ~= "Pawdler" and not u.gathering then
        if u.attack_type == "Ranged" then
            ranged[#ranged + 1] = u
            ranged_ids[#ranged_ids + 1] = u.id
        else
            melee[#melee + 1] = u
        end
    end
end

-- No combat units, nothing to do
if #melee == 0 and #ranged == 0 then
    return
end

-- === FOCUS FIRE ===
-- Find the weakest enemy near our army centroid for focus fire.
-- Killing one unit fast is better than damaging many.

-- Calculate army center
local cx, cy = 0, 0
local combat_count = #melee + #ranged
for _, u in ipairs(melee) do
    cx = cx + u.x
    cy = cy + u.y
end
for _, u in ipairs(ranged) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / combat_count
cy = cy / combat_count

-- Find weakest enemy within a reasonable engagement radius (12 tiles)
local target = ctx:weakest_enemy_in_range(cx, cy, 12)

if target then
    -- Priority override: if there's a high-DPS squishy target, prefer it
    -- Hissers (14 dmg, 70 hp) and Mousers (10 dmg, 55 hp) are priority kills
    local priority_target = nil
    for _, e in ipairs(enemies) do
        local dx = e.x - cx
        local dy = e.y - cy
        local dist_sq = dx * dx + dy * dy
        if dist_sq <= 144 then -- within 12 tiles
            if e.kind == "Hisser" or e.kind == "Mouser" then
                if priority_target == nil or e.hp < priority_target.hp then
                    priority_target = e
                end
            end
        end
    end

    -- Use priority target if found and it's not much healthier than weakest
    local chosen = target
    if priority_target and priority_target.hp <= target.hp * 2 then
        chosen = priority_target
    end

    -- Direct all melee to attack the chosen target
    if #melee > 0 then
        local melee_ids = collect_ids(melee)
        ctx:attack_units(melee_ids, chosen.id)
    end

    -- Ranged units also focus the target
    if #ranged > 0 then
        ctx:attack_units(ranged_ids, chosen.id)
    end
end

-- === KITING ===
-- Ranged units should maintain distance from approaching melee enemies.
-- Only kite if an enemy melee unit is closing in within range 2.
-- Hissers (range 5) should stay at range 4-5, not stand at range 1.

for _, r in ipairs(ranged) do
    -- Check if any melee enemy is dangerously close (within 2 tiles)
    local threats = ctx:threats_to(r.id)
    local close_melee = false
    local closest_threat = nil
    local closest_dist_sq = 999999

    for _, t in ipairs(threats) do
        if t.attack_type == "Melee" then
            local dx = t.x - r.x
            local dy = t.y - r.y
            local d2 = dx * dx + dy * dy
            if d2 <= 9 then -- within 3 tiles
                close_melee = true
                if d2 < closest_dist_sq then
                    closest_dist_sq = d2
                    closest_threat = t
                end
            end
        end
    end

    if close_melee and closest_threat then
        -- Move to maintain attack range distance
        local kx, ky = ctx:position_at_range(
            r.x, r.y,
            closest_threat.x, closest_threat.y,
            r.range
        )
        if kx then
            ctx:move_units({r.id}, kx, ky)
        end
    end
end
