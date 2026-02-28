-- basic_retreat: Move all selected units back toward the player's base
-- Intents: retreat, run, fall back, go home
local units = ctx:get_units()
local buildings = ctx:get_buildings()

if #units == 0 then return end

-- Find The Box (home base)
local home_x, home_y = 5, 5
for _, b in ipairs(buildings) do
    if b.kind == "TheBox" then
        home_x = b.x
        home_y = b.y
        break
    end
end

local ids = {}
for _, unit in ipairs(units) do
    table.insert(ids, unit.id)
end

ctx:move_units(ids, home_x, home_y)
