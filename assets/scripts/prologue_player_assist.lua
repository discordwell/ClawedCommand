-- @name: prologue_player_assist
-- @events: on_tick
-- @interval: 5

-- Gentle player assist for the prologue tutorial.
-- Only acts on idle units — never overrides player commands.
-- Auto-retreats Kelpie when HP is low.

local my_units = ctx:my_units()
if not my_units then return end
if #my_units == 0 then return end

local enemies = ctx:enemy_units()
if not enemies then return end

-- Find Kelpie and check HP
for _, u in ipairs(my_units) do
    local hp = ctx:hp_pct(u.id)
    if hp and hp < 0.4 then
        -- Retreat Kelpie toward safe west bank position
        ctx:move_units({u.id}, 4, 24)
        return
    end
end

-- Get idle units
local idle = ctx:idle_units()
if not idle then return end
if #idle == 0 then return end

-- No enemies left? Nothing to do.
if #enemies == 0 then return end

-- Find nearest enemy to idle group
local idle_ids = {}
for _, u in ipairs(idle) do
    table.insert(idle_ids, u.id)
end

-- Compute centroid of enemies
local cx, cy = 0, 0
for _, e in ipairs(enemies) do
    cx = cx + e.x
    cy = cy + e.y
end
cx = cx / #enemies
cy = cy / #enemies

-- Attack-move idle units toward nearest enemy cluster
ctx:attack_move(idle_ids, math.floor(cx), math.floor(cy))
