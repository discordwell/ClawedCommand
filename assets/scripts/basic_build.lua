-- basic_build: Order an idle Pawdler to build a Cat Tree
-- Intents: build, construct

local res = ctx:get_resources()
if res.food < 150 then return end

local units = ctx:my_units("Pawdler")
local buildings = ctx:my_buildings()

-- Find our Box position as reference
local base_x, base_y = 5, 5
for _, b in ipairs(buildings) do
    if b.kind == "TheBox" then
        base_x = b.x
        base_y = b.y
        break
    end
end

-- Find an idle Pawdler
for _, u in ipairs(units) do
    if u.idle then
        ctx:build(u.id, "CatTree", base_x + 3, base_y + 3)
        return
    end
end
