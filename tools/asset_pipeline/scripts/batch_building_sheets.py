#!/usr/bin/env python3
"""Batch generate building construction and ambient animation sprite sheets.

Generates 4096x1024 sprite sheets (4 frames of 1024x1024) for buildings:
- Construction sequence: progress-driven frames showing the building being built
- Ambient idle loop: subtle looping animation (smoke, lights, gears, etc.)

Uses ChatGPT via AppleScript browser automation, same pattern as regen_sprites.py.
Uploads existing building idle sprite as reference for style consistency.

Usage:
    python batch_building_sheets.py --dry-run                       Preview all prompts
    python batch_building_sheets.py --faction catgpt --dry-run      Preview one faction
    python batch_building_sheets.py --faction catgpt                Generate catGPT buildings
    python batch_building_sheets.py --faction catgpt --type construct  Only construction sheets
    python batch_building_sheets.py --building the_box              Single building (both sheets)
    python batch_building_sheets.py --all                           Generate all 96 sheets
"""
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image

# ── Setup paths ──────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).resolve().parent
PIPELINE_ROOT = SCRIPT_DIR.parent
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
BUILDING_SPRITE_DIR = PROJECT_ROOT / "assets" / "sprites" / "buildings"
PROMPT_DIR = PIPELINE_ROOT / "config" / "prompts"

# Add scripts dir to path so we can import siblings
sys.path.insert(0, str(SCRIPT_DIR))

from image_utils import wait_for_stable_image

# ── Building definitions ─────────────────────────────────────

# All 48 buildings by faction (8 per faction), matching ALL_BUILDING_KINDS order
BUILDINGS = {
    "catgpt": [
        ("the_box", "The Box", "Hq"),
        ("cat_tree", "Cat Tree", "Barracks"),
        ("fish_market", "Fish Market", "ResourceDepot"),
        ("litter_box", "Litter Box", "SupplyDepot"),
        ("server_rack", "Server Rack", "TechBuilding"),
        ("scratching_post", "Scratching Post", "Research"),
        ("cat_flap", "Cat Flap", "Garrison"),
        ("laser_pointer", "Laser Pointer", "DefenseTower"),
    ],
    "murder": [
        ("the_parliament", "The Parliament", "Hq"),
        ("rookery", "Rookery", "Barracks"),
        ("carrion_cache", "Carrion Cache", "ResourceDepot"),
        ("antenna_array", "Antenna Array", "TechBuilding"),
        ("panopticon", "Panopticon", "Research"),
        ("nest_box", "Nest Box", "SupplyDepot"),
        ("thorn_hedge", "Thorn Hedge", "Garrison"),
        ("watchtower", "Watchtower", "DefenseTower"),
    ],
    "clawed": [
        ("the_burrow", "The Burrow", "Hq"),
        ("nesting_box", "Nesting Box", "Barracks"),
        ("seed_vault", "Seed Vault", "ResourceDepot"),
        ("junk_transmitter", "Junk Transmitter", "TechBuilding"),
        ("gnaw_lab", "Gnaw Lab", "Research"),
        ("warren_expansion", "Warren Expansion", "SupplyDepot"),
        ("mousehole", "Mousehole", "Garrison"),
        ("squeak_tower", "Squeak Tower", "DefenseTower"),
    ],
    "seekers": [
        ("the_sett", "The Sett", "Hq"),
        ("war_hollow", "War Hollow", "Barracks"),
        ("burrow_depot", "Burrow Depot", "ResourceDepot"),
        ("core_tap", "Core Tap", "TechBuilding"),
        ("claw_marks", "Claw Marks", "Research"),
        ("deep_warren", "Deep Warren", "SupplyDepot"),
        ("bulwark_gate", "Bulwark Gate", "Garrison"),
        ("slag_thrower", "Slag Thrower", "DefenseTower"),
    ],
    "croak": [
        ("the_grotto", "The Grotto", "Hq"),
        ("spawning_pools", "Spawning Pools", "Barracks"),
        ("lily_market", "Lily Market", "ResourceDepot"),
        ("sunken_server", "Sunken Server", "TechBuilding"),
        ("fossil_stones", "Fossil Stones", "Research"),
        ("reed_bed", "Reed Bed", "SupplyDepot"),
        ("tidal_gate", "Tidal Gate", "Garrison"),
        ("spore_tower", "Spore Tower", "DefenseTower"),
    ],
    "llama": [
        ("the_dumpster", "The Dumpster", "Hq"),
        ("scrap_heap", "Scrap Heap", "ResourceDepot"),
        ("chop_shop", "Chop Shop", "Barracks"),
        ("junk_server", "Junk Server", "TechBuilding"),
        ("tinker_bench", "Tinker Bench", "Research"),
        ("trash_pile", "Trash Pile", "SupplyDepot"),
        ("dumpster_relay", "Dumpster Relay", "Garrison"),
        ("tetanus_tower", "Tetanus Tower", "DefenseTower"),
    ],
}

# Ambient animation details per building role
AMBIENT_DETAILS = {
    "Hq": "Flag waving gently, window lights flickering on and off",
    "Barracks": "Smoke puffing from chimney, door opening and closing slightly",
    "ResourceDepot": "Small workers moving crates in and out of doorway",
    "SupplyDepot": "Conveyor belt or stacking animation, crates shifting",
    "TechBuilding": "Antenna rotating slowly, LED lights blinking in sequence",
    "Research": "Glowing runes or screens pulsing with light",
    "Garrison": "Gate raising and lowering slightly, guard pacing",
    "DefenseTower": "Turret scanning left to right, spotlight sweeping",
}

# Faction display names
FACTION_NAMES = {
    "catgpt": "catGPT (cats)",
    "murder": "The Murder (corvids)",
    "clawed": "The Clawed (mice)",
    "seekers": "Seekers of the Deep (badgers)",
    "croak": "Croak (axolotls)",
    "llama": "LLAMA (raccoons)",
}

# ── ChatGPT browser automation ───────────────────────────────
# (Same pattern as regen_sprites.py)

_target_tab = (1, 1)


def applescript_js(js_code: str) -> str:
    """Execute JS in our tracked ChatGPT tab via AppleScript."""
    tmp = Path("/tmp/chrome_js.js")
    tmp.write_text(js_code)
    w, t = _target_tab
    result = subprocess.run(["osascript", "-e", f'''
    tell application "Google Chrome"
        tell tab {t} of window {w}
            execute javascript (read POSIX file "/tmp/chrome_js.js" as «class utf8»)
        end tell
    end tell
    '''], capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        return f"ERROR: {result.stderr.strip()[:100]}"
    return result.stdout.strip()


def find_and_focus_chatgpt_tab():
    """Find the ChatGPT tab, bring its window to front, and track its index."""
    global _target_tab
    result = subprocess.run(["osascript", "-e", '''
    tell application "Google Chrome"
        set wCount to count of windows
        repeat with w from 1 to wCount
            set tCount to count of tabs of window w
            repeat with t from 1 to tCount
                try
                    set tabURL to URL of tab t of window w
                    if tabURL contains "chatgpt.com" then
                        set active tab index of window w to t
                        set index of window w to 1
                        return (w as text) & "," & (t as text)
                    end if
                end try
            end repeat
        end repeat
        return "not_found"
    end tell
    '''], capture_output=True, text=True, timeout=10)
    loc = result.stdout.strip()
    if "," in loc:
        parts = loc.split(",")
        _target_tab = (1, int(parts[1]))
    return loc


def open_new_chat():
    """Navigate to a fresh ChatGPT conversation."""
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js('document.querySelector("#prompt-textarea") ? "ready" : "loading"')
        if "ready" in r:
            return True
    return False


def check_rate_limit() -> bool:
    """Returns True if ChatGPT is showing a rate limit message."""
    r = applescript_js('''
(function() {
    var body = document.body ? document.body.innerText : "";
    if (body.match(/rate limit|too many|try again later|usage cap/i)) return "rate_limited";
    return "ok";
})()
''')
    return "rate_limited" in r


def send_prompt(prompt_text: str) -> bool:
    """Send a prompt to ChatGPT using execCommand('insertText')."""
    import base64
    escaped = json.dumps(prompt_text)

    js_fill = f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    textarea.focus();
    document.execCommand("selectAll");
    document.execCommand("delete");
    document.execCommand("insertText", false, {escaped});
    return "ok";
}})()
'''
    r = applescript_js(js_fill)
    if "ok" not in r:
        return False

    time.sleep(0.5)

    js_send = '''
(function() {
    var btn = document.querySelector('button[data-testid="send-button"]') || document.querySelector('button[aria-label="Send prompt"]');
    if (btn) { btn.click(); return "sent"; }
    return "no_button";
})()
'''
    r = applescript_js(js_send)
    if "sent" in r:
        return True

    time.sleep(0.3)
    r2 = applescript_js('''
(function() {
    var textarea = document.querySelector("#prompt-textarea");
    if (textarea) {
        textarea.dispatchEvent(new KeyboardEvent("keydown", {key: "Enter", code: "Enter", bubbles: true}));
        return "enter_sent";
    }
    return "no_textarea";
})()
''')
    return "enter_sent" in r2


def upload_reference_image(image_path: Path) -> bool:
    """Upload a reference image to ChatGPT via chunked base64 + DragEvent drop."""
    import base64

    if not image_path.exists():
        print(f"    Reference image not found: {image_path}")
        return False

    img_data = image_path.read_bytes()
    b64 = base64.b64encode(img_data).decode("ascii")

    applescript_js("window._imgB64 = '';")

    chunk_size = 3000
    for i in range(0, len(b64), chunk_size):
        chunk = b64[i:i + chunk_size]
        escaped_chunk = json.dumps(chunk)
        applescript_js(f"window._imgB64 += {escaped_chunk};")

    js_drop = '''
(function() {
    var b64 = window._imgB64;
    if (!b64) return "no_data";
    var binary = atob(b64);
    var arr = new Uint8Array(binary.length);
    for (var i = 0; i < binary.length; i++) arr[i] = binary.charCodeAt(i);
    var blob = new Blob([arr], {type: "image/png"});
    var file = new File([blob], "reference.png", {type: "image/png"});
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    var dt = new DataTransfer();
    dt.items.add(file);
    var dropEvent = new DragEvent("drop", {
        bubbles: true, cancelable: true, dataTransfer: dt
    });
    textarea.dispatchEvent(dropEvent);
    window._imgB64 = "";
    return "uploaded";
})()
'''
    r = applescript_js(js_drop)
    if "uploaded" in r:
        time.sleep(1)
        return True
    print(f"    Image upload failed: {r}")
    return False


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate a fully rendered, stable image."""
    return wait_for_stable_image(applescript_js, timeout=timeout, settle_time=8)


def download_image(slug: str, sheet_type: str) -> bool:
    """Download generated building sheet image and save to assets/sprites/buildings/."""
    js_url = '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length === 0) return "no_images";
    return imgs[0].src;
})()
'''
    img_url = applescript_js(js_url)
    if "no_images" in img_url or not img_url.startswith("http"):
        print(f"    No image URL found: {img_url}")
        return False

    from urllib.parse import urlparse
    allowed_domains = ("oaiusercontent.com", "chatgpt.com", "openai.com")
    parsed = urlparse(img_url)
    if not any(parsed.hostname and parsed.hostname.endswith(d) for d in allowed_domains):
        print(f"    Untrusted image domain: {parsed.hostname}")
        return False

    cookies = applescript_js("document.cookie")

    dl_path = Path.home() / "Downloads" / f"building_{slug}_{sheet_type}.png"
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 10000:
        print(f"    Downloaded file too small: {dl_path.stat().st_size} bytes")
        dl_path.unlink()
        return False

    # Verify dimensions (should be 4096x1024 or close)
    try:
        img = Image.open(str(dl_path))
        w, h = img.size
        # Accept some variation; ChatGPT may produce slightly different sizes
        if w < 2048 or h < 512:
            print(f"    Image too small: {w}x{h} (expected ~4096x1024)")
            dl_path.unlink()
            return False

        # Resize to exact 4096x1024 if not already
        if w != 4096 or h != 1024:
            print(f"    Resizing {w}x{h} -> 4096x1024")
            img = img.resize((4096, 1024), Image.LANCZOS)

        # Save to output directory
        BUILDING_SPRITE_DIR.mkdir(parents=True, exist_ok=True)
        out_path = BUILDING_SPRITE_DIR / f"{slug}_{sheet_type}.png"
        img.save(str(out_path))
        print(f"    Saved: {out_path.name} ({out_path.stat().st_size // 1024}KB)")

    except Exception as e:
        print(f"    Image processing error: {e}")
        dl_path.unlink(missing_ok=True)
        return False

    dl_path.unlink(missing_ok=True)
    return True


# ── Prompt builders ──────────────────────────────────────────

def load_prompt_template(template_name: str) -> str:
    """Load a prompt template file."""
    path = PROMPT_DIR / template_name
    if not path.exists():
        raise FileNotFoundError(f"Prompt template not found: {path}")
    return path.read_text()


def build_construct_prompt(slug: str, name: str, faction: str) -> str:
    """Build a construction sequence prompt for a building."""
    template = load_prompt_template("building_construct.txt")
    faction_display = FACTION_NAMES.get(faction, faction)

    # Load description from catalog if available
    desc = get_building_description(slug)

    return template.replace("{name}", name).replace("{faction}", faction_display).replace("{description}", desc)


def build_ambient_prompt(slug: str, name: str, faction: str, role: str) -> str:
    """Build an ambient animation prompt for a building."""
    template = load_prompt_template("building_ambient.txt")
    faction_display = FACTION_NAMES.get(faction, faction)
    ambient_detail = AMBIENT_DETAILS.get(role, "Subtle ambient motion")

    desc = get_building_description(slug)

    return (template
            .replace("{name}", name)
            .replace("{faction}", faction_display)
            .replace("{description}", desc)
            .replace("{ambient_detail}", ambient_detail))


def get_building_description(slug: str) -> str:
    """Get building description from asset catalog."""
    try:
        import yaml
        catalog_path = PIPELINE_ROOT / "config" / "asset_catalog.yaml"
        with open(catalog_path) as f:
            catalog = yaml.safe_load(f)
        for category in catalog.values():
            if isinstance(category, dict) and slug in category:
                entry = category[slug]
                params = entry.get("params", {})
                return params.get("description", f"A {slug.replace('_', ' ')} building")
        return f"A {slug.replace('_', ' ')} building"
    except Exception:
        return f"A {slug.replace('_', ' ')} building"


# ── Generation pipeline ─────────────────────────────────────

def generate_one(slug: str, name: str, faction: str, role: str,
                 sheet_type: str) -> bool:
    """Generate one building animation sheet (construct or ambient)."""
    if sheet_type == "construct":
        prompt = build_construct_prompt(slug, name, faction)
    else:
        prompt = build_ambient_prompt(slug, name, faction, role)

    ref_path = BUILDING_SPRITE_DIR / f"{slug}.png"

    for attempt in range(3):
        if attempt > 0:
            print(f"    Retry {attempt}/2...")

        if check_rate_limit():
            print("    Rate limited -- pausing 5 minutes...")
            time.sleep(300)

        if not open_new_chat():
            print("    Failed to load new chat")
            continue

        # Upload reference idle sprite
        if ref_path.exists():
            print(f"    Uploading reference: {ref_path.name}")
            if not upload_reference_image(ref_path):
                print("    Warning: reference upload failed, continuing without it")
        else:
            print(f"    No reference sprite: {ref_path.name}")

        if not send_prompt(prompt):
            print("    Failed to send prompt")
            continue
        print("    Prompt sent, waiting for generation...")

        if not wait_for_image():
            print("    Timeout waiting for image")
            continue

        if download_image(slug, sheet_type):
            return True

    return False


# ── Main ─────────────────────────────────────────────────────

def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Batch generate building construction and ambient animation sheets"
    )
    parser.add_argument("--faction", "-f",
                        help="Filter by faction (catgpt, murder, clawed, seekers, croak, llama)")
    parser.add_argument("--building", "-b",
                        help="Generate sheets for a single building (by slug)")
    parser.add_argument("--type", "-t", choices=["construct", "ambient"],
                        help="Only generate one type of sheet")
    parser.add_argument("--all", action="store_true",
                        help="Generate all 96 sheets (48 buildings x 2 types)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print prompts without generating")
    args = parser.parse_args()

    # Build target list: [(slug, name, faction, role, sheet_type), ...]
    targets = []
    sheet_types = ["construct", "ambient"] if not args.type else [args.type]

    if args.building:
        # Find the building in all factions
        found = False
        for faction, buildings in BUILDINGS.items():
            for slug, name, role in buildings:
                if slug == args.building:
                    for st in sheet_types:
                        targets.append((slug, name, faction, role, st))
                    found = True
                    break
            if found:
                break
        if not found:
            print(f"Error: building '{args.building}' not found")
            all_slugs = [s for bs in BUILDINGS.values() for s, _, _ in bs]
            print(f"Available: {', '.join(sorted(all_slugs))}")
            sys.exit(1)
    elif args.faction:
        faction = args.faction.lower().strip()
        if faction not in BUILDINGS:
            print(f"Error: unknown faction '{faction}'")
            print(f"Available: {', '.join(BUILDINGS.keys())}")
            sys.exit(1)
        for slug, name, role in BUILDINGS[faction]:
            for st in sheet_types:
                targets.append((slug, name, faction, role, st))
    elif args.all:
        for faction, buildings in BUILDINGS.items():
            for slug, name, role in buildings:
                for st in sheet_types:
                    targets.append((slug, name, faction, role, st))
    else:
        parser.print_help()
        sys.exit(1)

    # Skip already-generated sheets
    skip_existing = []
    remaining = []
    for t in targets:
        slug, name, faction, role, st = t
        out_path = BUILDING_SPRITE_DIR / f"{slug}_{st}.png"
        if out_path.exists() and not args.dry_run:
            skip_existing.append(f"{slug}_{st}")
        else:
            remaining.append(t)

    if skip_existing:
        print(f"Skipping {len(skip_existing)} existing sheets: {', '.join(skip_existing[:5])}{'...' if len(skip_existing) > 5 else ''}")
    targets = remaining

    print(f"\n{'Previewing' if args.dry_run else 'Generating'} {len(targets)} building sheets")

    if args.dry_run:
        for slug, name, faction, role, st in targets:
            if st == "construct":
                prompt = build_construct_prompt(slug, name, faction)
            else:
                prompt = build_ambient_prompt(slug, name, faction, role)

            ref_path = BUILDING_SPRITE_DIR / f"{slug}.png"
            has_ref = ref_path.exists()

            print(f"\n{'='*60}")
            print(f"  {slug}_{st}  [{st.upper()}]  faction={faction}  role={role}")
            print(f"  Reference: {'YES' if has_ref else 'NO'} ({ref_path.name})")
            print(f"{'='*60}")
            print(prompt)
            print(f"\n  [{len(prompt)} chars]")
        return

    # Live generation
    loc = find_and_focus_chatgpt_tab()
    if "not_found" in loc:
        print("Error: No ChatGPT tab found. Open chatgpt.com in Chrome first.")
        sys.exit(1)
    print(f"ChatGPT tab: {loc}")

    done = 0
    failed = []

    for i, (slug, name, faction, role, st) in enumerate(targets):
        print(f"\n[{i+1}/{len(targets)}] {slug}_{st} ({st}, faction={faction}, role={role})")

        if generate_one(slug, name, faction, role, st):
            done += 1
            print(f"    Generated! ({done}/{len(targets)})")
        else:
            failed.append(f"{slug}_{st}")
            print(f"    FAILED")

    print(f"\n{'='*40}")
    print(f"Complete: {done}/{len(targets)} sheets generated")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
