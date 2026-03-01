-- @name: smart_fill
-- @events: on_tick
-- @interval: 5

-- Gen 12: Smart production fill — Hisser if affordable, else Chonk.
-- Nuisances are too weak (Gen 10 showed extra Nuisances feed the enemy).
-- Chonk (125 food) is expensive but tanky. Hisser (100 food) is ranged DPS.
-- Priority: Hisser > Chonk (if we can afford) > nothing.
-- Wait for 5+ completed buildings.
-- Also train from ServerRack if idle and we can afford advanced units.

local res = ctx:resources()
if not res then return end
if res.supply >= res.supply_cap then return end

-- Count completed buildings
local all_kinds = {"TheBox", "CatTree", "ServerRack", "LitterBox", "ScratchingPost", "FishMarket", "CatFlap", "LaserPointer"}
local completed = 0
for _, kind in ipairs(all_kinds) do
    local bs = ctx:my_buildings(kind)
    if bs then
        for _, b in ipairs(bs) do
            if not b.under_construction then
                completed = completed + 1
            end
        end
    end
end

if completed < 5 then return end

-- CatTree production: Hisser (100f) preferred
local cat_trees = ctx:my_buildings("CatTree")
if cat_trees then
    for _, b in ipairs(cat_trees) do
        if not b.under_construction and not b.producing then
            if res.food >= 100 then
                ctx:train(b.id, "Hisser")
                res.food = res.food - 100
            elseif res.food >= 75 then
                ctx:train(b.id, "Nuisance")
                res.food = res.food - 75
            end
        end
    end
end

-- ServerRack production: Mouser (75f, 25gpu) — stealth scout, cheap
local server_racks = ctx:my_buildings("ServerRack")
if server_racks and res.food >= 75 and res.gpu_cores >= 25 then
    for _, b in ipairs(server_racks) do
        if not b.under_construction and not b.producing then
            if res.food >= 75 and res.gpu_cores >= 25 then
                ctx:train(b.id, "Mouser")
                res.food = res.food - 75
                res.gpu_cores = res.gpu_cores - 25
            end
        end
    end
end
