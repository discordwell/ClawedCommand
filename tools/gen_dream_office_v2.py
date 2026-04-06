#!/usr/bin/env python3
"""Generate the expanded dream office map (96x64) for dream_office.ron.

96x64 military operations base with:
- West outdoor: parking lot, guard gate, courtyard, motor pool, helipad
- North wing: comms, briefing, CO office, SCIF (new), server room (new)
- Center: large ops center (~28 desks), Kell's desk at (47,30)
- East wing: armory, supply closet (new), chapel (new)
- South wing: rec room, laundry, gym, locker room, vending nook, mess hall, barracks
- Corridors: north (y~10), central cross (x~65), south connector (y~42)

Design intent: explorable on first visit but every interactable is a dead end.
The grind triangle (desk → energy drink → gym) is compact; exploration routes
pass through interesting rooms the player eventually stops noticing.
"""

W, H = 96, 64

# Terrain aliases
D = "DryWall"    # walls, impassable
C = "Concrete"   # corridors, industrial
T = "CarpetTile" # offices, ops center
M = "MetalGrate" # armory, SCIF, server room
L = "Linoleum"   # hallways, break rooms, mess
R = "Road"       # parking, driveways
G = "Grass"      # courtyard, outdoor
S = "Sand"       # running track (uses dirt path feel)

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


# ==================================================================
# EXTERIOR — west side (cols 1-14)
# ==================================================================

# Perimeter border (rows 0-1, 62-63, cols 0, 95) — stays DryWall

# Parking lot (west, full height)
fill(1, 2, 14, 44, R)

# Courtyard / smoking area (south-west outdoor)
fill(1, 44, 14, 55, G)
# Courtyard concrete paths
fill(4, 46, 12, 48, C)
fill(7, 44, 9, 52, C)

# Motor pool (south-west, below courtyard)
fill(1, 55, 14, 62, C)
# Motor pool road access
fill(1, 53, 14, 55, R)

# Helipad (south, large concrete pad)
fill(16, 55, 40, 62, C)
# Helipad markings (road stripes for visual contrast)
fill(24, 57, 32, 61, R)

# Main entrance corridor (col 14-16, connects parking to building)
fill(14, 2, 16, 55, C)

# ==================================================================
# NORTH WING (rows 2-13) — admin, comms, briefing, CO, SCIF, servers
# ==================================================================

# Guard shack (entrance area)
fill(16, 2, 26, 8, L)

# Comms room
fill(27, 2, 38, 8, L)

# Briefing room
fill(39, 2, 52, 8, T)

# CO's office
fill(53, 2, 65, 8, T)

# SCIF (new — restricted intelligence facility)
fill(66, 2, 78, 8, M)

# Server room (new — extends further east)
fill(79, 2, 92, 12, M)

# North corridor (east-west, y=8-10)
fill(16, 8, 92, 10, C)

# Walls between north rooms (row 8)
hwall(8, 16, 78, doors=[21, 32, 45, 58, 72])

# Vertical walls between north rooms
vwall(26, 2, 8, doors=[5])   # guard | comms
vwall(38, 2, 8, doors=[5])   # comms | briefing
vwall(52, 2, 8, doors=[5])   # briefing | CO
vwall(65, 2, 8, doors=[5])   # CO | SCIF
vwall(78, 2, 10, doors=[5, 9])  # SCIF | server room

# ==================================================================
# MID-NORTH ROOMS (rows 10-18) — break room, medical, armory
# ==================================================================

# Break room / lounge (west of north)
fill(16, 10, 30, 18, L)

# Medical bay
fill(31, 10, 44, 18, L)

# Mid corridor segment
fill(45, 10, 64, 18, C)

# Bulletin board / water fountain corridor area
fill(45, 10, 64, 12, C)

# Armory (east)
fill(79, 12, 92, 22, M)

# Supply closet (new, small dead-end room east)
fill(83, 22, 92, 30, C)

# Chapel / quiet room (new, east wing)
fill(83, 30, 92, 40, T)

# Walls
hwall(18, 16, 64, doors=[22, 37, 55])
vwall(30, 10, 18, doors=[14])  # break | medical
vwall(44, 10, 18, doors=[14])  # medical | corridor

# East wing walls
vwall(78, 10, 42, doors=[11, 16, 25, 35])
hwall(12, 79, 92)  # top of armory (server room above)
hwall(22, 79, 92, doors=[87])  # armory | supply closet
hwall(30, 83, 92, doors=[87])  # supply | chapel
hwall(40, 83, 92, doors=[87])  # chapel bottom

# ==================================================================
# OPS CENTER (rows 18-38) — the big room, heart of the base
# ==================================================================

fill(16, 18, 78, 40, T)

# Walls around ops center
hwall(18, 16, 78, doors=[22, 35, 47, 60, 72])  # already placed above partially
hwall(40, 16, 78, doors=[22, 35, 47, 60, 72])

# Central cross-corridor (x=64-68, runs north-south through building)
fill(64, 10, 68, 55, C)

# ==================================================================
# SOUTH WING (rows 40-55) — gym, rec, mess, barracks
# ==================================================================

# South corridor (y=40-42)
fill(16, 40, 78, 42, C)

# Rec room (new — pool table, arcade)
fill(16, 42, 30, 52, L)

# Laundry (new)
fill(31, 42, 42, 52, C)

# Gym / PT area
fill(16, 52, 34, 60, L)

# Locker room
fill(35, 52, 46, 60, L)

# Vending / coffee nook
fill(55, 42, 68, 46, L)

# Mess hall (large)
fill(55, 46, 82, 56, L)

# Barracks (south-east)
fill(68, 56, 92, 62, L)

# Sleep area in barracks
fill(79, 46, 92, 56, L)

# Walls between south rooms
hwall(42, 16, 55, doors=[22, 36, 48])
hwall(52, 16, 46, doors=[25, 40])
vwall(30, 42, 52, doors=[47])   # rec | laundry
vwall(42, 42, 52, doors=[47])   # laundry | corridor
vwall(34, 52, 60, doors=[56])   # gym | lockers
vwall(46, 52, 60, doors=[56])   # lockers | corridor
vwall(54, 42, 56, doors=[44, 50])  # nook/mess west wall
vwall(82, 46, 62, doors=[50, 58])  # mess | sleep/barracks
hwall(46, 55, 92, doors=[60, 72, 85])
hwall(56, 55, 92, doors=[60, 72, 85])

# ==================================================================
# Ensure borders are DryWall
# ==================================================================
for x in range(W):
    tiles[0][x] = D
    tiles[1][x] = D
    tiles[H - 1][x] = D
    tiles[H - 2][x] = D
for y in range(H):
    tiles[y][0] = D
    tiles[y][W - 1] = D

# ==================================================================
# Output — RON format
# ==================================================================

print(f"        width: {W},")
print(f"        height: {H},")
print("        tiles: [")
for y in range(H):
    row_str = ", ".join(tiles[y])
    # Row comments
    if y <= 1:
        c = "// north perimeter"
    elif y <= 7:
        c = "// north wing: guard | comms | briefing | CO | SCIF"
    elif y <= 9:
        c = "// north corridor"
    elif y <= 11:
        c = "// break room | medical | corridor | server room"
    elif y <= 17:
        c = "// break room | medical | corridor | armory"
    elif y <= 39:
        c = "// OPS CENTER + cross corridor + east wing"
    elif y <= 41:
        c = "// south corridor"
    elif y <= 51:
        c = "// rec room | laundry | vending | mess hall"
    elif y <= 59:
        c = "// gym | lockers | mess | barracks"
    else:
        c = "// south perimeter"
    print(f"            {row_str}, {c}")
print("        ],")
print("        elevation: [")
for y in range(H):
    print(f"            {','.join(['0'] * W)},")
print("        ],")

# ==================================================================
# Reference info
# ==================================================================
print("""
// === HERO POSITIONS ===
//   KellFisher:  (47, 30) — ops center desk
//   RexHarmon:   (49, 30) — next to Kell

// === GRIND TRIANGLE ===
//   Work:         (47, 30) — ops center
//   EnergyDrink:  (60, 44) — vending nook
//   WorkOut:      (25, 56) — gym

// === INTERACTION LOCATIONS (37 total) ===
// Enabled:
//   Work:           (47, 30)
//   EnergyDrink:    (60, 44)
//   WorkOut:        (25, 56)
// Disabled (personal):
//   CallAda:        (32, 5)  — comms room phone
//   Sleep:          (85, 50) — barracks bunk
//   Eat:            (68, 50) — mess hall
//   Talk:           (22, 14) — break room couch
// Disabled (explore):
//   LeaveBase:      (8, 30)  — parking lot
//   Storage:        (85, 17) — armory
//   BulletinBoard:  (50, 9)  — north corridor
//   WaterFountain:  (55, 9)  — north corridor
//   Window:         (8, 14)  — look outside
//   BriefingRoom:   (45, 5)  — briefing room
//   CoOffice:       (58, 5)  — CO's office
//   MedicalBay:     (37, 14) — medical bay
//   LockerRoom:     (40, 56) — locker room
//   Courtyard:      (8, 48)  — outdoor
//   GuardPost:      (21, 4)  — guard shack
//   Tv:             (18, 14) — break room TV
//   PhotoWall:      (24, 11) — break room photos
//   CoffeeMachine:  (62, 44) — broken coffee next to vending
// New:
//   SitOnBench:       (8, 50)  — courtyard bench
//   CheckVehicles:    (8, 58)  — motor pool
//   LookAtHelicopter: (28, 58) — helipad
//   EnterScif:        (72, 5)  — SCIF door
//   CheckServers:     (85, 7)  — server room
//   UseMicrowave:     (20, 12) — break room
//   GrabSupplies:     (87, 26) — supply closet
//   SitAndReflect:    (87, 35) — chapel
//   PlayPool:         (23, 47) — rec room
//   PlayArcade:       (17, 44) — rec room
//   DoLaundry:        (36, 47) — laundry
//   ReadMenuBoard:    (70, 48) — mess hall
//   SitWithOthers:    (65, 52) — mess hall
//   ReadLetter:       (80, 60) — barracks
//   GoForRun:         (8, 54)  — near track / courtyard

// === NPC POSITIONS (~20) ===
//   (21, 4)  — guard at entrance
//   (37, 14) — medic in medical bay
//   (25, 56) — soldier in gym
//   (68, 50) — soldier eating in mess
//   (85, 50) — soldier sleeping in barracks
//   (18, 14) — soldier watching TV in break room
//   (50, 9)  — soldier in north corridor
//   (58, 5)  — soldier near CO office
//   (30, 42) — soldier in south corridor
//   (85, 42) — soldier near east wing
//   (6, 47)  — smoker in courtyard
//   (10, 49) — smoker in courtyard
//   (8, 58)  — mechanic in motor pool
//   (72, 5)  — analyst near SCIF
//   (85, 7)  — tech in server room
//   (23, 47) — soldier at pool table
//   (36, 47) — soldier in laundry
//   (65, 52) — soldier eating in mess
//   (75, 52) — soldier eating in mess
//   (80, 60) — soldier sleeping in barracks
""")
