-- @name: tank_fill
-- @events: on_tick
-- @interval: 5

-- Gen 15: Tank-heavy composition — prioritize Chonks from idle CatTrees.
-- Gen 12's Hisser fill achieved 50% win rate. But Hissers are fragile.
-- Chonks (125 food, tank) absorb damage and let the rest of the army survive.
-- Hypothesis: a Chonk-heavy army trades more efficiently.
-- Fallback: Hisser if not enough food for Chonk, then Nuisance.

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

-- Count current Chonks to avoid over-tanking
local chonks = ctx:my_units("Chonk")
local chonk_count = 0
if chonks then
    for _, c in ipairs(chonks) do
        if not c.is_dead then chonk_count = chonk_count + 1 end
    end
end

local cat_trees = ctx:my_buildings("CatTree")
if cat_trees then
    for _, b in ipairs(cat_trees) do
        if not b.under_construction and not b.producing then
            -- Prioritize Chonk if < 3 and food allows (125f)
            if chonk_count < 3 and res.food >= 125 then
                ctx:train(b.id, "Chonk")
                res.food = res.food - 125
                chonk_count = chonk_count + 1
            elseif res.food >= 100 then
                ctx:train(b.id, "Hisser")
                res.food = res.food - 100
            elseif res.food >= 75 then
                ctx:train(b.id, "Nuisance")
                res.food = res.food - 75
            end
        end
    end
end

-- ServerRack: Mouser
local racks = ctx:my_buildings("ServerRack")
if racks and res.food >= 75 and res.gpu_cores >= 25 and res.supply < res.supply_cap then
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
