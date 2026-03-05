#!/usr/bin/env python3
"""Generate hand-crafted inline maps for campaign missions.

Usage:
    python gen_campaign_maps.py --map [m2|m3|m4|m5|m6|m7|m8|all] --preview --patch

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
# M5: The False Front — Mountain Defensive Stand (48x48)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m5():
    # ══════════════════════════════════════════════════════════════
    # M5 "The False Front" — Mountain defense against 3-axis assault
    #
    # Default: Rock (mountain interior). Carve passable areas.
    # Central plateau (elev 1): player's defensive position.
    # 3 approach corridors: N (main road), E (narrow ledge), S (tunnel exit).
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(48, 48, default_terrain="Rock", default_elevation=2)

    # ── Central Plateau (elev 1): player defensive position ──
    # 16x10 platform centered around x=24, y=34-43
    m.fill_rect(16, 32, 31, 43, "Dirt", 1)
    m.fill_rect(18, 33, 29, 42, "Grass", 1)
    # Partial Rock walls on plateau = defensive cover with gaps
    m.fill_rect(17, 32, 30, 32, "Rock", 2)  # North wall
    m.set_tile(22, 32, "Dirt", 1)  # Gap in north wall
    m.set_tile(26, 32, "Dirt", 1)  # Gap in north wall
    m.fill_rect(16, 33, 16, 42, "Rock", 2)  # West wall
    m.set_tile(16, 37, "Dirt", 1)  # Gap in west wall
    m.fill_rect(31, 33, 31, 42, "Rock", 2)  # East wall
    m.set_tile(31, 38, "Dirt", 1)  # Gap in east wall
    # Core Tap area (TechRuins on plateau)
    m.tech_ruins(24, 38, 3, 3, elevation=1)
    # Seekers mining equipment
    m.tech_ruins(19, 36, 2, 2, elevation=1)
    m.tech_ruins(28, 40, 2, 2, elevation=1)

    # ── N Approach: main road, wide corridor ──
    # Carve a corridor from top down to plateau
    m.fill_rect(18, 2, 28, 6, "Dirt", 0)     # Top staging area
    m.fill_rect(20, 3, 26, 5, "Grass", 0)
    # Road descending south
    m.road([(24, 5), (24, 10), (23, 15), (23, 20), (22, 25), (22, 30), (22, 32)],
           width=2, terrain="Road", elevation=0)
    # Widen corridor around road
    m.fill_rect(19, 7, 27, 12, "Dirt", 0)
    m.fill_rect(18, 14, 26, 20, "Dirt", 0)
    m.fill_rect(17, 22, 27, 30, "Grass", 0)
    m.fill_rect(19, 24, 25, 29, "Dirt", 0)
    # Ramp connecting corridor to plateau
    m.fill_rect(20, 30, 26, 31, "Ramp", 1)

    # ── E Approach: narrow mountain ledge ──
    # Narrow path from east border to plateau
    m.fill_rect(36, 16, 46, 24, "Dirt", 0)   # Eastern staging
    m.fill_rect(38, 18, 44, 22, "Grass", 0)
    # Narrow ledge path
    m.road([(42, 20), (38, 22), (35, 25), (33, 28), (32, 32), (32, 35), (31, 38)],
           width=1, terrain="Dirt", elevation=0)
    # Widen key sections
    m.fill_rect(33, 25, 36, 29, "Dirt", 0)
    m.fill_rect(32, 30, 34, 34, "Grass", 0)
    # Ramp to plateau from east
    m.fill_rect(31, 35, 32, 39, "Ramp", 1)

    # ── S Tunnel Exit: rear ambush route ──
    # Tunnel mouth at bottom, carve upward to plateau
    m.fill_rect(20, 44, 27, 47, "Dirt", 0)   # Tunnel mouth
    m.fill_rect(21, 45, 26, 46, "Grass", 0)
    m.road([(24, 45), (24, 43)], width=2, terrain="Road", elevation=0)
    # Ramp up to plateau from south
    m.fill_rect(20, 43, 27, 43, "Ramp", 1)

    # ── NE elevated position (elev 2, sniping spot) ──
    m.fill_rect(34, 6, 40, 12, "Grass", 2)
    m.fill_rect(36, 8, 38, 10, "Dirt", 2)
    # Ramp access
    m.fill_rect(34, 12, 36, 14, "Ramp", 1)
    m.fill_rect(34, 14, 36, 16, "Dirt", 0)

    # ── West side: underground stream (limits flanking) ──
    m.river([(6, 10), (8, 18), (10, 26), (12, 34), (14, 42)],
            width=2, bank_width=1)
    # Small passable area west of stream
    m.fill_rect(2, 20, 6, 28, "Dirt", 0)
    m.fill_rect(3, 22, 5, 26, "Grass", 0)

    # ── Rocky crags between corridors (block direct movement) ──
    # These are left as default Rock, already impassable

    # ── Scattered rock outcrops in corridors ──
    m.fill_rect(21, 16, 22, 17, "Rock", 1)
    m.fill_rect(25, 22, 26, 23, "Rock", 1)

    # ── Forest patches for concealment ──
    m.forest_patch(20, 8, 2, 0.6, seed=500)
    m.forest_patch(26, 15, 2, 0.5, seed=501)
    m.forest_patch(19, 26, 2, 0.4, seed=502)

    # ── Validate all spawn positions from RON ──
    spawns = {
        # Player setup
        "Kelpie": (24, 40),
        "MotherGranite": (26, 40),
        "Chonk_1": (22, 38),
        "Chonk_2": (24, 38),
        "Chonk_3": (26, 38),
        "Chonk_4": (28, 38),
        "Hisser_1": (22, 40),
        "Hisser_2": (23, 41),
        "Hisser_3": (27, 41),
        "Hisser_4": (28, 40),
        "Nuisance_1": (20, 39),
        "Nuisance_2": (30, 39),
        # Wave 1: scouts from N
        "scout_1": (22, 5),
        "scout_2": (24, 4),
        "scout_3": (26, 5),
        # Wave 2: flank from E
        "flank_1": (44, 20),
        "flank_2": (45, 21),
        "flank_3": (44, 22),
        "flank_4": (45, 19),
        "flank_5": (46, 20),
        # Wave 3: main from N
        "main_1": (20, 3),
        "main_2": (22, 4),
        "main_3": (24, 3),
        "main_4": (21, 5),
        "main_5": (25, 5),
        "main_6": (23, 2),
        "main_7": (26, 4),
        # Wave 4: rear from S
        "rear_1": (22, 46),
        "rear_2": (24, 47),
        "rear_3": (26, 46),
        "rear_4": (23, 47),
        "rear_5": (25, 47),
        "rear_6": (24, 46),
        # Attack targets
        "attack_target": (24, 40),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# M6: Triangulation — Surrounded High Ground Defense (48x48)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m6():
    # ══════════════════════════════════════════════════════════════
    # M6 "Triangulation" — Mountain peak with 4 approach valleys
    #
    # Default: Rock (mountain). Carve valleys and plateau.
    # Central plateau (elev 2 core, elev 1 ring).
    # 4 approach valleys: N (wide road), E (narrow), W (forest), S (player rear).
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(48, 48, default_terrain="Rock", default_elevation=2)

    # ── Central Plateau (elev 2 core + elev 1 ring) ──
    # Ring (elev 1): y=30-44, x=14-33
    m.fill_rect(14, 30, 33, 44, "Grass", 1)
    # Core (elev 2): y=33-41, x=17-30
    m.fill_rect(17, 33, 30, 41, "Dirt", 2)
    m.fill_rect(19, 35, 28, 39, "Grass", 2)
    # Ramps around plateau edges
    m.fill_rect(14, 30, 33, 30, "Ramp", 1)  # North edge ramp
    m.fill_rect(14, 30, 14, 44, "Ramp", 1)  # West edge ramp
    m.fill_rect(33, 30, 33, 44, "Ramp", 1)  # East edge ramp
    # TechRuins on plateau (Seekers equipment)
    m.tech_ruins(22, 36, 3, 3, elevation=2)
    m.tech_ruins(27, 38, 2, 2, elevation=2)
    m.tech_ruins(18, 40, 2, 2, elevation=1)

    # ── N Valley: wide road approach (main threat axis) ──
    m.fill_rect(8, 1, 40, 12, "Dirt", 0)
    m.fill_rect(10, 2, 38, 10, "Grass", 0)
    m.road([(24, 4), (24, 10), (24, 16), (24, 22), (24, 28), (24, 30)],
           width=2, terrain="Road", elevation=0)
    # Widen corridor between valley and plateau
    m.fill_rect(16, 12, 32, 16, "Dirt", 0)
    m.fill_rect(18, 14, 30, 16, "Grass", 0)
    m.fill_rect(16, 17, 32, 22, "Dirt", 0)
    m.fill_rect(18, 18, 30, 21, "Grass", 0)
    m.fill_rect(16, 23, 32, 29, "Grass", 0)
    m.fill_rect(18, 24, 30, 28, "Dirt", 0)
    # Rocky outcrops in N valley
    m.fill_rect(20, 8, 21, 9, "Rock", 1)
    m.fill_rect(27, 6, 28, 7, "Rock", 1)
    m.fill_rect(19, 19, 20, 20, "Rock", 1)

    # ── E Valley: narrow rocky ledge (pincer route) ──
    m.fill_rect(38, 14, 45, 24, "Dirt", 0)
    m.fill_rect(40, 16, 44, 22, "Grass", 0)
    # Narrow path connecting to plateau
    m.road([(42, 20), (38, 24), (36, 27), (34, 30)],
           width=1, terrain="Dirt", elevation=0)
    m.fill_rect(34, 26, 38, 30, "Grass", 0)
    m.fill_rect(35, 27, 37, 29, "Dirt", 0)
    # Cliff edges (left as Rock default)

    # ── W Valley: forested, wider, concealed (pincer route) ──
    m.fill_rect(2, 12, 12, 24, "Grass", 0)
    m.fill_rect(3, 14, 10, 22, "Forest", 0)
    # Connect to plateau
    m.road([(6, 18), (10, 22), (12, 26), (14, 30)],
           width=1, terrain="Dirt", elevation=0)
    m.fill_rect(10, 24, 14, 30, "Grass", 0)
    m.fill_rect(11, 25, 13, 29, "Forest", 0)
    # Extra forest cover
    m.forest_patch(5, 16, 3, 0.7, seed=600)
    m.forest_patch(8, 20, 2, 0.6, seed=601)

    # ── S: player's rear, minimal approach ──
    m.fill_rect(18, 44, 30, 47, "Grass", 1)
    m.fill_rect(20, 45, 28, 46, "Dirt", 1)

    # ── Scattered features ──
    m.forest_patch(16, 26, 2, 0.5, seed=602)
    m.forest_patch(30, 24, 2, 0.4, seed=603)

    # ── Validate all spawn positions from RON ──
    spawns = {
        # Player setup
        "Kelpie": (24, 38),
        "MotherGranite": (26, 38),
        "Chonk_1": (22, 36),
        "Chonk_2": (24, 36),
        "Chonk_3": (26, 36),
        "Hisser_1": (21, 38),
        "Hisser_2": (23, 39),
        "Hisser_3": (27, 39),
        "Hisser_4": (29, 38),
        "Nuisance_1": (20, 37),
        "Nuisance_2": (28, 37),
        "Nuisance_3": (24, 40),
        # Wave 1: probes from N
        "probe_1": (10, 8),
        "probe_2": (12, 7),
        "probe_3": (38, 6),
        # Wave 2: E pincer
        "east_1": (44, 18),
        "east_2": (45, 19),
        "east_3": (43, 20),
        "east_4": (44, 17),
        # Wave 3: W pincer
        "west_1": (4, 16),
        "west_2": (3, 18),
        "west_3": (5, 17),
        "west_4": (4, 15),
        "west_5": (3, 19),
        # Wave 4: central N
        "central_1": (22, 4),
        "central_2": (24, 3),
        "central_3": (26, 4),
        "central_4": (23, 5),
        "central_5": (25, 5),
        "central_6": (24, 2),
        # Wave 5: final N
        "final_1": (20, 2),
        "final_2": (28, 2),
        "final_3": (22, 3),
        "final_4": (26, 3),
        "final_5": (24, 1),
        "final_6": (21, 3),
        "final_7": (27, 3),
        # Attack targets
        "attack_target_1": (24, 24),
        "attack_target_2": (24, 36),
        "attack_target_3": (24, 38),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# M7: The Rex's Whisper — Epic Diagonal Battlefield (64x64)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m7():
    # ══════════════════════════════════════════════════════════════
    # M7 "The Rex's Whisper" — Massive battle with diagonal ridge
    #
    # Default: Grass. Central diagonal Rock ridge with 3 passes.
    # SW player base, NE enemy territory, SE excavation site.
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(64, 64, default_terrain="Grass", default_elevation=0)

    # ── Rock border ──
    m.border(2, "Rock", 2)

    # ══════════════════════════════════════════════════════════════
    # CENTRAL RIDGE: diagonal Rock wall (elev 2) NW→SE
    # Runs from ~(10,20) to ~(50,45). The defining map feature.
    # ══════════════════════════════════════════════════════════════
    m.road([(10, 20), (18, 26), (25, 30), (32, 34), (40, 38), (50, 45)],
           width=4, terrain="Rock", elevation=2)
    # Thicken the ridge in key sections
    m.fill_rect(12, 21, 16, 25, "Rock", 2)
    m.fill_rect(22, 28, 28, 33, "Rock", 2)
    m.fill_rect(34, 34, 38, 39, "Rock", 2)
    m.fill_rect(44, 40, 48, 44, "Rock", 2)

    # ── 3 Passes through ridge ──
    # NORTH PASS: wide, Road-surfaced, near (16,23)
    m.fill_rect(14, 22, 18, 25, "Road", 0)
    m.fill_rect(13, 22, 13, 25, "Dirt", 0)
    m.fill_rect(19, 22, 19, 25, "Dirt", 0)

    # CENTER PASS: medium width, Dirt, near (28,31)
    m.fill_rect(26, 29, 30, 33, "Dirt", 0)
    m.fill_rect(27, 30, 29, 32, "Road", 0)

    # SOUTH PASS: narrow, forested, near (42,40)
    m.fill_rect(40, 38, 44, 42, "Forest", 0)
    m.fill_rect(41, 39, 43, 41, "Dirt", 0)

    # ══════════════════════════════════════════════════════════════
    # SW PLAYER BASE: Forest cover + Dirt staging
    # ══════════════════════════════════════════════════════════════
    m.fill_rect(4, 44, 18, 56, "Grass", 0)
    m.fill_rect(5, 45, 17, 55, "Forest", 0)
    m.fill_rect(6, 47, 16, 53, "Dirt", 0)
    m.fill_rect(8, 48, 14, 52, "Grass", 0)
    # Road from base toward center pass
    m.road([(12, 50), (16, 46), (20, 40), (24, 36), (28, 32)],
           width=2, terrain="Road")

    # ══════════════════════════════════════════════════════════════
    # NE ENEMY TERRITORY: beyond the ridge
    # ══════════════════════════════════════════════════════════════
    m.fill_rect(42, 4, 60, 18, "Grass", 0)
    m.fill_rect(44, 6, 58, 16, "Dirt", 0)
    m.fill_rect(46, 8, 56, 14, "Grass", 0)
    # Forest cover for enemy staging
    m.forest_patch(48, 8, 4, 0.6, seed=700)
    m.forest_patch(54, 12, 3, 0.5, seed=701)
    # Road from enemy base through north pass
    m.road([(50, 10), (44, 14), (36, 18), (28, 22), (20, 24), (16, 23)],
           width=2, terrain="Road")
    # Open areas north of ridge
    m.fill_rect(20, 4, 40, 18, "Grass", 0)
    m.fill_rect(22, 6, 38, 16, "Dirt", 0)
    m.fill_rect(24, 8, 36, 14, "Grass", 0)
    # Hills (elev 1 patches)
    m.set_elevation_rect(26, 10, 34, 14, 1)
    m.set_elevation_rect(36, 6, 42, 10, 1)

    # ══════════════════════════════════════════════════════════════
    # SE EXCAVATION SITE: walled TechRuins compound
    # ══════════════════════════════════════════════════════════════
    # Rock perimeter
    m.fill_rect(50, 50, 60, 60, "Rock", 1)
    # Interior
    m.fill_rect(52, 52, 58, 58, "Dirt", 0)
    m.fill_rect(53, 53, 57, 57, "Grass", 0)
    # TechRuins core
    m.tech_ruins(55, 55, 3, 3)
    m.tech_ruins(53, 54, 2, 2)
    # Entry ramp from NW
    m.fill_rect(50, 53, 51, 55, "Ramp", 0)
    m.fill_rect(48, 52, 50, 56, "Dirt", 0)
    # Path from south pass to excavation
    m.road([(44, 42), (48, 46), (50, 50), (52, 54)],
           width=1, terrain="Dirt")

    # ══════════════════════════════════════════════════════════════
    # SOUTHERN PLAINS: rolling hills, scattered forest
    # ══════════════════════════════════════════════════════════════
    # Hills (elev 1 patches)
    m.set_elevation_rect(20, 38, 26, 42, 1)
    m.set_elevation_rect(30, 44, 36, 48, 1)
    m.set_elevation_rect(38, 50, 44, 54, 1)
    # Forest patches
    m.forest_patch(24, 46, 3, 0.5, seed=702)
    m.forest_patch(32, 50, 3, 0.4, seed=703)
    m.forest_patch(18, 38, 2, 0.5, seed=704)

    # ══════════════════════════════════════════════════════════════
    # WEST SIDE: mountain stream
    # ══════════════════════════════════════════════════════════════
    m.river([(4, 8), (5, 16), (6, 24), (7, 32), (8, 40)],
            width=2, bank_width=1)
    # Passable areas beside stream
    m.fill_rect(2, 8, 4, 16, "Dirt", 0)

    # ── East flank area ──
    m.fill_rect(56, 26, 61, 36, "Dirt", 0)
    m.fill_rect(57, 28, 60, 34, "Grass", 0)

    # ── West push area ──
    m.fill_rect(2, 6, 8, 16, "Dirt", 0)
    m.fill_rect(3, 8, 7, 14, "Grass", 0)

    # ── Validate all spawn positions from RON ──
    spawns = {
        # Player setup
        "Kelpie": (10, 50),
        "MotherGranite": (12, 50),
        "Chonk_1": (8, 48),
        "Chonk_2": (10, 48),
        "Chonk_3": (12, 48),
        "Chonk_4": (14, 48),
        "Chonk_5": (9, 46),
        "Chonk_6": (13, 46),
        "Hisser_1": (8, 50),
        "Hisser_2": (10, 51),
        "Hisser_3": (12, 51),
        "Hisser_4": (14, 50),
        "Hisser_5": (9, 52),
        "Hisser_6": (13, 52),
        "Nuisance_1": (6, 49),
        "Nuisance_2": (16, 49),
        "Nuisance_3": (7, 47),
        "Nuisance_4": (15, 47),
        "Yowler_1": (10, 53),
        "Yowler_2": (12, 53),
        # Wave 1: vanguard NE
        "van_1": (50, 10),
        "van_2": (52, 11),
        "van_3": (54, 10),
        "van_4": (51, 12),
        "van_5": (53, 12),
        # Wave 2: main NE
        "wm_1": (48, 8),
        "wm_2": (50, 9),
        "wm_3": (52, 8),
        "wm_4": (49, 10),
        "wm_5": (51, 10),
        "wm_6": (53, 10),
        "wm_7": (50, 7),
        "wm_8": (52, 7),
        # Wave 3: E flank
        "ef_1": (58, 30),
        "ef_2": (59, 32),
        "ef_3": (60, 31),
        "ef_4": (57, 33),
        "ef_5": (58, 29),
        "ef_6": (59, 34),
        # Wave 4: elite NE
        "el_1": (50, 6),
        "el_2": (52, 5),
        "el_3": (54, 6),
        "el_4": (51, 7),
        "el_5": (53, 7),
        "el_6": (49, 8),
        "el_7": (55, 8),
        # Wave 5: W push
        "wp_1": (4, 10),
        "wp_2": (3, 12),
        "wp_3": (5, 11),
        "wp_4": (2, 13),
        "wp_5": (4, 9),
        # Wave 6: final NE
        "fin_1": (48, 5),
        "fin_2": (50, 4),
        "fin_3": (52, 5),
        "fin_4": (54, 4),
        "fin_5": (49, 6),
        "fin_6": (51, 6),
        "fin_7": (53, 6),
        "fin_8": (55, 6),
        "fin_9": (50, 3),
        "fin_10": (52, 3),
        "fin_11": (54, 3),
        # Attack target
        "attack_target": (10, 50),
        # Excavation objective
        "excavation": (55, 55),
    }
    m.validate_positions(spawns)
    return m


# ═══════════════════════════════════════════════════════════════════════════════
# M8: The Oath-Breaker — Underground Tunnel Escape (48x64)
# ═══════════════════════════════════════════════════════════════════════════════

def build_m8():
    # ══════════════════════════════════════════════════════════════
    # M8 "The Oath-Breaker" — Stealth escape, south→north
    #
    # Default: Rock (underground, impassable). Carve corridors/chambers.
    # Main shaft 6-8 tiles wide running N-S through center.
    # 4 checkpoint zones with wider chambers.
    # Side passages as alternate routes.
    # ══════════════════════════════════════════════════════════════

    m = MapBuilder(48, 64, default_terrain="Rock", default_elevation=2)

    # ══════════════════════════════════════════════════════════════
    # MAIN SHAFT: runs N-S through center, 6 tiles wide
    # x=20-27, full height with chambers
    # ══════════════════════════════════════════════════════════════

    # ── Player Start (y=56-62): small safe chamber ──
    m.fill_rect(18, 55, 30, 62, "Dirt", 0)
    m.fill_rect(20, 56, 28, 61, "Grass", 0)
    m.fill_rect(22, 57, 26, 60, "Dirt", 0)
    # Road exit north
    m.road([(24, 56), (24, 54)], width=2, terrain="Road", elevation=0)

    # ── South Gate (y=44-54): entry chamber + narrowing ──
    m.fill_rect(16, 44, 32, 55, "Dirt", 0)
    m.fill_rect(18, 46, 30, 53, "Grass", 0)
    m.fill_rect(20, 48, 28, 52, "Dirt", 0)
    # Central Road through chamber
    m.road([(24, 54), (24, 50), (24, 44)], width=2, terrain="Road", elevation=0)
    # TechRuins: mining equipment
    m.tech_ruins(18, 50, 2, 2)
    # Pillars for hiding
    m.set_tile(20, 50, "Rock", 1)
    m.set_tile(28, 50, "Rock", 1)

    # ── Side passage W (bypasses south gate) ──
    m.fill_rect(8, 46, 14, 54, "Shallows", 0)   # Underground water seepage
    m.fill_rect(10, 48, 12, 52, "Dirt", 0)
    # Connect to main shaft
    m.road([(14, 50), (16, 50)], width=1, terrain="Dirt", elevation=0)
    m.road([(12, 46), (14, 44), (16, 44)], width=1, terrain="Dirt", elevation=0)

    # ── Mid Checkpoint (y=30-40): wider chamber with pillars ──
    m.fill_rect(14, 30, 34, 43, "Dirt", 0)
    m.fill_rect(16, 32, 32, 41, "Grass", 0)
    m.fill_rect(18, 34, 30, 38, "Dirt", 0)
    # Road through center
    m.road([(24, 44), (24, 40), (24, 34), (24, 30)], width=2, terrain="Road", elevation=0)
    # Pillars (Rock tiles within chamber)
    m.set_tile(20, 34, "Rock", 1)
    m.set_tile(28, 34, "Rock", 1)
    m.set_tile(20, 38, "Rock", 1)
    m.set_tile(28, 38, "Rock", 1)
    m.set_tile(24, 36, "Rock", 1)  # Center pillar
    # Side alcoves for hiding
    m.fill_rect(10, 34, 14, 40, "Dirt", 0)   # West alcove
    m.fill_rect(11, 35, 13, 39, "Grass", 0)
    m.fill_rect(34, 34, 38, 40, "Dirt", 0)   # East alcove
    m.fill_rect(35, 35, 37, 39, "Grass", 0)
    # TechRuins in mid section
    m.tech_ruins(16, 36, 2, 2)
    m.tech_ruins(30, 36, 2, 2)

    # ── Connector: mid checkpoint to tunnel zone ──
    m.fill_rect(20, 26, 28, 30, "Dirt", 0)
    m.road([(24, 30), (24, 26)], width=2, terrain="Road", elevation=0)

    # ── Tunnel Zone (y=16-26): patrol corridors ──
    m.fill_rect(10, 18, 38, 26, "Dirt", 0)
    m.fill_rect(12, 20, 36, 24, "Grass", 0)
    m.fill_rect(14, 21, 34, 23, "Dirt", 0)
    # Road through center
    m.road([(24, 26), (24, 20), (24, 16)], width=2, terrain="Road", elevation=0)
    # Side passage E (bypasses tunnel zone)
    m.fill_rect(36, 16, 42, 26, "Shallows", 0)
    m.fill_rect(38, 18, 40, 24, "Dirt", 0)
    # Connect to main shaft
    m.road([(36, 22), (34, 22)], width=1, terrain="Dirt", elevation=0)
    m.road([(38, 16), (34, 14), (28, 14)], width=1, terrain="Dirt", elevation=0)
    # Pillars
    m.set_tile(18, 22, "Rock", 1)
    m.set_tile(30, 22, "Rock", 1)
    m.set_tile(24, 22, "Rock", 1)
    # Slight uphill elevation change
    m.set_elevation_rect(10, 16, 38, 20, 1)

    # ── Connector: tunnel zone to exit zone ──
    m.fill_rect(20, 12, 28, 18, "Dirt", 0)
    m.fill_rect(22, 14, 26, 16, "Grass", 0)
    m.road([(24, 18), (24, 14)], width=2, terrain="Road", elevation=0)
    # Ramp between elevation levels
    m.fill_rect(20, 16, 28, 16, "Ramp", 1)

    # ── Exit Zone (y=2-12): wide chamber, multiple exits ──
    m.fill_rect(14, 2, 34, 12, "Dirt", 0)
    m.fill_rect(16, 3, 32, 11, "Grass", 0)
    m.fill_rect(18, 4, 30, 10, "Dirt", 0)
    # Road out to the north
    m.road([(24, 12), (24, 8), (24, 4)], width=2, terrain="Road", elevation=0)
    # Multiple corridor exits leading to border
    m.fill_rect(20, 2, 28, 3, "Road", 0)   # Main exit
    m.fill_rect(14, 2, 16, 4, "Dirt", 0)   # West exit
    m.fill_rect(32, 2, 34, 4, "Dirt", 0)   # East exit
    # TechRuins
    m.tech_ruins(18, 6, 2, 2)
    m.tech_ruins(28, 8, 2, 2)
    # Ironjaw's last stand area
    m.fill_rect(30, 6, 34, 10, "Dirt", 0)
    m.fill_rect(31, 7, 33, 9, "Grass", 0)

    # ── Validate all spawn positions from RON ──
    spawns = {
        # Player start
        "Kelpie": (24, 58),
        "Mouser_1": (22, 57),
        "Mouser_2": (26, 57),
        "Nuisance_1": (23, 59),
        "Nuisance_2": (25, 59),
        # Wave 1: S patrol
        "s_patrol_1": (20, 52),
        "s_patrol_2": (28, 52),
        "s_patrol_3": (24, 50),
        # Wave 2: mid checkpoint
        "mid_1": (18, 36),
        "mid_2": (30, 36),
        "mid_3": (24, 34),
        "mid_4": (22, 38),
        "mid_5": (26, 38),
        # Wave 3: tunnel patrol
        "tun_1": (12, 24),
        "tun_2": (14, 24),
        "tun_3": (13, 22),
        # Wave 4: exit guard
        "exit_1": (22, 10),
        "exit_2": (26, 10),
        "exit_3": (24, 8),
        "exit_4": (20, 12),
        "exit_5": (28, 12),
        # Patrol waypoints
        "patrol_s_1": (20, 52),
        "patrol_s_2": (28, 52),
        "patrol_s_3": (28, 48),
        "patrol_s_4": (20, 48),
        "patrol_t_1": (12, 24),
        "patrol_t_2": (36, 24),
        "patrol_t_3": (36, 20),
        "patrol_t_4": (12, 20),
        "patrol_e_1": (20, 10),
        "patrol_e_2": (28, 10),
        "patrol_e_3": (28, 6),
        "patrol_e_4": (20, 6),
        # Objective/trigger positions
        "obj_exit": (24, 5),
        "trig_54": (24, 54),
        "trig_44": (24, 44),
        "trig_40": (24, 40),
        "trig_32": (24, 32),
        "trig_24": (24, 24),
        "trig_14": (24, 14),
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
    "m5": ("act2_m5_false_front.ron", build_m5),
    "m6": ("act2_m6_triangulation.ron", build_m6),
    "m7": ("act2_m7_rexs_whisper.ron", build_m7),
    "m8": ("act2_m8_oath_breaker.ron", build_m8),
}


def main():
    parser = argparse.ArgumentParser(description="Generate campaign inline maps")
    parser.add_argument("--map", choices=["m2", "m3", "m4", "m5", "m6", "m7", "m8", "all"],
                        default="all", help="Which map(s) to generate")
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
