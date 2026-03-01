-- basic_retreat: Move all units back toward the base
-- Intents: retreat, run, fall back

local buildings = ctx:my_buildings()

-- Find our Box (HQ) position as rally point
local rally_x, rally_y = 5, 5
for _, b in ipairs(buildings) do
    if b.kind == "TheBox" then
        rally_x = b.x
        rally_y = b.y
        break
    end
end

-- Move all units toward the base
local units = ctx:my_units()
local ids = {}
for _, u in ipairs(units) do
    table.insert(ids, u.id)
end

if #ids > 0 then
    ctx:move_units(ids, rally_x, rally_y)
end
