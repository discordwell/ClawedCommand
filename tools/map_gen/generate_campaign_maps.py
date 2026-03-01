#!/usr/bin/env python3
"""Generate inline tile/elevation arrays for campaign missions.

Outputs RON-compatible terrain and elevation arrays for:
  - Prologue: "The Server in the River" (48x48)
  - Act 1 Mission 1: "Pond Defense" (48x48)

Usage:
    python tools/map_gen/generate_campaign_maps.py [--validate] [--print]
"""

import argparse
import sys
from typing import Optional

# Terrain enum values matching cc_core::terrain::TerrainType
GRASS = "Grass"
DIRT = "Dirt"
SAND = "Sand"
FOREST = "Forest"
WATER = "Water"
SHALLOWS = "Shallows"
ROCK = "Rock"
RAMP = "Ramp"
ROAD = "Road"
TECH_RUINS = "TechRuins"

# Impassable terrain types (Rock and Water block non-Croak units)
IMPASSABLE = {ROCK, WATER}


class MapGrid:
    """48x48 tile + elevation grid with helper methods."""

    def __init__(self, width: int = 48, height: int = 48):
        self.width = width
        self.height = height
        self.tiles = [[GRASS] * width for _ in range(height)]
        self.elevation = [[0] * width for _ in range(height)]

    def fill_rect(self, x0: int, y0: int, x1: int, y1: int,
                  terrain: str, elev: Optional[int] = None):
        """Fill rectangle [x0,x1] x [y0,y1] inclusive."""
        for y in range(max(0, y0), min(self.height, y1 + 1)):
            for x in range(max(0, x0), min(self.width, x1 + 1)):
                self.tiles[y][x] = terrain
                if elev is not None:
                    self.elevation[y][x] = elev

    def fill_circle(self, cx: int, cy: int, radius: int,
                    terrain: str, elev: Optional[int] = None):
        """Fill circle centered at (cx, cy) with given radius."""
        r2 = radius * radius
        for y in range(max(0, cy - radius), min(self.height, cy + radius + 1)):
            for x in range(max(0, cx - radius), min(self.width, cx + radius + 1)):
                if (x - cx) ** 2 + (y - cy) ** 2 <= r2:
                    self.tiles[y][x] = terrain
                    if elev is not None:
                        self.elevation[y][x] = elev

    def draw_line(self, x0: int, y0: int, x1: int, y1: int,
                  terrain: str, width: int = 1, elev: Optional[int] = None):
        """Draw a line with given width using Bresenham's algorithm."""
        dx = abs(x1 - x0)
        dy = abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx - dy
        half = width // 2

        while True:
            for wy in range(-half, half + 1):
                for wx in range(-half, half + 1):
                    px, py = x0 + wx, y0 + wy
                    if 0 <= px < self.width and 0 <= py < self.height:
                        self.tiles[py][px] = terrain
                        if elev is not None:
                            self.elevation[py][px] = elev
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 > -dy:
                err -= dy
                x0 += sx
            if e2 < dx:
                err += dx
                y0 += sy

    def scatter(self, x0: int, y0: int, x1: int, y1: int,
                terrain: str, density: float = 0.3,
                elev: Optional[int] = None, seed: int = 42):
        """Scatter terrain in rectangle with deterministic pseudo-random."""
        state = seed
        for y in range(max(0, y0), min(self.height, y1 + 1)):
            for x in range(max(0, x0), min(self.width, x1 + 1)):
                # Simple LCG
                state = (state * 1103515245 + 12345) & 0x7FFFFFFF
                if (state / 0x7FFFFFFF) < density:
                    self.tiles[y][x] = terrain
                    if elev is not None:
                        self.elevation[y][x] = elev

    def border(self, thickness: int, terrain: str, elev: Optional[int] = None):
        """Fill border of given thickness."""
        for y in range(self.height):
            for x in range(self.width):
                if (x < thickness or x >= self.width - thickness or
                        y < thickness or y >= self.height - thickness):
                    self.tiles[y][x] = terrain
                    if elev is not None:
                        self.elevation[y][x] = elev

    def set(self, x: int, y: int, terrain: str, elev: Optional[int] = None):
        """Set single tile."""
        if 0 <= x < self.width and 0 <= y < self.height:
            self.tiles[y][x] = terrain
            if elev is not None:
                self.elevation[y][x] = elev

    def get(self, x: int, y: int) -> tuple:
        """Get (terrain, elevation) at position."""
        return self.tiles[y][x], self.elevation[y][x]

    def is_passable(self, x: int, y: int) -> bool:
        """Check if tile is passable (not Rock or Water)."""
        if not (0 <= x < self.width and 0 <= y < self.height):
            return False
        return self.tiles[y][x] not in IMPASSABLE

    def validate_positions(self, positions: list, label: str) -> list:
        """Validate that all (x, y) positions are on passable terrain."""
        errors = []
        for x, y in positions:
            if not (0 <= x < self.width and 0 <= y < self.height):
                errors.append(f"{label}: ({x},{y}) is out of bounds")
            elif not self.is_passable(x, y):
                t, e = self.get(x, y)
                errors.append(f"{label}: ({x},{y}) is on impassable {t} (elev {e})")
        return errors

    def to_ron_tiles(self) -> str:
        """Generate RON array of terrain types (row-major)."""
        tiles = []
        for y in range(self.height):
            for x in range(self.width):
                tiles.append(self.tiles[y][x])
        return "[" + ", ".join(tiles) + "]"

    def to_ron_elevation(self) -> str:
        """Generate RON array of elevation values (row-major)."""
        elevs = []
        for y in range(self.height):
            for x in range(self.width):
                elevs.append(str(self.elevation[y][x]))
        return "[" + ", ".join(elevs) + "]"

    def stats(self) -> dict:
        """Return terrain type counts."""
        counts = {}
        for y in range(self.height):
            for x in range(self.width):
                t = self.tiles[y][x]
                counts[t] = counts.get(t, 0) + 1
        return counts


# ---------------------------------------------------------------------------
# Prologue Map: "The Server in the River" (48x48)
# ---------------------------------------------------------------------------

def generate_prologue_map() -> MapGrid:
    """Generate the prologue mission map.

    Layout:
    - Rock border (rows 0-1, 46-47, cols 0-1, 46-47), elevation 2
    - West bank (cols 2-17): Grass, elevation 0. Kelpie starts at (6, 24)
    - Millstone River (cols 18-22): Water core, Shallows banks
    - East bank (cols 23-45): Elevation 1 plateau
    - Ford crossing at rows 22-26, narrow ford at rows 10-12
    - TechRuins cluster at (30-32, 22-24)
    - Southern approach for flanking wave
    """
    m = MapGrid(48, 48)

    # 1. Rock border (2 tiles thick), elevation 2
    m.border(2, ROCK, elev=2)

    # 2. West bank — Grass, elevation 0 (cols 2-17)
    m.fill_rect(2, 2, 17, 45, GRASS, elev=0)

    # Scattered forest patches on west bank
    m.scatter(3, 5, 10, 15, FOREST, density=0.25, seed=100)
    m.scatter(4, 30, 12, 38, FOREST, density=0.2, seed=200)
    # Small grove near start
    m.fill_rect(8, 20, 10, 22, FOREST, elev=0)

    # 3. Millstone River (cols 18-22)
    # Water core (cols 19-21)
    m.fill_rect(19, 2, 21, 45, WATER, elev=0)
    # Shallows banks (cols 18, 22)
    m.fill_rect(18, 2, 18, 45, SHALLOWS, elev=0)
    m.fill_rect(22, 2, 22, 45, SHALLOWS, elev=0)

    # Ford crossing at rows 22-26 (all Shallows — main crossing)
    m.fill_rect(18, 22, 22, 26, SHALLOWS, elev=0)

    # Narrow ford at rows 10-12
    m.fill_rect(19, 10, 21, 12, SHALLOWS, elev=0)

    # 4. East bank (cols 23-45) — elevation 1 plateau
    m.fill_rect(23, 2, 45, 45, GRASS, elev=1)

    # Ramp strip at col 23 (elevation 0→1 transition)
    m.fill_rect(23, 2, 23, 45, RAMP, elev=0)

    # 5. Forest belt on east bank (cols 26-28)
    m.fill_rect(26, 4, 28, 20, FOREST, elev=1)
    m.fill_rect(26, 28, 28, 43, FOREST, elev=1)

    # 6. TechRuins cluster at (30-32, 22-24)
    m.fill_rect(30, 22, 32, 24, TECH_RUINS, elev=1)

    # 7. Road east-west at row 24 (east bank side)
    m.fill_rect(24, 24, 45, 24, ROAD, elev=1)
    # Road through ramp
    m.set(23, 24, ROAD, elev=0)

    # 8. Southern approach (rows 38-42, cols 12-17) — dirt path for flanking
    # Stay west of river (cols < 18) to avoid breaching water
    m.fill_rect(12, 38, 17, 42, DIRT, elev=0)

    # Ensure Kelpie start position is clear grass
    m.set(6, 24, GRASS, elev=0)

    # Ensure river integrity — re-apply water/shallows after dirt path
    m.fill_rect(19, 2, 21, 45, WATER, elev=0)
    m.fill_rect(18, 2, 18, 45, SHALLOWS, elev=0)
    m.fill_rect(22, 2, 22, 45, SHALLOWS, elev=0)
    # Re-apply ford crossings
    m.fill_rect(18, 22, 22, 26, SHALLOWS, elev=0)
    m.fill_rect(19, 10, 21, 12, SHALLOWS, elev=0)

    return m


def validate_prologue_positions(m: MapGrid) -> list:
    """Validate all unit/trigger positions for prologue."""
    errors = []

    # Player positions
    errors += m.validate_positions([(6, 24)], "Kelpie start")

    # Initial ferals (east bank, near river)
    errors += m.validate_positions([
        (30, 21), (32, 22), (31, 24), (33, 23),
    ], "initial_ferals")

    # Flanking wave (southern approach — west bank side of river)
    errors += m.validate_positions([
        (14, 40), (15, 41), (14, 42), (16, 40),
    ], "flanking_wave")

    # Pack Leader
    errors += m.validate_positions([(36, 24)], "pack_leader")

    # Trigger positions
    errors += m.validate_positions([(12, 24)], "water_movement trigger center")
    errors += m.validate_positions([(24, 24)], "ruins_reached trigger center")

    return errors


# ---------------------------------------------------------------------------
# Pond Defense Map (48x48)
# ---------------------------------------------------------------------------

def generate_pond_defense_map() -> MapGrid:
    """Generate the pond defense mission map.

    Layout:
    - Rock border, elevation 2
    - Player base (cols 2-15, rows 10-38): Grass, elevation 1
    - North Pond (cols 8-13, rows 8-14): Water center, Shallows ring
    - Lily Pond (cols 12-18, rows 21-27): Water center, Shallows ring
    - South Pond (cols 8-13, rows 33-39): Water center, Shallows ring
    - Midfield forest corridors with attack gaps
    - Enemy approach area (cols 33-45)
    """
    m = MapGrid(48, 48)

    # 1. Rock border (2 tiles thick), elevation 2
    m.border(2, ROCK, elev=2)

    # 2. Base fill — everything inside border is grass, elevation 0
    m.fill_rect(2, 2, 45, 45, GRASS, elev=0)

    # 3. Player base area — elevation 1
    m.fill_rect(2, 8, 15, 40, GRASS, elev=1)
    # Ramp at cols 15-16 for transition
    m.fill_rect(16, 8, 16, 40, RAMP, elev=0)

    # 4. North Pond (center ~10, 11)
    # Water core 3x3
    m.fill_rect(9, 10, 11, 12, WATER, elev=0)
    # Shallows ring
    m.fill_rect(8, 9, 12, 13, SHALLOWS, elev=0)
    # Re-place water core on top
    m.fill_rect(9, 10, 11, 12, WATER, elev=0)

    # 5. Lily Pond (center ~15, 24)
    # Water core 4x3
    m.fill_rect(14, 23, 17, 25, WATER, elev=0)
    # Shallows ring
    m.fill_rect(13, 22, 18, 26, SHALLOWS, elev=0)
    # Re-place water core
    m.fill_rect(14, 23, 17, 25, WATER, elev=0)

    # 6. South Pond (center ~10, 36)
    # Water core 3x3
    m.fill_rect(9, 35, 11, 37, WATER, elev=0)
    # Shallows ring
    m.fill_rect(8, 34, 12, 38, SHALLOWS, elev=0)
    # Re-place water core
    m.fill_rect(9, 35, 11, 37, WATER, elev=0)

    # 7. Road grid connecting ponds (player base side)
    # Stop roads before pond shallows rings to avoid overwriting water
    m.draw_line(5, 13, 5, 33, ROAD, width=1, elev=1)  # North-south spine (between ponds)
    m.draw_line(5, 13, 7, 13, ROAD, width=1, elev=1)   # To north pond edge
    m.draw_line(5, 24, 12, 24, ROAD, width=1, elev=1)  # To lily pond edge
    m.draw_line(5, 33, 7, 33, ROAD, width=1, elev=1)   # To south pond edge

    # 8. Midfield forest corridors (cols 18-30)
    # Dense forest with gaps for attack lanes
    m.fill_rect(19, 4, 30, 10, FOREST, elev=0)    # North forest
    m.fill_rect(19, 15, 30, 22, FOREST, elev=0)   # Mid-north forest
    m.fill_rect(19, 28, 30, 33, FOREST, elev=0)   # Mid-south forest
    m.fill_rect(19, 38, 30, 44, FOREST, elev=0)   # South forest

    # Attack gaps (open grass lanes)
    m.fill_rect(19, 11, 30, 14, GRASS, elev=0)    # North gap (row ~12)
    m.fill_rect(19, 23, 30, 27, GRASS, elev=0)    # Center gap (row ~24)
    m.fill_rect(19, 34, 30, 37, GRASS, elev=0)    # South gap (row ~35)

    # 9. TechRuins at midfield
    m.fill_rect(24, 23, 26, 25, TECH_RUINS, elev=0)

    # 10. Enemy approach area (cols 33-45) — grass/dirt staging
    m.fill_rect(33, 2, 45, 45, GRASS, elev=0)
    m.scatter(35, 5, 43, 43, DIRT, density=0.15, seed=300)

    # Dirt paths from enemy staging to gaps
    m.draw_line(35, 12, 44, 10, DIRT, width=2, elev=0)
    m.draw_line(35, 24, 44, 22, DIRT, width=2, elev=0)
    m.draw_line(35, 35, 44, 32, DIRT, width=2, elev=0)

    # 11. Transition zone (cols 31-32) — light forest/grass mix
    m.scatter(31, 3, 32, 44, FOREST, density=0.3, seed=400)

    return m


def validate_pond_defense_positions(m: MapGrid) -> list:
    """Validate all unit/trigger positions for pond defense."""
    errors = []

    # Player heroes
    errors += m.validate_positions([(5, 24), (7, 24)], "heroes (Kelpie, Patches)")

    # Player units
    errors += m.validate_positions([
        (4, 22), (6, 22), (4, 26), (6, 26),  # Nuisances
        (3, 24), (8, 24),                      # Hissers
    ], "player_units")

    # Wave 1: North raiders
    errors += m.validate_positions([
        (35, 10), (36, 11), (37, 10), (38, 11),
    ], "wave1_north")

    # Wave 2: East dual assault
    errors += m.validate_positions([
        (38, 20), (39, 21), (40, 20), (41, 21),
        (39, 19), (40, 22),
    ], "wave2_dual")

    # Wave 3: South push
    errors += m.validate_positions([
        (40, 30), (41, 31), (42, 30), (43, 31),
        (41, 29), (42, 32), (43, 29), (44, 31),
        (42, 28),
    ], "wave3_push")

    # AttackMove targets (near ponds but on passable terrain)
    errors += m.validate_positions([
        (10, 14), (13, 21), (10, 33),
    ], "attack_move_targets")

    return errors


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def print_map_stats(name: str, m: MapGrid):
    """Print terrain distribution for a map."""
    stats = m.stats()
    total = m.width * m.height
    print(f"\n=== {name} ({m.width}x{m.height} = {total} tiles) ===")
    for terrain, count in sorted(stats.items(), key=lambda x: -x[1]):
        pct = 100 * count / total
        print(f"  {terrain:12s}: {count:5d} ({pct:5.1f}%)")


def main():
    parser = argparse.ArgumentParser(description="Generate campaign maps")
    parser.add_argument("--validate", action="store_true",
                        help="Validate positions and exit")
    parser.add_argument("--print", action="store_true",
                        help="Print RON arrays to stdout")
    parser.add_argument("--stats", action="store_true",
                        help="Print terrain statistics")
    args = parser.parse_args()

    # Generate maps
    prologue = generate_prologue_map()
    pond_defense = generate_pond_defense_map()

    # Validate
    all_errors = []
    all_errors += validate_prologue_positions(prologue)
    all_errors += validate_pond_defense_positions(pond_defense)

    if all_errors:
        print("VALIDATION ERRORS:", file=sys.stderr)
        for err in all_errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)
    else:
        print("All positions validated OK.")

    if args.stats:
        print_map_stats("Prologue", prologue)
        print_map_stats("Pond Defense", pond_defense)

    if args.print:
        print("\n// === PROLOGUE TILES ===")
        print(prologue.to_ron_tiles())
        print("\n// === PROLOGUE ELEVATION ===")
        print(prologue.to_ron_elevation())
        print("\n// === POND DEFENSE TILES ===")
        print(pond_defense.to_ron_tiles())
        print("\n// === POND DEFENSE ELEVATION ===")
        print(pond_defense.to_ron_elevation())

    if args.validate:
        sys.exit(0)

    return prologue, pond_defense


if __name__ == "__main__":
    main()
