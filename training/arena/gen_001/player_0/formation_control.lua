-- @name: formation_control
-- @events: on_tick
-- @interval: 8

-- Formation Control: Split army into melee frontline and ranged backline.
-- Position units on favorable terrain when possible.
-- The FSM blob-rushes everything together. Proper formation means
-- Chonks absorb damage while Hissers deal it from safety.
-- Runs every 8 ticks (0.8s) -- formation doesn't need to be super responsive.

local function collect_ids(units)
    local ids = {}
    for _, u in ipairs(units) do
        ids[#ids + 1] = u.id
    end
    return ids
end

local enemies = ctx:enemy_units()

-- Only manage formations when enemies are visible and combat is happening
if #enemies == 0 then
    return
end

-- Find the nearest enemy cluster center
local ex, ey = 0, 0
for _, e in ipairs(enemies) do
    ex = ex + e.x
    ey = ey + e.y
end
ex = ex / #enemies
ey = ey / #enemies

-- Get our army and split it
local my_units = ctx:my_units()
local melee = {}
local ranged = {}

for _, u in ipairs(my_units) do
    if u.kind ~= "Pawdler" and not u.gathering then
        if u.attack_type == "Ranged" then
            ranged[#ranged + 1] = u
        elseif u.attack_type == "Melee" then
            melee[#melee + 1] = u
        end
    end
end

-- Need both roles for formation to matter
if #melee == 0 or #ranged == 0 then
    return
end

-- Calculate our army center
local ax, ay = 0, 0
local army_size = #melee + #ranged
for _, u in ipairs(melee) do
    ax = ax + u.x
    ay = ay + u.y
end
for _, u in ipairs(ranged) do
    ax = ax + u.x
    ay = ay + u.y
end
ax = ax / army_size
ay = ay / army_size

-- Distance to enemy cluster
local dx = ex - ax
local dy = ey - ay
local dist = math.sqrt(dx * dx + dy * dy)

-- Only manage formation when approaching (dist 5-20)
-- Too close = already fighting, too far = FSM handles movement
if dist < 5 or dist > 20 then
    return
end

-- Normalize direction toward enemy
local nx = dx / dist
local ny = dy / dist

-- === MELEE FRONTLINE ===
-- Push melee units 2 tiles ahead of army center, toward enemy
local front_x = math.floor(ax + nx * 3)
local front_y = math.floor(ay + ny * 3)

-- Clamp to map
local w, h = ctx:map_size()
front_x = math.max(1, math.min(w - 2, front_x))
front_y = math.max(1, math.min(h - 2, front_y))

-- Attack-move melee forward (they'll engage what they find)
local melee_ids = collect_ids(melee)
ctx:attack_move(melee_ids, front_x, front_y)

-- === RANGED BACKLINE ===
-- Keep ranged units 3 tiles behind army center, away from enemy
local back_x = math.floor(ax - nx * 2)
local back_y = math.floor(ay - ny * 2)
back_x = math.max(1, math.min(w - 2, back_x))
back_y = math.max(1, math.min(h - 2, back_y))

-- Check for cover at the backline position and nearby tiles
-- Prefer positions with cover for ranged units
local best_x, best_y = back_x, back_y
local best_cover = 0

for ox = -2, 2 do
    for oy = -2, 2 do
        local tx = back_x + ox
        local ty = back_y + oy
        if tx >= 1 and tx < w - 1 and ty >= 1 and ty < h - 1 then
            if ctx:is_passable(tx, ty) then
                local cover = ctx:cover_at(tx, ty)
                local cover_val = 0
                if cover == "Light" then cover_val = 1 end
                if cover == "Heavy" then cover_val = 2 end

                -- Also check elevation advantage
                local elev = ctx:elevation_at(tx, ty)
                local enemy_elev = ctx:elevation_at(math.floor(ex), math.floor(ey))
                if elev > enemy_elev then
                    cover_val = cover_val + 1
                end

                if cover_val > best_cover then
                    best_cover = cover_val
                    best_x = tx
                    best_y = ty
                end
            end
        end
    end
end

-- Move ranged to best backline position, using hold so they attack
-- without chasing into melee range
local ranged_ids = collect_ids(ranged)
ctx:move_units(ranged_ids, best_x, best_y)

-- === CHONK VANGUARD ===
-- If we have Chonks, push them even further forward as meat shields
-- They have 300 HP and can soak a lot of damage
local chonks = ctx:my_units("Chonk")
if #chonks > 0 then
    local vanguard_x = math.floor(ax + nx * 5)
    local vanguard_y = math.floor(ay + ny * 5)
    vanguard_x = math.max(1, math.min(w - 2, vanguard_x))
    vanguard_y = math.max(1, math.min(h - 2, vanguard_y))

    local chonk_ids = collect_ids(chonks)
    ctx:attack_move(chonk_ids, vanguard_x, vanguard_y)
end
