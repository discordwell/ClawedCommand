-- @name: siege_test_abilities
-- @events: on_tick
-- @interval: 3

-- Counter-turtle ability validation script.
-- Activates deploy/siege abilities on appropriate units per faction.
-- Run alongside combat_micro.lua to test turtle-breaking effectiveness.
--
-- Siege ability slots (0-indexed):
--   catGPT  Catnapper:     slot 2 = SiegeNap (toggle, range extend)
--   Seekers Cragback:      slot 1 = Entrench (toggle, DR + anti-static)
--   Murder  Hootseer:      slot 2 = DeathOmen (long-range snipe vs stationary)
--   LLAMA   GreaseMonkey:  slot 2 = JunkMortarMode (toggle, AoE range)
--   Croak   Croaker:       slot 2 = Inflate (bombardment mode)

local my_units = ctx:my_units()
if not my_units then return end

local current_tick = ctx:tick()

-- Only activate abilities after initial formation phase (tick 200+)
if current_tick < 200 then return end

for _, u in ipairs(my_units) do
    local kind = u.kind

    -- catGPT: Catnapper → SiegeNap (slot 2, self-cast toggle)
    if kind == "Catnapper" then
        ctx:ability(u.id, 2, "self")

    -- Seekers: Cragback → Entrench (slot 1, self-cast toggle)
    elseif kind == "Cragback" then
        ctx:ability(u.id, 1, "self")

    -- LLAMA: GreaseMonkey → JunkMortarMode (slot 2, self-cast toggle)
    elseif kind == "GreaseMonkey" then
        ctx:ability(u.id, 2, "self")

    -- Croak: Croaker → Inflate (slot 2, self-cast toggle)
    elseif kind == "Croaker" then
        ctx:ability(u.id, 2, "self")

    -- Murder: Hootseer → DeathOmen (slot 2, entity target — snipe weakest visible enemy)
    elseif kind == "Hootseer" then
        local enemies = ctx:enemy_units()
        if enemies and #enemies > 0 then
            -- Target the lowest HP enemy (most likely to kill)
            local weakest = nil
            local lowest_hp = 999999
            for _, e in ipairs(enemies) do
                if e.hp < lowest_hp then
                    lowest_hp = e.hp
                    weakest = e
                end
            end
            if weakest then
                ctx:ability(u.id, 2, "entity", nil, nil, weakest.id)
            end
        end
    end
end
