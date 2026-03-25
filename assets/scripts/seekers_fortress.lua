-- @name: seekers_fortress
-- @events: on_tick
-- @interval: 3

-- Seekers of the Deep: defensive fortress doctrine.
-- Entrench Cragbacks, ShieldWall Ironhides, Wardenmother buffs army.
-- Almost all abilities cost 0 GPU — spam freely.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()
local ability_used = {}

-- === CLASSIFY UNITS ===
local ironhides = {}
local cragbacks = {}
local wardens = {}
local sapjaws = {}
local gutrippers = {}
local embermaws = {}
local dustclaws = {}
local wardenmother = {}
local tunnelers = {}
local all_combat = {}
local all_combat_ids = {}

local WORKER_KINDS = { Delver = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if u.kind == "Ironhide" then table.insert(ironhides, u)
        elseif u.kind == "Cragback" then table.insert(cragbacks, u)
        elseif u.kind == "Warden" then table.insert(wardens, u)
        elseif u.kind == "Sapjaw" then table.insert(sapjaws, u)
        elseif u.kind == "Gutripper" then table.insert(gutrippers, u)
        elseif u.kind == "Embermaw" then table.insert(embermaws, u)
        elseif u.kind == "Dustclaw" then table.insert(dustclaws, u)
        elseif u.kind == "Wardenmother" then table.insert(wardenmother, u)
        elseif u.kind == "SeekerTunneler" then table.insert(tunnelers, u)
        end
    end
end

local my_count = #all_combat
if my_count == 0 then return end

-- === CENTROIDS ===
local army_cx, army_cy = 0, 0
for _, u in ipairs(all_combat) do
    army_cx = army_cx + u.x
    army_cy = army_cy + u.y
end
army_cx = army_cx / my_count
army_cy = army_cy / my_count

local enemies = ctx:enemy_units()
local enemy_count = enemies and #enemies or 0
local outnumbered = my_count < enemy_count
local strong_advantage = my_count >= enemy_count + 3

local enemy_cx, enemy_cy = army_cx, army_cy
if enemy_count > 0 then
    enemy_cx, enemy_cy = 0, 0
    for _, e in ipairs(enemies) do
        enemy_cx = enemy_cx + e.x
        enemy_cy = enemy_cy + e.y
    end
    enemy_cx = enemy_cx / enemy_count
    enemy_cy = enemy_cy / enemy_count
end

local dir_x = enemy_cx - army_cx
local dir_y = enemy_cy - army_cy
local dir_len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
if dir_len > 0.01 then
    dir_x = dir_x / dir_len
    dir_y = dir_y / dir_len
end

local resources = ctx:get_resources()
local gpu = resources.gpu_cores

-- Helper: closest enemy
local function closest_enemy(u)
    if enemy_count == 0 then return nil, 999999 end
    local best, best_d = nil, 999999
    for _, e in ipairs(enemies) do
        local dx = e.x - u.x
        local dy = e.y - u.y
        local d = dx * dx + dy * dy
        if d < best_d then best_d = d; best = e end
    end
    return best, best_d
end

-- =====================================================================
-- ABILITY LAYER
-- =====================================================================

-- Ironhide: ShieldWall (slot 1, free, self, ArmorBuff) when enemies close
for _, u in ipairs(ironhides) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 64 then -- within 8 tiles
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Ironhide: GrudgeCharge (slot 2, free, self, Speed+Dmg) for counter-attack
for _, u in ipairs(ironhides) do
    if not ability_used[u.id] and strong_advantage then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 64 and cd > 9 then -- charge range: 3-8 tiles
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
        end
    end
end

-- Cragback: Entrench (slot 1, toggle, free) — activate when not pushing
for _, u in ipairs(cragbacks) do
    if not strong_advantage and tick > 3 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Cragback: SeismicSlam (slot 2, free, range 3) on enemy clusters
for _, u in ipairs(cragbacks) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 9 then
            ctx:ability(u.id, 2, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- Warden: RallyCry (slot 2, free, range 6) when army clustered
for _, u in ipairs(wardens) do
    local nearby = 0
    for _, a in ipairs(all_combat) do
        local dx = a.x - u.x
        local dy = a.y - u.y
        if dx * dx + dy * dy <= 36 then nearby = nearby + 1 end
    end
    if nearby >= 3 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
    end
end

-- Warden: Intercept (slot 1, 4 GPU, range 6) to protect wounded allies
for _, u in ipairs(wardens) do
    if not ability_used[u.id] and gpu >= 4 then
        for _, a in ipairs(all_combat) do
            if a.id ~= u.id then
                local hp_pct = a.hp / math.max(a.hp_max, 1)
                if hp_pct < 0.40 then
                    local dx = a.x - u.x
                    local dy = a.y - u.y
                    if dx * dx + dy * dy <= 36 then
                        ctx:ability(u.id, 1, "entity", nil, nil, a.id)
                        ability_used[u.id] = true
                        gpu = gpu - 4
                        break
                    end
                end
            end
        end
    end
end

-- Sapjaw: Lockjaw (slot 2, free, range 1) on adjacent enemies
for _, u in ipairs(sapjaws) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 1 then
        ctx:ability(u.id, 2, "entity", nil, nil, ce.id)
        ability_used[u.id] = true
    end
end

-- Gutripper: RecklessLunge (slot 2, free, self, Speed+Dmg) when engaging
for _, u in ipairs(gutrippers) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 16 and cd > 1 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
    end
end

-- Embermaw: ScorchedEarth (slot 2, free, range 4) on enemy positions
for _, u in ipairs(embermaws) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 16 then
        ctx:ability(u.id, 2, "position", ce.x, ce.y)
        ability_used[u.id] = true
    end
end

-- Dustclaw: DustCloud (slot 0, free, range 3) for concealment near enemies
for _, u in ipairs(dustclaws) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 16 then
        ctx:ability(u.id, 0, "position", u.x, u.y)
        ability_used[u.id] = true
    end
end

-- Wardenmother: FortressProtocol (slot 1, 10 GPU, range 6) for army armor
for _, u in ipairs(wardenmother) do
    if gpu >= 10 and enemy_count > 0 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
        gpu = gpu - 10
    end
end

-- Wardenmother: CalculatedCounterstrike (slot 2, 6 GPU, range 4) for damage push
for _, u in ipairs(wardenmother) do
    if not ability_used[u.id] and gpu >= 6 and strong_advantage then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
        gpu = gpu - 6
    end
end

-- =====================================================================
-- COMBAT LAYER
-- =====================================================================

-- Rally point near base
local my_buildings = ctx:my_buildings()
local rally_x, rally_y = army_cx, army_cy
if my_buildings and #my_buildings > 0 then
    local best_dist = 999999
    for _, b in ipairs(my_buildings) do
        local dx = b.x - army_cx
        local dy = b.y - army_cy
        local d = dx * dx + dy * dy
        if d < best_dist then best_dist = d; rally_x = b.x; rally_y = b.y end
    end
end

-- Retreat wounded
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.25 and u.attacking and outnumbered then
            table.insert(retreat_ids, u.id)
        end
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- Formation: Ironhides forward, ranged behind
if enemy_count > 0 then
    local tank_ids = {}
    for _, u in ipairs(ironhides) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(tank_ids, u.id)
        end
    end
    if #tank_ids > 0 then
        local tx = math.floor(army_cx + dir_x * 4)
        local ty = math.floor(army_cy + dir_y * 4)
        tx = math.max(0, math.min(map_w - 1, tx))
        ty = math.max(0, math.min(map_h - 1, ty))
        ctx:attack_move(tank_ids, tx, ty)
    end

    -- Cragbacks + Embermaws stay behind tanks (ranged line)
    local ranged_ids = {}
    for _, u in ipairs(cragbacks) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    for _, u in ipairs(embermaws) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    if #ranged_ids > 0 then
        local rx = math.floor(army_cx - dir_x * 1)
        local ry = math.floor(army_cy - dir_y * 1)
        rx = math.max(0, math.min(map_w - 1, rx))
        ry = math.max(0, math.min(map_h - 1, ry))
        ctx:attack_move(ranged_ids, rx, ry)
    end
end

-- Focus fire (closest to army centroid)
local attackers = {}
for _, u in ipairs(all_combat) do
    if u.attacking and not ability_used[u.id] then
        table.insert(attackers, u)
    end
end

if #attackers >= 2 then
    local cx, cy = 0, 0
    for _, u in ipairs(attackers) do cx = cx + u.x; cy = cy + u.y end
    cx = cx / #attackers
    cy = cy / #attackers
    local focus = ctx:weakest_enemy_in_range(cx, cy, 12)
    if focus then
        local ids = {}
        for _, u in ipairs(attackers) do table.insert(ids, u.id) end
        ctx:attack_units(ids, focus.id)
    end
end

-- Push when clear or strong advantage
local should_push = (enemy_count == 0 and my_count >= 2) or strong_advantage
if should_push then
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings and #enemy_buildings > 0 then
        local HQ_KINDS = {
            TheBox = true, TheDumpster = true, TheGrotto = true,
            TheBurrow = true, TheNest = true, TheMound = true,
        }
        local hq, nearest = nil, nil
        local nearest_dist = 999999
        for _, b in ipairs(enemy_buildings) do
            if HQ_KINDS[b.kind] then hq = b end
            local dx = b.x - army_cx
            local dy = b.y - army_cy
            local d = dx * dx + dy * dy
            if d < nearest_dist then nearest_dist = d; nearest = b end
        end
        local push_ids = {}
        for _, id in ipairs(all_combat_ids) do
            if not ability_used[id] then table.insert(push_ids, id) end
        end
        local target = hq or nearest
        if target and #push_ids > 0 then
            ctx:attack_move(push_ids, target.x, target.y)
        end
    end
end
