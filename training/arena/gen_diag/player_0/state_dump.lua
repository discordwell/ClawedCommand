-- @name: state_dump
-- @events: on_tick
-- @interval: 5

-- Diagnostic: dump game state at key ticks to understand FSM behavior.
-- Uses deliberate errors to output data to match report.

local tick = ctx:tick()
local res = ctx:resources()

-- Only dump at specific ticks
local dump_ticks = {100, 200, 300, 500, 750, 1000, 1500, 2000, 2500, 3000}
local should_dump = false
for _, t in ipairs(dump_ticks) do
    if tick == t then
        should_dump = true
        break
    end
end

if not should_dump then return end

-- Build state string
local parts = {"T=" .. tick}

if res then
    parts[#parts + 1] = "F=" .. res.food
    parts[#parts + 1] = "G=" .. res.gpu_cores
    parts[#parts + 1] = "S=" .. res.supply .. "/" .. res.supply_cap
end

-- Buildings
local all_bldgs = {"TheBox", "CatTree", "ServerRack", "LitterBox", "ScratchingPost", "FishMarket"}
for _, kind in ipairs(all_bldgs) do
    local bs = ctx:my_buildings(kind)
    if bs and #bs > 0 then
        for _, b in ipairs(bs) do
            local state = "idle"
            if b.under_construction then
                state = "building"
            elseif b.producing then
                state = "prod"
            end
            parts[#parts + 1] = kind .. "=" .. state
        end
    end
end

-- Unit counts
local units = ctx:my_units()
local counts = {}
if units then
    for _, u in ipairs(units) do
        counts[u.kind] = (counts[u.kind] or 0) + 1
    end
end
local unit_str = ""
for k, v in pairs(counts) do
    unit_str = unit_str .. k .. ":" .. v .. " "
end
parts[#parts + 1] = "Units=" .. unit_str

error(table.concat(parts, " | "))
