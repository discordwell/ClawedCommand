-- @name llama_expand
-- @events tick
-- @interval 30

-- Find our raccoon builders (Pawdler = worker kind)
local workers = ctx:my_units("Pawdler")
if #workers == 0 then return end

-- Locate the nearest unclaimed resource deposit
local deposit = ctx:nearest_deposit()
if not deposit then return end

-- Bail if we already have a building covering this deposit
local buildings = ctx:my_buildings()
for _, b in ipairs(buildings) do
    if ctx:distance_squared_between(b, deposit) < 144 then
        -- Already expanded here (~12 tile radius), nothing to do
        return
    end
end

-- Pick the closest worker to the deposit
local closest_worker = nil
local closest_dist = math.huge
for _, w in ipairs(workers) do
    local d = ctx:distance_squared_between(w, deposit)
    if d < closest_dist then
        closest_dist = d
        closest_worker = w
    end
end

if not closest_worker then return end

-- Yank them off current task and send to build a base
ctx:stop({closest_worker})
ctx:build({closest_worker}, "TheBox", deposit)
