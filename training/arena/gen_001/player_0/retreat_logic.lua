-- @name: retreat_logic
-- @events: on_unit_attacked, on_tick
-- @interval: 5

-- Retreat Logic: Pull wounded units out of combat before they die.
-- Fires reactively on_unit_attacked AND periodic on_tick as safety net.
-- The FSM wastes units by fighting to the death. Even pulling one unit
-- back from 30% HP and letting it survive changes the attrition math.

-- Thresholds: units below these HP% retreat
local CRITICAL_HP = 0.25   -- always retreat, no exceptions
local LOW_HP = 0.40        -- retreat if outnumbered or squishy
local SAFE_DISTANCE_SQ = 100  -- 10 tiles from nearest enemy = safe

-- Units that should never retreat (waste of commands)
local NO_RETREAT = {
    Pawdler = true,  -- workers, not in combat ideally
}

-- Expensive units worth saving at higher HP thresholds
local HIGH_VALUE = {
    Hisser = true,
    MechCommander = true,
    Catnapper = true,
    Yowler = true,
}

local function collect_ids(units)
    local ids = {}
    for _, u in ipairs(units) do
        ids[#ids + 1] = u.id
    end
    return ids
end

-- Phase 1: Use the built-in behavior for critical wounds
-- This handles the simple case efficiently
local critical = ctx:wounded_units(CRITICAL_HP)
local retreat_critical = {}
for _, u in ipairs(critical) do
    if not NO_RETREAT[u.kind] and not u.gathering then
        retreat_critical[#retreat_critical + 1] = u
    end
end

if #retreat_critical > 0 then
    -- Use the behavior composite for retreat
    ctx.behaviors:retreat_wounded(CRITICAL_HP)
end

-- Phase 2: Smarter retreat for moderately wounded high-value units
-- Check if they're in danger (enemies closing in) before retreating
local moderate_wounded = ctx:wounded_units(LOW_HP)

for _, u in ipairs(moderate_wounded) do
    -- Skip if already below critical (handled above) or exempt
    if NO_RETREAT[u.kind] or u.gathering then
        -- skip
    elseif ctx:hp_pct(u.id) and ctx:hp_pct(u.id) > CRITICAL_HP then
        -- Only retreat moderate wounds if high-value OR outnumbered locally
        local dominated = false

        if HIGH_VALUE[u.kind] then
            dominated = true  -- always protect expensive units
        else
            -- Check local threat count vs local friendly count
            local threats = ctx:threats_to(u.id)
            local nearby_friends = ctx:enemies_in_range(u.x, u.y, 4)
            -- enemies_in_range gives enemies, we need friends in range
            -- Use a simple heuristic: if 2+ threats, retreat
            if #threats >= 2 then
                dominated = true
            end
        end

        if dominated then
            -- Find safe position and move there
            local safe_spots = ctx:safe_positions(u.id, 8)
            if safe_spots and #safe_spots > 0 then
                -- Pick the safe spot closest to our base (lowest x+y for P0
                -- since P0 typically spawns bottom-left)
                local best = safe_spots[1]
                local best_score = best.x + best.y
                for i = 2, #safe_spots do
                    local s = safe_spots[i]
                    local score = s.x + s.y
                    if score < best_score then
                        best = s
                        best_score = score
                    end
                end
                ctx:move_units({u.id}, best.x, best.y)
            end
        end
    end
end

-- Phase 3: Rally retreated units back once healed
-- Units that are far from enemies and above 70% HP should rejoin
-- (The FSM will re-issue attack-move, but this nudges them back faster)
local tick = ctx:tick()
if tick % 20 == 0 then  -- only every 2 seconds to save instructions
    local all = ctx:my_units()
    for _, u in ipairs(all) do
        if u.idle and not u.gathering and u.kind ~= "Pawdler" then
            local hp_pct = ctx:hp_pct(u.id)
            if hp_pct and hp_pct > 0.7 then
                local dist_sq = ctx:distance_squared_to_nearest_enemy(u.id)
                if dist_sq and dist_sq > SAFE_DISTANCE_SQ then
                    -- This idle healthy unit far from combat should rejoin
                    -- Let the FSM handle it on next attack-move cycle
                    -- (No action needed, FSM will pick it up)
                end
            end
        end
    end
end
