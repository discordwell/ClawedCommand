-- @name: catgpt_abilities
-- @events: on_tick
-- @interval: 3

-- catGPT: balanced doctrine with expensive but powerful abilities.
-- Chonk LoafMode tanks, Yowler HarmonicResonance buffs, Mouser stealth,
-- Nuisance Zoomies for hit-and-run, Hisser DisgustMortar for area damage.
-- High GPU costs — be selective with ability usage.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()
local ability_used = {}

-- === CLASSIFY UNITS ===
local nuisances = {}
local chonks = {}
local flying_foxes = {}
local hissers = {}
local yowlers = {}
local mousers = {}
local catnappers = {}
local ferret_sappers = {}
local mech_commanders = {}
local all_combat = {}
local all_combat_ids = {}

local WORKER_KINDS = { Pawdler = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if u.kind == "Nuisance" then table.insert(nuisances, u)
        elseif u.kind == "Chonk" then table.insert(chonks, u)
        elseif u.kind == "FlyingFox" then table.insert(flying_foxes, u)
        elseif u.kind == "Hisser" then table.insert(hissers, u)
        elseif u.kind == "Yowler" then table.insert(yowlers, u)
        elseif u.kind == "Mouser" then table.insert(mousers, u)
        elseif u.kind == "Catnapper" then table.insert(catnappers, u)
        elseif u.kind == "FerretSapper" then table.insert(ferret_sappers, u)
        elseif u.kind == "MechCommander" then table.insert(mech_commanders, u)
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

-- Helper: count enemies near position
local function enemies_near(px, py, range)
    local count = 0
    local sq = range * range
    if enemies then
        for _, e in ipairs(enemies) do
            local dx = e.x - px
            local dy = e.y - py
            if dx * dx + dy * dy <= sq then count = count + 1 end
        end
    end
    return count
end

-- =====================================================================
-- ABILITY LAYER
-- =====================================================================

-- Chonk: LoafMode (slot 1, toggle, free) — activate as frontline anchor
for _, u in ipairs(chonks) do
    if not strong_advantage and tick > 3 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Yowler: HarmonicResonance (slot 0, toggle, free, range 4) — ally buff aura
for _, u in ipairs(yowlers) do
    if tick < 4 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- Yowler: DissonantScreech (slot 1, 10 GPU, range 4) — area debuff
for _, u in ipairs(yowlers) do
    if not ability_used[u.id] and gpu >= 10 then
        local near = enemies_near(u.x, u.y, 4)
        if near >= 2 then
            local ce, _ = closest_enemy(u)
            if ce then
                ctx:ability(u.id, 1, "position", ce.x, ce.y)
                ability_used[u.id] = true
                gpu = gpu - 10
            end
        end
    end
end

-- Nuisance: Zoomies (slot 2, 10 GPU, self, SpeedBuff) — burst speed
for _, u in ipairs(nuisances) do
    if gpu >= 10 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 36 and cd > 4 then
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
            gpu = gpu - 10
        end
    end
end

-- Nuisance: Hairball (slot 1, 5 GPU, range 4) — ranged harassment
for _, u in ipairs(nuisances) do
    if not ability_used[u.id] and gpu >= 5 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 16 then
            ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
            gpu = gpu - 5
        end
    end
end

-- Hisser: DisgustMortar (slot 1, 10 GPU, range 6) — area damage on clusters
for _, u in ipairs(hissers) do
    if gpu >= 10 then
        local near = enemies_near(u.x, u.y, 6)
        if near >= 2 then
            ctx:ability(u.id, 1, "position", math.floor(enemy_cx), math.floor(enemy_cy))
            ability_used[u.id] = true
            gpu = gpu - 10
        end
    end
end

-- Hisser: Misinformation (slot 2, 20 GPU, range 5) — high-value debuff
for _, u in ipairs(hissers) do
    if not ability_used[u.id] and gpu >= 20 and enemy_count >= 4 then
        ctx:ability(u.id, 2, "position", math.floor(enemy_cx), math.floor(enemy_cy))
        ability_used[u.id] = true
        gpu = gpu - 20
    end
end

-- FlyingFox: EcholocationPulse (slot 0, 10 GPU, range 6) — reveal hidden enemies
for _, u in ipairs(flying_foxes) do
    if gpu >= 10 and enemy_count > 0 then
        ctx:ability(u.id, 0, "position", math.floor(enemy_cx), math.floor(enemy_cy))
        ability_used[u.id] = true
        gpu = gpu - 10
    end
end

-- Mouser: Tagged (slot 0, 5 GPU, range 5) — mark high-value target
for _, u in ipairs(mousers) do
    if gpu >= 5 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 25 then
            ctx:ability(u.id, 0, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
            gpu = gpu - 5
        end
    end
end

-- Mouser: ShadowNetwork (slot 2, 25 GPU, self) — army-wide stealth
for _, u in ipairs(mousers) do
    if not ability_used[u.id] and gpu >= 25 and outnumbered then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
        gpu = gpu - 25
    end
end

-- Catnapper: SiegeNap (slot 2, toggle, free) — deploy for long-range siege
for _, u in ipairs(catnappers) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 100 then -- siege range
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
    end
end

-- Catnapper: ContagiousYawning (slot 1, 15 GPU, range 3) — area sleep
for _, u in ipairs(catnappers) do
    if not ability_used[u.id] and gpu >= 15 then
        local near = enemies_near(u.x, u.y, 3)
        if near >= 2 then
            local ce, _ = closest_enemy(u)
            if ce then
                ctx:ability(u.id, 1, "position", ce.x, ce.y)
                ability_used[u.id] = true
                gpu = gpu - 15
            end
        end
    end
end

-- FerretSapper: ShapedCharge (slot 0, 10 GPU, range 1) on buildings
for _, u in ipairs(ferret_sappers) do
    if gpu >= 10 then
        local enemy_buildings = ctx:enemy_buildings()
        if enemy_buildings then
            for _, b in ipairs(enemy_buildings) do
                local dx = b.x - u.x
                local dy = b.y - u.y
                if dx * dx + dy * dy <= 4 then
                    ctx:ability(u.id, 0, "entity", nil, nil, b.id)
                    ability_used[u.id] = true
                    gpu = gpu - 10
                    break
                end
            end
        end
    end
end

-- FerretSapper: BoobyTrap (slot 1, 10 GPU, 3 charges) — defensive mines
for _, u in ipairs(ferret_sappers) do
    if not ability_used[u.id] and gpu >= 10 and tick < 30 then
        -- Place traps near our buildings early
        ctx:ability(u.id, 1, "position", u.x, u.y)
        ability_used[u.id] = true
        gpu = gpu - 10
    end
end

-- MechCommander: TacticalUplink (slot 0, toggle, free, range 5) — activate early
for _, u in ipairs(mech_commanders) do
    if tick < 4 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- MechCommander: Override (slot 1, 30 GPU, range 6) — take control of enemy
for _, u in ipairs(mech_commanders) do
    if not ability_used[u.id] and gpu >= 30 and enemy_count >= 3 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 36 then
            ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
            gpu = gpu - 30
        end
    end
end

-- MechCommander: LeChatUplink (slot 2, 50 GPU) — ultimate faction buff
for _, u in ipairs(mech_commanders) do
    if not ability_used[u.id] and gpu >= 50 and enemy_count >= 5 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
        gpu = gpu - 50
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

-- Formation: Chonks forward, ranged behind
if enemy_count > 0 then
    local tank_ids = {}
    for _, u in ipairs(chonks) do
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

    local ranged_ids = {}
    for _, u in ipairs(hissers) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    for _, u in ipairs(yowlers) do
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

-- Kite ranged when outnumbered
if outnumbered and enemies then
    local ranged_to_kite = {}
    for _, u in ipairs(hissers) do table.insert(ranged_to_kite, u) end
    for _, u in ipairs(yowlers) do table.insert(ranged_to_kite, u) end
    for _, r in ipairs(ranged_to_kite) do
        if r.attacking and not ability_used[r.id] then
            local ce, cd = closest_enemy(r)
            if ce and cd < 9 then
                local flee_x = r.x - (ce.x - r.x)
                local flee_y = r.y - (ce.y - r.y)
                flee_x = math.max(0, math.min(map_w - 1, flee_x))
                flee_y = math.max(0, math.min(map_h - 1, flee_y))
                ctx:move_units({r.id}, flee_x, flee_y)
            end
        end
    end
end

-- Push
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
