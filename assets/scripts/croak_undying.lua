-- @name: croak_undying
-- @events: on_tick
-- @interval: 3

-- Croak: regeneration + attrition doctrine.
-- Shellwardens Hunker as immovable frontline, Broodmothers heal,
-- Croakers Inflate for bombardment, Leapfrogs Hop for harassment,
-- MurkCommander GrokProtocol for army-wide buff.
-- Nearly all abilities are free — outlast everything.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()
local ability_used = {}

-- === CLASSIFY UNITS ===
local regenerons = {}
local broodmothers = {}
local gulpers = {}
local eftsabers = {}
local croakers = {}
local leapfrogs = {}
local shellwardens = {}
local bogwhispers = {}
local murk_commander = {}
local all_combat = {}
local all_combat_ids = {}

local WORKER_KINDS = { Ponderer = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if u.kind == "Regeneron" then table.insert(regenerons, u)
        elseif u.kind == "Broodmother" then table.insert(broodmothers, u)
        elseif u.kind == "Gulper" then table.insert(gulpers, u)
        elseif u.kind == "Eftsaber" then table.insert(eftsabers, u)
        elseif u.kind == "Croaker" then table.insert(croakers, u)
        elseif u.kind == "Leapfrog" then table.insert(leapfrogs, u)
        elseif u.kind == "Shellwarden" then table.insert(shellwardens, u)
        elseif u.kind == "Bogwhisper" then table.insert(bogwhispers, u)
        elseif u.kind == "MurkCommander" then table.insert(murk_commander, u)
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

-- Helper: most wounded ally
local function most_wounded_ally(u, range)
    local sq = range * range
    local best, best_pct = nil, 1.0
    for _, a in ipairs(all_combat) do
        if a.id ~= u.id then
            local hp_pct = a.hp / math.max(a.hp_max, 1)
            if hp_pct < best_pct and hp_pct < 0.70 then
                local dx = a.x - u.x
                local dy = a.y - u.y
                if dx * dx + dy * dy <= sq then
                    best_pct = hp_pct
                    best = a
                end
            end
        end
    end
    return best
end

-- =====================================================================
-- ABILITY LAYER
-- =====================================================================

-- Shellwarden: Hunker (slot 0, toggle, free) — immovable defense
for _, u in ipairs(shellwardens) do
    if not strong_advantage and tick > 3 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- Shellwarden: TidalMemory (slot 2, 6 GPU, ArmorBuff, 200 dur) — massive buff
for _, u in ipairs(shellwardens) do
    if not ability_used[u.id] and gpu >= 6 and enemy_count > 0 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
        gpu = gpu - 6
    end
end

-- Broodmother: Transfusion (slot 1, free, range 3) — heal wounded allies
for _, u in ipairs(broodmothers) do
    local wounded = most_wounded_ally(u, 3)
    if wounded then
        ctx:ability(u.id, 1, "entity", nil, nil, wounded.id)
        ability_used[u.id] = true
    end
end

-- Broodmother: PrimordialSoup (slot 2, free, Armor+Dmg, 120 dur) — big self-buff
for _, u in ipairs(broodmothers) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.50 or enemy_count >= 4 then
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
        end
    end
end

-- Croaker: Inflate (slot 2, free, InflatedBombardment, 30 dur) — artillery mode
for _, u in ipairs(croakers) do
    if enemy_count > 0 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
    end
end

-- Regeneron: LimbToss (slot 0, free, range 5, 30 CD!) — spam ranged harass
for _, u in ipairs(regenerons) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 25 then
        ctx:ability(u.id, 0, "entity", nil, nil, ce.id)
        ability_used[u.id] = true
    end
end

-- Regeneron: RegrowthBurst (slot 1, free, ArmorBuff) when wounded
for _, u in ipairs(regenerons) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.50 then
            ctx:ability(u.id, 1, "self")
            ability_used[u.id] = true
        end
    end
end

-- Eftsaber: Waterway (slot 1, free, SpeedBuff, 50 CD) — mobility burst
for _, u in ipairs(eftsabers) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 25 and cd > 4 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Eftsaber: Venomstrike (slot 2, free, range 3) — poison attack
for _, u in ipairs(eftsabers) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 9 then
            ctx:ability(u.id, 2, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
        end
    end
end

-- Leapfrog: Hop (slot 0, free, SpeedBuff, range 4) — gap closer
for _, u in ipairs(leapfrogs) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 25 and cd > 4 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- Leapfrog: TongueLash (slot 1, free, range 5) — ranged harass
for _, u in ipairs(leapfrogs) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 25 then
            ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
        end
    end
end

-- Gulper: Devour (slot 0, free, range 1) — eat an enemy
for _, u in ipairs(gulpers) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 1 then
        ctx:ability(u.id, 0, "entity", nil, nil, ce.id)
        ability_used[u.id] = true
    end
end

-- Gulper: Regurgitate (slot 1, free, range 4) — ranged spit
for _, u in ipairs(gulpers) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 16 then
            ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
        end
    end
end

-- Bogwhisper: MireCurse (slot 0, free, range 6) — debuff area
for _, u in ipairs(bogwhispers) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 36 then
        ctx:ability(u.id, 0, "position", ce.x, ce.y)
        ability_used[u.id] = true
    end
end

-- Bogwhisper: Prophecy (slot 1, 4 GPU, range 8) — reveal/debuff
for _, u in ipairs(bogwhispers) do
    if not ability_used[u.id] and gpu >= 4 and enemy_count >= 3 then
        ctx:ability(u.id, 1, "position", math.floor(enemy_cx), math.floor(enemy_cy))
        ability_used[u.id] = true
        gpu = gpu - 4
    end
end

-- MurkCommander: GrokProtocol (slot 1, 8 GPU, Dmg+Speed, 120 dur) — army buff
for _, u in ipairs(murk_commander) do
    if gpu >= 8 and enemy_count > 0 then
        local nearby = 0
        for _, a in ipairs(all_combat) do
            local dx = a.x - u.x
            local dy = a.y - u.y
            if dx * dx + dy * dy <= 64 then nearby = nearby + 1 end
        end
        if nearby >= 3 then
            ctx:ability(u.id, 1, "self")
            ability_used[u.id] = true
            gpu = gpu - 8
        end
    end
end

-- =====================================================================
-- COMBAT LAYER
-- =====================================================================

-- Rally point
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

-- Croak doesn't retreat easily — regeneration means wounded units stay
-- Only retreat when critically low AND outnumbered
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.15 and u.attacking and outnumbered then
            table.insert(retreat_ids, u.id)
        end
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- Formation: Shellwardens + Gulpers forward, ranged behind
if enemy_count > 0 then
    local tank_ids = {}
    for _, u in ipairs(shellwardens) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(tank_ids, u.id)
        end
    end
    for _, u in ipairs(gulpers) do
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

    -- Croakers + Bogwhispers stay behind
    local ranged_ids = {}
    for _, u in ipairs(croakers) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    for _, u in ipairs(bogwhispers) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    if #ranged_ids > 0 then
        local rx = math.floor(army_cx - dir_x * 2)
        local ry = math.floor(army_cy - dir_y * 2)
        rx = math.max(0, math.min(map_w - 1, rx))
        ry = math.max(0, math.min(map_h - 1, ry))
        ctx:attack_move(ranged_ids, rx, ry)
    end

    -- Broodmothers stay mid-army (healing range)
    local healer_ids = {}
    for _, u in ipairs(broodmothers) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(healer_ids, u.id)
        end
    end
    if #healer_ids > 0 then
        ctx:attack_move(healer_ids, math.floor(army_cx), math.floor(army_cy))
    end
end

-- Focus fire
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

-- Push — Croak pushes slowly but relentlessly
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
