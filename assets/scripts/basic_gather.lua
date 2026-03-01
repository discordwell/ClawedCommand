-- basic_gather: Send idle Pawdlers to nearest resource
-- Intents: gather, harvest, mine

local units = ctx:my_units("Pawdler")
local deposits = ctx:resource_deposits()

if #deposits == 0 then return end

for _, u in ipairs(units) do
    if u.idle then
        -- Find nearest deposit
        local best = nil
        local best_dist = math.huge
        for _, d in ipairs(deposits) do
            local dx = d.x - u.x
            local dy = d.y - u.y
            local dist = dx * dx + dy * dy
            if dist < best_dist then
                best_dist = dist
                best = d
            end
        end
        if best then
            ctx:gather({u.id}, best.id)
        end
    end
end
