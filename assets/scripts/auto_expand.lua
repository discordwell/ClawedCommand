-- @name: auto_expand
-- @events: on_tick
-- @interval: 10
-- Intents: expand, grow, economy, base

-- Faction-agnostic mid-game expansion.
-- Builds resource depots near distant deposits, trains additional workers,
-- and rallies new workers toward ungathered resources.

-- -----------------------------------------------------------------------
-- Faction detection from HQ building
-- -----------------------------------------------------------------------
local function detect_faction(buildings)
    for _, b in ipairs(buildings) do
        if b.kind == "TheBox" then return { worker = "Pawdler", hq = "TheBox", depot = "FishMarket", depot_cost = 100 } end
        if b.kind == "TheBurrow" then return { worker = "Nibblet", hq = "TheBurrow", depot = "SeedVault", depot_cost = 75 } end
        if b.kind == "TheGrotto" then return { worker = "Ponderer", hq = "TheGrotto", depot = "LilyMarket", depot_cost = 100 } end
        if b.kind == "TheParliament" then return { worker = "MurderScrounger", hq = "TheParliament", depot = "CarrionCache", depot_cost = 100 } end
        if b.kind == "TheSett" then return { worker = "Delver", hq = "TheSett", depot = "BurrowDepot", depot_cost = 100 } end
        if b.kind == "TheDumpster" then return { worker = "Scrounger", hq = "TheDumpster", depot = "ScrapHeap", depot_cost = 90 } end
    end
    return nil
end

-- -----------------------------------------------------------------------
-- Main logic
-- -----------------------------------------------------------------------
local buildings = ctx:my_buildings()
if not buildings or #buildings == 0 then return end

local faction = detect_faction(buildings)
if not faction then return end

local tick = ctx:tick()
local res = ctx:get_resources()

-- Find HQ position
local hq_x, hq_y = 0, 0
local has_depot = false
for _, b in ipairs(buildings) do
    if b.kind == faction.hq then
        hq_x = b.x
        hq_y = b.y
    end
    if b.kind == faction.depot then
        has_depot = true
    end
end

local workers = ctx:my_units(faction.worker)
if not workers then workers = {} end

local MAX_WORKERS = 6
local WORKER_COST = 50

-- === TRAIN ADDITIONAL WORKERS mid-game ===
if tick > 300 and #workers < MAX_WORKERS and res.food >= WORKER_COST then
    for _, b in ipairs(buildings) do
        if b.kind == faction.hq and not b.under_construction and not b.producing then
            ctx:train(b.id, faction.worker)
            break
        end
    end
end

-- === BUILD DEPOT near distant resource deposit ===
if tick > 500 and not has_depot and res.food >= faction.depot_cost then
    local deposits = ctx:resource_deposits()
    if deposits and #deposits > 0 then
        -- Find the most distant deposit with resources
        local best_dep = nil
        local best_dist = 0
        for _, d in ipairs(deposits) do
            if d.remaining > 0 then
                local dx = d.x - hq_x
                local dy = d.y - hq_y
                local dist = dx * dx + dy * dy
                if dist > best_dist then
                    best_dist = dist
                    best_dep = d
                end
            end
        end

        if best_dep then
            -- Find a passable tile near the deposit for the depot
            local idle = ctx:idle_units(faction.worker)
            if idle and #idle > 0 then
                local checks = 0
                for dist = 2, 5 do
                    for dx = -dist, dist do
                        for dy = -dist, dist do
                            if math.abs(dx) == dist or math.abs(dy) == dist then
                                local tx = math.floor(best_dep.x) + dx
                                local ty = math.floor(best_dep.y) + dy
                                if tx >= 0 and ty >= 0 then
                                    checks = checks + 1
                                    if ctx:is_passable(tx, ty) then
                                        ctx:build(idle[1].id, faction.depot, tx, ty)
                                        return
                                    end
                                    if checks >= 8 then break end
                                end
                            end
                        end
                        if checks >= 8 then break end
                    end
                    if checks >= 8 then break end
                end
            end
        end
    end
end

-- === RALLY HQ toward ungathered deposits ===
if tick > 300 then
    local deposits = ctx:resource_deposits()
    if deposits and #deposits > 0 then
        -- Find nearest deposit that isn't right next to HQ (farther expansion target)
        local best = nil
        local best_dist = math.huge
        for _, d in ipairs(deposits) do
            if d.remaining > 0 then
                local dx = d.x - hq_x
                local dy = d.y - hq_y
                local dist = dx * dx + dy * dy
                -- Pick the nearest deposit that's at least 5 tiles away
                if dist >= 25 and dist < best_dist then
                    best_dist = dist
                    best = d
                end
            end
        end
        if best then
            for _, b in ipairs(buildings) do
                if b.kind == faction.hq then
                    ctx:rally(b.id, math.floor(best.x), math.floor(best.y))
                    break
                end
            end
        end
    end
end
