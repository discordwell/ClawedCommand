-- @name: retreat_wounded
-- @events: on_unit_attacked
-- @interval: 8

-- Retreat critically wounded units (below 15% HP).
-- Higher threshold was too aggressive and weakened army during fights.

local wounded = ctx:wounded_units(0.15)
if #wounded == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base_x, base_y = boxes[1].x, boxes[1].y

local ids = {}
for _, u in ipairs(wounded) do
  if u.kind ~= "Pawdler" then
    table.insert(ids, u.id)
  end
end

if #ids > 0 then
  ctx:move_units(ids, base_x, base_y)
end
