-- @name: kite_ranged
-- @events: on_unit_attacked
-- @interval: 3

-- Only kite Hissers that are actively being hit by melee.
-- Don't interfere with FSM movement orders for non-threatened ranged units.

local hissers = ctx:my_units("Hisser")
for _, u in ipairs(hissers) do
  if u.is_dead then goto continue end

  -- Only kite if HP is dropping (we're being attacked)
  local hp_pct = ctx:hp_pct(u.id)
  if not hp_pct or hp_pct > 0.9 then goto continue end

  -- Check for melee threats specifically
  local threats = ctx:threats_to(u.id)
  local closest_melee = nil
  for _, t in ipairs(threats) do
    if t.attack_type == "Melee" then
      closest_melee = t
      break
    end
  end

  if closest_melee then
    -- Kite to maintain max range
    local kx, ky = ctx:position_at_range(u.x, u.y, closest_melee.x, closest_melee.y, 5)
    if kx then
      ctx:move_units({u.id}, kx, ky)
    end
  end

  ::continue::
end
