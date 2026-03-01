-- @name: econ_snowball
-- @events: on_tick
-- @interval: 5

-- Gen 14: Economic snowball — train extra Pawdlers for faster economy.
-- Gen 12 proved that training extra combat units from idle buildings works.
-- But what if we also train extra workers? More workers = more food income
-- = can sustain continuous production from CatTree.
-- Train 1 extra Pawdler from TheBox when idle + affordable.
-- Also keep Gen 12's smart_fill logic.

local res = ctx:resources()
if not res then return end

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

-- Count current workers to cap at 6 (diminishing returns after that)
local workers = ctx:my_units("Pawdler")
local worker_count = 0
if workers then
    for _, w in ipairs(workers) do
        if not w.is_dead then
            worker_count = worker_count + 1
        end
    end
end

-- TheBox: train extra Pawdler if we have < 6 workers and food >= 50
if res.supply < res.supply_cap and worker_count < 6 then
    local boxes = ctx:my_buildings("TheBox")
    if boxes then
        for _, b in ipairs(boxes) do
            if not b.under_construction and not b.producing then
                if res.food >= 50 then
                    ctx:train(b.id, "Pawdler")
                    res.food = res.food - 50
                end
            end
        end
    end
end

-- CatTree: Hisser > Nuisance (same as Gen 12)
if res.supply < res.supply_cap then
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
end

-- ServerRack: Mouser (same as Gen 12)
if res.supply < res.supply_cap then
    local racks = ctx:my_buildings("ServerRack")
    if racks and res.food >= 75 and res.gpu_cores >= 25 then
        for _, b in ipairs(racks) do
            if not b.under_construction and not b.producing then
                if res.food >= 75 and res.gpu_cores >= 25 then
                    ctx:train(b.id, "Mouser")
                    res.food = res.food - 75
                    res.gpu_cores = res.gpu_cores - 25
                end
            end
        end
    end
end
