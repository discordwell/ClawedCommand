-- @name: smart_focus
-- @events: on_tick
-- @interval: 5

-- Gen 3: Surgical focus fire at combat start + periodic re-evaluation.
--
-- STRATEGY:
-- The FSM attack-moves the whole army as a blob toward the enemy base.
-- When blobs collide (~tick 2000-2500), the side that concentrates damage
-- wins. We intervene ONCE at first contact to focus-fire the best target,
-- then re-evaluate every 20 ticks (2 seconds). Between interventions the
-- FSM keeps units in the fight naturally.
--
-- KEY RULES (lessons from Gen 1 + Gen 2):
-- 1. ONLY issue attack_units. Never move, attack_move, or stop.
-- 2. ONLY command units whose .attacking == true (already in combat).
-- 3. Don't require "3+ distinct targets" — that threshold never fires
--    in blob-vs-blob (Gen 2 proved this). Instead, always focus when
--    we have 2+ attackers and a good target exists.
-- 4. Use a cooldown so we don't spam commands every 5 ticks.
-- 5. Priority: Hissers > Mousers > weakest HP enemy.

-- Persistent state across ticks (Luau upvalues survive in sandbox)
if _G._sf_state == nil then
    _G._sf_state = {
        combat_started = false,   -- have we ever seen combat?
        last_focus_tick = -100,   -- tick when we last issued focus command
        cooldown = 20,            -- ticks between re-evaluations
    }
end
local state = _G._sf_state

local tick = ctx:tick()

-- Phase 1: Detect combat start.
-- Collect all our non-worker units that are currently attacking.
local my_units = ctx:my_units()
local attackers = {}

for _, u in ipairs(my_units) do
    if u.attacking and u.kind ~= "Pawdler" then
        attackers[#attackers + 1] = u
    end
end

-- No attackers means no combat. Nothing to do.
if #attackers == 0 then
    return
end

-- Phase 2: Cooldown check.
-- On first contact, fire immediately. After that, respect cooldown.
if state.combat_started then
    if (tick - state.last_focus_tick) < state.cooldown then
        return
    end
else
    -- First contact! Mark it and proceed immediately.
    state.combat_started = true
end

-- Phase 3: Find the best target to focus.
-- Priority kill order:
--   1. Hissers  (11.7 DPS, 70 HP, range 5 — the enemy's main damage)
--   2. Mousers  (12.5 DPS, 55 HP — fragile assassins)
--   3. Weakest enemy by current HP (finish off wounded units)
--
-- IMPORTANT: Only consider enemies that at least one of our attackers
-- can actually reach. Don't redirect units toward distant targets.

local enemies = ctx:enemy_units()
if #enemies == 0 then
    return
end

-- Build a set of enemies reachable by at least one attacker.
local reachable = {}  -- enemy_id -> enemy unit
for _, a in ipairs(attackers) do
    local targets = ctx:targets_for(a.id)
    if targets then
        for _, t in ipairs(targets) do
            if not reachable[t.id] then
                reachable[t.id] = t
            end
        end
    end
end

-- Search for priority targets among reachable enemies.
local best_hisser = nil
local best_mouser = nil
local weakest = nil

for _, e in pairs(reachable) do
    -- Track priority types
    if e.kind == "Hisser" then
        if best_hisser == nil or e.hp < best_hisser.hp then
            best_hisser = e
        end
    elseif e.kind == "Mouser" then
        if best_mouser == nil or e.hp < best_mouser.hp then
            best_mouser = e
        end
    end

    -- Track overall weakest
    if weakest == nil or e.hp < weakest.hp then
        weakest = e
    end
end

-- Pick target by priority order.
local chosen = best_hisser or best_mouser or weakest

if chosen == nil then
    return
end

-- Phase 4: Redirect attackers who CAN reach the chosen target.
-- Leave units alone if the target is out of their range — don't
-- pull them off whatever they're currently fighting.
local redirect_ids = {}

for _, a in ipairs(attackers) do
    local can_reach = false
    local targets = ctx:targets_for(a.id)
    if targets then
        for _, t in ipairs(targets) do
            if t.id == chosen.id then
                can_reach = true
                break
            end
        end
    end

    if can_reach then
        redirect_ids[#redirect_ids + 1] = a.id
    end
end

-- Only issue the command if we're actually redirecting someone.
if #redirect_ids > 0 then
    ctx:attack_units(redirect_ids, chosen.id)
    state.last_focus_tick = tick
end
