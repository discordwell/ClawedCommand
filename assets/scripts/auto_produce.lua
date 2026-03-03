-- @name: auto_produce
-- @events: on_tick
-- @interval: 5
-- Intents: produce, train, build, army, supply

-- Faction-agnostic army production and supply management.
-- Builds barracks, tech buildings, and supply depots. Trains combat units
-- in a faction-weighted composition from available production buildings.

-- -----------------------------------------------------------------------
-- Faction detection from HQ building
-- -----------------------------------------------------------------------
local function detect_faction(buildings)
    for _, b in ipairs(buildings) do
        if b.kind == "TheBox" then
            return {
                worker = "Pawdler", hq = "TheBox",
                barracks = "CatTree", supply = "LitterBox", tech = "ServerRack",
                barracks_cost = 150, barracks_gpu = 0,
                supply_cost = 75, supply_gpu = 0,
                tech_cost = 100, tech_gpu = 75,
                -- Weighted unit roster: {kind, weight} from barracks
                army = {{"Nuisance", 3}, {"Hisser", 2}, {"Chonk", 1}},
            }
        end
        if b.kind == "TheBurrow" then
            return {
                worker = "Nibblet", hq = "TheBurrow",
                barracks = "NestingBox", supply = "WarrenExpansion", tech = "JunkTransmitter",
                barracks_cost = 100, barracks_gpu = 0,
                supply_cost = 50, supply_gpu = 0,
                tech_cost = 75, tech_gpu = 50,
                army = {{"Swarmer", 3}, {"Gnawer", 2}, {"Plaguetail", 1}},
            }
        end
        if b.kind == "TheGrotto" then
            return {
                worker = "Ponderer", hq = "TheGrotto",
                barracks = "SpawningPools", supply = "ReedBed", tech = "SunkenServer",
                barracks_cost = 150, barracks_gpu = 0,
                supply_cost = 75, supply_gpu = 0,
                tech_cost = 100, tech_gpu = 75,
                army = {{"Regeneron", 3}, {"Croaker", 2}, {"Gulper", 1}},
            }
        end
        if b.kind == "TheParliament" then
            return {
                worker = "MurderScrounger", hq = "TheParliament",
                barracks = "Rookery", supply = "NestBox", tech = "AntennaArray",
                barracks_cost = 150, barracks_gpu = 0,
                supply_cost = 75, supply_gpu = 0,
                tech_cost = 100, tech_gpu = 75,
                army = {{"Rookclaw", 3}, {"Sentinel", 2}, {"Magpike", 1}},
            }
        end
        if b.kind == "TheSett" then
            return {
                worker = "Delver", hq = "TheSett",
                barracks = "WarHollow", supply = "DeepWarren", tech = "CoreTap",
                barracks_cost = 150, barracks_gpu = 0,
                supply_cost = 80, supply_gpu = 0,
                tech_cost = 125, tech_gpu = 100,
                army = {{"Sapjaw", 3}, {"Dustclaw", 2}, {"Ironhide", 1}},
            }
        end
        if b.kind == "TheDumpster" then
            return {
                worker = "Scrounger", hq = "TheDumpster",
                barracks = "ChopShop", supply = "TrashPile", tech = "JunkServer",
                barracks_cost = 140, barracks_gpu = 0,
                supply_cost = 70, supply_gpu = 0,
                tech_cost = 90, tech_gpu = 65,
                army = {{"Bandit", 3}, {"GreaseMonkey", 2}, {"Wrecker", 1}},
            }
        end
    end
    return nil
end

-- -----------------------------------------------------------------------
-- Spiral search for build placement near a position
-- -----------------------------------------------------------------------
local function find_build_site(bx, by, buildings)
    -- Build a set of occupied tiles from existing buildings
    local occupied = {}
    for _, b in ipairs(buildings) do
        occupied[b.x .. "," .. b.y] = true
    end

    -- Spiral outward from distance 3 to 8, limited to 10 passable checks
    local checks = 0
    for dist = 3, 8 do
        for dx = -dist, dist do
            for dy = -dist, dist do
                if math.abs(dx) == dist or math.abs(dy) == dist then
                    local tx = math.floor(bx) + dx
                    local ty = math.floor(by) + dy
                    if tx >= 0 and ty >= 0 and not occupied[tx .. "," .. ty] then
                        checks = checks + 1
                        if ctx:is_passable(tx, ty) then
                            return tx, ty
                        end
                        if checks >= 10 then return nil, nil end
                    end
                end
            end
        end
    end
    return nil, nil
end

-- -----------------------------------------------------------------------
-- Main logic
-- -----------------------------------------------------------------------
local buildings = ctx:my_buildings()
if not buildings or #buildings == 0 then return end

local faction = detect_faction(buildings)
if not faction then return end

local res = ctx:get_resources()

-- Find HQ position as reference for building placement
local hq_x, hq_y = 0, 0
local has_barracks = false
local has_tech = false
local production_buildings = {}

for _, b in ipairs(buildings) do
    if b.kind == faction.hq then
        hq_x = b.x
        hq_y = b.y
    end
    if b.kind == faction.barracks then
        has_barracks = true
        if not b.under_construction then
            table.insert(production_buildings, b)
        end
    end
    if b.kind == faction.tech then
        has_tech = true
        -- Tech buildings produce different units; don't add to barracks production pool
    end
end

-- Find an idle worker for building tasks
local function get_idle_worker()
    local workers = ctx:idle_units(faction.worker)
    if workers and #workers > 0 then
        return workers[1]
    end
    return nil
end

-- === SUPPLY CHECK: build supply depot if supply is nearly capped ===
if res.supply + 2 >= res.supply_cap then
    if res.food >= faction.supply_cost and res.gpu_cores >= faction.supply_gpu then
        local worker = get_idle_worker()
        if worker then
            local sx, sy = find_build_site(hq_x, hq_y, buildings)
            if sx then
                ctx:build(worker.id, faction.supply, sx, sy)
                return -- one build order per tick
            end
        end
    end
end

-- === BARRACKS CHECK: build if none exists ===
if not has_barracks then
    if res.food >= faction.barracks_cost and res.gpu_cores >= faction.barracks_gpu then
        local worker = get_idle_worker()
        if worker then
            local sx, sy = find_build_site(hq_x, hq_y, buildings)
            if sx then
                ctx:build(worker.id, faction.barracks, sx, sy)
                return
            end
        end
    end
end

-- === TECH BUILDING: build if barracks exists but no tech ===
if has_barracks and not has_tech then
    if res.food >= faction.tech_cost and res.gpu_cores >= faction.tech_gpu then
        local worker = get_idle_worker()
        if worker then
            local sx, sy = find_build_site(hq_x, hq_y, buildings)
            if sx then
                ctx:build(worker.id, faction.tech, sx, sy)
                return
            end
        end
    end
end

-- === TRAIN UNITS from production buildings ===
if res.supply >= res.supply_cap then return end -- supply blocked

-- Round-robin with weighted composition
local total_weight = 0
for _, entry in ipairs(faction.army) do
    total_weight = total_weight + entry[2]
end

-- Count existing combat units to determine what to train next
local tick = ctx:tick()
-- Use tick-based rotation to distribute production across unit types
local weight_idx = (math.floor(tick / 5) % total_weight) + 1
local chosen_kind = faction.army[1][1] -- fallback
local cumulative = 0
for _, entry in ipairs(faction.army) do
    cumulative = cumulative + entry[2]
    if weight_idx <= cumulative then
        chosen_kind = entry[1]
        break
    end
end

-- Train from first available non-busy production building
for _, pb in ipairs(production_buildings) do
    if not pb.producing and res.food >= 50 then
        ctx:train(pb.id, chosen_kind)
        break
    end
end
