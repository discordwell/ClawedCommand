-- @name: tactical_focus
-- @events: on_tick
-- @interval: 10

-- Gen 2: Conservative focus fire.
--
-- RULES (lessons from Gen 1 catastrophe):
-- 1. ONLY command units that are already attacking (attacking == true).
--    Never command moving, idle, or gathering units.
-- 2. ONLY issue attack_units commands. Never move, attack_move, or stop.
-- 3. Don't re-target units already attacking the chosen target.
-- 4. Require 3+ units attacking different targets before intervening.
--    If the army is already focused, do nothing.
-- 5. Pick the weakest enemy that our units are already engaged with.
--    Don't pick some distant enemy -- stay within the current fight.

-- Collect all our combat units that are actively attacking right now.
local my_units = ctx:my_units()
local attackers = {}

for _, u in ipairs(my_units) do
    if u.attacking and u.kind ~= "Pawdler" then
        attackers[#attackers + 1] = u
    end
end

-- Need at least 3 attacking units for focus fire to matter.
-- Below that, redirection overhead isn't worth it.
if #attackers < 3 then
    return
end

-- Find what targets our attackers are currently engaged with.
-- We don't have a direct "current target" field, so we check which enemies
-- are within attack range of each attacker. The enemy closest to each
-- attacker is likely their current target.
-- We track which enemies are being attacked by counting nearby attackers.
local enemy_engagement = {}  -- enemy_id -> {enemy=unit, attacker_count=N}
local distinct_targets = 0

for _, a in ipairs(attackers) do
    -- Find enemies within this unit's attack range
    local targets = ctx:targets_for(a.id)
    if targets and #targets > 0 then
        -- The nearest enemy in range is most likely the current target
        local nearest = targets[1]
        local nearest_dist = 999999
        for _, t in ipairs(targets) do
            local dx = t.x - a.x
            local dy = t.y - a.y
            local d2 = dx * dx + dy * dy
            if d2 < nearest_dist then
                nearest_dist = d2
                nearest = t
            end
        end

        if nearest then
            if not enemy_engagement[nearest.id] then
                enemy_engagement[nearest.id] = {enemy = nearest, attacker_count = 0}
                distinct_targets = distinct_targets + 1
            end
            enemy_engagement[nearest.id].attacker_count =
                enemy_engagement[nearest.id].attacker_count + 1
        end
    end
end

-- If 2 or fewer distinct targets, the army is already reasonably focused.
-- Don't intervene. This is the key conservatism check.
if distinct_targets <= 2 then
    return
end

-- Pick the best focus target from enemies our units are already fighting.
-- Priority: lowest HP enemy that at least one of our units can reach.
-- This ensures we're finishing off a wounded unit, not chasing a new one.
local best_target = nil
local best_hp = 999999

for _, info in pairs(enemy_engagement) do
    local e = info.enemy
    if e.hp < best_hp then
        best_hp = e.hp
        best_target = e
    end
end

-- Fallback: if somehow no target found (shouldn't happen), bail out.
if not best_target then
    return
end

-- Redirect attackers to focus on the chosen target.
-- CRITICAL: Skip units that are already attacking something near the
-- chosen target (within 2 tiles). We can't know exact targets, but
-- if a unit is attacking and the chosen target is the nearest enemy
-- in their range, they're probably already on it.
local redirect_ids = {}

for _, a in ipairs(attackers) do
    local targets = ctx:targets_for(a.id)
    if targets and #targets > 0 then
        -- Check if chosen target is in this unit's range
        local can_reach = false
        local already_nearest = false

        for _, t in ipairs(targets) do
            if t.id == best_target.id then
                can_reach = true
            end
        end

        if can_reach then
            -- Check if the chosen target is already the nearest enemy
            -- (meaning this unit is probably already attacking it)
            local nearest_dist = 999999
            local nearest_id = nil
            for _, t in ipairs(targets) do
                local dx = t.x - a.x
                local dy = t.y - a.y
                local d2 = dx * dx + dy * dy
                if d2 < nearest_dist then
                    nearest_dist = d2
                    nearest_id = t.id
                end
            end

            if nearest_id == best_target.id then
                -- This unit's nearest target IS the focus target.
                -- Probably already attacking it. Don't re-issue command.
                already_nearest = true
            end

            if not already_nearest then
                redirect_ids[#redirect_ids + 1] = a.id
            end
        end
        -- If the unit can't reach the target, leave it alone.
        -- Don't pull it off whatever it's currently fighting.
    end
end

-- Only issue the command if we're actually redirecting someone.
if #redirect_ids > 0 then
    ctx:attack_units(redirect_ids, best_target.id)
end
