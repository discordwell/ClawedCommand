-- @name: murder_synergy_cascade
-- @events: on_tick
-- @interval: 5

-- MURDER SYNERGY CASCADE — "The Corvid Network awakens."
--
-- Linear algebra synergy engine for The Murder faction.
-- Each active ability has a 6-dimensional effect vector.
-- Synergy(A->B) = dot(output_A, input_B).
-- When a trigger fires, all abilities scoring above THRESHOLD auto-cascade.
--
-- Vector space: [damage_amp, crowd_control, vision, debuff, buff, gap_close]

-- ─── persistent state ───────────────────────────────────────────────
if not _G.msc then
  _G.msc = { cds = {}, triggered = {} }
end
local S = _G.msc
local tick = ctx:tick()

local THRESHOLD = 1.8   -- minimum dot product to cascade
local GPU_RESERVE = 3   -- keep this much GPU for emergencies

-- ─── 6D vector dot product ──────────────────────────────────────────
local function dot(a, b)
  return a[1]*b[1] + a[2]*b[2] + a[3]*b[3]
       + a[4]*b[4] + a[5]*b[5] + a[6]*b[6]
end

-- ─── ability registry ───────────────────────────────────────────────
-- Only active/toggle abilities (passives fire themselves).
-- Fields: unit_kind, slot, cd, gpu, tgt ("self"|"position"|"entity"),
--         range (tiles), output (what it creates), input (what it wants)
--         Vectors: [dmg_amp, cc, vision, debuff, buff, gap_close]

local ABILITIES = {
  talon_dive = {
    kind = "Rookclaw", slot = 0, cd = 100, gpu = 0, tgt = "entity", range = 8,
    output = { 0.8, 0.2, 0.0, 0.6, 0.0, 0.9 },  -- gap close + Murder's Mark debuff
    input  = { 0.3, 0.8, 0.4, 0.0, 0.7, 0.0 },   -- loves CC setup + buffs
  },
  glitter_bomb = {
    kind = "Magpike", slot = 1, cd = 150, gpu = 0, tgt = "position", range = 5,
    output = { 0.0, 0.9, 0.0, 0.8, 0.0, 0.0 },   -- AoE blind = heavy CC + debuff
    input  = { 0.0, 0.0, 0.6, 0.0, 0.3, 0.5 },    -- benefits from vision + gap close
  },
  pilfer = {
    kind = "Magpike", slot = 0, cd = 180, gpu = 0, tgt = "entity", range = 4,
    output = { 0.3, 0.0, 0.0, 0.5, 0.4, 0.0 },   -- steals buffs = debuff + self-buff
    input  = { 0.0, 0.6, 0.3, 0.0, 0.2, 0.4 },    -- benefits from CC opening
  },
  signal_jam = {
    kind = "Magpyre", slot = 0, cd = 300, gpu = 4, tgt = "position", range = 8,
    output = { 0.0, 0.7, 0.0, 0.9, 0.0, 0.0 },   -- comms disruption
    input  = { 0.0, 0.3, 0.5, 0.0, 0.3, 0.3 },    -- benefits from vision
  },
  decoy_nest = {
    kind = "Magpyre", slot = 1, cd = 200, gpu = 0, tgt = "position", range = 0,
    output = { 0.0, 0.4, 0.0, 0.3, 0.3, 0.0 },   -- decoy confusion
    input  = { 0.0, 0.2, 0.2, 0.0, 0.1, 0.0 },    -- mild synergy
  },
  rewire = {
    kind = "Magpyre", slot = 2, cd = 250, gpu = 5, tgt = "entity", range = 3,
    output = { 0.2, 0.5, 0.0, 0.9, 0.0, 0.0 },   -- sabotage target
    input  = { 0.0, 0.7, 0.3, 0.0, 0.2, 0.5 },    -- needs CC + gap close
  },
  murder_rally = {
    kind = "Jaycaller", slot = 0, cd = 200, gpu = 0, tgt = "self", range = 5,
    output = { 0.4, 0.0, 0.0, 0.0, 1.0, 0.2 },   -- +15% AS/MS, +25% vs Exposed
    input  = { 0.5, 0.2, 0.3, 0.4, 0.0, 0.3 },    -- rallies harder after debuffs
  },
  cacophony = {
    kind = "Jaycaller", slot = 2, cd = 250, gpu = 0, tgt = "position", range = 4,
    output = { 0.0, 0.9, 0.0, 0.7, 0.0, 0.0 },   -- disorientation zone
    input  = { 0.0, 0.0, 0.5, 0.0, 0.4, 0.5 },    -- benefits from vision + engage
  },
  phantom_flock = {
    kind = "Jayflicker", slot = 0, cd = 250, gpu = 4, tgt = "self", range = 4,
    output = { 0.0, 0.5, 0.0, 0.4, 0.3, 0.0 },   -- decoy confusion
    input  = { 0.0, 0.3, 0.2, 0.0, 0.3, 0.2 },    -- mild synergy
  },
  mirror_pos = {
    kind = "Jayflicker", slot = 1, cd = 180, gpu = 0, tgt = "entity", range = 8,
    output = { 0.0, 0.3, 0.0, 0.0, 0.2, 0.9 },   -- gap close via swap
    input  = { 0.3, 0.5, 0.3, 0.0, 0.4, 0.0 },    -- benefits from CC
  },
  silent_strike = {
    kind = "Dusktalon", slot = 1, cd = 200, gpu = 0, tgt = "entity", range = 1,
    output = { 1.0, 0.6, 0.0, 0.8, 0.0, 0.3 },   -- 300% burst + Silence
    input  = { 0.2, 0.9, 0.3, 0.0, 0.7, 0.0 },    -- assassin loves CC + buffs
  },
  panoptic_gaze = {
    kind = "Hootseer", slot = 0, cd = 10, gpu = 0, tgt = "self", range = 6,
    output = { 0.0, 0.0, 1.0, 0.2, 0.0, 0.0 },   -- 120° cone vision
    input  = { 0.0, 0.0, 0.0, 0.3, 0.2, 0.0 },    -- mostly independent
  },
  omen = {
    kind = "Hootseer", slot = 2, cd = 300, gpu = 3, tgt = "position", range = 3,
    output = { 0.3, 0.0, 0.9, 0.7, 0.0, 0.0 },   -- reveal + Expose debuff
    input  = { 0.3, 0.2, 0.0, 0.3, 0.3, 0.0 },    -- benefits from existing pressure
  },
  mimic_call = {
    kind = "MurderScrounger", slot = 2, cd = 200, gpu = 2, tgt = "self", range = 6,
    output = { 0.0, 0.3, 0.0, 0.4, 0.0, 0.2 },   -- lure/distraction
    input  = { 0.0, 0.1, 0.2, 0.0, 0.1, 0.0 },    -- mostly independent
  },
  all_seeing = {
    kind = "CorvusRex", slot = 1, cd = 900, gpu = 8, tgt = "self", range = 0,
    output = { 0.0, 0.0, 1.0, 0.5, 0.4, 0.0 },   -- full map reveal 3s
    input  = { 0.5, 0.3, 0.0, 0.3, 0.0, 0.0 },    -- save for big moments
  },
}

-- ─── precompute synergy matrix ──────────────────────────────────────
-- SYN[a][b] = how much firing 'a' wants 'b' to also fire
if not _G.msc_matrix then
  local M = {}
  for a_name, a_def in pairs(ABILITIES) do
    M[a_name] = {}
    for b_name, b_def in pairs(ABILITIES) do
      if a_name ~= b_name then
        M[a_name][b_name] = dot(a_def.output, b_def.input)
      end
    end
  end
  _G.msc_matrix = M
end
local SYN = _G.msc_matrix

-- ─── helpers ────────────────────────────────────────────────────────
local function cd_ready(unit_id, slot, cd_ticks)
  local key = unit_id .. "_" .. slot
  return not S.cds[key] or (tick - S.cds[key]) >= cd_ticks
end

local function mark_cd(unit_id, slot)
  S.cds[unit_id .. "_" .. slot] = tick
end

local function gpu_available(cost)
  local res = ctx:resources()
  if not res then return false end
  return res.gpu_cores >= (cost + GPU_RESERVE)
end

-- Find best enemy target near a position for entity-targeted abilities
local function best_target_near(x, y, range)
  local enemies = ctx:enemies_in_range(x, y, range)
  if #enemies == 0 then return nil end
  -- prefer lowest HP% for maximum killiness
  local best = enemies[1]
  local best_pct = best.hp / math.max(best.hp_max, 1)
  for i = 2, #enemies do
    local pct = enemies[i].hp / math.max(enemies[i].hp_max, 1)
    if pct < best_pct then
      best = enemies[i]
      best_pct = pct
    end
  end
  return best
end

-- Compute enemy centroid near a position
local function enemy_centroid_near(x, y, range)
  local enemies = ctx:enemies_in_range(x, y, range)
  if #enemies == 0 then return nil, nil end
  local cx, cy = 0, 0
  for _, e in ipairs(enemies) do
    cx = cx + e.x
    cy = cy + e.y
  end
  return math.floor(cx / #enemies), math.floor(cy / #enemies)
end

-- Fire an ability on a unit given the ability definition
local function fire(unit, ab_name, ab_def)
  if not cd_ready(unit.id, ab_def.slot, ab_def.cd) then return false end
  if ab_def.gpu > 0 and not gpu_available(ab_def.gpu) then return false end

  if ab_def.tgt == "self" then
    ctx:ability(unit.id, ab_def.slot, "self")
    mark_cd(unit.id, ab_def.slot)
    return true
  elseif ab_def.tgt == "entity" then
    local target = best_target_near(unit.x, unit.y, ab_def.range)
    if not target then return false end
    ctx:ability(unit.id, ab_def.slot, "entity", nil, nil, target.id)
    mark_cd(unit.id, ab_def.slot)
    return true
  elseif ab_def.tgt == "position" then
    local ex, ey = enemy_centroid_near(unit.x, unit.y, ab_def.range)
    if not ex then return false end
    ctx:ability(unit.id, ab_def.slot, "position", ex, ey)
    mark_cd(unit.id, ab_def.slot)
    return true
  end
  return false
end

-- ─── Phase 1: evaluate triggers ─────────────────────────────────────
-- Trigger abilities are the initiators — high-impact openers.
-- Priority: Omen > GlitterBomb > Cacophony > SignalJam > MurderRally
-- Only fire triggers when enemies are in engagement range.

local triggered = {}  -- ability names that fired this tick

local function try_trigger(ab_name, min_enemies)
  local ab = ABILITIES[ab_name]
  if not ab then return end
  local units = ctx:my_units(ab.kind)
  for _, u in ipairs(units) do
    if u.hp > 0 then
      local nearby = ctx:enemies_in_range(u.x, u.y, ab.range + 2)
      if #nearby >= min_enemies then
        if fire(u, ab_name, ab) then
          triggered[#triggered + 1] = ab_name
          return  -- one trigger per ability type per tick
        end
      end
    end
  end
end

-- Omen: reveal + expose, fire when 2+ enemies spotted
try_trigger("omen", 2)
-- GlitterBomb: AoE blind, fire into 3+ enemy clusters
try_trigger("glitter_bomb", 3)
-- Cacophony: disorientation, fire into 2+ enemies
try_trigger("cacophony", 2)
-- SignalJam: comms disruption, fire when 2+ enemies (expensive)
try_trigger("signal_jam", 2)
-- MurderRallyCry: buff when any combat happening (1+ enemy nearby)
try_trigger("murder_rally", 1)

-- ─── Phase 2: synergy cascade ───────────────────────────────────────
-- For each triggered ability, compute synergy with all other abilities.
-- Fire any that score above THRESHOLD (sorted by score, descending).

for _, trigger_name in ipairs(triggered) do
  -- Build scored list of cascade candidates
  local candidates = {}
  for resp_name, score in pairs(SYN[trigger_name]) do
    if score >= THRESHOLD then
      candidates[#candidates + 1] = { name = resp_name, score = score }
    end
  end
  -- Sort by synergy score descending (greediest cascade first)
  table.sort(candidates, function(a, b) return a.score > b.score end)

  for _, cand in ipairs(candidates) do
    local ab = ABILITIES[cand.name]
    if ab then
      local units = ctx:my_units(ab.kind)
      for _, u in ipairs(units) do
        if u.hp > 0 then
          if fire(u, cand.name, ab) then
            break  -- one cascade per ability type per trigger
          end
        end
      end
    end
  end
end

-- ─── Phase 3: opportunistic solo fires ──────────────────────────────
-- Abilities that are good on their own even without synergy triggers.
-- These fire if off cooldown and conditions are met, regardless of cascade.

-- Rookclaw TalonDive: always dive when enemies in range (gap close is king)
for _, u in ipairs(ctx:my_units("Rookclaw") or {}) do
  if u.hp > 0 and cd_ready(u.id, 0, 100) then
    local target = best_target_near(u.x, u.y, 8)
    if target then
      ctx:ability(u.id, 0, "entity", nil, nil, target.id)
      mark_cd(u.id, 0)
    end
  end
end

-- Dusktalon SilentStrike: assassinate low HP targets (melee range)
for _, u in ipairs(ctx:my_units("Dusktalon") or {}) do
  if u.hp > 0 and cd_ready(u.id, 1, 200) then
    local target = best_target_near(u.x, u.y, 1)
    if target then
      local pct = target.hp / math.max(target.hp_max, 1)
      if pct < 0.5 then  -- execute threshold
        ctx:ability(u.id, 1, "entity", nil, nil, target.id)
        mark_cd(u.id, 1)
      end
    end
  end
end

-- Hootseer PanopticGaze: toggle on when enemies within 8 tiles
for _, u in ipairs(ctx:my_units("Hootseer") or {}) do
  if u.hp > 0 and cd_ready(u.id, 0, 10) then
    local nearby = ctx:enemies_in_range(u.x, u.y, 8)
    if #nearby > 0 then
      ctx:ability(u.id, 0, "self")
      mark_cd(u.id, 0)
    end
  end
end

-- Magpike Pilfer: steal buffs from high-value targets
for _, u in ipairs(ctx:my_units("Magpike") or {}) do
  if u.hp > 0 and cd_ready(u.id, 0, 180) then
    local target = best_target_near(u.x, u.y, 4)
    if target then
      ctx:ability(u.id, 0, "entity", nil, nil, target.id)
      mark_cd(u.id, 0)
    end
  end
end
