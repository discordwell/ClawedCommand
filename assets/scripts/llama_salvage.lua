-- @name: llama_salvage
-- @events: on_tick
-- @interval: 3

-- LLAMA: scavenger chaos doctrine.
-- PatchPossums heal, DumpsterDivers shield, GreaseMonkeys deploy turrets,
-- HeapTitans smash, JunkyardKing buffs the whole army.
-- Most abilities are free — scrap-powered economy.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()
local ability_used = {}

-- === CLASSIFY UNITS ===
local bandits = {}
local heap_titans = {}
local glitch_rats = {}
local patch_possums = {}
local grease_monkeys = {}
local dead_drops = {}
local wreckers = {}
local dumpster_divers = {}
local junkyard_king = {}
local all_combat = {}
local all_combat_ids = {}

local WORKER_KINDS = { Scrounger = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if u.kind == "Bandit" then table.insert(bandits, u)
        elseif u.kind == "HeapTitan" then table.insert(heap_titans, u)
        elseif u.kind == "GlitchRat" then table.insert(glitch_rats, u)
        elseif u.kind == "PatchPossum" then table.insert(patch_possums, u)
        elseif u.kind == "GreaseMonkey" then table.insert(grease_monkeys, u)
        elseif u.kind == "DeadDropUnit" then table.insert(dead_drops, u)
        elseif u.kind == "Wrecker" then table.insert(wreckers, u)
        elseif u.kind == "DumpsterDiver" then table.insert(dumpster_divers, u)
        elseif u.kind == "JunkyardKing" then table.insert(junkyard_king, u)
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

-- Helper: find most wounded ally within range
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

-- PatchPossum: DuctTapeFix (slot 0, free, range 4, ArmorBuff) — heal wounded
for _, u in ipairs(patch_possums) do
    local wounded = most_wounded_ally(u, 4)
    if wounded then
        ctx:ability(u.id, 0, "entity", nil, nil, wounded.id)
        ability_used[u.id] = true
    end
end

-- PatchPossum: SalvageResurrection (slot 1, free, range 1) on dead allies nearby
for _, u in ipairs(patch_possums) do
    if not ability_used[u.id] then
        -- Self-cast to attempt resurrection of nearest corpse
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- DumpsterDiver: RefuseShield (slot 1, free, range 3, ArmorBuff) — area shield
for _, u in ipairs(dumpster_divers) do
    local nearby_allies = 0
    for _, a in ipairs(all_combat) do
        local dx = a.x - u.x
        local dy = a.y - u.y
        if dx * dx + dy * dy <= 9 then nearby_allies = nearby_allies + 1 end
    end
    if nearby_allies >= 3 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- DumpsterDiver: StenchCloud (slot 2, free, range 3) — area denial
for _, u in ipairs(dumpster_divers) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 9 then
            ctx:ability(u.id, 2, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- GreaseMonkey: JunkMortarMode (slot 2, toggle, free) — activate for siege
for _, u in ipairs(grease_monkeys) do
    if tick < 4 then
        ctx:ability(u.id, 2, "self")
        ability_used[u.id] = true
    end
end

-- GreaseMonkey: SalvageTurret (slot 1, free, range 2, 1 charge) — deploy early
for _, u in ipairs(grease_monkeys) do
    if not ability_used[u.id] and tick > 10 and tick < 20 then
        ctx:ability(u.id, 1, "position", u.x, u.y)
        ability_used[u.id] = true
    end
end

-- HeapTitan: WreckBall (slot 1, free, range 5) — ranged smash
for _, u in ipairs(heap_titans) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 25 then
        ctx:ability(u.id, 1, "entity", nil, nil, ce.id)
        ability_used[u.id] = true
    end
end

-- HeapTitan: MagneticPulse (slot 2, free, range 3) — area pull
for _, u in ipairs(heap_titans) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 9 then
            ctx:ability(u.id, 2, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- Bandit: JuryRig (slot 1, free, range 1, ArmorBuff) — self-repair in combat
for _, u in ipairs(bandits) do
    local hp_pct = u.hp / math.max(u.hp_max, 1)
    if hp_pct < 0.60 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Bandit: Getaway (slot 2, free, SpeedBuff) — escape when low
for _, u in ipairs(bandits) do
    if not ability_used[u.id] then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.25 and u.attacking then
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
        end
    end
end

-- GlitchRat: SignalScramble (slot 1, 4 GPU, range 6) — disrupt enemy AI
for _, u in ipairs(glitch_rats) do
    if gpu >= 4 and enemy_count >= 3 then
        ctx:ability(u.id, 1, "position", math.floor(enemy_cx), math.floor(enemy_cy))
        ability_used[u.id] = true
        gpu = gpu - 4
    end
end

-- DeadDropUnit: TrashHeapAmbush (slot 1, free, Dmg+Speed) — ambush buff
for _, u in ipairs(dead_drops) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 25 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- DeadDropUnit: LeakInjection (slot 2, 5 GPU, DamageBuff) — sabotage
for _, u in ipairs(dead_drops) do
    if not ability_used[u.id] and gpu >= 5 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 16 then
            ctx:ability(u.id, 2, "self")
            ability_used[u.id] = true
            gpu = gpu - 5
        end
    end
end

-- Wrecker: PryBar (slot 1, free, range 1) on buildings
for _, u in ipairs(wreckers) do
    local enemy_buildings = ctx:enemy_buildings()
    if enemy_buildings then
        for _, b in ipairs(enemy_buildings) do
            local dx = b.x - u.x
            local dy = b.y - u.y
            if dx * dx + dy * dy <= 4 then
                ctx:ability(u.id, 1, "entity", nil, nil, b.id)
                ability_used[u.id] = true
                break
            end
        end
    end
end

-- Wrecker: ChainBreak (slot 2, free, range 3) on groups
for _, u in ipairs(wreckers) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 9 then
            ctx:ability(u.id, 2, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- JunkyardKing: OverclockCascade (slot 2, free, range 6, Dmg+Speed) — army buff
for _, u in ipairs(junkyard_king) do
    if enemy_count > 0 then
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
end

-- JunkyardKing: FrankensteinProtocol (slot 1, 10 GPU, range 3) — field upgrade
for _, u in ipairs(junkyard_king) do
    if not ability_used[u.id] and gpu >= 10 then
        local wounded = most_wounded_ally(u, 3)
        if wounded then
            ctx:ability(u.id, 1, "entity", nil, nil, wounded.id)
            ability_used[u.id] = true
            gpu = gpu - 10
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

-- Retreat wounded (not Bandits — they have Getaway)
local retreat_ids = {}
for _, u in ipairs(all_combat) do
    if not ability_used[u.id] and u.kind ~= "Bandit" then
        local hp_pct = u.hp / math.max(u.hp_max, 1)
        if hp_pct < 0.20 and u.attacking and outnumbered then
            table.insert(retreat_ids, u.id)
        end
    end
end
if #retreat_ids > 0 then
    ctx:move_units(retreat_ids, math.floor(rally_x), math.floor(rally_y))
end

-- Formation: HeapTitans forward, ranged behind
if enemy_count > 0 then
    local tank_ids = {}
    for _, u in ipairs(heap_titans) do
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
    for _, u in ipairs(grease_monkeys) do
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
