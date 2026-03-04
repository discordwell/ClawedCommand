#!/usr/bin/env python3
"""Batch generate sprites via ChatGPT + AppleScript download.

Sends prompts via MCP (claude-in-chrome), downloads via AppleScript.
Requires "Allow JavaScript from Apple Events" enabled in Chrome.

Usage:
  python3 batch_sprites.py [start_index]
  python3 batch_sprites.py --regen-bad
"""
import random
import subprocess, sys, time, shutil
from pathlib import Path

import numpy as np
from PIL import Image

from image_utils import (
    remove_background,
    validate_sprite_quality,
    wait_for_stable_image,
)

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
RAW_DIR = PROJECT / "tools/asset_pipeline/raw/units"
OUT_DIR = PROJECT / "assets/sprites/units"

# Faction membership for style reference selection
FACTIONS = {
    "murder": ["murder_scrounger", "sentinel", "rookclaw", "magpike", "magpyre",
               "jaycaller", "jayflicker", "dusktalon", "hootseer", "corvus_rex"],
    "seekers": ["delver", "ironhide", "cragback", "warden", "sapjaw",
                "wardenmother", "seeker_tunneler", "embermaw", "dustclaw", "gutripper"],
    "croak": ["ponderer", "regeneron", "broodmother", "gulper", "eftsaber",
              "croaker", "leapfrog", "shellwarden", "bogwhisper", "murk_commander"],
    "llama": ["scrounger", "bandit", "heap_titan", "glitch_rat", "patch_possum",
              "grease_monkey", "dead_drop_unit", "wrecker", "dumpster_diver", "junkyard_king"],
}

# Reverse lookup: slug → faction name
SLUG_TO_FACTION = {}
for faction, members in FACTIONS.items():
    for slug in members:
        SLUG_TO_FACTION[slug] = faction

# All 40 sprites: (slug, animal, description)
SPRITES = [
    # Murder (Corvids) — 10 units
    ("murder_scrounger", "Crow", "scruffy crow with a burlap satchel, picking at shiny debris"),
    ("sentinel", "Crow", "sleek crow perched on a small lookout post, keen eyes scanning"),
    ("rookclaw", "Crow", "muscular crow mid-dive, talons extended, aggressive pose"),
    ("magpike", "Magpie", "magpie with iridescent wings, carrying stolen trinkets in beak"),
    ("magpyre", "Magpie", "dark magpie with crossed wires wrapped around body, saboteur tools"),
    ("jaycaller", "Blue jay", "bright blue jay with chest puffed out, wings spread calling"),
    ("jayflicker", "Shimmering jay", "shimmering jay with translucent afterimage copies flanking it, dynamic motion pose"),
    ("dusktalon", "Owl", "dark owl on ground, hunched, glowing amber eyes, shadow cloak"),
    ("hootseer", "Owl", "large owl with head rotated, glowing concentric eye rings"),
    ("corvus_rex", "Armored crow", "massive armored crow in salvaged combat plating, glowing eye visor"),
    # Seekers of the Deep (Badgers) — 10 units
    ("delver", "Mole", "squat mole with oversized digging claws, miner helmet"),
    ("ironhide", "Badger", "broad-shouldered badger with heavy shield, standing firm"),
    ("cragback", "Badger", "massive badger carrying a boulder mortar on its back"),
    ("warden", "Badger", "armored badger in defensive stance, watching intently"),
    ("sapjaw", "Badger", "lean badger with oversized jaw, crouched for a bite"),
    ("wardenmother", "Badger in exosuit", "badger in bulky repurposed mining exosuit, glowing chest core"),
    ("seeker_tunneler", "Mole", "streamlined mole mid-burrow, dirt spraying behind"),
    ("embermaw", "Wolverine", "small fierce wolverine carrying an incendiary launcher"),
    ("dustclaw", "Mole", "quick mole emerging from a dust cloud, alert pose"),
    ("gutripper", "Wolverine", "wild-eyed wolverine, claws extended, feral stance"),
    # Croak (Axolotls) — 10 units
    ("ponderer", "Axolotl", "serene axolotl near a shallow pool, gathering posture"),
    ("regeneron", "Axolotl", "small scrappy axolotl with a regenerating limb, combat-ready"),
    ("broodmother", "Axolotl", "large nurturing axolotl surrounded by tiny spawn"),
    ("gulper", "Axolotl", "massive-jawed axolotl mid-gulp, bulky body"),
    ("eftsaber", "Newt", "sleek newt with twin poison daggers, assassin crouch"),
    ("croaker", "Frog", "stocky frog with a mortar tube strapped to its back"),
    ("leapfrog", "Frog", "athletic frog mid-leap, legs extended, dynamic pose"),
    ("shellwarden", "Turtle", "armored turtle hunched behind shell, defensive stance"),
    ("bogwhisper", "Axolotl", "mystical axolotl with swirling water tendrils around paws"),
    ("murk_commander", "Axolotl", "large axolotl in salvaged tech armor, command antenna"),
    # LLAMA (Raccoons) — 10 units
    ("scrounger", "Raccoon", "raccoon rummaging through a junk pile, carrying scrap"),
    ("bandit", "Raccoon", "sneaky raccoon with a bandana, holding stolen goods"),
    ("heap_titan", "Raccoon", "massive raccoon in bolted-together scrap armor plating"),
    ("glitch_rat", "Rat", "wiry rat with sparking exposed wires, manic grin"),
    ("patch_possum", "Possum", "possum with duct tape bandolier, medic cross patch"),
    ("grease_monkey", "Raccoon", "raccoon with goggles and a junk launcher slung over shoulder"),
    ("dead_drop_unit", "Raccoon", "hooded raccoon in shadow, ear to ground, stealth pose"),
    ("wrecker", "Raccoon", "burly raccoon with a crowbar, demolition harness"),
    ("dumpster_diver", "Raccoon", "raccoon climbing out of a dumpster, holding treasure"),
    ("junkyard_king", "Raccoon", "large raccoon on throne of scrap, crown of bent forks, regal pose"),
]


# Global: (window_index, tab_index) for the ChatGPT tab we're working in.
# Set by find_and_focus_mcp_tab(), used by applescript_js().
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


def find_and_focus_mcp_tab():
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
        # After set index of window w to 1, window is now at index 1
        _target_tab = (1, int(parts[1]))
    return loc


def open_new_chat():
    """Navigate our tracked tab to a new ChatGPT chat."""
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js('document.querySelector("#prompt-textarea") ? "ready" : "loading"')
        if "ready" in r:
            return True
    return False


def check_rate_limit() -> bool:
    """Check if ChatGPT is showing a rate limit message. Returns True if rate-limited."""
    r = applescript_js('''
(function() {
    var body = document.body ? document.body.innerText : "";
    if (body.match(/rate limit|too many|try again later|usage cap/i)) return "rate_limited";
    return "ok";
})()
''')
    return "rate_limited" in r


_good_sprites_cache: dict[str, list[str]] = {}

def get_good_sprites(faction: str) -> list[str]:
    """Return slugs of sprites that pass QC for a given faction. Cached."""
    if faction in _good_sprites_cache:
        return _good_sprites_cache[faction]
    good = []
    for slug in FACTIONS.get(faction, []):
        p = OUT_DIR / f"{slug}_idle.png"
        if p.exists():
            img = Image.open(str(p)).convert("RGBA")
            passed, _ = validate_sprite_quality(img, sprite_type="idle")
            if passed:
                good.append(slug)
    _good_sprites_cache[faction] = good
    return good


def get_style_refs(slug: str) -> list[str]:
    """Get 3 style reference paths: 2 same-faction + 1 cross-faction.
    Falls back gracefully if fewer are available."""
    faction = SLUG_TO_FACTION.get(slug)
    if not faction:
        return []

    # Same-faction good sprites (excluding self)
    same = [s for s in get_good_sprites(faction) if s != slug]

    # Cross-faction good sprites
    cross = []
    for other_faction in FACTIONS:
        if other_faction != faction:
            cross.extend(get_good_sprites(other_faction))

    refs = []
    # Pick up to 2 same-faction
    if same:
        refs.extend(random.sample(same, min(2, len(same))))
    # Pick 1 cross-faction
    if cross:
        refs.append(random.choice(cross))

    return [str(OUT_DIR / f"{s}_idle.png") for s in refs]


def upload_style_references(ref_paths: list[str]) -> int:
    """Upload multiple style reference images to the ChatGPT chat.
    Returns count of successfully uploaded references."""
    import base64

    if not ref_paths:
        return 0

    # Click "Add files" button first
    applescript_js('''
(function() {
    var addBtn = document.querySelector('button[aria-label="Add files and more"]');
    if (addBtn) addBtn.click();
})()
''')
    time.sleep(1)

    uploaded = 0
    for ref_path in ref_paths:
        if not Path(ref_path).exists():
            print(f"    Ref not found: {ref_path}")
            continue

        with open(ref_path, "rb") as f:
            b64 = base64.b64encode(f.read()).decode()

        ref_name = Path(ref_path).stem
        js_inject = f'''
(function() {{
    var b64 = "{b64}";
    var byteChars = atob(b64);
    var byteArray = new Uint8Array(byteChars.length);
    for (var i = 0; i < byteChars.length; i++) {{
        byteArray[i] = byteChars.charCodeAt(i);
    }}
    var blob = new Blob([byteArray], {{type: "image/png"}});
    var file = new File([blob], "{ref_name}.png", {{type: "image/png"}});
    var dt = new DataTransfer();
    dt.items.add(file);
    var fileInput = document.querySelector('input[type="file"]');
    if (fileInput) {{
        fileInput.files = dt.files;
        fileInput.dispatchEvent(new Event("change", {{bubbles: true}}));
        return "uploaded";
    }}
    return "no_file_input";
}})()
'''
        r = applescript_js(js_inject)
        if "uploaded" in r:
            uploaded += 1
            time.sleep(1.5)  # Wait between uploads

    if uploaded > 0:
        print(f"    {uploaded} style reference(s) uploaded")
        time.sleep(1)
    return uploaded


def send_prompt(animal: str, description: str, ref_paths: list[str] | None = None) -> bool:
    """Send sprite generation prompt with faction-specific style references."""
    has_refs = ref_paths and len(ref_paths) > 0
    if has_refs:
        upload_style_references(ref_paths)

    style_line = "Use the attached images as STYLE REFERENCES. Match their art style: clean vector art, bold dark outlines, flat colors, isometric perspective. The character should look like it belongs in the same faction/army." if has_refs else ""

    js_fill = f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    var lines = [
        "Generate a 128x128 isometric sprite for a 2D RTS game.",
        "{style_line}",
        "",
        "Subject: {animal} — {description}. Facing south-east.",
        "",
        "Requirements:",
        "- Isometric view (~30 degrees), facing south-east",
        "- Transparent PNG background",
        "- Body/fur/feathers in neutral gray (#B0B0B0-#D0D0D0) — team color applied in-engine",
        "- Only accent colors on eyes, equipment, special features",
        "- Clean vector art, flat colors, bold dark outlines (2-3px)",
        "- Into the Breach / Northgard aesthetic",
        "- No gradients, 2-3 value steps per hue",
        "",
        "Generate ONLY this one image. Do not add extra characters."
    ];
    textarea.innerHTML = lines.map(function(l) {{ return "<p>" + (l || "<br>") + "</p>"; }}).join("");
    textarea.dispatchEvent(new Event("input", {{bubbles: true}}));
    return "ok";
}})()
'''
    r = applescript_js(js_fill)
    if "ok" not in r:
        return False

    # Small delay then click send in a separate call (setTimeout doesn't fire reliably)
    time.sleep(0.5)
    js_send = '''
(function() {
    var btn = document.querySelector('button[data-testid="send-button"]') || document.querySelector('button[aria-label="Send prompt"]');
    if (btn) { btn.click(); return "sent"; }
    return "no_button";
})()
'''
    r = applescript_js(js_send)
    return "sent" in r


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate a fully rendered, stable image."""
    return wait_for_stable_image(applescript_js, timeout=timeout, settle_time=8)


def download_and_process(slug: str) -> bool:
    """Download first generated image via curl, apply rembg if needed, QC gate, process to 128x128."""
    # Get image URL and cookies via AppleScript JS
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

    cookies = applescript_js("document.cookie")

    # Download via curl with browser cookies
    dl_path = Path.home() / "Downloads" / f"sprite_{slug}.png"
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl download failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 5000:
        print(f"    Downloaded file too small: {dl_path.stat().st_size} bytes")
        dl_path.unlink()
        return False

    # Copy raw
    raw_path = RAW_DIR / f"{slug}_raw.png"
    shutil.copy2(dl_path, raw_path)

    # Process to 128x128
    img = Image.open(str(raw_path)).convert("RGBA")

    # rembg fallback: if image is fully opaque, ChatGPT didn't give transparency
    alpha = np.array(img.split()[-1])
    if alpha.min() > 200:
        print("    Fully opaque — applying rembg...")
        img = remove_background(img)

    bbox = img.getbbox()
    if not bbox:
        print(f"    Empty image (no bounding box)")
        dl_path.unlink()
        return False

    cropped = img.crop(bbox)
    cropped.thumbnail((128, 128), Image.LANCZOS)
    canvas = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
    x = (128 - cropped.width) // 2
    y = (128 - cropped.height) // 2
    canvas.paste(cropped, (x, y))

    # Quality gate on final 128x128 output (downscaling concentrates edge detail,
    # so QC must run after resize, not on the raw 1024x1024 rembg output)
    passed, reason = validate_sprite_quality(canvas, sprite_type="idle")
    if not passed:
        print(f"    QC FAIL: {reason}")
        dl_path.unlink(missing_ok=True)
        return False

    out_path = OUT_DIR / f"{slug}_idle.png"
    canvas.save(str(out_path))
    print(f"    {cropped.width}x{cropped.height} -> 128x128 ({reason})")

    dl_path.unlink(missing_ok=True)
    return True


def generate_one(slug: str, animal: str, description: str) -> bool:
    """Full pipeline for one sprite, with up to 2 retries on QC failure."""
    # Get faction-specific style references (2 same-faction + 1 cross-faction)
    ref_paths = get_style_refs(slug)
    faction = SLUG_TO_FACTION.get(slug, "unknown")
    ref_names = [Path(r).stem for r in ref_paths]

    for attempt in range(3):
        if attempt > 0:
            print(f"    Retry {attempt}/2...")

        if attempt == 0:
            print(f"\n  [{slug}] {animal} (faction={faction}, refs={ref_names})")

        # Check for rate limiting
        if check_rate_limit():
            print("    Rate limited — pausing 5 minutes...")
            time.sleep(300)

        # Navigate to new chat
        if not open_new_chat():
            print("    Failed to load new chat")
            continue

        # Send prompt with faction-specific references
        if not send_prompt(animal, description, ref_paths=ref_paths):
            print("    Failed to send prompt")
            continue
        print("    Prompt sent, waiting for generation...")

        # Wait for image
        if not wait_for_image():
            print("    Timeout waiting for image")
            continue

        # Download and process (includes QC gate)
        if download_and_process(slug):
            return True

    return False


def scan_bad_sprites() -> list[str]:
    """Scan sprites from SPRITES list: return slugs that are missing or fail QC."""
    bad = []
    for slug, animal, desc in SPRITES:
        p = OUT_DIR / f"{slug}_idle.png"
        if not p.exists():
            print(f"  MISSING: {slug}")
            bad.append(slug)
            continue

        img = Image.open(str(p)).convert("RGBA")
        passed, reason = validate_sprite_quality(img, sprite_type="idle")
        if not passed:
            print(f"  BAD: {slug} — {reason}")
            bad.append(slug)
        else:
            print(f"  ok:  {slug} — {reason}")
    return bad


def main():
    # --regen-bad mode: scan, delete bad, regenerate
    if "--regen-bad" in sys.argv:
        print("Scanning for bad sprites...")
        bad_slugs = scan_bad_sprites()
        if not bad_slugs:
            print("\nAll sprites pass QC!")
            return

        print(f"\n{len(bad_slugs)} bad sprites found. Deleting and regenerating...")

        # Delete bad outputs so the skip-if-exists check won't skip them
        for slug in bad_slugs:
            out = OUT_DIR / f"{slug}_idle.png"
            if out.exists():
                out.unlink()
                print(f"  Deleted {out.name}")

        # Find ChatGPT tab
        loc = find_and_focus_mcp_tab()
        print(f"ChatGPT tab: {loc}")

        # Build lookup for sprite data
        sprite_map = {s[0]: s for s in SPRITES}

        done = 0
        failed = []
        for slug in bad_slugs:
            _, animal, desc = sprite_map[slug]
            if generate_one(slug, animal, desc):
                done += 1
                print(f"    Regenerated! ({done}/{len(bad_slugs)})")
            else:
                failed.append(slug)
                print(f"    REGEN FAILED")

        print(f"\nRegenerated: {done}/{len(bad_slugs)}")
        if failed:
            print(f"Still failed: {', '.join(failed)}")
        return

    # Normal mode
    start_idx = int(sys.argv[1]) if len(sys.argv) > 1 else 0

    # Check which sprites already exist
    existing = set()
    for p in OUT_DIR.glob("*_idle.png"):
        existing.add(p.stem.replace("_idle", ""))

    # Find the MCP window
    loc = find_and_focus_mcp_tab()
    print(f"MCP window: {loc}")

    done = 0
    failed = []
    for i, (slug, animal, desc) in enumerate(SPRITES):
        if i < start_idx:
            continue
        if slug in existing:
            print(f"  [{slug}] already exists, skipping")
            done += 1
            continue

        if generate_one(slug, animal, desc):
            done += 1
            print(f"    Done! ({done}/{len(SPRITES)})")
        else:
            failed.append(slug)
            print(f"    FAILED")

    print(f"\nComplete: {done}/{len(SPRITES)} sprites")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
