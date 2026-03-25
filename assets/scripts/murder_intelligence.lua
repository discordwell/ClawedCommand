-- @name: murder_intelligence
-- @events: on_tick
-- @interval: 3

-- The Murder: intel/espionage + aerial strike doctrine.
-- Sentinels on Overwatch, Rookclaws TalonDive flankers, Jaycaller rallies,
-- Hootseer debuffs, Dusktalon assassinates low-HP targets.
-- CRITICAL: Never issue ability AND attack commands to same unit in same tick.

local my_units = ctx:my_units()
if not my_units then return end

local map_w, map_h = ctx:map_size()
local tick = ctx:tick()
local ability_used = {}

-- === CLASSIFY UNITS ===
local sentinels = {}
local rookclaws = {}
local magpikes = {}
local magpyres = {}
local jaycallers = {}
local jayflickers = {}
local dusktalons = {}
local hootseers = {}
local corvus_rex = {}
local all_combat = {}
local all_combat_ids = {}

local WORKER_KINDS = { MurderScrounger = true }

for _, u in ipairs(my_units) do
    if not WORKER_KINDS[u.kind] then
        table.insert(all_combat, u)
        table.insert(all_combat_ids, u.id)
        if u.kind == "Sentinel" then table.insert(sentinels, u)
        elseif u.kind == "Rookclaw" then table.insert(rookclaws, u)
        elseif u.kind == "Magpike" then table.insert(magpikes, u)
        elseif u.kind == "Magpyre" then table.insert(magpyres, u)
        elseif u.kind == "Jaycaller" then table.insert(jaycallers, u)
        elseif u.kind == "Jayflicker" then table.insert(jayflickers, u)
        elseif u.kind == "Dusktalon" then table.insert(dusktalons, u)
        elseif u.kind == "Hootseer" then table.insert(hootseers, u)
        elseif u.kind == "CorvusRex" then table.insert(corvus_rex, u)
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

-- Helper: weakest enemy within range
local function weakest_enemy_near(px, py, range)
    if enemy_count == 0 then return nil end
    local sq = range * range
    local best, best_hp = nil, 999999
    for _, e in ipairs(enemies) do
        local dx = e.x - px
        local dy = e.y - py
        if dx * dx + dy * dy <= sq and e.hp < best_hp then
            best_hp = e.hp
            best = e
        end
    end
    return best
end

-- =====================================================================
-- ABILITY LAYER
-- =====================================================================

-- Sentinel: Overwatch (slot 1, toggle, free) — keep active for ranged defense
for _, u in ipairs(sentinels) do
    if tick < 4 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
    end
end

-- Hootseer: PanopticGaze (slot 0, toggle, free) — activate early for vision
for _, u in ipairs(hootseers) do
    if tick < 4 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- Hootseer: DeathOmen (slot 2, 4 GPU, range 10) — debuff strongest cluster
for _, u in ipairs(hootseers) do
    if not ability_used[u.id] and gpu >= 4 and enemy_count > 0 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 100 then -- 10 tile range
            ctx:ability(u.id, 2, "entity", nil, nil, ce.id)
            ability_used[u.id] = true
            gpu = gpu - 4
        end
    end
end

-- Jaycaller: MurderRallyCry (slot 0, free, range 5) — buff nearby allies
for _, u in ipairs(jaycallers) do
    local nearby = 0
    for _, a in ipairs(all_combat) do
        local dx = a.x - u.x
        local dy = a.y - u.y
        if dx * dx + dy * dy <= 25 then nearby = nearby + 1 end
    end
    if nearby >= 3 then
        ctx:ability(u.id, 0, "self")
        ability_used[u.id] = true
    end
end

-- Jaycaller: Cacophony (slot 2, free, range 4) — area debuff on enemy groups
for _, u in ipairs(jaycallers) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 16 then
            ctx:ability(u.id, 2, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- Rookclaw: TalonDive (slot 0, free, range 8) — dive onto weak targets
for _, u in ipairs(rookclaws) do
    local weak = weakest_enemy_near(u.x, u.y, 8)
    if weak then
        ctx:ability(u.id, 0, "entity", nil, nil, weak.id)
        ability_used[u.id] = true
    end
end

-- Magpike: Pilfer (slot 0, free, range 4) — speed buff + resource steal
for _, u in ipairs(magpikes) do
    local ce, cd = closest_enemy(u)
    if ce and cd <= 16 then
        ctx:ability(u.id, 0, "entity", nil, nil, ce.id)
        ability_used[u.id] = true
    end
end

-- Magpike: GlitterBomb (slot 1, free, range 5) on groups
for _, u in ipairs(magpikes) do
    if not ability_used[u.id] then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 25 then
            ctx:ability(u.id, 1, "position", ce.x, ce.y)
            ability_used[u.id] = true
        end
    end
end

-- Magpyre: SignalJam (slot 0, 4 GPU, range 8) — disable enemy abilities
for _, u in ipairs(magpyres) do
    if gpu >= 4 and enemy_count >= 3 then
        ctx:ability(u.id, 0, "position", math.floor(enemy_cx), math.floor(enemy_cy))
        ability_used[u.id] = true
        gpu = gpu - 4
    end
end

-- Jayflicker: PhantomFlock (slot 0, 4 GPU, range 4) — illusion decoys
for _, u in ipairs(jayflickers) do
    if gpu >= 4 and enemy_count > 0 then
        local ce, cd = closest_enemy(u)
        if ce and cd <= 16 then
            ctx:ability(u.id, 0, "position", ce.x, ce.y)
            ability_used[u.id] = true
            gpu = gpu - 4
        end
    end
end

-- Jayflicker: MirrorPosition (slot 1, free, range 8) — reposition flank
for _, u in ipairs(jayflickers) do
    if not ability_used[u.id] and enemy_count > 0 then
        -- Teleport behind enemy cluster
        local behind_x = math.floor(enemy_cx + dir_x * 3)
        local behind_y = math.floor(enemy_cy + dir_y * 3)
        behind_x = math.max(0, math.min(map_w - 1, behind_x))
        behind_y = math.max(0, math.min(map_h - 1, behind_y))
        ctx:ability(u.id, 1, "position", behind_x, behind_y)
        ability_used[u.id] = true
    end
end

-- Dusktalon: SilentStrike (slot 1, free, range 1) — assassin finisher
for _, u in ipairs(dusktalons) do
    local weak = weakest_enemy_near(u.x, u.y, 2)
    if weak then
        ctx:ability(u.id, 1, "entity", nil, nil, weak.id)
        ability_used[u.id] = true
    end
end

-- CorvusRex: AllSeeingLie (slot 1, 8 GPU) — big tactical reveal
for _, u in ipairs(corvus_rex) do
    if gpu >= 8 and enemy_count >= 4 then
        ctx:ability(u.id, 1, "self")
        ability_used[u.id] = true
        gpu = gpu - 8
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

-- Formation: Sentinels hold back (ranged overwatch), Rookclaws forward
if enemy_count > 0 then
    local ranged_ids = {}
    for _, u in ipairs(sentinels) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(ranged_ids, u.id)
        end
    end
    for _, u in ipairs(hootseers) do
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

    -- Melee divers forward
    local melee_ids = {}
    for _, u in ipairs(rookclaws) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(melee_ids, u.id)
        end
    end
    for _, u in ipairs(dusktalons) do
        if not u.in_combat and not ability_used[u.id] then
            table.insert(melee_ids, u.id)
        end
    end
    if #melee_ids > 0 then
        local mx = math.floor(army_cx + dir_x * 4)
        local my_y = math.floor(army_cy + dir_y * 4)
        mx = math.max(0, math.min(map_w - 1, mx))
        my_y = math.max(0, math.min(map_h - 1, my_y))
        ctx:attack_move(melee_ids, mx, my_y)
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

-- Kite ranged (Sentinels, Hootseers) when enemies too close
if outnumbered and enemies then
    local ranged_units = {}
    for _, u in ipairs(sentinels) do table.insert(ranged_units, u) end
    for _, u in ipairs(hootseers) do table.insert(ranged_units, u) end
    for _, r in ipairs(ranged_units) do
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
