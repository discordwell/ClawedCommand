-- @name: priority_focus
-- @events: on_tick
-- @interval: 3

-- Focus fire on high-value targets, but DON'T redirect units near enemy base.
-- Key insight: The FSM orders attack-move to enemy Box. If we redirect those units
-- to fight individual enemies, we prevent base destruction (which wins the game).
-- Only redirect units that are in combat but NOT near the enemy base.

local PRIORITY = {
  Hisser = 1,
  Mouser = 2,
  FlyingFox = 3,
  Nuisance = 4,
  Yowler = 5,
  FerretSapper = 6,
  Catnapper = 7,
  Chonk = 8,
  Pawdler = 9,
  MechCommander = 10,
}

local army = ctx:my_units()
if #army == 0 then return end

-- Find enemy base to exclude units near it
local enemy_buildings = ctx:enemy_buildings()
local enemy_base_x, enemy_base_y = nil, nil
for _, b in ipairs(enemy_buildings) do
  if b.kind == "TheBox" then
    enemy_base_x, enemy_base_y = b.x, b.y
    break
  end
end

-- Gather combat units that are in combat but NOT near enemy base
local fighters = {}
for _, u in ipairs(army) do
  if u.kind == "Pawdler" or u.is_dead then goto skip end
  if not u.is_attacking then goto skip end

  -- Skip units within 6 tiles of enemy base — let them hit buildings
  if enemy_base_x then
    local dx = u.x - enemy_base_x
    local dy = u.y - enemy_base_y
    if dx * dx + dy * dy <= 36 then
      goto skip
    end
  end

  table.insert(fighters, u)
  ::skip::
end

if #fighters == 0 then return end

-- Compute centroid of eligible fighters
local cx, cy = 0, 0
for _, u in ipairs(fighters) do
  cx = cx + u.x
  cy = cy + u.y
end
cx = math.floor(cx / #fighters)
cy = math.floor(cy / #fighters)

-- Find enemies near our fighters
local enemies = ctx:enemies_in_range(cx, cy, 8)
if #enemies == 0 then return end

-- Pick best target by priority and HP
local best = nil
local best_score = 999
for _, e in ipairs(enemies) do
  local prio = PRIORITY[e.kind] or 6
  local hp_frac = e.hp / math.max(e.hp_max, 1)
  local score = prio * 100 + hp_frac * 50
  -- Massive bonus for nearly-dead units
  if hp_frac < 0.25 then
    score = score - 250
  end
  if score < best_score then
    best_score = score
    best = e
  end
end

if not best then return end

local ids = {}
for _, u in ipairs(fighters) do
  table.insert(ids, u.id)
end
ctx:attack_units(ids, best.id)
