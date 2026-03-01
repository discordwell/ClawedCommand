-- @name: production_fill
-- @events: on_tick
-- @interval: 7

-- Gen 6: Conservative production gap filling.
--
-- LESSONS FROM GEN 4 AND GEN 5:
-- Gen 4 (production_boost, thresholds 100/150/200): P0 built 5 buildings (same
-- as baseline). Seed 31415 K/D improved from 13/16 to 15/9. Other seeds mixed.
-- Gen 5 (econ_overdrive, thresholds 55/105/80): P0 built only 4 buildings
-- (one fewer than baseline). 0% win rate. Script stole food from FSM's
-- building budget, preventing CatTree/ServerRack construction.
--
-- THIS SCRIPT: Uses Gen 4's conservative approach. High thresholds ensure
-- the FSM always has enough food to build what it needs. We only fill
-- production gaps when there's genuine surplus.
--
-- REMOVED from Gen 4: priority_kill combat micro (hurt seeds 42, 123).
-- ADDED: idle worker reassignment (safe, only uses gather command).
--
-- ZERO COMBAT COMMANDS. Only train and gather.

local res = ctx:resources()
if not res then return end

-- =======================================================================
-- JOB 0: Idle worker reassignment
-- =======================================================================
-- The FSM leaves Pawdlers idle after build orders complete or deposits
-- deplete. Reassign them to nearest deposit. This is pure upside: no
-- food cost, no interaction with FSM decisions.

local idle_workers = ctx:idle_units("Pawdler")
if idle_workers then
    for _, w in ipairs(idle_workers) do
        if w.idle and not w.gathering and not w.moving and not w.attacking then
            local dep = ctx:nearest_deposit(w.x, w.y)
            if dep and dep.remaining > 0 then
                ctx:gather({w.id}, dep.id)
            end
        end
    end
end

-- =======================================================================
-- Supply check
-- =======================================================================
if res.supply >= res.supply_cap then
    return
end

local headroom = res.supply_cap - res.supply

-- =======================================================================
-- JOB 1: CatTree -> Hisser (only with large food surplus)
-- =======================================================================
-- Hisser: 100 food, 2 supply, 80 tick train time, 11.7 DPS at range 5.
-- Threshold 200 food: ensures FSM can still afford its most expensive
-- building (ServerRack: 100 food + 75 GPU) even after we train a unit.

if headroom >= 2 and res.food >= 200 then
    local trees = ctx:my_buildings("CatTree")
    if trees then
        for _, b in ipairs(trees) do
            if not b.producing and not b.under_construction then
                ctx:train(b.id, "Hisser")
                res.food = res.food - 100
                headroom = headroom - 2
                break
            end
        end
    end
end

-- =======================================================================
-- JOB 2: ServerRack -> Mouser (only with GPU + food surplus)
-- =======================================================================
-- Mouser: 75 food, 25 GPU, 1 supply, 12.5 DPS (highest in game).
-- Threshold 200 food + 100 GPU: very conservative. Only train when
-- we clearly have surplus.

if headroom >= 1 and res.food >= 200 and res.gpu_cores >= 100 then
    local racks = ctx:my_buildings("ServerRack")
    if racks then
        for _, b in ipairs(racks) do
            if not b.producing and not b.under_construction then
                ctx:train(b.id, "Mouser")
                break
            end
        end
    end
end
