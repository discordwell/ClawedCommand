-- @name: prod_only
-- @events: on_tick
-- @interval: 10

-- Gen 7: Absolute minimum intervention production script.
--
-- EVERYTHING ELSE FAILED:
-- Gen 1-3: Combat micro = no improvement or worse
-- Gen 4: priority_kill + production_boost = inconsistent
-- Gen 5: Aggressive economy = 0% (stole FSM building budget)
-- Gen 6: Conservative economy + idle workers = 0% (WORSE: idle worker
--         reassignment overrode FSM build commands, reducing buildings)
--
-- THIS SCRIPT: Does ONE thing. When a CatTree sits idle (not producing,
-- not under construction), and we have 250+ food (enormous surplus),
-- train a Hisser. That's it. No worker management. No other buildings.
-- No combat commands. No resource tracking.
--
-- WHY 250 FOOD THRESHOLD:
-- The most expensive cat building is ServerRack (100 food + 75 GPU).
-- The most expensive cat unit is Chonk (125 food + 25 GPU).
-- At 250 food, we can afford the most expensive building AND the most
-- expensive unit simultaneously. This guarantees we NEVER steal food
-- from the FSM.
--
-- WHY INTERVAL 10:
-- The FSM evaluates every 5 ticks. At interval 10, we check half as
-- often as the FSM. This means the FSM always gets first pick at idle
-- buildings. We only catch buildings the FSM chose NOT to use.
--
-- WHY ONLY CATTREE:
-- CatTree produces Hissers (11.7 DPS, range 5). This is the single
-- highest-value production action available. If we can only do one
-- thing, this is it.

local res = ctx:resources()
if not res then return end

-- Absolute requirements
if res.supply >= res.supply_cap then return end
if res.supply_cap - res.supply < 2 then return end  -- Hisser costs 2 supply
if res.food < 250 then return end

-- Find an idle CatTree
local trees = ctx:my_buildings("CatTree")
if not trees then return end

for _, b in ipairs(trees) do
    if not b.producing and not b.under_construction then
        ctx:train(b.id, "Hisser")
        return  -- Only one per cycle, absolute minimum intervention
    end
end
