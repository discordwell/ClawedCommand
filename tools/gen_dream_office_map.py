#!/usr/bin/env python3
"""Generate the dream office inline map (64x48) for dream_office.ron.

A full military operations base with:
- Large ops center (20+ desks) — the heart of the base
- Barracks wing (bunks, lockers)
- Mess hall + kitchen
- Gym / PT area
- Comms room (secure phones, radio)
- Break room / lounge (TV, couches)
- Medical bay
- Armory / storage
- CO's office (Kell's boss)
- Briefing room (maps, screens)
- Hallways connecting everything
- Main entrance / guard post
- Parking lot / exterior
- Courtyard / smoking area
"""

W, H = 64, 48

# Terrain aliases
D = "DryWall"
C = "Concrete"
T = "CarpetTile"
M = "MetalGrate"
L = "Linoleum"
R = "Road"

tiles = [[D] * W for _ in range(H)]

def fill(x1, y1, x2, y2, t):
    for y in range(y1, y2):
        for x in range(x1, x2):
            if 0 <= x < W and 0 <= y < H:
                tiles[y][x] = t

def hwall(y, x1, x2, doors=None):
    """Horizontal wall with optional door positions."""
    doors = doors or []
    for x in range(x1, x2):
        tiles[y][x] = C if x in doors else D

def vwall(x, y1, y2, doors=None):
    """Vertical wall with optional door positions."""
    doors = doors or []
    for y in range(y1, y2):
        tiles[y][x] = C if y in doors else D

# ============================================================
# EXTERIOR
# ============================================================

# Rows 0-1: North exterior wall (already DryWall)
# Rows 46-47: South exterior wall
# Cols 0, 63: East/West exterior walls

# Parking lot (west side, cols 1-8)
fill(1, 2, 9, 46, R)

# Courtyard / smoking area (south-west, between parking and building)
fill(1, 38, 9, 46, R)

# Main entrance corridor (col 9, full height access)
fill(9, 2, 11, 46, C)

# ============================================================
# NORTH WING (rows 2-13) — admin, comms, briefing
# ============================================================

# Guard post / reception (just inside entrance)
fill(11, 2, 19, 6, L)

# CO's office (top-right of north wing)
fill(50, 2, 62, 8, T)

# Briefing room (large, center-right)
fill(30, 2, 49, 8, T)

# Comms room (secure)
fill(20, 2, 29, 8, L)

# North corridor
fill(11, 8, 62, 10, C)

# Break room / lounge
fill(11, 10, 22, 14, L)

# Medical bay
fill(23, 10, 34, 14, L)

# Armory / storage
fill(50, 10, 62, 14, M)

# Secondary corridor between armory area and briefing
fill(35, 10, 49, 14, C)

# Walls between north rooms (row 8 = corridor, rooms above)
hwall(8, 11, 62, doors=[15, 25, 35, 45, 55])
# Walls between row-10-14 rooms
hwall(14, 11, 62, doors=[16, 28, 40, 55])
# Vertical walls in north rooms
vwall(19, 2, 8, doors=[5])   # guard | comms
vwall(29, 2, 8, doors=[5])   # comms | briefing
vwall(49, 2, 8, doors=[5])   # briefing | CO office
vwall(22, 10, 14, doors=[12])  # break | medical
vwall(34, 10, 14, doors=[12])  # medical | corridor
vwall(49, 10, 14, doors=[12])  # corridor | armory

# ============================================================
# OPS CENTER (rows 15-33) — the big room
# ============================================================

fill(11, 15, 62, 34, T)

# Walls around ops center
hwall(15, 11, 62, doors=[20, 30, 40, 50])  # north wall of ops
hwall(34, 11, 62, doors=[20, 30, 40, 50])  # south wall of ops

# ============================================================
# SOUTH WING (rows 35-45) — gym, mess, barracks
# ============================================================

# South corridor
fill(11, 35, 62, 37, C)

# Gym / PT area (south-west)
fill(11, 37, 24, 45, M)

# Locker room (next to gym)
fill(25, 37, 32, 45, L)

# Mess hall + kitchen (center-south)
fill(33, 37, 48, 45, L)

# Barracks (south-east, bunks)
fill(49, 37, 62, 45, L)

# Walls between south rooms
hwall(37, 11, 62, doors=[17, 28, 40, 55])
vwall(24, 37, 45, doors=[41])   # gym | lockers
vwall(32, 37, 45, doors=[41])   # lockers | mess
vwall(48, 37, 45, doors=[41])   # mess | barracks

# ============================================================
# Ensure borders are DryWall
# ============================================================
for x in range(W):
    tiles[0][x] = D
    tiles[1][x] = D
    tiles[H-1][x] = D
    tiles[H-2][x] = D
for y in range(H):
    tiles[y][0] = D
    tiles[y][W-1] = D

# ============================================================
# Output
# ============================================================

print(f"        width: {W},")
print(f"        height: {H},")
print("        tiles: [")
for y in range(H):
    row_str = ", ".join(tiles[y])
    # Compact comments
    if y <= 1: c = "// north wall"
    elif y <= 7: c = "// north wing: guard | comms | briefing | CO office"
    elif y <= 9: c = "// north corridor"
    elif y <= 13: c = "// break room | medical | corridor | armory"
    elif y == 14: c = "// wall"
    elif y <= 33: c = "// OPS CENTER"
    elif y <= 36: c = "// south corridor"
    elif y <= 44: c = "// south wing: gym | lockers | mess hall | barracks"
    else: c = "// south wall"
    print(f"            {row_str}, {c}")
print("        ],")
print("        elevation: [")
for y in range(H):
    print(f"            {','.join(['0']*W)},")
print("        ],")

# ============================================================
# Interaction locations + NPC patrol positions
# ============================================================
print("""
// === INTERACTION LOCATIONS ===
// Enabled:
//   Work:           (35, 24) — ops center, Kell's desk
//   EnergyDrink:    (30, 36) — south corridor vending machine
//   WorkOut:        (17, 41) — gym
// Disabled (personal):
//   CallAda:        (24, 5)  — comms room, secure phone
//   Sleep:          (55, 41) — barracks, bunk
//   Eat:            (40, 41) — mess hall
//   Talk:           (16, 12) — break room, couch
// Disabled (explore):
//   LeaveBase:      (5, 24)  — parking lot
//   Storage:        (55, 12) — armory
//   BulletinBoard:  (15, 9)  — north corridor, notice board
//   WaterFountain:  (30, 9)  — north corridor, fountain
//   Window:         (5, 10)  — parking lot, look outside
//   Briefing room:  (39, 5)  — briefing room, maps
//   CO office:      (55, 5)  — CO's office, door
//   Medical bay:    (28, 12) — medical bay
//   Locker room:    (28, 41) — locker room
//   Courtyard:      (5, 42)  — smoking area outside

// === HERO POSITIONS ===
//   KellFisher:  (35, 24) — at his desk
//   RexHarmon:   (37, 24) — next to Kell

// === NPC PATROL ROUTES (soldiers walking around) ===
// Corridor walker 1: (11,9) → (60,9) → loop
// Corridor walker 2: (11,36) → (60,36) → loop
// Ops center wanderer: random within (12,16)-(61,33)
// Guard at entrance: stationary at (15, 4)
// Gym user: stationary at (17, 40)
// Mess hall eater: stationary at (40, 40)
// Barracks sleeper: stationary at (55, 40)
// Medic: stationary at (28, 11)
""")
