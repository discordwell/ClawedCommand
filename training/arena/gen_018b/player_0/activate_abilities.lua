-- @name: activate_abilities
-- @events: on_tick
-- @interval: 5

-- Gen 18: Ability activation micro — completely untapped by FSM and previous gens.
-- Targets three high-value, low-risk abilities:
-- 1. LoafMode (Chonk slot 1): Toggle, 0 GPU, 10 tick CD. On when stationary+fighting.
-- 2. Zoomies (Nuisance slot 2): 30 tick speed boost, 10 GPU, 120 tick CD. Engage/retreat.
-- 3. DissonantScreech (Yowler slot 1): AoE stun, 10 GPU, 80 tick CD. 3+ clustered enemies.

-- Track cooldowns in script-local state (not exposed in unit snapshot)
if not _G.ability_cooldowns then _G.ability_cooldowns = {} end
local cds = _G.ability_cooldowns
local tick = ctx:tick()

-- LoafMode for stationary Chonks in combat (toggle, free)
local chonks = ctx:my_units("Chonk")
for _, c in ipairs(chonks) do
  if c.hp <= 0 then goto skip_chonk end
  local loaf_key = "loaf_active_" .. c.id
  local cd_key = "loaf_cd_" .. c.id
  local should_loaf = not c.moving and c.attacking
  local is_loafing = cds[loaf_key] or false
  local cd_ok = not cds[cd_key] or tick - cds[cd_key] > 10

  if should_loaf and not is_loafing and cd_ok then
    -- Toggle ON
    ctx:ability(c.id, 1, "self")
    cds[loaf_key] = true
    cds[cd_key] = tick
  elseif not should_loaf and is_loafing and cd_ok then
    -- Toggle OFF (Chonk needs to move)
    ctx:ability(c.id, 1, "self")
    cds[loaf_key] = false
    cds[cd_key] = tick
  end
  ::skip_chonk::
end

-- Zoomies for Nuisances engaging or retreating at low HP
local nuisances = ctx:my_units("Nuisance")
for _, n in ipairs(nuisances) do
  if n.hp <= 0 then goto skip_nui end
  local key = "zoom_" .. n.id
  local last = cds[key] or 0
  if tick - last < 120 then goto skip_nui end  -- On cooldown

  local hp_pct = n.hp / math.max(n.hp_max, 1)
  local res = ctx:resources()
  if res and res.gpu_cores >= 10 then
    -- Retreat boost (low HP) or engagement boost (healthy + fighting)
    if hp_pct < 0.3 or (n.attacking and hp_pct > 0.5) then
      ctx:ability(n.id, 2, "self")
      cds[key] = tick
    end
  end
  ::skip_nui::
end

-- DissonantScreech for Yowlers when enemies are clustered
local yowlers = ctx:my_units("Yowler")
for _, y in ipairs(yowlers) do
  if y.hp <= 0 then goto skip_yowl end
  local key = "screech_" .. y.id
  local last = cds[key] or 0
  if tick - last < 80 then goto skip_yowl end  -- On cooldown

  local res = ctx:resources()
  if not res or res.gpu_cores < 10 then goto skip_yowl end

  -- Check for 3+ enemies near this yowler (range ~4 tiles)
  local nearby = ctx:enemies_in_range(y.x, y.y, 4)
  if #nearby >= 3 then
    -- Target center of enemy cluster
    local ex, ey = 0, 0
    for _, e in ipairs(nearby) do
      ex = ex + e.x
      ey = ey + e.y
    end
    ex = math.floor(ex / #nearby)
    ey = math.floor(ey / #nearby)
    ctx:ability(y.id, 1, "position", ex, ey)
    cds[key] = tick
  end
  ::skip_yowl::
end
