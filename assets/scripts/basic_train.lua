-- basic_train: Train units from production buildings
-- Intents: train, make units, produce

local res = ctx:get_resources()
local buildings = ctx:my_buildings()

for _, b in ipairs(buildings) do
    -- Cat Tree produces combat units
    if b.kind == "CatTree" and not b.under_construction then
        if res.food >= 50 then
            ctx:train(b.id, "Nuisance")
        end
    end
end
