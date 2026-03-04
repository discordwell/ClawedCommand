#!/usr/bin/env python3
"""Generate the Act 2 'Rat King's Maze' campaign mission RON file.

Paints a 64x64 maze with concentric defensive rings and outputs
assets/campaign/act2_rat_kings_maze.ron.

Map layout (row 0 = north):
  Rows 0-1:   Rock border (elev 2)
  Rows 2-7:   Central Keep (TheBurrow, GnawLab, JunkTransmitter)
  Rows 8-9:   Water moat with 2 Ramp bridges (cols 20, 44)
  Rows 10-19: Inner Ring — Rock corridors, SqueakTowers (elev 2)
  Rows 20-39: Middle Ring — Road corridors, chokepoint towers (elev 1)
  Rows 40-55: Outer Ring — Forest corridors, NestingBox warrens (elev 0-1)
  Rows 56-62: Player start — open Grass (elev 0)
  Row 63:     Rock border
"""

import os
import sys

W, H = 64, 64

# Terrain grid: default Rock (impassable)
tiles = [["Rock"] * W for _ in range(H)]
elev = [[0] * W for _ in range(H)]

# ── Helpers ──────────────────────────────────────────────────────────────────

def set_tile(x, y, terrain, elevation=None):
    if 0 <= x < W and 0 <= y < H:
        tiles[y][x] = terrain
        if elevation is not None:
            elev[y][x] = elevation

def fill_rect(x0, y0, x1, y1, terrain, elevation=None):
    for y in range(max(0, y0), min(H, y1 + 1)):
        for x in range(max(0, x0), min(W, x1 + 1)):
            set_tile(x, y, terrain, elevation)

def hline(y, x0, x1, terrain, elevation=None):
    fill_rect(x0, y, x1, y, terrain, elevation)

def vline(x, y0, y1, terrain, elevation=None):
    fill_rect(x, y0, x, y1, terrain, elevation)


# ═══════════════════════════════════════════════════════════════════════════════
# PAINT THE MAP
# ═══════════════════════════════════════════════════════════════════════════════

# ── 1. Borders (rows 0-1, row 63, cols 0-1, cols 62-63) ─────────────────────
fill_rect(0, 0, 63, 1, "Rock", 2)
fill_rect(0, 63, 63, 63, "Rock", 2)
fill_rect(0, 0, 1, 63, "Rock", 2)
fill_rect(62, 0, 63, 63, "Rock", 2)

# ── 2. Player Start (rows 56-62) ────────────────────────────────────────────
fill_rect(2, 56, 61, 62, "Grass", 0)
# Entry roads heading north
vline(20, 56, 62, "Road", 0)
vline(32, 56, 62, "Road", 0)
vline(44, 56, 62, "Road", 0)

# ── 3. Outer Ring (rows 40-55) ──────────────────────────────────────────────
# Base: Rock walls everywhere at elev 1
fill_rect(2, 40, 61, 55, "Rock", 1)

# 3 N-S Forest corridors (3 tiles wide each)
for col_center in [16, 32, 48]:
    fill_rect(col_center - 1, 40, col_center + 1, 55, "Forest", 0)

# E-W cross-corridors connecting the verticals (Grass, 2 tiles high)
for row_center in [43, 48, 53]:
    hline(row_center, 14, 50, "Grass", 0)
    hline(row_center + 1, 14, 50, "Dirt", 0)

# Widen corridor exits at the south edge (rows 55) to connect to player start
for col_center in [16, 32, 48]:
    fill_rect(col_center - 2, 55, col_center + 2, 56, "Grass", 0)

# Road paths continue through outer ring
for col in [20, 44]:
    vline(col, 40, 55, "Road", 0)

# ── 4. Middle Ring (rows 20-39) ─────────────────────────────────────────────
fill_rect(2, 20, 61, 39, "Rock", 1)

# 2 main N-S Road corridors (3 tiles wide)
for col_center in [20, 44]:
    fill_rect(col_center - 1, 20, col_center + 1, 39, "Road", 1)

# 3 E-W cross-corridors (Dirt, 2 tiles high)
for row_center in [25, 30, 35]:
    hline(row_center, 18, 46, "Dirt", 1)
    hline(row_center + 1, 18, 46, "Dirt", 1)

# Central N-S passage connecting middle to outer
vline(32, 35, 40, "Dirt", 1)
vline(33, 35, 40, "Dirt", 1)

# ── 5. Inner Ring (rows 10-19) ──────────────────────────────────────────────
fill_rect(2, 10, 61, 19, "Rock", 2)

# 2 narrow corridors continuing from bridge ramps (2 tiles wide)
for col_center in [20, 44]:
    fill_rect(col_center - 1, 10, col_center, 19, "Dirt", 2)

# E-W cross-corridor connecting the two vertical passages
hline(15, 18, 46, "Dirt", 2)
hline(16, 18, 46, "Dirt", 2)

# Widen corridor exits at bottom of inner ring
for col_center in [20, 44]:
    fill_rect(col_center - 1, 19, col_center + 1, 20, "Dirt", 1)

# ── 6. Water Moat (rows 8-9) ────────────────────────────────────────────────
fill_rect(2, 8, 61, 9, "Water", 2)

# 2 Ramp bridges
for col_center in [20, 44]:
    set_tile(col_center - 1, 8, "Ramp", 2)
    set_tile(col_center, 8, "Ramp", 2)
    set_tile(col_center - 1, 9, "Ramp", 2)
    set_tile(col_center, 9, "Ramp", 2)

# ── 7. Central Keep (rows 2-7) ──────────────────────────────────────────────
fill_rect(2, 2, 61, 7, "Rock", 2)

# Interior: Dirt courtyard with Road path
fill_rect(10, 3, 54, 6, "Dirt", 2)
hline(4, 10, 54, "Road", 2)
hline(5, 10, 54, "Road", 2)

# Connect keep to ramp bridges
for col_center in [20, 44]:
    fill_rect(col_center - 1, 6, col_center, 8, "Ramp", 2)


# ═══════════════════════════════════════════════════════════════════════════════
# VALIDATE: all spawn positions must be on passable terrain
# ═══════════════════════════════════════════════════════════════════════════════

PASSABLE = {"Grass", "Dirt", "Sand", "Forest", "Shallows", "Ramp", "Road", "TechRuins"}

def check_passable(x, y, label):
    t = tiles[y][x]
    if t not in PASSABLE:
        print(f"WARNING: {label} at ({x},{y}) is on {t} (impassable!)", file=sys.stderr)
        return False
    return True


# ═══════════════════════════════════════════════════════════════════════════════
# MISSION DATA
# ═══════════════════════════════════════════════════════════════════════════════

# ── Buildings ────────────────────────────────────────────────────────────────

enemy_buildings = [
    # Keep
    ("TheBurrow",         32, 4, 1),
    ("GnawLab",           26, 4, 1),
    ("JunkTransmitter",   38, 4, 1),
    # Inner ring towers
    ("SqueakTower",       20, 11, 1),
    ("SqueakTower",       44, 11, 1),
    # Middle ring towers
    ("SqueakTower",       20, 22, 1),
    ("SqueakTower",       44, 22, 1),
    ("SqueakTower",       20, 37, 1),
    ("SqueakTower",       44, 37, 1),
    # Outer ring warrens
    ("NestingBox",        16, 43, 1),
    ("NestingBox",        32, 43, 1),
    ("NestingBox",        48, 43, 1),
    ("NestingBox",        16, 53, 1),
    ("NestingBox",        48, 53, 1),
]

player_buildings = [
    ("CatTree", 32, 60, 0),
]

# Validate building positions
for kind, bx, by, pid in enemy_buildings + player_buildings:
    check_passable(bx, by, f"Building {kind}")

# ── Player Units ─────────────────────────────────────────────────────────────

player_heroes = [
    ("Kelpie",  32, 59, True),
    ("Patches", 30, 59, False),
]

player_units = [
    # 4 Chonk (tank line)
    ("Chonk", 28, 58), ("Chonk", 30, 58), ("Chonk", 34, 58), ("Chonk", 36, 58),
    # 6 Hisser (ranged)
    ("Hisser", 26, 60), ("Hisser", 28, 60), ("Hisser", 34, 60),
    ("Hisser", 36, 60), ("Hisser", 38, 60), ("Hisser", 30, 61),
    # 4 Nuisance (scouts)
    ("Nuisance", 24, 59), ("Nuisance", 40, 59), ("Nuisance", 26, 61), ("Nuisance", 38, 61),
    # 2 Yowler (support)
    ("Yowler", 32, 61), ("Yowler", 34, 61),
]

for kind, ux, uy in player_units:
    check_passable(ux, uy, f"Player unit {kind}")
for name, hx, hy, _ in player_heroes:
    check_passable(hx, hy, f"Hero {name}")

# ── Enemy Waves ──────────────────────────────────────────────────────────────

# Wave 1: outer_patrol (Immediate) - scattered in outer ring corridors
outer_patrol_units = []
# Left corridor (col 16)
for i, y in enumerate([42, 45, 50, 54]):
    outer_patrol_units.append(("Swarmer", 16, y))
for y in [44, 52]:
    outer_patrol_units.append(("Nibblet", 15, y))
# Center corridor (col 32)
for y in [41, 46, 51, 55]:
    outer_patrol_units.append(("Swarmer", 32, y))
for y in [43, 49]:
    outer_patrol_units.append(("Nibblet", 33, y))
# Right corridor (col 48)
for y in [42, 47, 50, 53]:
    outer_patrol_units.append(("Swarmer", 48, y))
for y in [44, 48]:
    outer_patrol_units.append(("Nibblet", 47, y))

# Wave 2: middle_garrison (OnTrigger "outer_breached")
middle_garrison_units = [
    ("Quillback", 20, 25), ("Quillback", 44, 25),
    ("Quillback", 20, 35), ("Quillback", 44, 35),
    ("Shrieker", 21, 26), ("Shrieker", 45, 26),
    ("Shrieker", 21, 31), ("Shrieker", 45, 31),
    ("Shrieker", 21, 36), ("Shrieker", 45, 36),
    ("Swarmer", 19, 30), ("Swarmer", 43, 30),
    ("Swarmer", 19, 35), ("Swarmer", 43, 35),
]

# Waves 3-7: reinforcements (AtTick, gated by FlagSet)
reinforce_waves = []
for i in range(5):
    tick = 150 + i * 50  # 150, 200, 250, 300, 350
    # Alternate spawning from left and right warrens
    if i % 2 == 0:
        units = [
            ("Swarmer", 16, 43), ("Swarmer", 15, 43),
            ("Swarmer", 17, 43), ("Swarmer", 16, 44),
        ]
    else:
        units = [
            ("Swarmer", 48, 43), ("Swarmer", 47, 43),
            ("Swarmer", 49, 43), ("Swarmer", 48, 44),
        ]
    reinforce_waves.append((f"reinforce_{i+1}", tick, units))

# Wave 8: inner_guard (OnTrigger "middle_breached")
inner_guard_units = [
    ("Sparks", 20, 15), ("Sparks", 44, 15),
    ("Whiskerwitch", 21, 16), ("Whiskerwitch", 43, 16),
    ("WarrenMarshal", 32, 15),
    ("Swarmer", 19, 14), ("Swarmer", 20, 14),
    ("Swarmer", 44, 14), ("Swarmer", 44, 13),
    ("Swarmer", 31, 16), ("Swarmer", 33, 16),
]

# Wave 9: last_stand (OnTrigger "inner_breached")
last_stand_units = [
    ("Swarmer", 30, 5), ("Swarmer", 31, 5),
    ("Swarmer", 33, 5), ("Swarmer", 34, 5),
    ("Quillback", 28, 5), ("Quillback", 36, 5),
]

# Validate all wave spawns
for kind, ux, uy in outer_patrol_units:
    check_passable(ux, uy, f"outer_patrol {kind}")
for kind, ux, uy in middle_garrison_units:
    check_passable(ux, uy, f"middle_garrison {kind}")
for wname, _, units in reinforce_waves:
    for kind, ux, uy in units:
        check_passable(ux, uy, f"{wname} {kind}")
for kind, ux, uy in inner_guard_units:
    check_passable(ux, uy, f"inner_guard {kind}")
for kind, ux, uy in last_stand_units:
    check_passable(ux, uy, f"last_stand {kind}")

# ── Dialogue ─────────────────────────────────────────────────────────────────

dialogue = [
    # [0-2] Opening
    ("Le Chat", "Alright team — Claudeus Maximus has holed up inside his little labyrinth. Three rings of walls, towers, and more mice than a cheese factory.",
     "AiVoice", "portrait_le_chat"),
    ("Kelpie", "I can smell the cheese from here. How do we get through?",
     "Normal", "portrait_kelpie"),
    ("Patches", "Three corridors in the outer ring. Watch for ambushes at the junctions — warrens can pump out reinforcements fast.",
     "Normal", "portrait_patches"),

    # [3-4] Enter outer ring
    ("Claudeus Maximus", "Ah, visitors! Welcome to my MAGNIFICENT maze! I spent WEEKS on the interior decorating. Please admire the towers before they obliterate you.",
     "AiVoice", "portrait_claudeus"),
    ("Kelpie", "He seems... confident.",
     "Normal", "portrait_kelpie"),

    # [5-6] First kills
    ("Claudeus Maximus", "That was... that was just the welcome committee! The REAL defenses are further in!",
     "AiVoice", "portrait_claudeus"),
    ("Le Chat", "His voice went up an octave. Good sign.",
     "AiVoice", "portrait_le_chat"),

    # [7-8] Outer breached
    ("Claudeus Maximus", "Fine! So you got through the outer ring! That was SUPPOSED to be the easy part! I PLANNED it that way!",
     "AiVoice", "portrait_claudeus"),
    ("Patches", "Middle ring ahead. Two main corridors — expect heavier resistance. Quillbacks for sure.",
     "Normal", "portrait_patches"),

    # [9-10] Reinforcements
    ("Claudeus Maximus", "RELEASE THE RESERVES! All warrens, full production! Swarm them! SWARM THEM!",
     "AiVoice", "portrait_claudeus"),
    ("Kelpie", "More coming from the nesting boxes. Focus fire and push through!",
     "Normal", "portrait_kelpie"),

    # [11-12] Middle breached
    ("Claudeus Maximus", "This is FINE. Everything is FINE. The inner ring has never been breached. NEVER! The data says so!",
     "AiVoice", "portrait_claudeus"),
    ("Le Chat", "I checked his data. He built the inner ring last Tuesday.",
     "AiVoice", "portrait_le_chat"),

    # [13-14] Inner ring entry
    ("Claudeus Maximus", "Sparks! Whiskerwitches! Do your JOBS! I pay you in premium artisanal cheese!",
     "AiVoice", "portrait_claudeus"),
    ("Patches", "Careful — Sparks units hit hard and the Whiskerwitches have area denial. Keep the Chonks forward.",
     "Normal", "portrait_patches"),

    # [15-16] Kelpie HP warning
    ("Le Chat", "Kelpie, your vitals are dropping. Fall back behind the tank line!",
     "AiVoice", "portrait_le_chat"),
    ("Kelpie", "I'm fine! ...Mostly fine.",
     "Normal", "portrait_kelpie"),

    # [17-18] Last stand
    ("Claudeus Maximus", "EVERYONE TO THE KEEP! GUARD THE BURROW WITH YOUR LIVES! I'LL REMEMBER YOUR SACRIFICE! ...probably!",
     "Shout", "portrait_claudeus"),
    ("Kelpie", "Last push. Take out the Burrow and it's over.",
     "Normal", "portrait_kelpie"),

    # [19-20] Victory
    ("Claudeus Maximus", "No no no NO! My beautiful maze! My PERFECTLY OPTIMAL defensive layout!",
     "AiVoice", "portrait_claudeus"),
    ("Le Chat", "For what it's worth, Claudeus, the maze WAS pretty. Bad at its job, but pretty.",
     "AiVoice", "portrait_le_chat"),

    # [21] Bonus — all warrens destroyed
    ("Patches", "All nesting boxes neutralized. No more reinforcements.",
     "Normal", "portrait_patches"),

    # [22] Thimble eliminated (high kill count)
    ("Kelpie", "That's the last of them. Every single defender — gone.",
     "Normal", "portrait_kelpie"),
]


# ═══════════════════════════════════════════════════════════════════════════════
# GENERATE RON
# ═══════════════════════════════════════════════════════════════════════════════

def ron_unit_spawn(kind, x, y, player_id):
    return f'(kind: {kind}, position: (x: {x}, y: {y}), player_id: {player_id})'

def ron_building_spawn(kind, x, y, player_id, pre_built=True):
    return f'(kind: {kind}, position: (x: {x}, y: {y}), player_id: {player_id}, pre_built: {str(pre_built).lower()})'

def ron_hero_spawn(hero_id, x, y, mission_critical, player_id=0):
    return f'(hero_id: {hero_id}, position: (x: {x}, y: {y}), mission_critical: {str(mission_critical).lower()}, player_id: {player_id})'

def ron_dialogue(speaker, text, voice_style, portrait):
    escaped = text.replace('\\', '\\\\').replace('"', '\\"')
    return f'(speaker: "{speaker}", text: "{escaped}", voice_style: {voice_style}, portrait: "{portrait}")'


# ── Flatten tiles ────────────────────────────────────────────────────────────

flat_tiles = []
flat_elev = []
for y in range(H):
    for x in range(W):
        flat_tiles.append(tiles[y][x])
        flat_elev.append(elev[y][x])

assert len(flat_tiles) == 4096, f"Expected 4096 tiles, got {len(flat_tiles)}"
assert len(flat_elev) == 4096, f"Expected 4096 elevations, got {len(flat_elev)}"

# ── Build RON string ─────────────────────────────────────────────────────────

ron_lines = []
ron_lines.append('(')
ron_lines.append('    id: "act2_rat_kings_maze",')
ron_lines.append('    name: "The Rat King\'s Maze",')
ron_lines.append('    act: 2,')
ron_lines.append('    mission_index: 9,')

# Map
ron_lines.append('    map: Inline(')
ron_lines.append(f'        width: {W},')
ron_lines.append(f'        height: {H},')

# Tiles — one row per line for readability
ron_lines.append('        tiles: [')
for y in range(H):
    row_str = ", ".join(flat_tiles[y * W:(y + 1) * W])
    comma = "," if y < H - 1 else ""
    ron_lines.append(f'            {row_str}{comma}')
ron_lines.append('        ],')

# Elevation
ron_lines.append('        elevation: [')
for y in range(H):
    row_str = ", ".join(str(e) for e in flat_elev[y * W:(y + 1) * W])
    comma = "," if y < H - 1 else ""
    ron_lines.append(f'            {row_str}{comma}')
ron_lines.append('        ],')
ron_lines.append('    ),')

# Player setup
ron_lines.append('    player_setup: (')
ron_lines.append('        heroes: [')
for name, hx, hy, crit in player_heroes:
    ron_lines.append(f'            {ron_hero_spawn(name, hx, hy, crit)},')
ron_lines.append('        ],')
ron_lines.append('        units: [')
for kind, ux, uy in player_units:
    ron_lines.append(f'            {ron_unit_spawn(kind, ux, uy, 0)},')
ron_lines.append('        ],')
ron_lines.append('        buildings: [')
for kind, bx, by, pid in player_buildings:
    ron_lines.append(f'            {ron_building_spawn(kind, bx, by, pid)},')
# Enemy buildings also go in player_setup.buildings with player_id 1
for kind, bx, by, pid in enemy_buildings:
    ron_lines.append(f'            {ron_building_spawn(kind, bx, by, pid)},')
ron_lines.append('        ],')
ron_lines.append('        starting_food: 200,')
ron_lines.append('        starting_gpu: 50,')
ron_lines.append('        starting_nfts: 0,')
ron_lines.append('    ),')

# Enemy waves
ron_lines.append('    enemy_waves: [')

# Wave 1: outer_patrol
patrol_waypoints = '[(x: 16, y: 42), (x: 48, y: 42), (x: 48, y: 55), (x: 16, y: 55)]'
ron_lines.append('        (')
ron_lines.append('            wave_id: "outer_patrol",')
ron_lines.append('            trigger: Immediate,')
ron_lines.append('            units: [')
for kind, ux, uy in outer_patrol_units:
    ron_lines.append(f'                {ron_unit_spawn(kind, ux, uy, 1)},')
ron_lines.append('            ],')
ron_lines.append(f'            ai_behavior: Patrol({patrol_waypoints}),')
ron_lines.append('        ),')

# Wave 2: middle_garrison
ron_lines.append('        (')
ron_lines.append('            wave_id: "middle_garrison",')
ron_lines.append('            trigger: OnTrigger("outer_breached"),')
ron_lines.append('            units: [')
for kind, ux, uy in middle_garrison_units:
    ron_lines.append(f'                {ron_unit_spawn(kind, ux, uy, 1)},')
ron_lines.append('            ],')
ron_lines.append('            ai_behavior: Defend,')
ron_lines.append('        ),')

# Waves 3-7: reinforcements
for wname, tick, units in reinforce_waves:
    ron_lines.append('        (')
    ron_lines.append(f'            wave_id: "{wname}",')
    ron_lines.append(f'            trigger: AtTick({tick}),')
    ron_lines.append('            units: [')
    for kind, ux, uy in units:
        ron_lines.append(f'                {ron_unit_spawn(kind, ux, uy, 1)},')
    ron_lines.append('            ],')
    ron_lines.append('            ai_behavior: AttackMove((x: 32, y: 60)),')
    ron_lines.append('        ),')

# Wave 8: inner_guard
ron_lines.append('        (')
ron_lines.append('            wave_id: "inner_guard",')
ron_lines.append('            trigger: OnTrigger("middle_breached"),')
ron_lines.append('            units: [')
for kind, ux, uy in inner_guard_units:
    ron_lines.append(f'                {ron_unit_spawn(kind, ux, uy, 1)},')
ron_lines.append('            ],')
ron_lines.append('            ai_behavior: Defend,')
ron_lines.append('        ),')

# Wave 9: last_stand
ron_lines.append('        (')
ron_lines.append('            wave_id: "last_stand",')
ron_lines.append('            trigger: OnTrigger("inner_breached"),')
ron_lines.append('            units: [')
for kind, ux, uy in last_stand_units:
    ron_lines.append(f'                {ron_unit_spawn(kind, ux, uy, 1)},')
ron_lines.append('            ],')
ron_lines.append('            ai_behavior: Defend,')
ron_lines.append('        ),')

ron_lines.append('    ],')

# Objectives
ron_lines.append('    objectives: [')
ron_lines.append('        (')
ron_lines.append('            id: "destroy_burrow",')
ron_lines.append('            description: "Destroy the Burrow",')
ron_lines.append('            primary: true,')
ron_lines.append('            condition: Manual,')
ron_lines.append('        ),')
ron_lines.append('        (')
ron_lines.append('            id: "kelpie_survives",')
ron_lines.append('            description: "Kelpie must survive",')
ron_lines.append('            primary: true,')
ron_lines.append('            condition: HeroDied(Kelpie),')
ron_lines.append('        ),')
ron_lines.append('        (')
ron_lines.append('            id: "destroy_warrens",')
ron_lines.append('            description: "Destroy all Nesting Boxes",')
ron_lines.append('            primary: false,')
ron_lines.append('            condition: Manual,')
ron_lines.append('        ),')
ron_lines.append('        (')
ron_lines.append('            id: "eliminate_thimble",')
ron_lines.append('            description: "Eliminate all defenders",')
ron_lines.append('            primary: false,')
ron_lines.append('            condition: Manual,')
ron_lines.append('        ),')
ron_lines.append('    ],')

# Triggers
ron_lines.append('    triggers: [')

# T1: Opening dialogue (AtTick 1)
ron_lines.append('        (')
ron_lines.append('            id: "opening",')
ron_lines.append('            condition: AtTick(1),')
ron_lines.append('            actions: [ShowDialogue([0, 1, 2])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T2: Enter outer ring (Kelpie crosses y~50)
ron_lines.append('        (')
ron_lines.append('            id: "enter_outer",')
ron_lines.append('            condition: HeroAtPos(hero: Kelpie, position: (x: 32, y: 50), radius: 16),')
ron_lines.append('            actions: [ShowDialogue([3, 4])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T3: First kills — kill count 6
ron_lines.append('        (')
ron_lines.append('            id: "first_kills",')
ron_lines.append('            condition: EnemyKillCount(6),')
ron_lines.append('            actions: [ShowDialogue([5, 6])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T4: Outer breached — kill count 14 + set flag
ron_lines.append('        (')
ron_lines.append('            id: "outer_breached",')
ron_lines.append('            condition: EnemyKillCount(14),')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([7, 8]),')
ron_lines.append('                SetFlag("outer_breached"),')
ron_lines.append('                SpawnWave("middle_garrison"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T5: Reinforcement dialogue
ron_lines.append('        (')
ron_lines.append('            id: "reinforce_dialogue",')
ron_lines.append('            condition: All([EnemyKillCount(20), FlagSet("outer_breached")]),')
ron_lines.append('            actions: [ShowDialogue([9, 10])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T6: Warren destruction secondary complete (kill count 30 = most swarmers dead)
ron_lines.append('        (')
ron_lines.append('            id: "warrens_cleared",')
ron_lines.append('            condition: EnemyKillCount(35),')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([21]),')
ron_lines.append('                CompleteObjective("destroy_warrens"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T7: Middle breached — Kelpie reaches y~20 area
ron_lines.append('        (')
ron_lines.append('            id: "middle_breached",')
ron_lines.append('            condition: HeroAtPos(hero: Kelpie, position: (x: 32, y: 22), radius: 14),')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([11, 12]),')
ron_lines.append('                SetFlag("middle_breached"),')
ron_lines.append('                SpawnWave("inner_guard"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T8: Inner ring entry
ron_lines.append('        (')
ron_lines.append('            id: "inner_entry",')
ron_lines.append('            condition: HeroAtPos(hero: Kelpie, position: (x: 32, y: 15), radius: 10),')
ron_lines.append('            actions: [ShowDialogue([13, 14])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T9: Inner breached — Kelpie near moat
ron_lines.append('        (')
ron_lines.append('            id: "inner_breached",')
ron_lines.append('            condition: HeroAtPos(hero: Kelpie, position: (x: 32, y: 10), radius: 6),')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([17, 18]),')
ron_lines.append('                SetFlag("inner_breached"),')
ron_lines.append('                SpawnWave("last_stand"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T10-T14: Reinforcement wave triggers (gated by flag)
for i in range(5):
    wname = f"reinforce_{i+1}"
    tick = 150 + i * 50
    ron_lines.append('        (')
    ron_lines.append(f'            id: "spawn_{wname}",')
    ron_lines.append(f'            condition: All([AtTick({tick}), FlagSet("outer_breached")]),')
    ron_lines.append(f'            actions: [SpawnWave("{wname}")],')
    ron_lines.append('            once: true,')
    ron_lines.append('        ),')

# T15: Kelpie HP warning
ron_lines.append('        (')
ron_lines.append('            id: "kelpie_hp_warning",')
ron_lines.append('            condition: HeroHpBelow(hero: Kelpie, percentage: 30),')
ron_lines.append('            actions: [ShowDialogue([15, 16])],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T16: Thimble eliminated (very high kill count = every defender dead)
ron_lines.append('        (')
ron_lines.append('            id: "thimble_eliminated",')
ron_lines.append('            condition: EnemyKillCount(55),')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([22]),')
ron_lines.append('                CompleteObjective("eliminate_thimble"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

# T17: Victory — all enemies dead
ron_lines.append('        (')
ron_lines.append('            id: "victory",')
ron_lines.append('            condition: AllEnemiesDead,')
ron_lines.append('            actions: [')
ron_lines.append('                ShowDialogue([19, 20]),')
ron_lines.append('                CompleteObjective("destroy_burrow"),')
ron_lines.append('            ],')
ron_lines.append('            once: true,')
ron_lines.append('        ),')

ron_lines.append('    ],')

# Dialogue
ron_lines.append('    dialogue: [')
for speaker, text, voice, portrait in dialogue:
    ron_lines.append(f'        {ron_dialogue(speaker, text, voice, portrait)},')
ron_lines.append('    ],')

# Briefing / debrief
ron_lines.append('    briefing_text: "Claudeus Maximus — self-proclaimed Rat King and supreme strategist — has retreated into his labyrinth fortress. Three concentric rings of walls, towers, and swarming defenders stand between you and his Burrow. Push through the corridors, silence the warrens, and topple his throne of cheese.",')
ron_lines.append('    debrief_text: "The maze has fallen. Claudeus Maximus flees into the sewers, muttering about optimal tower placement. His forces are scattered, and the Clawed lose their strongest foothold in the region.",')
ron_lines.append(')')

# ═══════════════════════════════════════════════════════════════════════════════
# OUTPUT
# ═══════════════════════════════════════════════════════════════════════════════

script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
out_path = os.path.join(project_root, "assets", "campaign", "act2_rat_kings_maze.ron")

os.makedirs(os.path.dirname(out_path), exist_ok=True)

ron_text = "\n".join(ron_lines) + "\n"
with open(out_path, "w") as f:
    f.write(ron_text)

# Stats
tile_count = len(flat_tiles)
elev_count = len(flat_elev)
passable_count = sum(1 for t in flat_tiles if t in PASSABLE)
rock_count = sum(1 for t in flat_tiles if t == "Rock")
water_count = sum(1 for t in flat_tiles if t == "Water")

print(f"Generated: {out_path}")
print(f"  Tiles: {tile_count}  Elevation: {elev_count}")
print(f"  Passable: {passable_count}  Rock: {rock_count}  Water: {water_count}")
print(f"  Dialogue lines: {len(dialogue)}")
print(f"  Enemy waves: {2 + len(reinforce_waves) + 2}")
print(f"  Triggers: 17")
print(f"  Objectives: 4")
