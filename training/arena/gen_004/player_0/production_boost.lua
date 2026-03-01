-- @name: production_boost
-- @events: on_tick
-- @interval: 7

-- Gen 4: Fill idle building production gaps.
--
-- THE PROBLEM THIS SOLVES:
-- The FSM trains units at phase transitions (every 15-30 ticks). Between
-- transitions, a building finishes production and sits idle until the FSM
-- checks again. Over a 6000-tick game, each building might waste 200-500
-- ticks of idle time -- that's 2-5 extra units never produced.
--
-- Gen 2's economy_boost only assigned idle workers. Gen 3's economy_boost
-- was identical. Neither script ever queued a single unit. The FSM's own
-- training logic was never supplemented.
--
-- THIS SCRIPT IS DIFFERENT:
-- Every 7 ticks, we check all own buildings. If a building is done
-- constructing and NOT currently producing, we queue a unit immediately.
-- This doesn't conflict with the FSM because:
--   (a) The FSM checks every 15-30 ticks; we check every 7.
--   (b) If the FSM queues first, building.producing == true and we skip.
--   (c) If we queue first, the FSM sees the building is producing and skips.
-- Net result: buildings produce continuously instead of having idle gaps.
--
-- RESOURCE AWARENESS:
-- We check resources before queueing to avoid wasting the FSM's budget on
-- something we want when it needs food for a building. We only train when
-- we have comfortable resource surplus.
--
-- UNIT SELECTION:
-- - TheBox idle → Pawdler (only if < 4 total, keeping early economy strong)
-- - CatTree idle → Hisser (best ranged DPS at 11.7, no GPU cost, range 5)
-- - ServerRack idle → Nuisance from CatTree is better value; but ServerRack
--   produces FlyingFox (fast harasser, good for reaching backline).
--   However, FlyingFox costs 25 GPU. If we don't have GPU, skip.
--
-- SUPPLY AWARENESS:
-- Don't train if at supply cap. The FSM handles building LitterBoxes.

local tick = ctx:tick()

-- Get current resources and supply.
local res = ctx:resources()
if not res then
    return
end

-- Don't train if we're at or above supply cap. The FSM builds LitterBoxes
-- and we don't want to interfere with that decision.
if res.supply >= res.supply_cap then
    return
end

local supply_headroom = res.supply_cap - res.supply

-- ============================================================
-- RULE: TheBox → Pawdler (only if < 4 Pawdlers and early game)
-- ============================================================
-- Pawdler costs 50 food. Only worth it early game (before tick 1000)
-- when more gatherers = faster economy. After that, food is better
-- spent on combat units.

if tick < 1000 and supply_headroom >= 1 then
    local pawdler_count = ctx:count_units("Pawdler")

    if pawdler_count < 4 and res.food >= 100 then
        -- 100 threshold (2x cost) ensures we don't steal food the FSM needs
        -- for a building. The FSM's first CatTree costs 150 food.
        local boxes = ctx:my_buildings("TheBox")
        if boxes then
            for _, b in ipairs(boxes) do
                if not b.producing and not b.under_construction then
                    ctx:train(b.id, "Pawdler")
                    -- Only train one per tick cycle to be conservative.
                    break
                end
            end
        end
    end
end

-- ============================================================
-- RULE: CatTree → Hisser (primary combat unit)
-- ============================================================
-- Hisser costs 100 food, 0 GPU. Best ranged DPS (11.7), range 5.
-- Train whenever the CatTree is idle and we can afford it.
-- We use a 150 food threshold to leave buffer for the FSM.
-- The FSM trains Nuisances (75 food) from CatTree in BuildUp phase.
-- We train Hissers because they're higher value and the FSM doesn't
-- specifically prioritize them.

if supply_headroom >= 2 and res.food >= 150 then
    local cat_trees = ctx:my_buildings("CatTree")
    if cat_trees then
        for _, b in ipairs(cat_trees) do
            if not b.producing and not b.under_construction then
                ctx:train(b.id, "Hisser")
                break
            end
        end
    end
end

-- ============================================================
-- RULE: ServerRack → FlyingFox (fast harasser)
-- ============================================================
-- FlyingFox costs 100 food + 25 GPU. Fastest unit (0.225 speed),
-- ranged, great for reaching enemy backline. Only train if we have
-- GPU surplus (the FSM needs GPU for buildings and research).
-- Threshold: 200 food + 75 GPU to leave plenty of buffer.

if supply_headroom >= 2 and res.food >= 200 and res.gpu_cores >= 75 then
    local racks = ctx:my_buildings("ServerRack")
    if racks then
        for _, b in ipairs(racks) do
            if not b.producing and not b.under_construction then
                ctx:train(b.id, "FlyingFox")
                break
            end
        end
    end
end
