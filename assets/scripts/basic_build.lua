-- basic_build: Order a Pawdler to build a Cat Tree near the base
-- Intents: build, construct, make building
local units = ctx:get_units()
local buildings = ctx:get_buildings()

-- Find a Pawdler
local pawdler = nil
for _, u in ipairs(units) do
    if u.kind == "Pawdler" then
        pawdler = u
        break
    end
end

if not pawdler then return end

-- Find The Box for reference position
local base_x, base_y = 5, 5
for _, b in ipairs(buildings) do
    if b.kind == "TheBox" then
        base_x = b.x
        base_y = b.y
        break
    end
end

-- Build Cat Tree offset from base
ctx:build(pawdler.id, "CatTree", base_x + 3, base_y + 2)
