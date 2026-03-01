-- @name: idle_prod
-- @events: on_tick
-- @interval: 5

-- Gen 8: Train from buildings idle for 30+ ticks.
--
-- THE KEY INSIGHT:
-- The FSM evaluates every 5 ticks. If a building has been idle for 30 ticks
-- (6 FSM evaluation cycles), the FSM CHOSE not to train from it. This means
-- either: (a) the FSM doesn't want to produce right now (saving for building),
-- or (b) the FSM's phase doesn't include production from this building.
--
-- Case (a) is safe because the FSM already decided not to spend food.
-- Case (b) is where we add value: the FSM skips CatTree production during
-- some phases, leaving Hisser potential on the table.
--
-- We use persistent state (_G) to track how many consecutive ticks each
-- building has been idle. Only when idle_ticks >= 30 do we train.
--
-- THRESHOLD: 150 food. This is enough to cover the most expensive building
-- (CatTree: 150 food). If the FSM wanted to build something, it would have
-- spent the food in the last 30 ticks. It didn't, so we can safely spend 100
-- on a Hisser and still have 50 left.
--
-- NO IDLE WORKER MANAGEMENT. Gen 6 proved that idle worker reassignment
-- overrides FSM build orders. We don't touch workers at all.
--
-- NO COMBAT COMMANDS. Only train.

-- Initialize persistent state
if _G._idle_tracker == nil then
    _G._idle_tracker = {}  -- building_id -> idle_ticks
end
local tracker = _G._idle_tracker

local res = ctx:resources()
if not res then return end

-- Update idle tracking for all CatTrees
local trees = ctx:my_buildings("CatTree")
if not trees then return end

for _, b in ipairs(trees) do
    if b.under_construction then
        tracker[b.id] = nil  -- Don't track buildings under construction
    elseif b.producing then
        tracker[b.id] = 0    -- Reset counter when producing
    else
        -- Building is idle
        tracker[b.id] = (tracker[b.id] or 0) + 5  -- +5 because interval is 5
    end
end

-- Supply check
if res.supply >= res.supply_cap then return end
if res.supply_cap - res.supply < 2 then return end  -- Hisser costs 2 supply

-- Food check: need enough surplus that we're not stealing from FSM
if res.food < 150 then return end

-- Find a CatTree that's been idle for 30+ ticks
for _, b in ipairs(trees) do
    local idle_time = tracker[b.id] or 0
    if idle_time >= 30 and not b.producing and not b.under_construction then
        ctx:train(b.id, "Hisser")
        tracker[b.id] = 0  -- Reset after training
        return  -- Only one per cycle
    end
end
