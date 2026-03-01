-- @name: supply_ahead
-- @events: on_tick
-- @interval: 10

-- Gen 5: Proactive supply building to prevent production stalls.
--
-- THE PROBLEM THIS SOLVES:
-- The FSM builds LitterBoxes reactively: it notices supply is full, THEN
-- orders a Pawdler to build one. A LitterBox takes 75 ticks (7.5 seconds)
-- to construct. During those 75 ticks, the FSM AND our econ_overdrive
-- script are both blocked from training ANY units because supply is capped.
-- At 10 DPS of army output, that's ~75 damage-seconds of army we DON'T
-- have when the first big fight happens.
--
-- THIS SCRIPT:
-- Checks supply headroom every 10 ticks. When headroom <= 3, and we have
-- >= 80 food (LitterBox costs 75), and no LitterBox is currently under
-- construction, we order an idle Pawdler to build one near our base.
--
-- WHY HEADROOM 3:
-- A Hisser costs 2 supply. A Nuisance costs 1. If we have 3 headroom,
-- we can train 1 Hisser + 1 Nuisance before hitting cap. But the Hisser
-- takes 80 ticks to train and the LitterBox takes 75 ticks to build.
-- By starting the LitterBox at headroom 3, it finishes right as we're
-- about to need the extra supply. Perfect pipeline.
--
-- WHY NOT HEADROOM 5+:
-- Building too early wastes a worker's gathering time. At headroom 5,
-- we still have room for 2 Hissers before stalling. No urgency.
--
-- PLACEMENT:
-- Build near our TheBox. We find TheBox position and offset by (2, 2)
-- tiles. This keeps the LitterBox near our main base for safety and
-- doesn't require pathfinding to a distant location.
--
-- ZERO COMBAT COMMANDS. Only build orders.

local res = ctx:resources()
if not res then return end

local headroom = res.supply_cap - res.supply

-- Only act when supply is getting tight
if headroom > 3 then
    return
end

-- Don't build if we can't afford it (75 food)
if res.food < 80 then
    return
end

-- Check if a LitterBox is already under construction. If so, no need
-- to build another one yet.
local litter_boxes = ctx:my_buildings("LitterBox")
if litter_boxes then
    for _, lb in ipairs(litter_boxes) do
        if lb.under_construction then
            return
        end
    end
end

-- Find our base position for placement reference
local boxes = ctx:my_buildings("TheBox")
if not boxes or #boxes == 0 then
    return
end
local base = boxes[1]

-- Find an idle Pawdler to be the builder.
-- We specifically want one that's idle (not gathering, not building,
-- not moving). Taking a gathering worker off a deposit is acceptable
-- because the LitterBox build takes 75 ticks and the supply unlock
-- enables 10+ supply worth of units. The ROI is massive.
local idle_workers = ctx:idle_units("Pawdler")
local builder = nil

if idle_workers and #idle_workers > 0 then
    builder = idle_workers[1]
end

-- If no idle workers, grab a gathering worker as a last resort.
-- The supply stall is worse than losing 75 ticks of gathering.
if not builder then
    local all_workers = ctx:my_units("Pawdler")
    if all_workers then
        for _, w in ipairs(all_workers) do
            if w.gathering and not w.moving then
                builder = w
                break
            end
        end
    end
end

if not builder then
    return
end

-- Place LitterBox near base. Offset by (2, 2) to avoid overlapping
-- existing buildings. We try a few positions in case one is blocked.
local offsets = {
    {2, 2}, {-2, 2}, {2, -2}, {-2, -2},
    {3, 0}, {0, 3}, {-3, 0}, {0, -3},
}

for _, off in ipairs(offsets) do
    local bx = base.x + off[1]
    local by = base.y + off[2]

    -- Check passability to avoid building on water/mountains
    if ctx:is_passable(bx, by) then
        ctx:build(builder.id, "LitterBox", bx, by)
        return
    end
end
