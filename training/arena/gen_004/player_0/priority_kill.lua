-- @name: priority_kill
-- @events: on_tick
-- @interval: 5

-- Gen 4: Priority targeting of backline threats.
--
-- THE PROBLEM THIS SOLVES:
-- The FSM's target_acquisition_system makes every unit attack the nearest
-- enemy. In a blob-vs-blob fight, this means our melee units pound away at
-- the enemy frontline (Nuisances, Chonks) while enemy Hissers (14 DPS,
-- range 5) and Mousers (12.5 DPS) shred our army from behind with impunity.
--
-- Gen 3 failed because its "focus fire" still targeted the nearest/weakest
-- enemy in range -- the same target the FSM was already hitting. Zero net
-- effect. Identical to baseline.
--
-- THIS SCRIPT IS DIFFERENT:
-- We specifically look for Hissers and Mousers that are NOT the nearest
-- enemy to any of our attackers. If a Hisser is sitting 3 tiles behind
-- the front line dealing 14 DPS, nobody is touching it because a Nuisance
-- is closer. We redirect 2 fast melee units (Nuisances preferred) to charge
-- past the front and assassinate that Hisser.
--
-- WHY THIS WORKS:
-- - Hisser has 70 HP. Two Nuisances deal 8 DPS each = 16 DPS combined.
--   That's ~4.4 seconds to kill a Hisser. The Hisser dies before our
--   Nuisances do (Nuisance has 80 HP, Hisser does 14 damage per 12 ticks).
-- - Removing a Hisser eliminates 14 DPS from the enemy army permanently.
--   That's worth more than two Nuisances fighting the front.
-- - Mouser has 55 HP. Even faster to kill.
--
-- CONSTRAINTS:
-- 1. Only issue attack_units commands. Never move/stop/attack_move.
-- 2. Only redirect units that are attacking == true (already in combat).
-- 3. Only redirect when we have 2+ attackers in combat (not solo fights).
-- 4. 25-tick cooldown between redirects to avoid command spam.
-- 5. Maximum 2 units redirected per cycle to avoid weakening the front.

-- Persistent state
if _G._pk_state == nil then
    _G._pk_state = {
        last_redirect_tick = -100,
        cooldown = 25,
    }
end
local state = _G._pk_state

local tick = ctx:tick()

-- Cooldown check first (cheap early exit)
if (tick - state.last_redirect_tick) < state.cooldown then
    return
end

-- Collect our non-worker combat units that are currently attacking.
local my_units = ctx:my_units()
local attackers = {}

for _, u in ipairs(my_units) do
    if u.attacking and u.kind ~= "Pawdler" then
        attackers[#attackers + 1] = u
    end
end

-- Need at least 2 attackers in combat for this to make sense.
-- With 1 attacker, redirecting it off its target is counterproductive.
if #attackers < 2 then
    return
end

-- Get all visible enemies.
local enemies = ctx:enemy_units()
if #enemies == 0 then
    return
end

-- STEP 1: Build a set of "nearest enemy" for each attacker.
-- These are the targets the FSM is already handling. We want to find
-- priority targets that are NOT in this set.
local fsm_targeted = {}  -- enemy_id -> true (enemies the FSM is already attacking)

for _, a in ipairs(attackers) do
    local nearest = ctx:nearest_enemy(a.x, a.y)
    if nearest then
        fsm_targeted[nearest.id] = true
    end
end

-- STEP 2: Find priority targets (Hissers/Mousers) that are NOT nearest
-- to any of our attackers. These are the backline threats nobody is touching.
local priority_targets = {}

for _, e in ipairs(enemies) do
    if (e.kind == "Hisser" or e.kind == "Mouser") and not fsm_targeted[e.id] then
        priority_targets[#priority_targets + 1] = e
    end
end

-- No untouched backline threats? Nothing to do. The FSM has it covered.
if #priority_targets == 0 then
    return
end

-- STEP 3: Pick the best priority target.
-- Prefer lowest HP (easiest to finish off). Among equal HP, prefer Hissers
-- (higher DPS = bigger threat to remove).
local chosen = priority_targets[1]
for i = 2, #priority_targets do
    local t = priority_targets[i]
    if t.hp < chosen.hp then
        chosen = t
    elseif t.hp == chosen.hp and t.kind == "Hisser" and chosen.kind ~= "Hisser" then
        chosen = t
    end
end

-- STEP 4: Select up to 2 units to redirect.
-- Preference order:
--   1. Nuisances (fastest melee, 0.18 speed, can close distance quickly)
--   2. Mousers  (fast melee, 0.20 speed, high DPS)
--   3. Any other melee attacker
-- We ONLY pick melee units because ranged units (Hissers) can already
-- shoot over the front line -- they don't need to be redirected.
-- We also skip Chonks (too slow to reach the backline, 0.08 speed).

local candidates = {}

for _, a in ipairs(attackers) do
    if a.attack_type == "Melee" and a.kind ~= "Chonk" and a.kind ~= "Pawdler" then
        candidates[#candidates + 1] = a
    end
end

-- Sort candidates: Nuisance first, then Mouser, then others.
-- Within same type, prefer higher HP (more likely to survive the charge).
table.sort(candidates, function(a, b)
    local rank_a = (a.kind == "Nuisance" and 1) or (a.kind == "Mouser" and 2) or 3
    local rank_b = (b.kind == "Nuisance" and 1) or (b.kind == "Mouser" and 2) or 3
    if rank_a ~= rank_b then
        return rank_a < rank_b
    end
    return a.hp > b.hp
end)

-- Take at most 2 candidates.
local redirect_ids = {}
local max_redirect = 2

for i = 1, math.min(max_redirect, #candidates) do
    redirect_ids[#redirect_ids + 1] = candidates[i].id
end

-- Issue the redirect command.
if #redirect_ids > 0 then
    ctx:attack_units(redirect_ids, chosen.id)
    state.last_redirect_tick = tick
end
