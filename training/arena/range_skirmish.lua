-- @name range_skirmish
-- @events tick
-- @interval 3

-- Skirmisher auto-kite: ranged units maintain standoff distance
-- against shorter-ranged enemies. Uses position_at_range for optimal
-- repositioning with terrain-aware fallback. Still focus-fires when safe.
-- Budget: ~15-25 per tick (queries + position_at_range calls)

local my = ctx:my_units()
local enemies = ctx:enemy_units()
if #enemies == 0 then return end

-- Separate ranged combatants from everyone else
local workers = { Pawdler=true, Scrounger=true, Delver=true, Ponderer=true }
local ranged = {}
local all_combat = {}

for _, u in ipairs(my) do
    if not workers[u.kind] then
        table.insert(all_combat, u)
        if u.atk_type == "Ranged" and u.range > 1 then
            table.insert(ranged, u)
        end
    end
end

if #ranged == 0 then return end

-- Combat centroid for focus-fire targeting (proven #1 behavior)
local cx, cy = 0, 0
for _, u in ipairs(all_combat) do
    cx = cx + u.x
    cy = cy + u.y
end
cx = cx / #all_combat
cy = cy / #all_combat

-- Focus target: closest enemy to our centroid within 12 tiles
local focus_target = nil
local focus_dist = 999999
for _, e in ipairs(enemies) do
    local dx = e.x - cx
    local dy = e.y - cy
    local d2 = dx * dx + dy * dy
    if d2 < focus_dist and d2 < 144 then
        focus_dist = d2
        focus_target = e
    end
end

-- Per-unit skirmish logic
local kited = {}

for _, r in ipairs(ranged) do
    -- Find closest enemy that we outrange
    local threat = nil
    local threat_d2 = 999999

    for _, e in ipairs(enemies) do
        if e.range < r.range then
            local dx = e.x - r.x
            local dy = e.y - r.y
            local d2 = dx * dx + dy * dy
            if d2 < threat_d2 then
                threat_d2 = d2
                threat = e
            end
        end
    end

    if threat then
        local threat_dist = math.sqrt(threat_d2)
        -- Danger zone: enemy's range + 1.5 tile buffer
        -- Start repositioning before they can close to attack range
        local danger_zone = threat.range + 1.5

        if threat_dist <= danger_zone then
            -- Primary: find optimal position at our max attack range
            local desired = math.floor(r.range)
            local kx, ky = ctx:position_at_range(
                r.x, r.y, threat.x, threat.y, desired
            )

            if kx then
                local mc = ctx:movement_cost(kx, ky)
                if mc and mc < 1.3 then
                    ctx:move_units({r.id}, kx, ky)
                    kited[r.id] = true
                end
            end

            -- Fallback: terrain-aware directional flee
            if not kited[r.id] then
                local dx = r.x - threat.x
                local dy = r.y - threat.y
                local dist = math.sqrt(dx * dx + dy * dy)
                if dist < 0.01 then dist = 1 end
                dx = dx / dist
                dy = dy / dist

                local flee_x = math.floor(r.x + dx * 3 + 0.5)
                local flee_y = math.floor(r.y + dy * 3 + 0.5)

                local mc = ctx:movement_cost(flee_x, flee_y)
                if mc and mc < 1.3 then
                    ctx:move_units({r.id}, flee_x, flee_y)
                    kited[r.id] = true
                else
                    -- Try perpendicular escape routes
                    local perps = {
                        { math.floor(r.x + dy * 3 + 0.5), math.floor(r.y - dx * 3 + 0.5) },
                        { math.floor(r.x - dy * 3 + 0.5), math.floor(r.y + dx * 3 + 0.5) },
                    }
                    for _, p in ipairs(perps) do
                        local pmc = ctx:movement_cost(p[1], p[2])
                        if pmc and pmc < 1.3 then
                            ctx:move_units({r.id}, p[1], p[2])
                            kited[r.id] = true
                            break
                        end
                    end
                end
            end
        end
    end

    -- Not kiting this tick — focus fire the centroid target
    if not kited[r.id] and focus_target then
        ctx:attack_units({r.id}, focus_target.id)
    end
end
