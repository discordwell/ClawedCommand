-- @name: activate_abilities
-- @events: on_tick
-- @interval: 5

-- Gen 22: Ability activation — Luau-compatible (no goto, no _G mutation).
-- Stateless: toggles on each tick without tracking cooldowns.
-- The ability system itself handles cooldown enforcement.

-- LoafMode for stationary Chonks in combat (toggle, free, slot 1)
local chonks = ctx:my_units("Chonk")
if chonks then
    for _, c in ipairs(chonks) do
        if c.hp > 0 and not c.moving and c.attacking then
            ctx:ability(c.id, 1, "self")
        end
    end
end

-- Zoomies for Nuisances (speed boost, 10 GPU, slot 2)
-- Activate when: low HP (retreat boost) or healthy and fighting (engage boost)
local nuisances = ctx:my_units("Nuisance")
if nuisances then
    local res = ctx:resources()
    if res and res.gpu_cores >= 10 then
        for _, n in ipairs(nuisances) do
            if n.hp > 0 then
                local hp_pct = n.hp / math.max(n.hp_max, 1)
                if hp_pct < 0.3 or (n.attacking and hp_pct > 0.5) then
                    ctx:ability(n.id, 2, "self")
                end
            end
        end
    end
end

-- DissonantScreech for Yowlers (AoE stun, 10 GPU, slot 1)
-- Only when 3+ enemies clustered within 4 tiles
local yowlers = ctx:my_units("Yowler")
if yowlers then
    local res = ctx:resources()
    if res and res.gpu_cores >= 10 then
        for _, y in ipairs(yowlers) do
            if y.hp > 0 then
                local nearby = ctx:enemies_in_range(y.x, y.y, 4)
                if nearby and #nearby >= 3 then
                    local ex, ey = 0, 0
                    for _, e in ipairs(nearby) do
                        ex = ex + e.x
                        ey = ey + e.y
                    end
                    ex = math.floor(ex / #nearby)
                    ey = math.floor(ey / #nearby)
                    ctx:ability(y.id, 1, "position", ex, ey)
                end
            end
        end
    end
end
