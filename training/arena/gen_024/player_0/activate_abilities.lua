-- @name: activate_abilities
-- @events: on_tick
-- @interval: 5

-- Gen 24: Extended ability activation — builds on Gen 18b success.
-- Additions over Gen 18b:
-- 4. TacticalUplink (MechCommander slot 0): Toggle team CD reduction. Keep active always.
-- 5. Lower DissonantScreech threshold from 3 to 2 clustered enemies.
-- 6. More aggressive Zoomies: trigger at <0.4 HP (was 0.3) and healthy+attacking.

if not _G.ability_cooldowns then _G.ability_cooldowns = {} end
local cds = _G.ability_cooldowns
local tick = ctx:tick()

-- TacticalUplink for MechCommanders — toggle ON and leave on
local mechs = ctx:my_units("MechCommander")
for _, m in ipairs(mechs) do
  if m.hp <= 0 then goto skip_mech end
  local key = "uplink_active_" .. m.id
  local cd_key = "uplink_cd_" .. m.id
  local is_active = cds[key] or false
  local cd_ok = not cds[cd_key] or tick - cds[cd_key] > 10

  if not is_active and cd_ok then
    ctx:ability(m.id, 0, "self")
    cds[key] = true
    cds[cd_key] = tick
  end
  ::skip_mech::
end

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
    ctx:ability(c.id, 1, "self")
    cds[loaf_key] = true
    cds[cd_key] = tick
  elseif not should_loaf and is_loafing and cd_ok then
    ctx:ability(c.id, 1, "self")
    cds[loaf_key] = false
    cds[cd_key] = tick
  end
  ::skip_chonk::
end

-- Zoomies for Nuisances — more aggressive thresholds
local nuisances = ctx:my_units("Nuisance")
for _, n in ipairs(nuisances) do
  if n.hp <= 0 then goto skip_nui end
  local key = "zoom_" .. n.id
  local last = cds[key] or 0
  if tick - last < 120 then goto skip_nui end

  local hp_pct = n.hp / math.max(n.hp_max, 1)
  local res = ctx:resources()
  if res and res.gpu_cores >= 10 then
    -- Retreat at <40% HP (was 30%), engage when healthy + fighting
    if hp_pct < 0.4 or (n.attacking and hp_pct > 0.5) then
      ctx:ability(n.id, 2, "self")
      cds[key] = tick
    end
  end
  ::skip_nui::
end

-- DissonantScreech for Yowlers — lowered threshold to 2+ enemies
local yowlers = ctx:my_units("Yowler")
for _, y in ipairs(yowlers) do
  if y.hp <= 0 then goto skip_yowl end
  local key = "screech_" .. y.id
  local last = cds[key] or 0
  if tick - last < 80 then goto skip_yowl end

  local res = ctx:resources()
  if not res or res.gpu_cores < 10 then goto skip_yowl end

  local nearby = ctx:enemies_in_range(y.x, y.y, 4)
  if #nearby >= 2 then
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
