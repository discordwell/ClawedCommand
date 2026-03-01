-- @name: nuisance_fill
-- @events: on_tick
-- @interval: 5

-- Gen 10b: Train Nuisances from idle CatTrees ONLY after FSM is done building.
-- Key insight: FSM builds 5 buildings. If we train before FSM finishes its
-- build order, we steal food from buildings. Wait until building count >= 4
-- (FSM has 4+ non-Box buildings) to avoid interference.
-- Also requires food >= 80 (Nuisance costs 75).

local res = ctx:resources()
if not res then return end
if res.food < 80 then return end
if res.supply >= res.supply_cap then return end

-- Count our completed buildings (not under construction)
local all_building_kinds = {"TheBox", "CatTree", "ServerRack", "LitterBox", "ScratchingPost", "FishMarket", "CatFlap", "LaserPointer"}
local completed_count = 0
for _, kind in ipairs(all_building_kinds) do
    local bs = ctx:my_buildings(kind)
    if bs then
        for _, b in ipairs(bs) do
            if not b.under_construction then
                completed_count = completed_count + 1
            end
        end
    end
end

-- Only activate after FSM has finished its build order (4+ completed non-Box buildings)
-- TheBox is pre-built, so 5 completed = TheBox + 4 built buildings
if completed_count < 5 then return end

local cat_trees = ctx:my_buildings("CatTree")
if not cat_trees then return end

for _, b in ipairs(cat_trees) do
    if not b.under_construction and not b.producing then
        if res.food >= 75 then
            ctx:train(b.id, "Nuisance")
            res.food = res.food - 75
        end
    end
end
