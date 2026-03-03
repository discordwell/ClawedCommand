-- @name: auto_economy
-- @events: on_tick
-- @interval: 5
-- Intents: gather, harvest, mine, economy, workers

-- Faction-agnostic worker management and resource gathering.
-- Detects faction from HQ building kind, trains workers from HQ,
-- and assigns idle workers to nearest resource deposits.

-- -----------------------------------------------------------------------
-- Faction detection from HQ building
-- -----------------------------------------------------------------------
local function detect_faction(buildings)
    for _, b in ipairs(buildings) do
        if b.kind == "TheBox" then return { worker = "Pawdler", hq = "TheBox" } end
        if b.kind == "TheBurrow" then return { worker = "Nibblet", hq = "TheBurrow" } end
        if b.kind == "TheGrotto" then return { worker = "Ponderer", hq = "TheGrotto" } end
        if b.kind == "TheParliament" then return { worker = "MurderScrounger", hq = "TheParliament" } end
        if b.kind == "TheSett" then return { worker = "Delver", hq = "TheSett" } end
        if b.kind == "TheDumpster" then return { worker = "Scrounger", hq = "TheDumpster" } end
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

local res = ctx:get_resources()
local workers = ctx:my_units(faction.worker)
if not workers then workers = {} end

local TARGET_WORKERS = 4
local WORKER_COST = 50

-- Train workers from HQ if below target
if #workers < TARGET_WORKERS and res.food >= WORKER_COST then
    for _, b in ipairs(buildings) do
        if b.kind == faction.hq and not b.under_construction and not b.producing then
            ctx:train(b.id, faction.worker)
            break
        end
    end
end

-- Assign idle workers to resource deposits
local deposits = ctx:resource_deposits()
if not deposits or #deposits == 0 then return end

-- Prefer food if low, otherwise balance
local prefer_food = res.food < 200

for _, w in ipairs(workers) do
    if w.idle then
        local best = nil
        local best_dist = math.huge
        for _, d in ipairs(deposits) do
            -- Skip depleted deposits
            if d.remaining > 0 then
                local dx = d.x - w.x
                local dy = d.y - w.y
                local dist = dx * dx + dy * dy
                -- Bias toward food deposits when food is low
                if prefer_food and d.kind == "Food" then
                    dist = dist * 0.5
                end
                if dist < best_dist then
                    best_dist = dist
                    best = d
                end
            end
        end
        if best then
            ctx:gather({w.id}, best.id)
        end
    end
end
