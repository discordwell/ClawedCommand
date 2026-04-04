#!/usr/bin/env python3
"""Generate dream_strait.ron — Strait of Hormuz geography.

Map: 300x60 tiles. Ships transit west→east through the narrows.

Geography (based on Strait of Hormuz):
  NORTH (y=0): Iran — hostile coast. Rocky, mountainous, jagged.
    - Qeshm-like island around x=160-200
    - Larak-like island around x=220-240
    - Multiple peninsulas and inlets for launcher concealment
  CENTER: The strait proper — deep water shipping lane.
    - Narrowest point around x=180 (coasts pinch to ~20 tiles apart)
    - Western opening (x<60) is wider (Persian Gulf)
    - Eastern opening (x>240) is wider (Gulf of Oman)
  SOUTH (y=59): UAE/Oman — friendly coast. Smoother, with Dubai base.
    - Dubai base area around x=50, y=50 (southwest)
    - Smoother coastline with gentle bays
    - Musandam-like peninsula around x=200 (Oman)

Shipping lane: y=30 (center), ships enter at x=0, exit at x=299.
"""

import math
import sys

WIDTH = 300
HEIGHT = 60

ROCK = "Rock"
SHALLOWS = "Shallows"
WATER = "Water"


def iran_coast_south_edge(x: int) -> int:
    """Y of Iran's southernmost Rock tile (higher y = further south into strait).
    Iran starts at y=0 and extends south to this edge."""
    # Base coast depth varies by region
    if x < 60:
        # Western Persian Gulf approach — coast is far north, wide opening
        base = 8 + round(3.0 * math.sin(x * 0.08))
    elif x < 140:
        # Coast starts pushing south — approaching the narrows
        base = 10 + round(2.0 * math.sin(x * 0.06 + 1.0)) + (x - 60) // 20
    elif x < 220:
        # The narrows — Iran coast pushes furthest south
        base = 18 + round(3.0 * math.sin(x * 0.1 + 0.5))
    else:
        # Eastern exit to Gulf of Oman — coast retreats north
        base = 18 - (x - 220) // 8 + round(2.0 * math.sin(x * 0.07))

    # Qeshm-like island (large, x=155-195): extends the "coast" further south
    if 155 <= x <= 195:
        island_center_x = 175
        dist = abs(x - island_center_x)
        island_extent = max(0, 5 - dist // 4)
        base = max(base, 20 + island_extent)

    # Larak-like island (small, x=225-240)
    if 225 <= x <= 240:
        island_center_x = 232
        dist = abs(x - island_center_x)
        island_extent = max(0, 3 - dist // 3)
        base = max(base, 15 + island_extent)

    # Small rocky outcrops
    for cx in [80, 120, 150, 210, 260]:
        if abs(x - cx) <= 3:
            base = max(base, base + 2 - abs(x - cx))

    return min(base, 35)  # Never past the shipping lane


def uae_coast_north_edge(x: int) -> int:
    """Y of UAE/Oman's northernmost Rock tile (lower y = further north into strait).
    Friendly coast extends from this edge south to y=59."""
    # Base coast
    if x < 80:
        # Dubai region — coast stays far south, gentle
        base = 50 - round(1.5 * math.sin(x * 0.05 + 0.3))
    elif x < 180:
        # Approaching the narrows — coast pushes north
        base = 48 - (x - 80) // 15 + round(1.5 * math.sin(x * 0.07 + 2.0))
    elif x < 220:
        # Musandam peninsula (Oman) — dramatic northward push
        musandam_center = 200
        dist = abs(x - musandam_center)
        push = max(0, 8 - dist // 3)
        base = 42 - push + round(1.0 * math.sin(x * 0.12))
    else:
        # Eastern Gulf of Oman — coast retreats south
        base = 45 + (x - 220) // 10 + round(1.5 * math.sin(x * 0.06 + 1.0))

    return max(base, 30)  # Never past the shipping lane


def generate_tile(x: int, y: int) -> tuple:
    """Return (terrain_type, elevation) for tile at (x, y)."""
    iran_edge = iran_coast_south_edge(x)
    uae_edge = uae_coast_north_edge(x)

    # Iran (hostile) coast
    if y <= iran_edge - 3:
        return (ROCK, 0)
    if y <= iran_edge:
        return (SHALLOWS, 0)

    # UAE/Oman (friendly) coast
    if y >= uae_edge + 3:
        return (ROCK, 0)
    if y >= uae_edge:
        return (SHALLOWS, 0)

    # Deep water (the strait)
    return (WATER, 0)


def main():
    tiles = []
    elevations = []
    for y in range(HEIGHT):
        for x in range(WIDTH):
            terrain, elev = generate_tile(x, y)
            tiles.append(terrain)
            elevations.append(elev)

    tiles_str = ", ".join(tiles)
    elev_str = ", ".join(str(e) for e in elevations)

    # Stats
    from collections import Counter
    counts = Counter(tiles)
    print(f"Map: {WIDTH}x{HEIGHT} = {WIDTH*HEIGHT} tiles", file=sys.stderr)
    for t, c in sorted(counts.items(), key=lambda x: -x[1]):
        print(f"  {t}: {c} ({100*c/(WIDTH*HEIGHT):.1f}%)", file=sys.stderr)

    # Measure narrowest point
    min_gap = WIDTH
    min_gap_x = 0
    for x in range(WIDTH):
        gap = uae_coast_north_edge(x) - iran_coast_south_edge(x)
        if gap < min_gap:
            min_gap = gap
            min_gap_x = x
    print(f"Narrowest point: x={min_gap_x}, gap={min_gap} tiles", file=sys.stderr)

    ron = f'''// Dream Sequence Part 3: The Strait of Hormuz
// DEFCON-style drone warfare interlude.
// 300x60 map based on Strait of Hormuz geography.
// Iran (north, hostile) — UAE/Dubai (south, friendly).
// Ships transit west→east through the narrows.
(
    id: "dream_strait",
    name: "The Strait",
    act: 3,
    mission_index: 16,
    map: Inline(
        width: {WIDTH},
        height: {HEIGHT},
        tiles: [{tiles_str}],
        elevation: [{elev_str}],
    ),
    player_setup: (
        heroes: [
            (
                hero_id: KellFisher,
                position: (x: 50, y: 53),
                mission_critical: true,
            ),
        ],
        units: [],
        buildings: [],
        starting_food: 0,
        starting_gpu: 0,
        starting_nfts: 0,
    ),
    enemy_waves: [],
    objectives: [
        (
            id: "protect_convoy",
            description: "Protect the tanker convoy (at least 8 of 12 must arrive safely)",
            primary: true,
            condition: Manual,
        ),
        (
            id: "deploy_all_zero_days",
            description: "Deploy all 4 zero-day exploit types",
            primary: false,
            condition: Manual,
        ),
    ],
    triggers: [
        // Opening: Kell wakes up at the C2 console
        (
            id: "opening",
            condition: AtTick(5),
            actions: [ShowDialogue([0, 1, 2, 3])],
            once: true,
        ),
        // Deployment hint
        (
            id: "deploy_hint",
            condition: AtTick(60),
            actions: [ShowDialogue([4, 5])],
            once: true,
        ),
        // Mid-mission: escalation
        (
            id: "escalation_wave2",
            condition: AtTick(2000),
            actions: [ShowDialogue([6, 7])],
            once: true,
        ),
        // Rex check-in
        (
            id: "rex_check_in",
            condition: AtTick(4000),
            actions: [ShowDialogue([8])],
            once: true,
        ),
        // Late game
        (
            id: "late_game",
            condition: AtTick(6000),
            actions: [ShowDialogue([9, 10])],
            once: true,
        ),
        // Zero-day hint
        (
            id: "zero_day_hint",
            condition: AtTick(1200),
            actions: [ShowDialogue([11, 12])],
            once: true,
        ),
    ],
    dialogue: [
        // 0: Kell wakes at the C2 console
        (
            speaker: "Cmdr. Kell Fisher",
            text: "...dreamt about a sword. Made of code. Weird. Doesn\\'t matter. What\\'s on the board?",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 1: Rex briefs — Hormuz specifics
        (
            speaker: "Lt. Rex Harmon",
            text: "Twelve tankers queued for strait transit. Iranian coast is hot — mobile launchers in the islands, AA screen covering the narrows. Three hundred klicks of hostile coastline.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 2: Kell takes charge
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Compute allocation: sixty percent drone vision, twenty satellite reserve, twenty to the zero-day pipeline. Deploy drones from Dubai station.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 3: Rex confirms
        (
            speaker: "Lt. Rex Harmon",
            text: "Sixteen drones standing by at Dubai. Deploy them to patrol positions, then give the all-clear to launch the convoy. Press Enter when ready.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 4: Deployment tutorial
        (
            speaker: "Lt. Rex Harmon",
            text: "Click drones to select, right-click to deploy. Or activate the coverage script from the terminal. When your screen is covered, press Enter to launch the convoy.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 5: Script hint
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Three hundred klicks is too much to micro by hand. Load the coverage script — let the code do the watching.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 6: Escalation
        (
            speaker: "Lt. Rex Harmon",
            text: "Multiple contacts on the Iranian coast. They\\'re setting up in the Qeshm island shadow. AA drones inbound — they\\'re hunting our birds.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 7: Kell adapts
        (
            speaker: "Cmdr. Kell Fisher",
            text: "They\\'re adapting. Good. So are we. If they knock out a drone in the narrows, switch to satellite. V key, click the gap.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 8: Rex check-in
        (
            speaker: "Lt. Rex Harmon",
            text: "Halfway through. Interceptor count is thin, Kell. Blind spots opening up east of Qeshm.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 9: Late game
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Last batch coming through the narrows. They\\'ll throw everything. Stay clinical.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 10: Rex humanity
        (
            speaker: "Lt. Rex Harmon",
            text: "Those tankers have crew, Kell. Just... keeping that in mind.",
            voice_style: Whisper,
            portrait: "portrait_rex_human",
        ),
        // 11: Zero-day hint
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Zero-day pipeline should be cooking. When it\\'s ready, brick their launchers on Qeshm. That\\'s where the real threat is.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 12: Rex on zero-days
        (
            speaker: "Lt. Rex Harmon",
            text: "Build exploits during lulls. Deploy when you\\'ve got a confirmed launcher. Don\\'t waste them on decoys.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
    ],
    briefing_text: "You dream of who you were — and what you did with it.",
    debrief_text: "",
    next_mission: Fixed("act4_m14_junkyard_fort"),
    mutators: [
        DreamSequence(
            skip_briefing: false,
            skip_debrief: true,
            scene_type: Strait,
        ),
        NoBuildMode,
        NoAiControl,
    ],
)
'''

    out_path = "assets/campaign/dream_strait.ron"
    with open(out_path, "w") as f:
        f.write(ron)
    print(f"Written to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
