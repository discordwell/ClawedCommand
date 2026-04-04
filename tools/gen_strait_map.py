#!/usr/bin/env python3
"""Generate dream_strait.ron with a 300x60 inline map.

Strait layout (y-axis):
  y=0-7:    Hostile coastline (Rock). Irregular southern edge via sin() jitter.
  y=8-10:   Hostile shallows (launcher staging). Rocky peninsulas at x=50,120,200,260.
  y=11-48:  Deep water (38 rows). Shipping lane runs at y=30 (center).
  y=49-51:  Friendly shallows.
  y=52-59:  Friendly coastline (Rock). Irregular northern edge.

Coastline irregularity:
  hostile_coast_edge(x) = 7 + round(1.5*sin(x*0.12) + sin(x*0.05+1.3))
  friendly_coast_edge(x) = 52 - round(1.2*sin(x*0.1+0.7) + 0.8*sin(x*0.04+2.1))
  Peninsulas at specific x-ranges push 3-4 tiles into shallows.
"""

import math
import sys

WIDTH = 300
HEIGHT = 60

ROCK = "Rock"
SHALLOWS = "Shallows"
WATER = "Water"

# Peninsula x-ranges (hostile coast): rocky features jutting into shallows
HOSTILE_PENINSULAS = [
    range(48, 56),    # x=48-55
    range(118, 126),  # x=118-125
    range(198, 206),  # x=198-205
    range(258, 266),  # x=258-265
]

# Peninsula x-ranges (friendly coast)
FRIENDLY_PENINSULAS = [
    range(30, 38),
    range(90, 98),
    range(160, 168),
    range(230, 238),
    range(280, 288),
]


def hostile_coast_edge(x: int) -> int:
    """Y coordinate of the hostile coast's southern boundary (inclusive Rock)."""
    base = 7
    jitter = round(1.5 * math.sin(x * 0.12) + math.sin(x * 0.05 + 1.3))
    # Peninsulas push further south
    for pen in HOSTILE_PENINSULAS:
        if x in pen:
            center = (pen.start + pen.stop) // 2
            dist = abs(x - center)
            jitter += max(0, 3 - dist)
            break
    return base + jitter


def friendly_coast_edge(x: int) -> int:
    """Y coordinate of the friendly coast's northern boundary (inclusive Rock)."""
    base = 52
    jitter = round(1.2 * math.sin(x * 0.1 + 0.7) + 0.8 * math.sin(x * 0.04 + 2.1))
    for pen in FRIENDLY_PENINSULAS:
        if x in pen:
            center = (pen.start + pen.stop) // 2
            dist = abs(x - center)
            jitter -= max(0, 3 - dist)
            break
    return base - jitter


def generate_tile(x: int, y: int) -> tuple:
    """Return (terrain_type, elevation) for tile at (x, y)."""
    h_edge = hostile_coast_edge(x)
    f_edge = friendly_coast_edge(x)

    # Hostile coast
    if y <= h_edge - 3:
        return (ROCK, 0)

    # Hostile shallows band (3 tiles south of coast edge)
    if y <= h_edge:
        # Within peninsula zones, these can be Rock instead
        for pen in HOSTILE_PENINSULAS:
            if x in pen:
                center = (pen.start + pen.stop) // 2
                dist = abs(x - center)
                if dist <= 1:
                    return (ROCK, 0)
        return (SHALLOWS, 0)

    # Friendly coast
    if y >= f_edge + 3:
        return (ROCK, 0)

    # Friendly shallows band
    if y >= f_edge:
        for pen in FRIENDLY_PENINSULAS:
            if x in pen:
                center = (pen.start + pen.stop) // 2
                dist = abs(x - center)
                if dist <= 1:
                    return (ROCK, 0)
        return (SHALLOWS, 0)

    # Deep water (the strait itself)
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

    # Count terrain distribution
    from collections import Counter
    counts = Counter(tiles)
    print(f"Map: {WIDTH}x{HEIGHT} = {WIDTH*HEIGHT} tiles", file=sys.stderr)
    for t, c in sorted(counts.items(), key=lambda x: -x[1]):
        print(f"  {t}: {c} ({100*c/(WIDTH*HEIGHT):.1f}%)", file=sys.stderr)

    ron = f'''// Dream Sequence Part 3: The Strait
// DEFCON-style drone warfare interlude.
// 300x60 procedurally-generated strait map.
// Cmdr. Kell Fisher wakes from the code sword dream and runs a drone op
// to protect oil tankers transiting a narrow strait.
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
                position: (x: 150, y: 55),
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
        // Opening: Kell wakes up at the console
        (
            id: "opening",
            condition: AtTick(5),
            actions: [ShowDialogue([0, 1, 2, 3])],
            once: true,
        ),
        // Tutorial: first tanker enters
        (
            id: "first_tanker",
            condition: AtTick(65),
            actions: [ShowDialogue([4, 5])],
            once: true,
        ),
        // Mid-mission: escalation
        (
            id: "escalation_wave2",
            condition: AtTick(400),
            actions: [ShowDialogue([6, 7])],
            once: true,
        ),
        // Rex periodic commentary
        (
            id: "rex_check_in",
            condition: AtTick(700),
            actions: [ShowDialogue([8])],
            once: true,
        ),
        // Late game tension
        (
            id: "late_game",
            condition: AtTick(1000),
            actions: [ShowDialogue([9, 10])],
            once: true,
        ),
        // Zero-day tutorial hint
        (
            id: "zero_day_hint",
            condition: AtTick(250),
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
        // 1: Rex briefs the situation
        (
            speaker: "Lt. Rex Harmon",
            text: "Twelve tankers queued for strait transit. Hostile coast is hot — mobile launchers, AA screen, the usual. Three hundred klicks of coastline to cover.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 2: Kell takes charge
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Compute allocation: sixty percent drone vision, twenty satellite reserve, twenty to the zero-day pipeline. Get my drones in the air.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 3: Rex confirms
        (
            speaker: "Lt. Rex Harmon",
            text: "Sixteen drones launching. First tanker enters the strait in sixty seconds. That\\'s a lot of water to cover, Commander.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 4: First tanker enters
        (
            speaker: "Cmdr. Kell Fisher",
            text: "First tanker in the lane. Keep drone coverage tight on the shipping channel. Any launcher that sets up, I want eyes on it.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 5: Tutorial — drone patrol
        (
            speaker: "Lt. Rex Harmon",
            text: "Drone patrol routes are scripted through the terminal. Click a drone to select, right-click to redirect. Or write a coverage script — you\\'ll need one for this much coastline.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 6: Escalation — wave 2
        (
            speaker: "Lt. Rex Harmon",
            text: "Multiple contacts on the northern coast. They\\'re setting up in pairs now. And I\\'m picking up AA signatures — they\\'re hunting our drones.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 7: Kell adapts
        (
            speaker: "Cmdr. Kell Fisher",
            text: "They\\'re adapting. Good. So are we. If they knock out a drone, switch to satellite in that sector. V key, click the gap.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 8: Rex check-in
        (
            speaker: "Lt. Rex Harmon",
            text: "Halfway through. Interceptor count is looking thin, Kell. And we\\'ve got blind spots opening up on the eastern flank.",
            voice_style: Normal,
            portrait: "portrait_rex_human",
        ),
        // 9: Late game — Kell clinical
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Last batch coming through. They\\'ll throw everything they have. Stay clinical.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 10: Rex\\'s humanity
        (
            speaker: "Lt. Rex Harmon",
            text: "Those tankers have crew, Kell. Just... keeping that in mind.",
            voice_style: Whisper,
            portrait: "portrait_rex_human",
        ),
        // 11: Zero-day tutorial
        (
            speaker: "Cmdr. Kell Fisher",
            text: "Zero-day pipeline should be cooking. When it\\'s ready, we can brick their launchers permanently — if we can see them.",
            voice_style: Normal,
            portrait: "portrait_kell_fisher",
        ),
        // 12: Rex on zero-days
        (
            speaker: "Lt. Rex Harmon",
            text: "Build exploits during lulls. Deploy when you\\'ve got a confirmed launcher position. Don\\'t waste them on decoys.",
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

    # Write to file
    out_path = "assets/campaign/dream_strait.ron"
    with open(out_path, "w") as f:
        f.write(ron)
    print(f"Written to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
