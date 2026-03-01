#!/usr/bin/env python3
"""Generate the demo_canyon.ron mission file with an 80x48 inline map.

Canyon layout (y-axis):
  y=0-5:    Rock walls (elevation 2)
  y=6-7:    Ramp transition (elevation 1)
  y=8-19:   Grass plateau north (elevation 1) — P0 territory
  y=20:     North riverbank — Shallows/Road (elevation 0)
  y=21-26:  River — Water (elevation 0), with crossing points
  y=27:     South riverbank — Shallows/Road (elevation 0)
  y=28-39:  Grass plateau south (elevation 1) — P1 territory
  y=40-41:  Ramp transition (elevation 1)
  y=42-47:  Rock walls (elevation 2)

Crossing points (x-ranges):
  x=14-18:  Western ford (Shallows)
  x=38-42:  Central bridge (Road)
  x=60-64:  Eastern ford (Shallows)

Forests placed along plateau edges for tactical cover.
"""

WIDTH = 80
HEIGHT = 48

# Terrain type names matching RON serialization of TerrainType enum
GRASS = "Grass"
ROCK = "Rock"
WATER = "Water"
SHALLOWS = "Shallows"
ROAD = "Road"
RAMP = "Ramp"
FOREST = "Forest"
DIRT = "Dirt"

# Crossing x-ranges (inclusive)
WEST_FORD = range(14, 19)   # x=14..18
BRIDGE = range(38, 43)       # x=38..42
EAST_FORD = range(60, 65)    # x=60..64

def generate_tile(x: int, y: int) -> tuple[str, int]:
    """Return (terrain_type, elevation) for tile at (x, y)."""
    # Rock walls (top and bottom)
    if y <= 5 or y >= 42:
        return (ROCK, 2)

    # Ramp zones
    if y in (6, 7) or y in (40, 41):
        return (RAMP, 1)

    # River zone (y=20-27)
    if 20 <= y <= 27:
        is_crossing = x in WEST_FORD or x in BRIDGE or x in EAST_FORD

        if y == 20 or y == 27:
            # Riverbanks
            if is_crossing:
                if x in BRIDGE:
                    return (ROAD, 0)
                else:
                    return (SHALLOWS, 0)
            return (DIRT, 0)

        # River interior (y=21-26)
        if is_crossing:
            if x in BRIDGE:
                return (ROAD, 0)
            else:
                return (SHALLOWS, 0)
        return (WATER, 0)

    # Ramp rows bridging plateau (elev 1) to riverbank (elev 0)
    if y == 19 or y == 28:
        return (RAMP, 1)

    # Plateau zones (y=8-19 north, y=28-39 south)
    # Forest along river edges for tactical cover
    if y == 18 or y == 29:
        # Forest belt near river, but gaps at crossing approaches
        is_approach = x in WEST_FORD or x in BRIDGE or x in EAST_FORD
        if not is_approach and x % 3 != 0:
            return (FOREST, 1)

    # Roads leading to bridge
    if x in (39, 40, 41) and (8 <= y <= 19 or 28 <= y <= 39):
        return (ROAD, 1)

    # Some scattered forest for cover on plateaus
    if (8 <= y <= 17 or 30 <= y <= 39):
        # Sparse trees in tactical positions
        if (x + y) % 11 == 0 and x > 3 and x < 76:
            return (FOREST, 1)

    return (GRASS, 1)


def main():
    tiles = []
    elevations = []
    for y in range(HEIGHT):
        for x in range(WIDTH):
            terrain, elev = generate_tile(x, y)
            tiles.append(terrain)
            elevations.append(elev)

    # Build RON string
    tiles_str = ", ".join(tiles)
    elev_str = ", ".join(str(e) for e in elevations)

    ron = f'''(
    id: "demo_canyon",
    name: "Canyon Battle",
    act: 0,
    mission_index: 0,
    map: Inline(
        width: {WIDTH},
        height: {HEIGHT},
        tiles: [{tiles_str}],
        elevation: [{elev_str}],
    ),
    player_setup: (
        heroes: [],
        units: [],
        buildings: [
            (kind: TheBox, position: (x: 10, y: 10), player_id: 0, pre_built: true),
            (kind: TheBurrow, position: (x: 70, y: 38), player_id: 1, pre_built: true),
        ],
        starting_food: 0,
        starting_gpu: 0,
        starting_nfts: 0,
    ),
    enemy_waves: [
        (
            wave_id: "p0_army",
            trigger: Immediate,
            units: [
                (kind: Chonk, position: (x: 8, y: 12), player_id: 0),
                (kind: Chonk, position: (x: 12, y: 12), player_id: 0),
                (kind: Nuisance, position: (x: 7, y: 13), player_id: 0),
                (kind: Nuisance, position: (x: 9, y: 13), player_id: 0),
                (kind: Nuisance, position: (x: 11, y: 13), player_id: 0),
                (kind: Nuisance, position: (x: 13, y: 13), player_id: 0),
                (kind: Hisser, position: (x: 8, y: 14), player_id: 0),
                (kind: Hisser, position: (x: 10, y: 14), player_id: 0),
                (kind: Hisser, position: (x: 12, y: 14), player_id: 0),
                (kind: Yowler, position: (x: 10, y: 15), player_id: 0),
                (kind: Mouser, position: (x: 6, y: 11), player_id: 0),
                (kind: FlyingFox, position: (x: 14, y: 11), player_id: 0),
            ],
            ai_behavior: AttackMove((x: 70, y: 38)),
        ),
        (
            wave_id: "p1_army",
            trigger: Immediate,
            units: [
                (kind: Quillback, position: (x: 68, y: 36), player_id: 1),
                (kind: Quillback, position: (x: 72, y: 36), player_id: 1),
                (kind: Swarmer, position: (x: 67, y: 35), player_id: 1),
                (kind: Swarmer, position: (x: 69, y: 35), player_id: 1),
                (kind: Swarmer, position: (x: 71, y: 35), player_id: 1),
                (kind: Swarmer, position: (x: 73, y: 35), player_id: 1),
                (kind: Swarmer, position: (x: 68, y: 34), player_id: 1),
                (kind: Swarmer, position: (x: 72, y: 34), player_id: 1),
                (kind: Shrieker, position: (x: 69, y: 33), player_id: 1),
                (kind: Shrieker, position: (x: 71, y: 33), player_id: 1),
                (kind: Sparks, position: (x: 67, y: 34), player_id: 1),
                (kind: Sparks, position: (x: 73, y: 34), player_id: 1),
                (kind: Whiskerwitch, position: (x: 70, y: 32), player_id: 1),
                (kind: Gnawer, position: (x: 70, y: 37), player_id: 1),
            ],
            ai_behavior: AttackMove((x: 10, y: 10)),
        ),
    ],
    objectives: [
        (
            id: "eliminate_all",
            description: "Destroy all enemy forces",
            primary: true,
            condition: EliminateAll,
        ),
    ],
    triggers: [],
    dialogue: [],
    briefing_text: "A canyon divided by a raging river. Two armies face off across the chasm. Only one will survive.",
    debrief_text: "The canyon falls silent.",
    next_mission: None,
)
'''

    out_path = "assets/campaign/demo_canyon.ron"
    with open(out_path, "w") as f:
        f.write(ron)
    print(f"Written {out_path} ({len(tiles)} tiles, {WIDTH}x{HEIGHT})")

    # Verify tile count
    assert len(tiles) == WIDTH * HEIGHT, f"Expected {WIDTH*HEIGHT} tiles, got {len(tiles)}"
    assert len(elevations) == WIDTH * HEIGHT


if __name__ == "__main__":
    main()
