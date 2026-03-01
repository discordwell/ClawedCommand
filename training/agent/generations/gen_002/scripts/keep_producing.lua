-- @name: keep_producing
-- @events: on_tick
-- @interval: 10

-- Keep production buildings busy. The FSM reserves food for planned buildings,
-- which means it sometimes leaves production buildings idle when it could train.
-- This script queues units at idle production buildings if we have surplus resources.
-- Only trains if queue is empty (no FSM queued unit this tick) and food is above
-- a safety threshold to avoid bankrupting building plans.

local res = ctx:resources()
if not res then return end

-- Don't train if we're supply capped
if res.supply >= res.supply_cap then return end

-- Safety threshold: only train if food is comfortably above 0
-- This avoids starving the FSM's building plans
local food_threshold = 100

-- Check CatTree
local cat_trees = ctx:my_buildings("CatTree")
for _, ct in ipairs(cat_trees) do
  if ct.under_construction then goto skip_ct end
  if #ct.production_queue > 0 then goto skip_ct end
  -- CatTree is idle — queue a unit
  if res.food >= food_threshold + 100 then
    -- Prefer Hissers (highest DPS/supply for ranged), alternate with Nuisance
    local tick = ctx:tick()
    local unit_type = "Nuisance"
    if tick % 30 < 10 then
      unit_type = "Hisser"
    elseif tick % 30 < 20 then
      unit_type = "Chonk"
    end
    ctx:train(ct.id, unit_type)
  end
  ::skip_ct::
end

-- Check ServerRack
local racks = ctx:my_buildings("ServerRack")
for _, sr in ipairs(racks) do
  if sr.under_construction then goto skip_sr end
  if #sr.production_queue > 0 then goto skip_sr end
  -- ServerRack is idle — queue a unit
  if res.food >= food_threshold + 100 and res.gpu_cores >= 25 then
    ctx:train(sr.id, "FlyingFox")
  end
  ::skip_sr::
end

-- Check TheBox — keep training workers if we have few
local boxes = ctx:my_buildings("TheBox")
local workers = ctx:my_units("Pawdler")
for _, b in ipairs(boxes) do
  if #b.production_queue > 0 then goto skip_box end
  -- Train more workers if below 5 and have food
  if #workers < 5 and res.food >= food_threshold + 50 then
    ctx:train(b.id, "Pawdler")
  end
  ::skip_box::
end
