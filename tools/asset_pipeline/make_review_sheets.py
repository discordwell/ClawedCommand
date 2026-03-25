#!/usr/bin/env python3
"""Generate contact sheets for sprite review. One sheet per faction showing all units and buildings."""
import os
from PIL import Image

BASE = os.path.join(os.path.dirname(__file__), '..', '..', 'assets', 'sprites')
OUT = os.path.join(os.path.dirname(__file__), 'review_sheets')
os.makedirs(OUT, exist_ok=True)

UNITS = {
    'catgpt':  ['pawdler','nuisance','chonk','flying_fox','hisser','yowler','mouser','catnapper','ferret_sapper','mech_commander'],
    'clawed':  ['nibblet','swarmer','gnawer','shrieker','tunneler','sparks','quillback','whiskerwitch','plaguetail','warren_marshal'],
    'murder':  ['murder_scrounger','sentinel','rookclaw','magpike','magpyre','jaycaller','jayflicker','dusktalon','hootseer','corvus_rex'],
    'seekers': ['delver','ironhide','cragback','warden','sapjaw','wardenmother','seeker_tunneler','embermaw','dustclaw','gutripper'],
    'croak':   ['ponderer','regeneron','broodmother','gulper','eftsaber','croaker','leapfrog','shellwarden','bogwhisper','murk_commander'],
    'llama':   ['scrounger','bandit','heap_titan','glitch_rat','patch_possum','grease_monkey','dead_drop_unit','wrecker','dumpster_diver','junkyard_king'],
}

BUILDINGS = {
    'catgpt':  ['the_box','cat_tree','fish_market','litter_box','server_rack','scratching_post','cat_flap','laser_pointer'],
    'clawed':  ['the_burrow','nesting_box','seed_vault','junk_transmitter','gnaw_lab','warren_expansion','mousehole','squeak_tower'],
    'murder':  ['the_parliament','rookery','carrion_cache','antenna_array','panopticon','nest_box','thorn_hedge','watchtower'],
    'seekers': ['the_sett','war_hollow','burrow_depot','core_tap','claw_marks','deep_warren','bulwark_gate','slag_thrower'],
    'croak':   ['the_grotto','spawning_pools','lily_market','sunken_server','fossil_stones','reed_bed','tidal_gate','spore_tower'],
    'llama':   ['the_dumpster','scrap_heap','chop_shop','junk_server','tinker_bench','trash_pile','dumpster_relay','tetanus_tower'],
}

CELL = 96  # thumbnail size
PAD = 4
LABEL_H = 16  # space for row labels

def load_thumb(path, size=CELL):
    """Load and thumbnail an image. For sheets, show frame 1 only."""
    try:
        img = Image.open(path).convert('RGBA')
        w, h = img.size
        # If it's a sheet (wider than tall), crop first frame
        if w > h * 1.5:
            frame_w = w // 4
            img = img.crop((0, 0, frame_w, h))
        # If it's a large building (1024x1024), resize
        img.thumbnail((size, size), Image.LANCZOS)
        # Center on cell
        cell = Image.new('RGBA', (size, size), (20, 20, 40, 255))
        ox = (size - img.width) // 2
        oy = (size - img.height) // 2
        cell.paste(img, (ox, oy), img)
        return cell
    except Exception as e:
        # Return error placeholder
        cell = Image.new('RGBA', (size, size), (60, 20, 20, 255))
        return cell

def make_unit_sheet(faction_id, units):
    """Create contact sheet: 10 rows (units) × 3 cols (idle/walk/attack)."""
    cols = 3
    rows = len(units)
    w = cols * (CELL + PAD) + PAD + 120  # 120px for name labels
    h = rows * (CELL + PAD) + PAD + LABEL_H

    sheet = Image.new('RGBA', (w, h), (17, 17, 30, 255))

    for i, unit in enumerate(units):
        y = LABEL_H + i * (CELL + PAD) + PAD
        for j, suffix in enumerate(['idle', 'walk', 'attack']):
            x = 120 + j * (CELL + PAD) + PAD
            path = os.path.join(BASE, 'units', f'{unit}_{suffix}.png')
            thumb = load_thumb(path)
            sheet.paste(thumb, (x, y), thumb)

    return sheet

def make_building_sheet(faction_id, buildings):
    """Create contact sheet: 8 rows (buildings) × 3 cols (static/construct/ambient)."""
    cols = 3
    rows = len(buildings)
    w = cols * (CELL + PAD) + PAD + 120
    h = rows * (CELL + PAD) + PAD + LABEL_H

    sheet = Image.new('RGBA', (w, h), (17, 17, 30, 255))

    for i, bld in enumerate(buildings):
        y = LABEL_H + i * (CELL + PAD) + PAD
        for j, suffix in enumerate(['', '_construct', '_ambient']):
            x = 120 + j * (CELL + PAD) + PAD
            path = os.path.join(BASE, 'buildings', f'{bld}{suffix}.png')
            thumb = load_thumb(path)
            sheet.paste(thumb, (x, y), thumb)

    return sheet

# Add text labels using basic pixel drawing (no font dependency)
from PIL import ImageDraw, ImageFont

try:
    font = ImageFont.truetype("/System/Library/Fonts/SFNSMono.ttf", 11)
except:
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Menlo.ttc", 11)
    except:
        font = ImageFont.load_default()

try:
    header_font = ImageFont.truetype("/System/Library/Fonts/SFNSMono.ttf", 10)
except:
    header_font = font

for faction_id in UNITS:
    print(f"Generating {faction_id} units...")
    units = UNITS[faction_id]
    sheet = make_unit_sheet(faction_id, units)
    draw = ImageDraw.Draw(sheet)

    # Column headers
    headers = ['IDLE', 'WALK', 'ATTACK']
    for j, h in enumerate(headers):
        x = 120 + j * (CELL + PAD) + PAD + CELL // 2
        draw.text((x, 2), h, fill=(150, 150, 150), font=header_font, anchor='mt')

    # Row labels
    for i, unit in enumerate(units):
        y = LABEL_H + i * (CELL + PAD) + PAD + CELL // 2
        name = unit.replace('_', ' ').title()
        draw.text((4, y), name, fill=(180, 180, 200), font=font, anchor='lm')

    out_path = os.path.join(OUT, f'{faction_id}_units.png')
    sheet.save(out_path, optimize=True)
    print(f"  -> {out_path} ({os.path.getsize(out_path) // 1024}KB)")

for faction_id in BUILDINGS:
    print(f"Generating {faction_id} buildings...")
    buildings = BUILDINGS[faction_id]
    sheet = make_building_sheet(faction_id, buildings)
    draw = ImageDraw.Draw(sheet)

    # Column headers
    headers = ['STATIC', 'BUILD', 'AMBIENT']
    for j, h in enumerate(headers):
        x = 120 + j * (CELL + PAD) + PAD + CELL // 2
        draw.text((x, 2), h, fill=(150, 150, 150), font=header_font, anchor='mt')

    # Row labels
    for i, bld in enumerate(buildings):
        y = LABEL_H + i * (CELL + PAD) + PAD + CELL // 2
        name = bld.replace('_', ' ').title()
        draw.text((4, y), name, fill=(180, 180, 200), font=font, anchor='lm')

    out_path = os.path.join(OUT, f'{faction_id}_buildings.png')
    sheet.save(out_path, optimize=True)
    print(f"  -> {out_path} ({os.path.getsize(out_path) // 1024}KB)")

# General assets sheet
print("Generating general assets...")
TERRAIN = ['grass_base','dirt_base','sand_base','forest_base','water_base','road_base','rock_base','shallows_base','ramp_base','tech_ruins_base']
RESOURCES = ['berry_bush','fish_pond','gpu_deposit','monkey_mine']
PROJECTILES = ['explosive','generic','laser_beam','mech_shot','sonic_wave','spit']
PORTRAITS = [
    'ai_geppity','ai_claudeus_maximus','ai_gemineye','ai_deepseek','ai_grok','ai_llhama',
    'hero_felix_nine','hero_kelpie','hero_king_ringtail','hero_mother_granite',
    'hero_patches','hero_rex_solstice','hero_the_eternal','hero_thimble'
]

# Terrain + Resources row
general_items = []
for t in TERRAIN:
    general_items.append(('terrain', t, os.path.join(BASE, 'terrain', f'{t}.png')))
for r in RESOURCES:
    general_items.append(('resource', r, os.path.join(BASE, 'resources', f'{r}.png')))
for p in PROJECTILES:
    general_items.append(('projectile', p, os.path.join(BASE, 'projectiles', f'{p}.png')))
for p in PORTRAITS:
    general_items.append(('portrait', p, os.path.join(BASE, 'portraits', f'{p}.png')))

cols = 7
rows = (len(general_items) + cols - 1) // cols
w = cols * (CELL + PAD) + PAD
h = rows * (CELL + PAD + 14) + PAD

sheet = Image.new('RGBA', (w, h), (17, 17, 30, 255))
draw = ImageDraw.Draw(sheet)

for idx, (cat, name, path) in enumerate(general_items):
    r = idx // cols
    c = idx % cols
    x = c * (CELL + PAD) + PAD
    y = r * (CELL + PAD + 14) + PAD
    thumb = load_thumb(path)
    sheet.paste(thumb, (x, y), thumb)
    label = name.replace('_', ' ').replace(' base', '')[:14]
    draw.text((x + CELL // 2, y + CELL + 2), label, fill=(140, 140, 160), font=header_font, anchor='mt')

out_path = os.path.join(OUT, 'general.png')
sheet.save(out_path, optimize=True)
print(f"  -> {out_path} ({os.path.getsize(out_path) // 1024}KB)")

print("\nDone! Review sheets in:", OUT)
