-- @name: smart_retreat
-- @events: on_unit_attacked
-- @interval: 8

-- Conservative retreat: Only pull back units about to die.
-- Key lesson from Gen 0: retreating too many units reduces our DPS.
-- Only retreat individual units below 10% HP (they're about to die anyway,
-- better to try to save them than lose them completely).
-- Never retreat entire army — that gives the enemy free damage.

local wounded = ctx:wounded_units(0.10)
if #wounded == 0 then return end

local boxes = ctx:my_buildings("TheBox")
if #boxes == 0 then return end
local base_x, base_y = boxes[1].x, boxes[1].y

-- Check for nearby towers to retreat to instead
local towers = ctx:my_buildings("LaserPointer")
local retreat_x, retreat_y = base_x, base_y
if #towers > 0 then
  for _, t in ipairs(towers) do
    for _, w in ipairs(wounded) do
      local dx = t.x - w.x
      local dy = t.y - w.y
      local db = (base_x - w.x) * (base_x - w.x) + (base_y - w.y) * (base_y - w.y)
      if dx * dx + dy * dy < db then
        retreat_x, retreat_y = t.x, t.y
      end
      break
    end
    break
  end
end

local ids = {}
for _, u in ipairs(wounded) do
  -- Don't retreat workers (FSM handles them) or dead units
  if u.kind ~= "Pawdler" and not u.is_dead then
    table.insert(ids, u.id)
  end
end

if #ids > 0 then
  ctx:move_units(ids, retreat_x, retreat_y)
end
