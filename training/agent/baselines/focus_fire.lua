-- @name: focus_fire
-- @events: on_tick
-- @interval: 3

-- Focus fire on weakest enemy, but ONLY for units already in combat.
-- Don't override FSM attack-move orders for units still marching.

local army = ctx:my_units()
if #army == 0 then return end

-- Only consider units that are currently attacking (already in combat)
local in_combat = {}
for _, u in ipairs(army) do
  if u.kind ~= "Pawdler" and not u.is_dead and u.is_attacking then
    table.insert(in_combat, u)
  end
end

if #in_combat == 0 then return end

-- Compute centroid of engaged units
local cx, cy = 0, 0
for _, u in ipairs(in_combat) do
  cx = cx + u.x
  cy = cy + u.y
end
cx = math.floor(cx / #in_combat)
cy = math.floor(cy / #in_combat)

-- Find weakest enemy near our fighting units
local target = ctx:weakest_enemy_in_range(cx, cy, 8)
if not target then return end

-- Redirect all fighting units to focus on the weakest target
local ids = {}
for _, u in ipairs(in_combat) do
  table.insert(ids, u.id)
end

ctx:attack_units(ids, target.id)
