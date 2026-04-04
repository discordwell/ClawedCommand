#!/usr/bin/env python3
"""Generate the dream office inline map (48x36) for dream_office.ron."""

W, H = 48, 36

# Terrain aliases
D = "DryWall"
C = "Concrete"
T = "CarpetTile"
M = "MetalGrate"
L = "Linoleum"
R = "Road"

tiles = [[D] * W for _ in range(H)]

def fill(x1, y1, x2, y2, t):
    """Fill rectangle [x1..x2) x [y1..y2) with terrain t."""
    for y in range(y1, y2):
        for x in range(x1, x2):
            if 0 <= x < W and 0 <= y < H:
                tiles[y][x] = t

# === EXTERIOR WALLS (rows 0-1, 34-35, cols 0, 47) ===
# Already DryWall by default

# === PARKING LOT / EXTERIOR (left side) ===
fill(1, 2, 7, 34, R)  # parking area cols 1-6

# === MAIN ENTRANCE CORRIDOR (row 2-3, full width) ===
fill(8, 2, 47, 4, C)  # wide corridor across top

# === NORTH WING (rows 4-9) ===
# Comms room (phone)
fill(8, 4, 15, 9, L)
# Break room (talk)
fill(16, 4, 23, 9, L)
# North corridor
fill(24, 4, 28, 9, C)
# Barracks (sleep)
fill(29, 4, 38, 9, L)
# Storage/utility
fill(39, 4, 47, 9, L)

# Interior walls between north rooms
for y in range(4, 9):
    tiles[y][15] = D  # between comms and break
    tiles[y][23] = D  # between break and corridor
    tiles[y][28] = D  # between corridor and barracks
    tiles[y][38] = D  # between barracks and storage

# Doorways in south wall of north wing (row 9 → corridor row 10)
fill(8, 9, 47, 10, D)  # wall between north wing and corridor
# Doorways
for x in [11, 19, 26, 33, 43]:
    tiles[9][x] = C

# === CENTRAL CORRIDOR (row 10) ===
fill(8, 10, 47, 12, C)

# === OPS CENTER (rows 12-27, cols 8-47) — the big room ===
fill(8, 12, 47, 28, T)

# Left-side corridor (cols 7-8, connecting parking to ops)
fill(7, 2, 8, 34, C)

# === WALL between ops center and south wing (row 28) ===
fill(8, 28, 47, 29, D)
# Doorways in this wall
for x in [12, 20, 26, 35, 42]:
    tiles[28][x] = C

# === SOUTH CORRIDOR (row 29) ===
fill(8, 29, 47, 30, C)

# === SOUTH WING (rows 30-33) ===
# Gym (workout)
fill(8, 30, 18, 34, M)
# South corridor segment
fill(19, 30, 23, 34, C)
# Mess hall (eat)
fill(24, 30, 36, 34, L)
# South corridor segment
fill(37, 30, 39, 34, C)
# Recreation / lounge
fill(40, 30, 47, 34, L)

# Interior walls between south rooms
for y in range(30, 34):
    tiles[y][18] = D
    tiles[y][23] = D
    tiles[y][36] = D
    tiles[y][39] = D

# Doorways in north wall of south wing
for x in [13, 21, 30, 38, 43]:
    tiles[29][x] = C  # already concrete from corridor fill, but explicit

# === BASE EXIT corridor (left side, connecting parking to corridor) ===
# Already handled by the left corridor fill

# === Verify all border tiles are DryWall ===
for x in range(W):
    tiles[0][x] = D
    tiles[1][x] = D
    tiles[H-1][x] = D
    tiles[H-2][x] = D
for y in range(H):
    tiles[y][0] = D
    tiles[y][W-1] = D

# Print RON tile array
print(f"        width: {W},")
print(f"        height: {H},")
print("        tiles: [")
for y in range(H):
    row_str = ", ".join(tiles[y])
    comment = ""
    if y <= 1: comment = " // exterior wall"
    elif y <= 3: comment = " // entrance corridor"
    elif y <= 8: comment = " // north wing: comms | break | corridor | barracks | storage"
    elif y == 9: comment = " // wall + doorways"
    elif y <= 11: comment = " // central corridor"
    elif y <= 27: comment = " // OPS CENTER"
    elif y == 28: comment = " // wall + doorways"
    elif y == 29: comment = " // south corridor"
    elif y <= 33: comment = " // south wing: gym | corridor | mess | corridor | lounge"
    else: comment = " // exterior wall"
    print(f"            {row_str},{comment}")
print("        ],")

# Elevation: all flat
print("        elevation: [")
for y in range(H):
    print(f"            {','.join(['0']*W)},")
print("        ],")

# Also print updated location positions for the bigger map
print("\n// Suggested interaction locations:")
print(f"//   Work:         (20, 20) — center of ops center")
print(f"//   EnergyDrink:  (21, 29) — south corridor vending")
print(f"//   WorkOut:      (13, 32) — gym")
print(f"//   CallAda:      (11, 6)  — comms room")
print(f"//   Sleep:        (33, 6)  — barracks")
print(f"//   Eat:          (30, 32) — mess hall")
print(f"//   Talk:         (19, 6)  — break room")
print(f"//   Base exit:    (4, 18)  — parking lot")

# Hero positions
print(f"\n// Hero positions:")
print(f"//   KellFisher:  (20, 20) — at his desk in ops center")
print(f"//   RexHarmon:   (22, 20) — next to Kell")
