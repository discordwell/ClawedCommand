-- basic_train: Train units from available production buildings
-- Intents: train, make units, produce, build army
local buildings = ctx:get_buildings()

for _, b in ipairs(buildings) do
    if b.kind == "CatTree" and b.producing == false then
        ctx:train_unit(b.id, "Nuisance")
        return
    end
    if b.kind == "TheBox" and b.producing == false then
        ctx:train_unit(b.id, "Pawdler")
        return
    end
end
