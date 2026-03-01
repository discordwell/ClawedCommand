-- @name: hisser_fill
-- @events: on_tick
-- @interval: 5

-- Gen 11: Train Hissers (100 food, ranged) from idle CatTrees.
-- Gen 10 trained Nuisances but they're too weak — extra cheap units
-- just feed the enemy. Hissers are ranged and deal real damage.
-- Wait for 5+ completed buildings to avoid stealing FSM build budget.

local res = ctx:resources()
if not res then return end
if res.food < 100 then return end
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

local cat_trees = ctx:my_buildings("CatTree")
if not cat_trees then return end

for _, b in ipairs(cat_trees) do
    if not b.under_construction and not b.producing then
        if res.food >= 100 then
            ctx:train(b.id, "Hisser")
            res.food = res.food - 100
        end
    end
end
