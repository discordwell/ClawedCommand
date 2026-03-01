-- @name: formation_orient
-- @events: on_tick
-- @interval: 5

-- Maintains combat formation oriented toward nearest enemy.
-- Tanks and heavies lead at the front, ranged and fragile units stay in back.
-- Only repositions non-attacking units to avoid interrupting combat.
-- Units reorient as the nearest enemy changes — can get "spun" by fast flankers.

local WORKERS = {
  Pawdler = true, Scrounger = true, Delver = true,
  Nibblet = true, MurderScrounger = true, Ponderer = true,
}

-- Units above this HP are beefy enough to lead the formation
local TANK_HP = 180
-- Units below this HP are fragile — keep them in the back even if melee
local FRAGILE_HP = 70

local all_units = ctx:my_units()
if not all_units or #all_units == 0 then return end

local enemies = ctx:enemy_units()
if not enemies or #enemies == 0 then return end

-- Classify all combat units, separate out repositionable ones by role
local all_combat = {}
local front = {}   -- tanks/heavies: lead the charge
local mid = {}     -- standard melee: follow behind tanks
local back = {}    -- ranged, fragile, support: stay behind

for _, u in ipairs(all_units) do
  if not WORKERS[u.kind] and not u.gathering then
    table.insert(all_combat, u)

    -- Only reposition units not currently fighting
    if not u.attacking then
      if u.hp_max >= TANK_HP then
        table.insert(front, u)
      elseif u.attack_type == "Ranged" or u.hp_max < FRAGILE_HP then
        table.insert(back, u)
      else
        table.insert(mid, u)
      end
    end
  end
end

-- Need a real army to form up (< 3 units just fight in a blob)
if #all_combat < 3 then return end

-- Nothing to reposition
if #front + #mid + #back == 0 then return end

-- Army centroid (all combat units, not just idle ones)
local cx, cy = 0, 0
for _, u in ipairs(all_combat) do
  cx = cx + u.x
  cy = cy + u.y
end
cx = cx / #all_combat
cy = cy / #all_combat

-- Find nearest enemy to army centroid — formation pivots on this
local nearest = nil
local nearest_d2 = 999999
for _, e in ipairs(enemies) do
  local dx = e.x - cx
  local dy = e.y - cy
  local d2 = dx * dx + dy * dy
  if d2 < nearest_d2 then
    nearest_d2 = d2
    nearest = e
  end
end

if not nearest then return end

-- Direction vector toward nearest enemy
local dx = nearest.x - cx
local dy = nearest.y - cy
local dist = math.sqrt(dx * dx + dy * dy)

-- Too close to shuffle — let them brawl
if dist < 4 then return end

local nx = dx / dist
local ny = dy / dist

-- Perpendicular vector for lateral spread
local px = -ny
local py = nx

-- === Issue formation commands ===

-- Front line: tanks 4 tiles ahead, spread perpendicular
if #front > 0 then
  local half = (#front - 1) / 2
  for i, u in ipairs(front) do
    local spread = (i - 1 - half) * 2
    local tx = math.floor(cx + nx * 4 + px * spread)
    local ty = math.floor(cy + ny * 4 + py * spread)
    ctx:attack_move({u.id}, tx, ty)
  end
end

-- Mid line: standard melee 2 tiles ahead
if #mid > 0 then
  local half = (#mid - 1) / 2
  for i, u in ipairs(mid) do
    local spread = (i - 1 - half) * 2
    local tx = math.floor(cx + nx * 2 + px * spread)
    local ty = math.floor(cy + ny * 2 + py * spread)
    ctx:attack_move({u.id}, tx, ty)
  end
end

-- Back line: ranged and fragile 3 tiles behind, seek cover and elevation
if #back > 0 then
  local half = (#back - 1) / 2
  for i, u in ipairs(back) do
    local spread = (i - 1 - half) * 2
    local bx = math.floor(cx - nx * 3 + px * spread)
    local by = math.floor(cy - ny * 3 + py * spread)

    -- Search 3x3 area for best defensive position (cover + elevation)
    local best_x, best_y = bx, by
    local best_score = -1
    for ox = -1, 1 do
      for oy = -1, 1 do
        local tx, ty = bx + ox, by + oy
        if ctx:is_passable(tx, ty) then
          local cover = ctx:cover_at(tx, ty)
          local elev = ctx:elevation_at(tx, ty) or 0
          local score = elev
          if cover == "Heavy" then score = score + 4
          elseif cover == "Light" then score = score + 2 end
          if score > best_score then
            best_score = score
            best_x, best_y = tx, ty
          end
        end
      end
    end

    ctx:move_units({u.id}, best_x, best_y)
  end
end
