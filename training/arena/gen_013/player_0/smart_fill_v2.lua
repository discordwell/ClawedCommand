-- @name: smart_fill_v2
-- @events: on_tick
-- @interval: 5

-- Gen 13: Smart fill v2 — adds supply management.
-- Gen 12 trained extra units but sometimes hit supply cap.
-- This version also builds LitterBoxes when supply is tight,
-- and trains from both CatTree AND ServerRack.
-- Key rules:
--   1. Never build before FSM finishes (5+ completed buildings)
--   2. Build LitterBox when supply headroom <= 3 and we have a free worker
--   3. Train Hisser > Nuisance from idle CatTree
--   4. Train Mouser from idle ServerRack

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

-- 1. Supply management: if supply headroom <= 3, build LitterBox (75 food)
local headroom = res.supply_cap - res.supply
if headroom <= 3 and res.food >= 75 then
    -- Find an idle worker near our base
    local workers = ctx:my_units("Pawdler")
    if workers then
        for _, w in ipairs(workers) do
            if not w.is_dead and not w.moving then
                -- Build LitterBox near TheBox
                local boxes = ctx:my_buildings("TheBox")
                if boxes and #boxes > 0 then
                    local bx = boxes[1].x + 3
                    local by = boxes[1].y + 3
                    ctx:build(w.id, "LitterBox", bx, by)
                    res.food = res.food - 75
                    break
                end
            end
        end
    end
end

-- 2. CatTree production
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

-- 3. ServerRack production: Mouser (75f, 25gpu)
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
