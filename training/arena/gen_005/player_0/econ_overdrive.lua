-- @name: econ_overdrive
-- @events: on_tick
-- @interval: 5

-- Gen 5: Aggressive production filling with low thresholds.
--
-- STRATEGY:
-- 4 generations of combat micro scripts all failed to beat baseline (20%).
-- The only positive signal came from Gen 4's production_boost, which improved
-- seed 31415 from 13/16 K/D to 15/9 K/D. But its thresholds were too high
-- (100 food for a 50-cost Pawdler, 150 for a 100-cost Hisser). It likely
-- never queued units on most seeds because the FSM was spending food first.
--
-- This script takes the production_boost concept and makes it MUCH more
-- aggressive:
--   - Pawdler threshold: 55 food (cost 50 + 5 buffer)
--   - Hisser threshold:  105 food (cost 100 + 5 buffer)
--   - Nuisance fallback: 80 food (cost 75 + 5 buffer)
--   - Mouser from rack:  80 food + 30 GPU (cost 75/25 + 5 buffer)
--
-- The small buffers mean we'll sometimes "steal" food the FSM wanted for
-- a building. But buildings cost 100-150 food and take 100-150 ticks to
-- build. A Hisser costs 100 food and takes 80 ticks to train. By the time
-- the building finishes, we've gathered more food. The net effect is more
-- units on the field.
--
-- ZERO COMBAT COMMANDS. This script never issues move, stop, hold, attack,
-- or attack_move. It only trains units and reassigns idle workers.

local tick = ctx:tick()
local res = ctx:resources()
if not res then return end

-- =======================================================================
-- JOB 0: Idle worker reassignment (every cycle)
-- =======================================================================
-- The FSM sometimes leaves Pawdlers idle after builds or deposit depletion.
-- Reassign them immediately so gathering never stops.

local idle_workers = ctx:idle_units("Pawdler")
if idle_workers then
    for _, w in ipairs(idle_workers) do
        if w.idle and not w.gathering and not w.moving and not w.attacking then
            local dep = ctx:nearest_deposit(w.x, w.y, "Food")
            if dep and dep.remaining > 0 then
                ctx:gather({w.id}, dep.id)
            else
                -- No food deposit? Try GPU.
                local gpu_dep = ctx:nearest_deposit(w.x, w.y, "GpuCores")
                if gpu_dep and gpu_dep.remaining > 0 then
                    ctx:gather({w.id}, gpu_dep.id)
                end
            end
        end
    end
end

-- =======================================================================
-- Supply check: if at cap, skip all production
-- =======================================================================
if res.supply >= res.supply_cap then
    return
end

local headroom = res.supply_cap - res.supply

-- =======================================================================
-- JOB 1: TheBox -> Pawdler (early game worker advantage)
-- =======================================================================
-- The FSM trains Pawdlers to a target of 4 in EarlyGame phase. But the
-- FSM only checks every 5 ticks and has phase transition delays. If we
-- beat it to the punch, we get workers gathering sooner.
--
-- After tick 600 (1 minute), stop training workers. 4 is enough; beyond
-- that we need combat units. We also cap at 5 workers total to never
-- over-invest.

if tick < 600 and headroom >= 1 then
    local pawdler_count = ctx:count_units("Pawdler")
    if pawdler_count < 5 and res.food >= 55 then
        local boxes = ctx:my_buildings("TheBox")
        if boxes then
            for _, b in ipairs(boxes) do
                if not b.producing and not b.under_construction then
                    ctx:train(b.id, "Pawdler")
                    break
                end
            end
        end
    end
end

-- =======================================================================
-- JOB 2: CatTree -> Hisser (primary), Nuisance (fallback)
-- =======================================================================
-- Hisser: 100 food, 0 GPU, 2 supply, 80 tick train time.
--   DPS 11.7 at range 5. Best cost-effective ranged unit.
--   Training Hissers instead of the FSM's default Nuisance mix gives
--   a ranged advantage: our army can deal damage before the blobs merge.
--
-- Nuisance fallback: 75 food, 0 GPU, 1 supply, 60 tick train time.
--   If we can't afford a Hisser but CAN afford a Nuisance, train one.
--   A Nuisance now is better than waiting for Hisser food.

if headroom >= 1 then
    local trees = ctx:my_buildings("CatTree")
    if trees then
        for _, b in ipairs(trees) do
            if not b.producing and not b.under_construction then
                if headroom >= 2 and res.food >= 105 then
                    -- Hisser: best choice
                    ctx:train(b.id, "Hisser")
                    -- Deduct from local tracking so we don't double-queue
                    -- if there are multiple CatTrees
                    res.food = res.food - 100
                    headroom = headroom - 2
                elseif res.food >= 80 then
                    -- Nuisance: fallback when we can't afford Hisser
                    ctx:train(b.id, "Nuisance")
                    res.food = res.food - 75
                    headroom = headroom - 1
                end
            end
        end
    end
end

-- =======================================================================
-- JOB 3: ServerRack -> Mouser (high DPS assassin)
-- =======================================================================
-- Mouser: 75 food, 25 GPU, 1 supply, 60 tick train time.
--   DPS 12.5 (highest in the game!), melee, 55 HP (fragile).
--   Much better DPS-per-cost than FlyingFox (5.0 DPS, 100+25 cost).
--   The FSM may train FlyingFox or Catnapper. We override with Mouser
--   whenever the rack is idle.
--
-- Only train if we have GPU to spare (the FSM needs GPU for buildings
-- like ServerRack itself: 100 food + 75 GPU).

if headroom >= 1 and res.food >= 80 and res.gpu_cores >= 30 then
    local racks = ctx:my_buildings("ServerRack")
    if racks then
        for _, b in ipairs(racks) do
            if not b.producing and not b.under_construction then
                ctx:train(b.id, "Mouser")
                res.food = res.food - 75
                res.gpu_cores = res.gpu_cores - 25
                headroom = headroom - 1
                break
            end
        end
    end
end
