-- @name: gap_fill
-- @events: on_tick
-- @interval: 5

-- Gen 9: Fill production gaps with low threshold + short idle gate.
--
-- Gen 7 (250 food threshold): Never triggered. Exact baseline.
-- Gen 8 (150 food, 30 tick idle): Never triggered. Exact baseline.
-- The FSM keeps buildings busy and spends food as fast as it accumulates.
--
-- This script tries 110 food and 15 tick idle wait. The idea: the FSM
-- evaluates every 5 ticks. If a building has been idle for 15 ticks (3
-- FSM cycles) AND food is 110+ (just 10 over Hisser cost), we train.
-- This is more aggressive but the idle gate ensures the FSM had 3 chances
-- to act first.

if _G._gap_tracker == nil then
    _G._gap_tracker = {}
end
local tracker = _G._gap_tracker

local res = ctx:resources()
if not res then return end

-- Track CatTree idle time
local trees = ctx:my_buildings("CatTree")
if not trees then return end

for _, b in ipairs(trees) do
    if b.under_construction then
        tracker[b.id] = nil
    elseif b.producing then
        tracker[b.id] = 0
    else
        tracker[b.id] = (tracker[b.id] or 0) + 5
    end
end

-- Also track ServerRack
local racks = ctx:my_buildings("ServerRack")
if racks then
    for _, b in ipairs(racks) do
        if b.under_construction then
            tracker[b.id] = nil
        elseif b.producing then
            tracker[b.id] = 0
        else
            tracker[b.id] = (tracker[b.id] or 0) + 5
        end
    end
end

-- Supply check
if res.supply >= res.supply_cap then return end
local headroom = res.supply_cap - res.supply

-- CatTree -> Hisser (primary)
if headroom >= 2 and res.food >= 110 then
    for _, b in ipairs(trees) do
        local idle = tracker[b.id] or 0
        if idle >= 15 and not b.producing and not b.under_construction then
            ctx:train(b.id, "Hisser")
            tracker[b.id] = 0
            res.food = res.food - 100
            headroom = headroom - 2
            break
        end
    end
end

-- ServerRack -> Mouser (secondary, only if surplus GPU)
if racks and headroom >= 1 and res.food >= 85 and res.gpu_cores >= 35 then
    for _, b in ipairs(racks) do
        local idle = tracker[b.id] or 0
        if idle >= 15 and not b.producing and not b.under_construction then
            ctx:train(b.id, "Mouser")
            tracker[b.id] = 0
            break
        end
    end
end
