-- @name: scout_and_adapt
-- @events: on_tick, on_enemy_spotted
-- @interval: 10

-- Scout and Adapt: Send a fast unit to scout early, then track enemy
-- composition to shift production priorities.
-- The FSM is blind -- it never scouts and commits armies blind.
-- Even basic intel about enemy composition is a huge advantage.
-- Runs every 10 ticks (1s) since scouting is not time-critical.

local tick = ctx:tick()
local w, h = ctx:map_size()

-- === SCOUTING PHASE ===
-- Before tick 1500 (~2.5 min), send the first available fast unit to scout.
-- Prefer Nuisance (fast, cheap, already trained by FSM in BuildUp).
-- Scout toward enemy half of the map.

if tick < 1500 then
    -- Check if we have a scout candidate (not gathering, idle or nearly idle)
    local scouts = ctx:my_units("Nuisance")
    if #scouts == 0 then
        scouts = ctx:my_units("FlyingFox")
    end

    -- Only assign ONE scout, don't waste army
    local scout = nil
    for _, u in ipairs(scouts) do
        if u.idle and not u.gathering then
            scout = u
            break
        end
    end

    if scout then
        -- Scout waypoints: sweep the enemy half of the map
        -- P0 typically spawns low coords, P1 high coords
        -- Send scout toward center first, then enemy base area
        local waypoints = {
            {x = math.floor(w / 2), y = math.floor(h / 2)},         -- map center
            {x = math.floor(w * 3 / 4), y = math.floor(h * 3 / 4)}, -- enemy quarter
            {x = math.floor(w - 5), y = math.floor(h - 5)},         -- enemy corner
        }
        ctx.behaviors:scout_pattern(scout.id, waypoints)
    end
end

-- === COMPOSITION TRACKING ===
-- Count enemy unit types from visible units and adjust production.
-- This is reactive intelligence: we see what they're making and counter.

local enemies = ctx:enemy_units()
if #enemies == 0 then
    return
end

-- Count enemy composition
local enemy_counts = {}
local total_enemies = 0
for _, e in ipairs(enemies) do
    enemy_counts[e.kind] = (enemy_counts[e.kind] or 0) + 1
    total_enemies = total_enemies + 1
end

-- Determine dominant enemy strategy
local melee_count = (enemy_counts["Nuisance"] or 0) + (enemy_counts["Chonk"] or 0)
                  + (enemy_counts["Mouser"] or 0) + (enemy_counts["FerretSapper"] or 0)
local ranged_count = (enemy_counts["Hisser"] or 0) + (enemy_counts["FlyingFox"] or 0)
                   + (enemy_counts["Catnapper"] or 0) + (enemy_counts["Yowler"] or 0)

-- === PRODUCTION ADAPTATION ===
-- Only adapt after we have buildings to produce from (tick > 1000)
-- and enough intel (5+ enemy units seen)
if tick > 1000 and total_enemies >= 3 then
    local cat_trees = ctx:my_buildings("CatTree")
    local server_racks = ctx:my_buildings("ServerRack")

    -- Counter-build logic:
    -- Enemy heavy melee -> train Hissers (ranged, high DPS, range 5)
    -- Enemy heavy ranged -> train Nuisances (fast melee to close distance)
    -- Enemy Chonk-heavy -> train Hissers (kite the slow tanks)
    -- Lots of Mousers -> train Chonks (tank their burst)

    if melee_count > ranged_count and #cat_trees > 0 then
        -- Counter melee with ranged
        for _, ct in ipairs(cat_trees) do
            if not ct.producing then
                if (enemy_counts["Chonk"] or 0) >= 2 then
                    -- Hissers kite Chonks perfectly
                    ctx:train(ct.id, "Hisser")
                else
                    -- General anti-melee: Hissers still good
                    ctx:train(ct.id, "Hisser")
                end
            end
        end
    elseif ranged_count > melee_count and #cat_trees > 0 then
        -- Counter ranged with fast melee closers
        for _, ct in ipairs(cat_trees) do
            if not ct.producing then
                ctx:train(ct.id, "Nuisance")
            end
        end
    end

    -- If enemy has MechCommander, we need burst damage
    if (enemy_counts["MechCommander"] or 0) > 0 and #server_racks > 0 then
        for _, sr in ipairs(server_racks) do
            if not sr.producing then
                -- FerretSappers have 20 melee damage, good vs heroes
                ctx:train(sr.id, "FerretSapper")
            end
        end
    end
end

-- === EARLY WARNING ===
-- If enemies spotted near our base before Attack phase, signal defend
-- by moving idle combat units toward the threat
local my_buildings = ctx:my_buildings()
if #my_buildings > 0 and tick < 2500 then
    -- Find base center
    local bx, by = 0, 0
    for _, b in ipairs(my_buildings) do
        bx = bx + b.x
        by = by + b.y
    end
    bx = math.floor(bx / #my_buildings)
    by = math.floor(by / #my_buildings)

    -- Check for enemies near base
    local near_base = ctx:enemies_in_range(bx, by, 12)
    if #near_base > 0 then
        -- Rally idle combat units to defend
        local defenders = ctx:idle_units()
        local def_ids = {}
        for _, u in ipairs(defenders) do
            if u.kind ~= "Pawdler" and not u.gathering then
                def_ids[#def_ids + 1] = u.id
            end
        end
        if #def_ids > 0 then
            ctx:attack_move(def_ids, near_base[1].x, near_base[1].y)
        end
    end
end
