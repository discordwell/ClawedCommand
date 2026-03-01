-- @name: economy_boost
-- @events: on_tick
-- @interval: 15

-- Gen 3: Worker management + critical-HP retreat.
--
-- Two lightweight jobs, both proven safe:
--
-- JOB 1: Idle worker assignment (identical logic to Gen 2 worker_helper).
-- The FSM sometimes leaves Pawdlers idle after build orders or when
-- deposits deplete. This catches those gaps.
--
-- JOB 2: Retreat units below 10% HP.
-- A unit at 10% is 1-2 hits from death regardless. Pulling it back
-- can't make things worse (it's contributing almost nothing) and might
-- save it for one more attack cycle. We issue ONE move command per unit
-- per 30 ticks to avoid spamming and overriding the FSM continuously.
--
-- KEY RULES:
-- 1. Worker job ONLY touches Pawdlers. Zero interaction with combat.
-- 2. Retreat job uses move_units ONLY on sub-10% HP units, ONLY once
--    per 30-tick window per unit.
-- 3. Retreat target is toward our own base (TheBox position), not some
--    random safe position that might scatter the army.

-- Persistent state for retreat cooldowns.
if _G._eb_state == nil then
    _G._eb_state = {
        retreat_cooldowns = {},  -- unit_id -> last_retreat_tick
    }
end
local state = _G._eb_state
local tick = ctx:tick()

-- ============================================================
-- JOB 1: Idle Worker Assignment
-- ============================================================

local idle_workers = ctx:idle_units("Pawdler")

for _, worker in ipairs(idle_workers) do
    -- Triple-check: only truly idle workers
    if worker.idle and not worker.gathering and not worker.moving and not worker.attacking then
        local deposit = ctx:nearest_deposit(worker.x, worker.y)
        if deposit and deposit.remaining > 0 then
            ctx:gather({worker.id}, deposit.id)
        end
    end
end

-- ============================================================
-- JOB 2: Critical HP Retreat
-- ============================================================

-- Find units below 10% HP that are in combat.
local critical = ctx:wounded_units(0.10)

if #critical == 0 then
    return
end

-- Find our base position (TheBox) as the retreat target.
-- Retreating toward base keeps the unit near the army's rally point
-- rather than scattering to some arbitrary "safe" position.
local base = nil
local boxes = ctx:my_buildings("TheBox")
if boxes and #boxes > 0 then
    base = boxes[1]
end

if base == nil then
    -- No base? We've already lost. Don't bother retreating.
    return
end

local RETREAT_COOLDOWN = 30  -- ticks between retreat commands per unit

for _, u in ipairs(critical) do
    -- Only retreat combat units that are actively in the fight.
    -- Don't touch Pawdlers (Job 1 handles them) and don't touch
    -- units that are already moving (they might be retreating already
    -- or the FSM is repositioning them).
    if u.kind ~= "Pawdler" and u.attacking then
        local last_retreat = state.retreat_cooldowns[u.id] or -100

        if (tick - last_retreat) >= RETREAT_COOLDOWN then
            -- Move toward base, but not all the way — just pull back
            -- a few tiles. We compute a point 5 tiles toward base from
            -- the unit's current position.
            local dx = base.x - u.x
            local dy = base.y - u.y
            local dist = math.sqrt(dx * dx + dy * dy)

            if dist > 1 then
                -- Normalize and move 5 tiles toward base
                local step = math.min(5, dist)
                local rx = math.floor(u.x + (dx / dist) * step)
                local ry = math.floor(u.y + (dy / dist) * step)
                ctx:move_units({u.id}, rx, ry)
                state.retreat_cooldowns[u.id] = tick
            end
        end
    end
end

-- Cleanup: remove stale cooldown entries for dead units.
-- We only do this every ~150 ticks (10 calls at interval 15) to avoid
-- overhead. Simple modular check.
if tick % 150 == 0 then
    local alive = {}
    local my_units = ctx:my_units()
    for _, u in ipairs(my_units) do
        alive[u.id] = true
    end
    for uid, _ in pairs(state.retreat_cooldowns) do
        if not alive[uid] then
            state.retreat_cooldowns[uid] = nil
        end
    end
end
