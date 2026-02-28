-- basic_gather: Send idle Pawdlers to the nearest resource deposit
-- Intents: gather, harvest, mine, collect, get resources
local units = ctx:get_units()
local resources = ctx:get_resources()

-- Find Pawdlers not already gathering
local idle_pawdlers = {}
for _, u in ipairs(units) do
    if u.kind == "Pawdler" and not u.gathering then
        table.insert(idle_pawdlers, u)
    end
end

if #idle_pawdlers == 0 then return end

-- Find nearest deposit to first idle Pawdler
local best_dist = math.huge
local best_deposit = nil
local p = idle_pawdlers[1]
for _, d in ipairs(resources.deposits or {}) do
    local dx = d.x - p.x
    local dy = d.y - p.y
    local dist = dx * dx + dy * dy
    if dist < best_dist then
        best_dist = dist
        best_deposit = d
    end
end

if best_deposit then
    local ids = {}
    for _, pw in ipairs(idle_pawdlers) do
        table.insert(ids, pw.id)
    end
    ctx:gather_resource(ids, best_deposit.id)
end
