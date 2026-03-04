#!/usr/bin/env python3
"""Generate hand-crafted inline maps for Act 1 campaign missions (M2-M4).

Usage:
    python gen_campaign_maps.py --map [m2|m3|m4|all] --preview --patch

Flags:
    --map NAME   Which map(s) to generate (default: all)
    --preview    Print ASCII art preview of each map
    --patch      Replace Generated(...) blocks in RON files with Inline(...)
"""

import argparse
import math
import os
import random
import re
import sys

# ═══════════════════════════════════════════════════════════════════════════════
# MapBuilder
# ═══════════════════════════════════════════════════════════════════════════════

PASSABLE = {"Grass", "Dirt", "Sand", "Forest", "Shallows", "Ramp", "Road", "TechRuins"}
ALL_TERRAIN = {"Grass", "Dirt", "Sand", "Forest", "Water", "Shallows", "Rock", "Ramp", "Road", "TechRuins"}
MAX_ELEVATION = 2


class MapBuilder:
    """Grid-based campaign map builder with RON output."""

    def __init__(self, width, height, default_terrain="Grass", default_elevation=0):
        assert default_terrain in ALL_TERRAIN, f"Unknown terrain: {default_terrain}"
        assert 0 <= default_elevation <= MAX_ELEVATION
        self.width = width
        self.height = height
        self.tiles = [[default_terrain] * width for _ in range(height)]
        self.elev = [[default_elevation] * width for _ in range(height)]

    # ── Primitives ─────────────────────────────────────────────────────────

    def set_tile(self, x, y, terrain, elevation=None):
        if 0 <= x < self.width and 0 <= y < self.height:
            assert terrain in ALL_TERRAIN, f"Unknown terrain: {terrain}"
            self.tiles[y][x] = terrain
            if elevation is not None:
                assert 0 <= elevation <= MAX_ELEVATION, f"Elevation {elevation} > {MAX_ELEVATION}"
                self.elev[y][x] = elevation

    def fill_rect(self, x0, y0, x1, y1, terrain, elevation=None):
        for y in range(max(0, y0), min(self.height, y1 + 1)):
            for x in range(max(0, x0), min(self.width, x1 + 1)):
                self.set_tile(x, y, terrain, elevation)

    def hline(self, y, x0, x1, terrain, elevation=None):
        self.fill_rect(x0, y, x1, y, terrain, elevation)

    def vline(self, x, y0, y1, terrain, elevation=None):
        self.fill_rect(x, y0, x, y1, terrain, elevation)

    # ── High-level ─────────────────────────────────────────────────────────

    def border(self, thickness=2, terrain="Rock", elevation=2):
        """Paint impassable border around map edges."""
        t = thickness
        w, h = self.width, self.height
        self.fill_rect(0, 0, w - 1, t - 1, terrain, elevation)       # top
        self.fill_rect(0, h - t, w - 1, h - 1, terrain, elevation)   # bottom
        self.fill_rect(0, 0, t - 1, h - 1, terrain, elevation)       # left
        self.fill_rect(w - t, 0, w - 1, h - 1, terrain, elevation)   # right

    def road(self, waypoints, width=1, terrain="Road", elevation=None):
        """Draw a road along waypoints using Bresenham line segments."""
        for i in range(len(waypoints) - 1):
            x0, y0 = waypoints[i]
            x1, y1 = waypoints[i + 1]
            self._bresenham_thick(x0, y0, x1, y1, width, terrain, elevation)

    def _bresenham_thick(self, x0, y0, x1, y1, width, terrain, elevation):
        """Bresenham line with thickness."""
        dx = abs(x1 - x0)
        dy = abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx - dy
        half = width // 2
        cx, cy = x0, y0
        while True:
            for oy in range(-half, half + 1):
                for ox in range(-half, half + 1):
                    self.set_tile(cx + ox, cy + oy, terrain, elevation)
            if cx == x1 and cy == y1:
                break
            e2 = 2 * err
            if e2 > -dy:
                err -= dy
                cx += sx
            if e2 < dx:
                err += dx
                cy += sy

    def forest_patch(self, cx, cy, radius, density=0.7, seed=42):
        """Scatter Forest tiles in a circular patch with seeded RNG."""
        rng = random.Random(seed)
        for y in range(cy - radius, cy + radius + 1):
            for x in range(cx - radius, cx + radius + 1):
                if (x - cx) ** 2 + (y - cy) ** 2 <= radius ** 2:
                    if rng.random() < density:
                        self.set_tile(x, y, "Forest")

    def river(self, waypoints, width=3, bank_width=1):
        """Draw Water core with Shallows banks along waypoints."""
        # First draw banks (wider)
        total = width + 2 * bank_width
        for i in range(len(waypoints) - 1):
            x0, y0 = waypoints[i]
            x1, y1 = waypoints[i + 1]
            self._bresenham_thick(x0, y0, x1, y1, total, "Shallows", None)
        # Then draw water core
        for i in range(len(waypoints) - 1):
            x0, y0 = waypoints[i]
            x1, y1 = waypoints[i + 1]
            self._bresenham_thick(x0, y0, x1, y1, width, "Water", None)

    def tech_ruins(self, cx, cy, w=3, h=3, elevation=None):
        """Place a TechRuins cluster."""
        x0 = cx - w // 2
        y0 = cy - h // 2
        self.fill_rect(x0, y0, x0 + w - 1, y0 + h - 1, "TechRuins", elevation)

    def scatter(self, terrain, positions, elevation=None):
        """Set terrain at a list of (x, y) positions."""
        for x, y in positions:
            self.set_tile(x, y, terrain, elevation)

    def elevation_ridge(self, waypoints, width=2, elevation=1):
        """Set elevation along waypoints without changing terrain."""
        for i in range(len(waypoints) - 1):
            x0, y0 = waypoints[i]
            x1, y1 = waypoints[i + 1]
            self._bresenham_elev(x0, y0, x1, y1, width, elevation)

    def _bresenham_elev(self, x0, y0, x1, y1, width, elevation):
        """Bresenham line that sets only elevation."""
        dx = abs(x1 - x0)
        dy = abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx - dy
        half = width // 2
        cx, cy = x0, y0
        while True:
            for oy in range(-half, half + 1):
                for ox in range(-half, half + 1):
                    if 0 <= cx + ox < self.width and 0 <= cy + oy < self.height:
                        self.elev[cy + oy][cx + ox] = elevation
            if cx == x1 and cy == y1:
                break
            e2 = 2 * err
            if e2 > -dy:
                err -= dy
                cx += sx
            if e2 < dx:
                err += dx
                cy += sy

    def set_elevation_rect(self, x0, y0, x1, y1, elevation):
        """Set elevation for a rectangle without changing terrain."""
        for y in range(max(0, y0), min(self.height, y1 + 1)):
            for x in range(max(0, x0), min(self.width, x1 + 1)):
                self.elev[y][x] = elevation

    # ── Validation ─────────────────────────────────────────────────────────

    def validate_positions(self, positions):
        """Warn if any named positions are impassable. Returns True if all OK."""
        ok = True
        for label, (x, y) in positions.items():
            if not (0 <= x < self.width and 0 <= y < self.height):
                print(f"WARNING: {label} at ({x},{y}) is OUT OF BOUNDS", file=sys.stderr)
                ok = False
            elif self.tiles[y][x] not in PASSABLE:
                print(f"WARNING: {label} at ({x},{y}) is on {self.tiles[y][x]} (impassable!)",
                      file=sys.stderr)
                ok = False
        return ok

    # ── Output ─────────────────────────────────────────────────────────────

    def to_ron_inline(self):
        """Return RON Inline(...) block string matching existing format."""
        lines = []
        lines.append("    map: Inline(")
        lines.append(f"        width: {self.width},")
        lines.append(f"        height: {self.height},")

        # Tiles — one row per line
        lines.append("        tiles: [")
        for y in range(self.height):
            row = ", ".join(self.tiles[y])
            comma = "," if y < self.height - 1 else ""
            lines.append(f"            {row}{comma}")
        lines.append("        ],")

        # Elevation — one row per line
        lines.append("        elevation: [")
        for y in range(self.height):
            row = ", ".join(str(self.elev[y][x]) for x in range(self.width))
            comma = "," if y < self.height - 1 else ""
            lines.append(f"            {row}{comma}")
        lines.append("        ],")

        lines.append("    ),")
        return "\n".join(lines)

    def ascii_preview(self):
        """Return ASCII art map for debugging."""
        terrain_chars = {
            "Grass": ".",
            "Dirt": ",",
            "Sand": "~",
            "Forest": "T",
            "Water": "W",
            "Shallows": "w",
            "Rock": "#",
            "Ramp": "/",
            "Road": "=",
            "TechRuins": "X",
        }
        lines = []
        lines.append(f"  Map: {self.width}x{self.height}")
        # Column numbers (tens)
        header1 = "    "
        header2 = "    "
        for x in range(self.width):
            header1 += str(x // 10) if x % 5 == 0 else " "
            header2 += str(x % 10) if x % 5 == 0 else " "
        lines.append(header1)
        lines.append(header2)
        for y in range(self.height):
            row_label = f"{y:3d} "
            row_chars = ""
            for x in range(self.width):
                row_chars += terrain_chars.get(self.tiles[y][x], "?")
            lines.append(row_label + row_chars)
        return "\n".join(lines)

    def stats(self):
        """Return tile type counts."""
        counts = {}
        for y in range(self.height):
            for x in range(self.width):
                t = self.tiles[y][x]
                counts[t] = counts.get(t, 0) + 1
        return counts


# ═══════════════════════════════════════════════════════════════════════════════
# M2: Dead Drop — Darkhollow Woods (48x48)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m2():
    # ══════════════════════════════════════════════════════════════
    # M2 "Darkhollow Woods" — Stealth recon through 4 distinct zones
    #
    # Layout concept (48x48):
    #
    #   NW: Rocky Overlook         NE: The Staging Compound
    #   Elevated plateau (e1-2)    Cleared + fortified w/ TechRuins
    #   Obj 1: patrol here         Obj 3: main target, guards
    #   Ramp down to trail         Road access, cleared sightlines
    #         \                   /
    #          --- Forest Core ---
    #         Dense forest, narrow winding trails
    #         Central clearing = Obj 2
    #         /                   \
    #   SW: Player Start           SE: Sunken Marsh
    #   Grass clearing             Shallows/Water lowland
    #   Safe insertion point       Alternate hidden route south
    #
    # The rocky ridge runs diagonally NW→center, splitting the
    # overlook from the forest floor. The stream feeds into the
    # SE marsh. Trails weave between all zones.
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(48, 48, default_terrain="Forest", default_elevation=0)

    # ── Rock border ──
    m.border(2, "Rock", 2)

    # ════════════════════════════════════════════════════════════
    # ZONE 1: Rocky Overlook (NW quadrant, rows 2-18, cols 2-22)
    # Elevated plateau with grass top, rocky cliffs on south/east
    # ════════════════════════════════════════════════════════════
    # Cliff walls (impassable rock, elev 2) forming the south and east edges
    m.fill_rect(2, 16, 22, 18, "Rock", 2)    # South cliff face
    m.fill_rect(20, 2, 22, 18, "Rock", 2)    # East cliff face
    # Plateau surface (elev 1)
    m.fill_rect(2, 2, 19, 15, "Grass", 1)
    # Dirt areas on plateau
    m.fill_rect(4, 4, 10, 8, "Dirt", 1)       # Patrol camp area
    m.fill_rect(14, 6, 18, 10, "Dirt", 1)     # Eastern lookout
    # Scattered rocks on plateau for cover
    m.fill_rect(11, 4, 12, 5, "Rock", 2)
    m.fill_rect(7, 10, 8, 11, "Rock", 2)
    m.fill_rect(16, 12, 17, 13, "Rock", 2)
    # Forest patches on plateau edges (scrubby growth)
    m.forest_patch(4, 13, 3, 0.5, seed=200)
    m.forest_patch(15, 3, 2, 0.6, seed=201)
    # RAMP DOWN: south side of cliff (the only way up/down)
    m.fill_rect(12, 16, 15, 18, "Ramp", 1)
    # Patrol position (Obj 1): the dirt camp
    # Spawns at (20,10) and (22,10) — need to adjust to be ON the plateau
    # Move patrol to plateau interior: (8,6) and (10,6) area
    # BUT: existing RON has spawns at (20,10) and (22,10)...
    # Those are ON the cliff wall. Let me put a ramp/clearing there.
    # Actually, let me make a notch in the east cliff for the patrol to stand
    m.fill_rect(20, 8, 22, 12, "Dirt", 1)     # Notch in east cliff
    m.set_tile(19, 10, "Ramp", 1)             # Access from plateau

    # ════════════════════════════════════════════════════════════
    # ZONE 2: Forest Core (center, the dense dark heart)
    # ════════════════════════════════════════════════════════════
    # (Already Forest by default — this IS the forest)
    # Central clearing (Obj 2) — a natural bowl
    m.fill_rect(21, 19, 30, 26, "Grass", 0)
    m.fill_rect(23, 20, 28, 25, "Dirt", 0)
    # Small pond in the clearing (atmosphere)
    m.fill_rect(28, 22, 30, 24, "Shallows", 0)
    m.set_tile(29, 23, "Water", 0)
    # Fallen log / rocky rubble around clearing edges
    m.set_tile(21, 20, "Rock", 0)
    m.set_tile(21, 25, "Rock", 0)
    m.set_tile(30, 20, "Rock", 0)

    # ════════════════════════════════════════════════════════════
    # ZONE 3: Staging Compound (NE, rows 4-22, cols 30-45)
    # Cleared forest with TechRuins, road access, perimeter
    # ════════════════════════════════════════════════════════════
    # Cleared perimeter (the Clawed cut down trees around their base)
    m.fill_rect(30, 4, 45, 22, "Grass", 0)
    # Dirt compound interior
    m.fill_rect(32, 6, 43, 20, "Dirt", 0)
    # Perimeter log wall (Rock, elev 1) — partial walls with gaps
    m.fill_rect(31, 5, 44, 5, "Rock", 1)      # North wall
    m.fill_rect(31, 21, 44, 21, "Rock", 1)     # South wall
    m.fill_rect(31, 5, 31, 21, "Rock", 1)      # West wall
    m.fill_rect(44, 5, 44, 21, "Rock", 1)      # East wall
    # Gaps: west gate, south gate
    m.fill_rect(31, 12, 31, 15, "Dirt", 0)     # West gate
    m.fill_rect(36, 21, 39, 21, "Dirt", 0)     # South gate
    # TechRuins: equipment, crates, tunneler rigs
    m.tech_ruins(35, 9, 4, 3)                  # Main equipment
    m.tech_ruins(40, 13, 3, 4)                 # Tunneler rigs
    m.tech_ruins(34, 17, 3, 3)                 # Supply cache
    # Road into the compound from the south gate
    m.road([(37, 21), (37, 26), (34, 30)], width=2, terrain="Road")
    # Watchtower position (elevated rock)
    m.fill_rect(42, 7, 43, 8, "Rock", 2)
    m.set_tile(41, 8, "Ramp", 1)

    # ════════════════════════════════════════════════════════════
    # ZONE 4: Sunken Marsh (SE, rows 28-45, cols 28-45)
    # Low-lying area with water, shallows — alternate stealth route
    # ════════════════════════════════════════════════════════════
    # Transition: forest thins to grass before marsh
    m.fill_rect(28, 30, 45, 45, "Grass", 0)
    # Marsh water features
    m.fill_rect(32, 34, 40, 42, "Shallows", 0)
    m.fill_rect(34, 36, 38, 40, "Water", 0)
    m.fill_rect(30, 38, 32, 41, "Shallows", 0)
    m.fill_rect(40, 35, 42, 38, "Shallows", 0)
    # Dry islands in the marsh (Grass/Dirt)
    m.fill_rect(35, 37, 37, 39, "Grass", 0)
    m.fill_rect(36, 38, 36, 38, "Dirt", 0)
    # Reed beds (Forest tiles = tall reeds for cover)
    m.forest_patch(31, 33, 2, 0.7, seed=210)
    m.forest_patch(41, 40, 2, 0.7, seed=211)
    m.forest_patch(34, 43, 2, 0.6, seed=212)
    # Marsh is at low elevation conceptually (elev 0, same as forest)

    # ════════════════════════════════════════════════════════════
    # ZONE 5: Player Start (SW, rows 36-45, cols 2-14)
    # Safe clearing at the forest edge
    # ════════════════════════════════════════════════════════════
    m.fill_rect(2, 36, 14, 45, "Grass", 0)
    m.fill_rect(3, 37, 12, 44, "Dirt", 0)
    # A few trees in the clearing (not totally bare)
    m.set_tile(10, 38, "Forest", 0)
    m.set_tile(11, 40, "Forest", 0)
    m.set_tile(4, 44, "Forest", 0)

    # ════════════════════════════════════════════════════════════
    # TRAIL NETWORK: connects all zones
    # ════════════════════════════════════════════════════════════
    # Main trail: Start → up through forest → ramp → overlook
    m.road([(8, 38), (10, 34), (12, 28), (13, 22), (13, 18)],
           width=2, terrain="Dirt")
    # Trail from ramp base → central clearing
    m.road([(15, 18), (18, 19), (21, 20)], width=2, terrain="Dirt")
    # Trail: central clearing → staging compound (west gate)
    m.road([(28, 22), (30, 18), (31, 14)], width=2, terrain="Dirt")
    # Trail: central clearing south → southern patrol area
    m.road([(23, 25), (20, 28), (17, 30)], width=2, terrain="Dirt")
    # Southern patrol clearing (near patrol_south spawns)
    m.fill_rect(13, 28, 20, 33, "Grass", 0)
    m.fill_rect(14, 29, 19, 32, "Dirt", 0)
    # Trail: southern patrol → start (loop back)
    m.road([(15, 32), (12, 35), (10, 37)], width=2, terrain="Dirt")
    # Trail: southern patrol → marsh (alternate route)
    m.road([(19, 31), (24, 33), (28, 34)], width=2, terrain="Dirt")
    # Trail: marsh → staging compound road (connect the sneaky route)
    m.road([(34, 30), (34, 26), (34, 23), (31, 21)], width=1, terrain="Dirt")

    # ════════════════════════════════════════════════════════════
    # ELEVATION: ridgeline connecting overlook to center
    # ════════════════════════════════════════════════════════════
    m.elevation_ridge([(22, 16), (26, 18), (28, 20)], width=1, elevation=1)

    # ── Validate all spawn positions ──
    spawns = {
        "Kelpie": (3, 40),
        "Patches": (5, 40),
        "Mouser_1": (4, 41),
        "Mouser_2": (6, 41),
        "Mouser_3": (5, 42),
        "patrol_n_1": (20, 10),
        "patrol_n_2": (22, 10),
        "patrol_c_1": (25, 22),
        "patrol_c_2": (26, 23),
        "patrol_c_3": (27, 22),
        "patrol_s_1": (15, 30),
        "patrol_s_2": (17, 30),
        "staging_1": (38, 15),
        "staging_2": (39, 16),
        "staging_3": (40, 15),
        "staging_4": (38, 17),
        "staging_5": (40, 17),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# M3: Counter-raid — Darkhollow Staging Area (64x64)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m3():
    # ══════════════════════════════════════════════════════════════
    # M3 "Counter-raid" — Full assault on the Clawed staging area
    #
    # Layout concept (64x64):
    #
    # ┌──────────────────────────────────────────────────────────┐
    # │ North Ridge (Rock elev 2, impassable cliffs)             │
    # │  Ramp down at x=15        Ramp down at x=45             │
    # ├────────┬──────┬────────────┬──────────────┬──────────────┤
    # │        │      │            │              │              │
    # │ FOREST │ Open │  STREAM    │  NO MAN'S    │  COMPOUND    │
    # │ COVER  │ with │  3 water   │  LAND        │  Rock walls  │
    # │        │ road │  crossings │  Hills,rocks │  TechRuins   │
    # │ Player │ and  │  -bridge N │  scattered   │  Garrison    │
    # │ start  │ Dirt │  -main ford│  forest,     │  defends     │
    # │ here   │      │  -wade S   │  cover       │  here        │
    # │        │      │            │              │              │
    # │ Felix  │      │            │              │ BOX CANYON   │
    # │ arrives│      │            │              │ (flankers)   │
    # │ south  │      │            │              │              │
    # ├────────┴──────┴────────────┴──────────────┴──────────────┤
    # │ South Ridge (Rock elev 2, impassable cliffs)             │
    # └──────────────────────────────────────────────────────────┘
    #
    # Key: stream is a PARTIAL barrier (y=12 to y=52) with 3 crossings.
    # No Man's Land is the main battle space.
    # Compound has walls you must breach.
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(64, 64, default_terrain="Grass", default_elevation=0)

    # ── Rock border ──
    m.border(2, "Rock", 2)

    # ════════════════════════════════════════════════════════════
    # RIDGELINES: North and South (impassable cliffs with ramp access)
    # These constrain the map vertically and provide high ground
    # ════════════════════════════════════════════════════════════
    # North ridge
    m.fill_rect(2, 2, 61, 7, "Rock", 2)
    m.fill_rect(2, 8, 61, 9, "Rock", 1)
    # Ramp access points down from north ridge
    m.fill_rect(14, 8, 16, 9, "Ramp", 1)
    m.fill_rect(44, 8, 46, 9, "Ramp", 1)
    # Grass ledge on top for units to stand on
    m.fill_rect(10, 4, 20, 6, "Grass", 2)
    m.fill_rect(40, 4, 50, 6, "Grass", 2)

    # South ridge
    m.fill_rect(2, 55, 61, 58, "Rock", 1)
    m.fill_rect(2, 59, 61, 61, "Rock", 2)
    # Ramp access
    m.fill_rect(14, 55, 16, 56, "Ramp", 1)
    m.fill_rect(44, 55, 46, 56, "Ramp", 1)
    # Grass ledge
    m.fill_rect(10, 57, 20, 58, "Grass", 2)

    # ════════════════════════════════════════════════════════════
    # WESTERN FOREST (cols 2-18): Assembly area with cover
    # Organic shape — thick in center, thins at edges
    # ════════════════════════════════════════════════════════════
    m.fill_rect(2, 14, 16, 50, "Forest", 0)
    # Thin out at north and south (organic edges)
    m.forest_patch(10, 12, 5, 0.6, seed=300)
    m.forest_patch(8, 51, 4, 0.5, seed=301)
    # Thinning on east edge
    m.forest_patch(17, 20, 3, 0.4, seed=302)
    m.forest_patch(18, 30, 3, 0.5, seed=303)
    m.forest_patch(17, 40, 3, 0.4, seed=304)
    # Internal clearings (not just a solid block)
    m.fill_rect(4, 18, 8, 22, "Grass", 0)     # Northern gap
    m.fill_rect(10, 38, 14, 42, "Grass", 0)   # Southern gap

    # ── Player start: large clearing carved out of the forest ──
    m.fill_rect(2, 25, 14, 39, "Grass", 0)
    m.fill_rect(3, 26, 13, 38, "Dirt", 0)
    # Forward staging area (Grass between forest and open ground)
    m.fill_rect(14, 28, 18, 36, "Grass", 0)

    # ── Felix arrival zone (SW) ──
    m.fill_rect(2, 44, 10, 53, "Grass", 0)
    m.fill_rect(3, 45, 9, 52, "Dirt", 0)
    # Dirt path from Felix to main force through forest
    m.road([(6, 44), (6, 42), (8, 38)], width=2, terrain="Dirt")
    m.road([(8, 38), (10, 36)], width=2, terrain="Dirt")

    # ════════════════════════════════════════════════════════════
    # MAIN ROAD: east-west assault axis
    # ════════════════════════════════════════════════════════════
    m.road([(14, 32), (20, 32), (24, 32), (28, 31), (34, 30),
            (40, 30), (46, 30), (52, 30)], width=2, terrain="Road")
    # Dirt shoulders
    m.road([(14, 30), (20, 30), (24, 30), (28, 29), (34, 28),
            (40, 28), (46, 28), (52, 28)], width=1, terrain="Dirt")
    m.road([(14, 34), (20, 34), (24, 34), (28, 33), (34, 32),
            (40, 32), (46, 32), (52, 32)], width=1, terrain="Dirt")

    # ════════════════════════════════════════════════════════════
    # THE STREAM (x≈25, y=12 to y=52): partial barrier
    # 3 crossing points with different tradeoffs
    # ════════════════════════════════════════════════════════════
    # Main water channel
    m.river([(25, 12), (24, 20), (24, 28), (25, 36), (26, 44), (26, 52)],
            width=2, bank_width=1)

    # Crossing 1 — NORTH BRIDGE (Road, easy but exposed, y≈16)
    m.fill_rect(22, 15, 28, 17, "Road", 0)
    m.fill_rect(22, 14, 28, 14, "Dirt", 0)
    m.fill_rect(22, 18, 28, 18, "Dirt", 0)

    # Crossing 2 — MAIN FORD (Dirt, wide, at road level, y≈30-32)
    m.fill_rect(22, 29, 28, 34, "Shallows", 0)
    m.fill_rect(23, 30, 27, 33, "Dirt", 0)

    # Crossing 3 — SOUTHERN WADE (Shallows only, slow but hidden, y≈46)
    m.fill_rect(23, 44, 28, 48, "Shallows", 0)
    # A few stepping stone Dirt tiles
    m.set_tile(25, 45, "Dirt", 0)
    m.set_tile(26, 47, "Dirt", 0)

    # ════════════════════════════════════════════════════════════
    # NO MAN'S LAND (cols 28-44): the main battle space
    # Rolling terrain with cover options
    # ════════════════════════════════════════════════════════════
    # Hill positions (elevated Grass, good defensive positions)
    m.fill_rect(30, 14, 36, 20, "Grass", 1)
    m.fill_rect(31, 15, 35, 19, "Dirt", 1)
    m.fill_rect(32, 16, 34, 18, "Grass", 1)   # Top of hill

    m.fill_rect(36, 38, 42, 44, "Grass", 1)
    m.fill_rect(37, 39, 41, 43, "Dirt", 1)

    # Rocky outcrops (hard cover)
    m.fill_rect(32, 24, 34, 26, "Rock", 1)
    m.fill_rect(38, 20, 40, 22, "Rock", 1)
    m.fill_rect(34, 36, 36, 37, "Rock", 1)
    m.fill_rect(42, 26, 43, 28, "Rock", 1)

    # Forest patches (concealment)
    m.forest_patch(30, 38, 3, 0.6, seed=310)
    m.forest_patch(38, 14, 3, 0.5, seed=311)
    m.forest_patch(35, 46, 4, 0.4, seed=312)
    m.forest_patch(30, 50, 3, 0.5, seed=313)

    # Small TechRuins outpost (forward observation post)
    m.fill_rect(36, 28, 39, 31, "Dirt", 0)
    m.tech_ruins(37, 29, 2, 2)

    # ════════════════════════════════════════════════════════════
    # THE COMPOUND (cols 44-60): fortified staging area
    # Rock perimeter walls with gates, TechRuins interior
    # ════════════════════════════════════════════════════════════
    # Perimeter walls (Rock, elev 1)
    m.fill_rect(46, 22, 58, 22, "Rock", 1)    # North wall
    m.fill_rect(46, 38, 58, 38, "Rock", 1)    # South wall
    m.fill_rect(46, 22, 46, 38, "Rock", 1)    # West wall
    m.fill_rect(58, 22, 58, 38, "Rock", 1)    # East wall
    # Interior compound floor
    m.fill_rect(47, 23, 57, 37, "Dirt", 0)
    # GATES (gaps in walls)
    m.fill_rect(46, 28, 46, 34, "Dirt", 0)    # West gate (main entry)
    m.fill_rect(50, 22, 54, 22, "Dirt", 0)    # North gate
    m.fill_rect(50, 38, 54, 38, "Dirt", 0)    # South gate
    # Approach road to west gate
    m.road([(44, 30), (46, 30)], width=2, terrain="Dirt")
    # Interior structure
    m.tech_ruins(50, 26, 4, 3)                 # Main equipment block
    m.tech_ruins(55, 30, 3, 4)                 # Tunneler rigs
    m.tech_ruins(49, 34, 3, 3)                 # Supply dump
    # Open yards between buildings
    m.fill_rect(48, 28, 50, 30, "Grass", 0)
    m.fill_rect(53, 25, 56, 27, "Grass", 0)
    m.fill_rect(48, 35, 50, 37, "Grass", 0)
    # Watchtower (elevated position inside compound)
    m.fill_rect(56, 24, 57, 25, "Rock", 2)
    m.set_tile(55, 25, "Ramp", 1)

    # ════════════════════════════════════════════════════════════
    # BOX CANYON (SE, cols 52-60, rows 42-53)
    # Flanking force hides here, sealed by rock walls
    # ════════════════════════════════════════════════════════════
    m.fill_rect(50, 42, 61, 53, "Rock", 2)
    m.fill_rect(52, 44, 59, 51, "Dirt", 0)
    # Interior features
    m.fill_rect(54, 46, 57, 49, "Grass", 0)
    m.tech_ruins(55, 47, 2, 2)
    # Ramp entrance
    m.fill_rect(50, 46, 51, 48, "Ramp", 1)

    # ════════════════════════════════════════════════════════════
    # SOUTHERN AREA: between south ridge and compound
    # Not empty — has cover and flanking routes
    # ════════════════════════════════════════════════════════════
    m.forest_patch(18, 48, 4, 0.5, seed=320)
    m.forest_patch(32, 52, 3, 0.5, seed=321)
    m.fill_rect(40, 48, 44, 52, "Dirt", 0)
    m.fill_rect(41, 49, 43, 51, "Grass", 0)
    # Rocky cover
    m.fill_rect(22, 52, 24, 53, "Rock", 1)

    # ── Validate all spawn positions ──
    spawns = {
        "Kelpie": (5, 32),
        "Patches": (7, 32),
        "Felix": (5, 50),
        "Nuisance_1": (4, 30),
        "Nuisance_2": (6, 30),
        "Nuisance_3": (4, 34),
        "Nuisance_4": (6, 34),
        "Nuisance_5": (3, 32),
        "Nuisance_6": (8, 32),
        "Hisser_1": (3, 28),
        "Hisser_2": (8, 28),
        "Hisser_3": (5, 26),
        "Chonk_1": (4, 31),
        "Chonk_2": (6, 33),
        "garrison_1": (50, 28),
        "garrison_2": (51, 29),
        "garrison_3": (52, 28),
        "garrison_4": (53, 29),
        "garrison_5": (50, 32),
        "garrison_6": (51, 33),
        "garrison_7": (52, 32),
        "garrison_8": (53, 33),
        "garrison_9": (54, 30),
        "garrison_10": (54, 32),
        "flank_1": (55, 45),
        "flank_2": (56, 46),
        "flank_3": (57, 45),
        "flank_4": (58, 46),
        "flank_5": (55, 47),
        "flank_6": (56, 48),
        "flank_7": (57, 47),
        "flank_8": (58, 48),
        "flank_9": (54, 44),
        "flank_10": (59, 44),
        "flank_11": (54, 49),
        "flank_12": (59, 49),
        "reinf_1": (3, 48),
        "reinf_2": (5, 48),
        "reinf_3": (7, 48),
        "reinf_4": (4, 50),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# M4: The Envoy — The Borderlands (64x48)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m4():
    m = MapBuilder(64, 48, default_terrain="Grass", default_elevation=0)

    # 1. Rock border
    m.border(2, "Rock", 2)

    # 2. Northern rocky highlands (Rock rows with elev 1-2, ramp access)
    m.fill_rect(2, 2, 61, 8, "Rock", 2)
    m.fill_rect(2, 9, 61, 11, "Rock", 1)
    # Ramp access points into the highlands
    m.fill_rect(18, 9, 20, 11, "Ramp", 1)
    m.fill_rect(38, 9, 40, 11, "Ramp", 1)

    # 3. Southern rocky highlands
    m.fill_rect(2, 37, 61, 43, "Rock", 1)
    m.fill_rect(2, 44, 61, 45, "Rock", 2)
    # Ramp access points
    m.fill_rect(18, 37, 20, 39, "Ramp", 1)
    m.fill_rect(38, 37, 40, 39, "Ramp", 1)

    # 4. Main road west→east through center
    m.road([(4, 24), (15, 24), (25, 23), (35, 22), (45, 23), (52, 23), (58, 22)],
           width=1, terrain="Road")
    # Dirt shoulders
    m.road([(4, 23), (15, 23), (25, 22), (35, 21), (45, 22), (52, 22), (58, 21)],
           width=1, terrain="Dirt")
    m.road([(4, 25), (15, 25), (25, 24), (35, 23), (45, 24), (52, 24), (58, 23)],
           width=1, terrain="Dirt")

    # 5. Player start area (Grass/Dirt clearing)
    m.fill_rect(2, 20, 10, 28, "Grass", 0)
    m.fill_rect(3, 21, 9, 27, "Dirt", 0)

    # 6. Rolling hill elevation (scattered elev 1 patches)
    m.set_elevation_rect(14, 14, 20, 18, 1)
    m.set_elevation_rect(28, 26, 34, 30, 1)
    m.set_elevation_rect(42, 14, 48, 18, 1)
    m.set_elevation_rect(16, 28, 22, 32, 1)

    # 7. Forest patches
    m.forest_patch(12, 16, 4, 0.6, seed=401)
    m.forest_patch(22, 28, 4, 0.5, seed=402)
    m.forest_patch(30, 16, 4, 0.5, seed=403)
    m.forest_patch(40, 30, 4, 0.4, seed=404)
    m.forest_patch(48, 16, 3, 0.5, seed=405)
    m.forest_patch(18, 32, 3, 0.5, seed=406)
    m.forest_patch(36, 28, 3, 0.4, seed=407)

    # 8. TechRuins at each ambush point with cleared Grass surrounds
    # Ambush 1 (~25, 20)
    m.fill_rect(23, 18, 29, 24, "Grass", 0)
    m.tech_ruins(26, 20, 3, 3)
    # Ambush 2 (~35, 18)
    m.fill_rect(33, 16, 39, 22, "Grass", 0)
    m.tech_ruins(36, 18, 3, 3)
    # Ambush 3 (~45, 22)
    m.fill_rect(43, 20, 49, 25, "Grass", 0)
    m.tech_ruins(46, 22, 3, 3)

    # 9. Seekers border wall (Rock column, narrow Grass pass)
    m.fill_rect(56, 12, 58, 18, "Rock", 2)   # North wall section
    m.fill_rect(56, 26, 58, 35, "Rock", 2)   # South wall section
    # Narrow pass at y=19-25 (kept Grass, already default)
    m.fill_rect(56, 19, 58, 25, "Grass", 0)

    # 10. Border marker (Road tiles at objective point)
    m.fill_rect(57, 21, 59, 23, "Road", 0)

    # 11. Validate all spawn positions
    spawns = {
        # Player
        "Kelpie": (5, 24),
        "Patches": (7, 24),
        "Nuisance_1": (4, 22),
        "Nuisance_2": (6, 22),
        "Nuisance_3": (4, 26),
        "Nuisance_4": (6, 26),
        "Hisser_1": (3, 24),
        "Hisser_2": (8, 24),
        # Ambush 1
        "amb1_1": (25, 20),
        "amb1_2": (26, 21),
        "amb1_3": (27, 20),
        "amb1_4": (25, 22),
        "amb1_5": (27, 22),
        # Ambush 2
        "amb2_1": (35, 18),
        "amb2_2": (36, 19),
        "amb2_3": (37, 18),
        "amb2_4": (35, 20),
        # Ambush 3
        "amb3_1": (45, 22),
        "amb3_2": (46, 23),
        "amb3_3": (47, 22),
        # Border fight
        "border_1": (52, 20),
        "border_2": (53, 21),
        "border_3": (54, 20),
        "border_4": (52, 24),
        "border_5": (53, 25),
        "border_6": (54, 24),
        # Objective
        "objective": (58, 22),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# RON Patching
# ═══════════════════════════════════════════════════════════════════════════════

def patch_ron_file(ron_path, inline_block):
    """Replace a Generated(...) or existing Inline(...) block with a new Inline(...) block."""
    with open(ron_path, "r") as f:
        content = f.read()

    # Try Generated block first
    pattern = r'    map: Generated\(\s*\n\s*seed: \d+,\s*\n\s*width: \d+,\s*\n\s*height: \d+,\s*\n\s*\),'
    match = re.search(pattern, content)
    if match:
        new_content = content[:match.start()] + inline_block + content[match.end():]
        with open(ron_path, "w") as f:
            f.write(new_content)
        print(f"Patched (Generated→Inline): {ron_path}")
        return True

    # Try existing Inline block (for re-patching)
    # Find "    map: Inline(" and match until the closing "    ),"
    start_pattern = r'    map: Inline\('
    start_match = re.search(start_pattern, content)
    if start_match:
        # Find the matching closing ")," — count parentheses
        depth = 1
        pos = start_match.end()
        while pos < len(content) and depth > 0:
            if content[pos] == '(':
                depth += 1
            elif content[pos] == ')':
                depth -= 1
            pos += 1
        # pos now points just after the closing ')'
        # Skip the trailing comma
        if pos < len(content) and content[pos] == ',':
            pos += 1
        new_content = content[:start_match.start()] + inline_block + content[pos:]
        with open(ron_path, "w") as f:
            f.write(new_content)
        print(f"Patched (Inline→Inline): {ron_path}")
        return True

    print(f"ERROR: Could not find map block in {ron_path}", file=sys.stderr)
    return False


# ═══════════════════════════════════════════════════════════════════════════════
# CLI
# ═══════════════════════════════════════════════════════════════════════════════

MAP_BUILDERS = {
    "m2": ("act1_m2_dead_drop.ron", build_m2),
    "m3": ("act1_m3_counter_raid.ron", build_m3),
    "m4": ("act1_m4_envoy.ron", build_m4),
}


def main():
    parser = argparse.ArgumentParser(description="Generate Act 1 campaign inline maps")
    parser.add_argument("--map", choices=["m2", "m3", "m4", "all"], default="all",
                        help="Which map(s) to generate")
    parser.add_argument("--preview", action="store_true",
                        help="Print ASCII art preview")
    parser.add_argument("--patch", action="store_true",
                        help="Replace Generated blocks in RON files")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.abspath(os.path.join(script_dir, "..", "..", ".."))
    campaign_dir = os.path.join(project_root, "assets", "campaign")

    maps_to_build = list(MAP_BUILDERS.keys()) if args.map == "all" else [args.map]

    for map_id in maps_to_build:
        ron_name, builder_fn = MAP_BUILDERS[map_id]
        print(f"\n{'='*60}")
        print(f"Building {map_id}: {ron_name}")
        print(f"{'='*60}")

        m = builder_fn()

        # Stats
        s = m.stats()
        total = m.width * m.height
        passable = sum(v for k, v in s.items() if k in PASSABLE)
        print(f"  Size: {m.width}x{m.height} ({total} tiles)")
        print(f"  Passable: {passable} ({100*passable//total}%)")
        for terrain, count in sorted(s.items(), key=lambda x: -x[1]):
            print(f"    {terrain}: {count}")

        if args.preview:
            print()
            print(m.ascii_preview())

        if args.patch:
            ron_path = os.path.join(campaign_dir, ron_name)
            if not os.path.exists(ron_path):
                print(f"ERROR: RON file not found: {ron_path}", file=sys.stderr)
                continue
            inline_block = m.to_ron_inline()
            patch_ron_file(ron_path, inline_block)

    print("\nDone.")


if __name__ == "__main__":
    main()
