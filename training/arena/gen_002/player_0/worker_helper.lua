-- @name: worker_helper
-- @events: on_tick
-- @interval: 15

-- Gen 2: Idle worker assignment.
--
-- RULES:
-- 1. ONLY touch Pawdlers. Zero interaction with combat units.
-- 2. ONLY command idle Pawdlers (idle == true, not gathering, not moving).
-- 3. Send them to the nearest resource deposit.
-- 4. That's it. Nothing else.
--
-- The FSM sometimes leaves workers idle after they finish a build order
-- or when deposits deplete. This catches those gaps.

local idle_workers = ctx:idle_units("Pawdler")

-- Nothing to do if all workers are busy.
if #idle_workers == 0 then
    return
end

for _, worker in ipairs(idle_workers) do
    -- Double-check: skip if moving, attacking, gathering, or building
    if worker.idle and not worker.gathering and not worker.moving and not worker.attacking then
        -- Find the nearest resource deposit to this specific worker
        local deposit = ctx:nearest_deposit(worker.x, worker.y)
        if deposit and deposit.remaining > 0 then
            ctx:gather({worker.id}, deposit.id)
        end
    end
end
