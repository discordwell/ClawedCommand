-- @name: pulse_stop_bonus
-- @events: on_tick
-- @interval: 1

-- Pulse Stop: every 5 seconds, stops all units for exactly 1 tick
-- to trigger the voice command stop bonus, then resumes activity.
-- Combat units attack-move toward enemies; workers re-gather.
-- Stateless (fresh VM each tick) — uses tick % COOLDOWN for phasing.

local COOLDOWN = 50 -- 5 seconds at 10hz
local tick = ctx:tick()
local phase = tick % COOLDOWN

-- 48 out of 50 ticks: do nothing
if phase > 1 then return end

local units = ctx:my_units()
if not units or #units == 0 then return end

-- Phase 0: stop all units
if phase == 0 then
    local ids = {}
    for _, u in ipairs(units) do
        ids[#ids + 1] = u.id
    end
    ctx:stop(ids)
    return
end

-- Phase 1: resume previous activity
local enemies = ctx:enemy_units()

-- Separate workers from combat units
local combat_ids = {}
local workers = {}
for _, u in ipairs(units) do
    if u.kind == "Pawdler" or u.kind == "Scrounger"
        or u.kind == "Delver" or u.kind == "Ponderer"
        or u.kind == "MurderScrounger" or u.kind == "Nibblet" then
        workers[#workers + 1] = u
    else
        combat_ids[#combat_ids + 1] = u.id
    end
end

-- Combat units: attack-move toward enemy centroid
if #combat_ids > 0 and enemies and #enemies > 0 then
    local ex, ey = 0, 0
    for _, e in ipairs(enemies) do
        ex = ex + e.x
        ey = ey + e.y
    end
    ex = math.floor(ex / #enemies)
    ey = math.floor(ey / #enemies)
    ctx:attack_move(combat_ids, ex, ey)
end

-- Workers: re-gather on nearest deposit
for _, w in ipairs(workers) do
    local dep = ctx:nearest_deposit(w.x, w.y)
    if dep then
        ctx:gather({w.id}, dep.id)
    end
end
