#!/usr/bin/env python3
"""
Defines all 14 building animation sheet prompts for ChatGPT generation.
Prints each prompt so it can be copy-pasted or piped into browser automation.
"""

BUILDINGS = {
    "cat_tree": {
        "desc": "A tall multi-level cat tree tower made from stacked cardboard boxes, sisal rope-wrapped columns, multiple platforms at different heights, dangling toy mice, and a cat flag on top. Brown cardboard with blue tech accents.",
        "construct": [
            "Just a flat cardboard base platform with scattered building materials, rope coils, and loose cardboard panels",
            "Lower 2 levels assembled — cardboard boxes stacked, sisal rope columns started, first platform placed",
            "3 levels built, upper platforms being placed, dangling toys being hung, almost complete",
            "Fully complete cat tree tower with flag, all platforms, dangling toys, and blue LED strips",
        ],
        "ambient": [
            "Complete cat tree with flag blowing left, blue LEDs on, toy mouse swinging left",
            "Complete cat tree with flag blowing right, blue LEDs flickering, toy mouse swinging right",
            "Complete cat tree with flag straight up, blue LEDs bright, toy mouse at center",
            "Complete cat tree with flag blowing left, blue LEDs dim, toy mouse swinging left again",
        ],
    },
    "fish_market": {
        "desc": "A wide open-air fish stall made from cardboard with a striped blue-and-white awning, hanging fish, ice boxes, crates of fish, and a fish-shaped cardboard sign. Brown cardboard with blue tech accents.",
        "construct": [
            "Cardboard base platform with scattered planks, a folded-up awning, and empty crates",
            "Support posts erected, awning frame up but not stretched, some boxes placed",
            "Awning partially unfurled with stripes visible, fish crates being stocked, sign being placed",
            "Fully complete fish market with stretched awning, hanging fish, stocked crates, lit price display",
        ],
        "ambient": [
            "Complete fish market, hanging fish swinging left, price display showing '3', awning flapping slightly",
            "Complete fish market, hanging fish swinging right, price display showing '5', steam from ice boxes",
            "Complete fish market, hanging fish centered, price display showing '2', awning still",
            "Complete fish market, hanging fish swinging left, price display showing '7', slight awning flutter",
        ],
    },
    "litter_box": {
        "desc": "A supply depot shaped like a hooded/covered cat litter box — rounded dome structure with arched entrance, sandbags around base, supply crates, and a shovel. Brown cardboard dome with blue tech accents.",
        "construct": [
            "Flat cardboard base with sandbags being stacked, curved cardboard panels lying flat",
            "Lower walls formed into partial dome shape, entrance arch framed, sandbags halfway up",
            "Dome mostly formed with gap at top, entrance arch complete, supply crates placed, inventory screen off",
            "Fully complete hooded litter box dome, entrance lit with blue LED strip, inventory screen on, shovel placed",
        ],
        "ambient": [
            "Complete litter box dome, blue LED strip around entrance glowing bright, inventory screen showing data",
            "Complete litter box dome, LED strip pulsing dimmer, inventory screen scrolling, slight dust puff from entrance",
            "Complete litter box dome, LED strip bright again, inventory screen showing graph, sandbag slightly shifted",
            "Complete litter box dome, LED strip medium glow, inventory screen flashing, small supply crate moved",
        ],
    },
    "server_rack": {
        "desc": "A tech building that looks like 2-3 tall server rack cabinets. Very tall and narrow with rows of blinking blue and green LEDs, ventilation slats, thick cables, and a small cat cushion on top. Brown cardboard with metal brackets.",
        "construct": [
            "Base platform with loose cardboard panels, metal brackets scattered, cables coiled on ground",
            "First rack cabinet partially assembled — lower half built with some LED slots visible, cables hanging",
            "Two racks standing, third being assembled, most LEDs dark, cables being routed between racks",
            "Fully complete server rack bank — all LEDs blinking blue/green, cables connected, cat cushion on top",
        ],
        "ambient": [
            "Complete server racks, top row LEDs blue, middle green, bottom blue, cables steady",
            "Complete server racks, top row LEDs green, middle blue, bottom off, one cable swaying",
            "Complete server racks, all LEDs blue and bright, heat shimmer from vents, cables steady",
            "Complete server racks, alternating blue-green pattern, one LED red (warning), cables steady",
        ],
    },
    "scratching_post": {
        "desc": "A research lab built around a massive cylindrical scratching post — thick sisal rope-wrapped column with wide circular mushroom-cap platform on top. Glowing blue runes in the rope, research equipment at base. Wooden base platform.",
        "construct": [
            "Wooden base platform with the cardboard core tube lying on its side, loose rope coils, scattered equipment",
            "Core tube erected vertically, lower section wrapped in sisal rope, base equipment being placed",
            "Post fully wrapped in rope, mushroom cap platform being placed on top, blue runes starting to glow faintly",
            "Fully complete scratching post with bright blue glowing runes, mushroom cap platform, all research equipment active",
        ],
        "ambient": [
            "Complete scratching post, blue runes glowing in spiral pattern upward, equipment screens on",
            "Complete scratching post, blue runes pulsing in wave pattern downward, equipment blinking",
            "Complete scratching post, all runes bright simultaneously, equipment screens showing data",
            "Complete scratching post, runes dimming in cascade from top, equipment in standby mode",
        ],
    },
    "cat_flap": {
        "desc": "A garrison gate — thick cardboard box wall segment with reinforced cat flap door in center, security camera above, spikes on top. Very wide, very low. Brown cardboard boxes with duct tape and blue tech accents.",
        "construct": [
            "Ground-level foundation with a few cardboard boxes placed in a line, spike stakes lying nearby",
            "Wall half-height, boxes stacking wider, flap door frame being installed in center gap",
            "Wall nearly full height, flap door hinged, spikes being placed on top, camera mount installed",
            "Fully complete garrison wall with reinforced flap door, all spikes up, camera active with blue LED",
        ],
        "ambient": [
            "Complete wall, security camera pointed left, flap door closed, blue screen showing 'SECURE'",
            "Complete wall, security camera pointed right, flap door slightly ajar, blue screen flickering",
            "Complete wall, security camera pointed center, flap door closed, blue screen showing radar sweep",
            "Complete wall, security camera panning left, flap door closed, blue screen showing 'ALERT'",
        ],
    },
    "laser_pointer": {
        "desc": "A defense tower — single tall thin cardboard tube pole with motorized laser pointer turret on top, red laser beam, guy-wires from pole to small cardboard base platform. Silver/blue metallic turret.",
        "construct": [
            "Small cardboard base platform with the tube pole lying on its side, loose guy-wire cables, turret parts",
            "Pole being raised upright (tilted ~45 degrees), base anchored, guy-wires slack",
            "Pole fully vertical, guy-wires tensioned, turret being mounted on top (not yet connected)",
            "Fully complete defense tower — pole vertical, guy-wires taut, turret active with visible red laser beam",
        ],
        "ambient": [
            "Complete tower, turret aimed far right, red laser beam shooting right, guy-wires steady",
            "Complete tower, turret aimed center-left, red laser beam shooting left, slight wire vibration",
            "Complete tower, turret aimed far left, red laser beam angled down-left, wires taut",
            "Complete tower, turret aimed center-right, red laser beam shooting right and up, wires steady",
        ],
    },
}

def make_prompt(building_name, info, anim_type):
    """Generate the ChatGPT prompt for a given building and animation type."""
    desc = info["desc"]
    frames = info[anim_type]

    if anim_type == "construct":
        anim_label = "CONSTRUCTION SEQUENCE — 4 stages of building being assembled"
    else:
        anim_label = "AMBIENT IDLE ANIMATION — 4 frames of subtle looping animation on the complete building"

    return f"""Generate a 1024x1024 image containing a 2x2 GRID of 4 animation frames for a building sprite sheet. OUTPUT AS PNG WITH TRANSPARENT BACKGROUND.

BUILDING: "{building_name.replace('_', ' ').title()}" — {desc}

ANIMATION: {anim_label}:
- TOP-LEFT (Frame 1): {frames[0]}
- TOP-RIGHT (Frame 2): {frames[1]}
- BOTTOM-LEFT (Frame 3): {frames[2]}
- BOTTOM-RIGHT (Frame 4): {frames[3]}

Each frame occupies exactly one quadrant (512x512). Clear visual separation between frames. Isometric 3/4 view. TRANSPARENT BACKGROUND. Cel-shaded, NOT pixel art. Into the Breach / Advance Wars aesthetic. Bold dark outlines.

Generate ONLY this one image. No characters or creatures."""


if __name__ == "__main__":
    # Print all prompts with labels
    for building, info in BUILDINGS.items():
        for anim_type in ["construct", "ambient"]:
            print(f"\n{'='*60}")
            print(f"  {building}_{anim_type}")
            print(f"{'='*60}")
            print(make_prompt(building, info, anim_type))
