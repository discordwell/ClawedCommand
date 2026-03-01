-- @name: push_buildings
-- @events: on_tick
-- @interval: 5

-- When the area is clear of enemy combat units, redirect idle attackers
-- to push enemy buildings. The FSM's Attack phase does attack-move to the
-- enemy Box, but units can get stuck fighting individual enemies forever.
-- This script ensures we actually destroy buildings when we have the chance.

local army = ctx:my_units()
if #army == 0 then return end

local enemy_buildings = ctx:enemy_buildings()
if #enemy_buildings == 0 then return end

-- Find idle or finished-fighting military units near enemy territory
local pushers = {}
for _, u in ipairs(army) do
  if u.kind == "Pawdler" or u.is_dead then goto skip end
  -- Include idle units and units that have no current target
  if u.is_idle or (not u.is_attacking and not u.is_moving) then
    table.insert(pushers, u)
  end
  ::skip::
end

if #pushers == 0 then return end

-- Find nearest enemy building to our pushers
local px, py = 0, 0
for _, u in ipairs(pushers) do
  px = px + u.x
  py = py + u.y
end
px = math.floor(px / #pushers)
py = math.floor(py / #pushers)

-- Check if area near pushers is safe (no strong enemy force)
local nearby_enemies = ctx:enemies_in_range(px, py, 8)
local enemy_combat = 0
for _, e in ipairs(nearby_enemies) do
  if e.kind ~= "Pawdler" then
    enemy_combat = enemy_combat + 1
  end
end

-- Only push buildings if enemy combat units nearby are few
if enemy_combat > #pushers / 2 then return end

-- Find closest enemy building
local best_building = nil
local best_dist = 999999
for _, b in ipairs(enemy_buildings) do
  local dx = b.x - px
  local dy = b.y - py
  local d2 = dx * dx + dy * dy
  if d2 < best_dist then
    best_dist = d2
    best_building = b
  end
end

if not best_building then return end

-- Send pushers to attack-move toward the building
local ids = {}
for _, u in ipairs(pushers) do
  table.insert(ids, u.id)
end
ctx:attack_move(ids, best_building.x, best_building.y)
