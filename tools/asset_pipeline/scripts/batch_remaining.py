#!/usr/bin/env python3
"""Batch generate remaining assets via ChatGPT + AppleScript download.

Handles terrain, resources, portraits, and walk/attack animation sheets.
Requires "Allow JavaScript from Apple Events" enabled in Chrome.

Usage:
  python3 batch_remaining.py [category] [start_index]
  python3 batch_remaining.py portraits --regen-bad
  Categories: terrain, resources, portraits, walk, attack, all
"""
import subprocess, sys, time, shutil, base64
from pathlib import Path

import numpy as np
from PIL import Image

from image_utils import (
    remove_background,
    validate_sprite_quality,
    wait_for_stable_image,
)

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
SPRITE_DIR = PROJECT / "assets/sprites"

# Global: (window_index, tab_index) for the ChatGPT tab.
_target_tab = (1, 1)


# ── Asset definitions ────────────────────────────────────────────

TERRAIN = [
    ("grass_base", "Grass", "Lush pastoral meadow grass, soft green with subtle blade variation, the default map tile"),
    ("dirt_base", "Dirt", "Worn dirt ground with subtle paw-print impressions, cleared earth paths"),
    ("sand_base", "Sand", "Warm sandy ground at coastlines, fine-grained with occasional tiny shells"),
    ("forest_base", "Forest", "Dense forest floor with tree trunks rising into height zone, dappled leaf shadows"),
    ("water_base", "Deep Water", "Dark blue water surface with subtle wave ripples, impassable"),
    ("shallows_base", "Shallows", "Shallow translucent blue-green water with sandy bottom visible through"),
    ("rock_base", "Rock", "Solid gray rocky outcrop with cracks and lichen, impassable barrier"),
    ("ramp_base", "Ramp", "Sloped terrain connecting elevation levels, packed earth incline"),
    ("road_base", "Road", "Old-world cobblestone road, worn but functional paving stones"),
    ("tech_ruins_base", "Tech Ruins", "Cracked concrete with exposed circuitry, faint blue-green tech glow from old server farms"),
]

RESOURCES = [
    ("fish_pond", "Fish Pond", "Small pond teeming with jumping fish, splashing visible, primary Food source", 128),
    ("berry_bush", "Berry Bush", "Wild berry bush with bright red/purple berries, secondary Food source", 128),
    ("gpu_deposit", "GPU Deposit", "Old-world tech ruin with exposed circuit boards and glowing GPU chips in rubble", 128),
    ("monkey_mine", "Monkey Mine", "Neutral data center entrance with monkey graffiti and banana peels, NFT symbols scratched in walls", 128),
]

PORTRAITS = [
    # Hero portraits
    ("hero_kelpie", "Kelpie", "otter", "Unaligned", "Young otter with mischievous eyes, wet fur, blue-green glow near ear, scavenged utility vest"),
    ("hero_felix_nine", "Commander Felix Nine", "scarred tabby cat", "catGPT", "Scarred tabby in mech cockpit, long scar across left cheek, pragmatic dry expression, command antenna behind head"),
    ("hero_thimble", "Marshal Thimble", "mouse", "The Clawed", "Grizzled old mouse wearing a thimble as helmet, military coat collar, tired determined eyes, white whiskers"),
    ("hero_mother_granite", "Mother Granite", "badger", "Seekers of the Deep", "Ancient badger with wise eyes behind mining exosuit visor, classic badger stripe, infinite patience expression"),
    ("hero_rex_solstice", "Rex Solstice", "crow", "The Murder", "Massive augmented crow with calculating eyes, salvaged armor plates with zodiac etchings glowing purple"),
    ("hero_king_ringtail", "King Ringtail", "raccoon", "LLAMA", "Raccoon in patchwork mech helmet from six welded enemies, cheerful chaos expression, sparks near edges"),
    ("hero_the_eternal", "The Eternal", "axolotl", "Croak", "Axolotl face through diving helmet porthole, external gills fanning, pale pink, dark patient eyes, bubbles"),
    ("hero_patches", "Patches", "patchy calico cat", "catGPT", "Nervous calico cat with mismatched fur patches, wide anxious eyes, sensor collar glow, ears half-flattened"),
    # AI avatar portraits
    ("ai_le_chat", "Le Chat", "digital cat hologram", "catGPT", "Holographic AI cat face, blue glowing eyes, geometric ears, data stream whiskers, eager expression, scan lines"),
    ("ai_claudeus_maximus", "Claudeus Maximus", "digital mouse hologram", "The Clawed", "Holographic AI mouse, radar dish ears, scrolling text overlay, earnest overwhelmed expression, green glow"),
    ("ai_deepseek", "Deepseek", "digital badger hologram", "Seekers of the Deep", "Holographic AI badger, badger stripe as loading bar, amber-gold eyes, patient deliberate expression"),
    ("ai_gemineye", "Gemineye", "digital crow hologram", "The Murder", "Holographic AI crow with third eye on forehead, zodiac constellations orbiting, smug expression, purple glow"),
    ("ai_llhama", "Llhama", "digital raccoon hologram", "LLAMA", "Holographic AI raccoon, data mask glitching outward leaking data, cheerfully oblivious, orange glow"),
    ("ai_grok", "Grok", "digital axolotl hologram", "Croak", "Holographic AI axolotl, gills as antenna arrays, cute face trying to look edgy, skull motifs, teal glow"),
]

WALK_SHEETS = [
    ("pawdler_walk", "Pawdler", "cat with fish basket", "trudging walk, basket sways, reluctant lazy gait"),
    ("nuisance_walk", "Nuisance", "small scrappy cat", "quick darting jittery movement, tail twitching"),
    ("chonk_walk", "Chonk", "very fat cat", "heavy slow waddle, belly visibly swaying side to side"),
    ("flying_fox_walk", "Flying Fox", "fruit bat with wings", "wing flap hover cycle, wings spread through full flap"),
    ("hisser_walk", "Hisser", "arched-back cat", "aggressive stalking posture, tail puffed up, raised hackles"),
    ("yowler_walk", "Yowler", "support cat", "walking with mouth slightly ajar, sound wave hints around head"),
    ("mouser_walk", "Mouser", "sleek black cat", "low-profile slinky stealth movement, nearly invisible"),
    ("catnapper_walk", "Catnapper", "sleeping siege cat", "very slow reluctant drag, eyes closed, Zzz bubbles persist"),
    ("ferret_sapper_walk", "Ferret Sapper", "wiry ferret with explosives", "bouncy ferret trot, bomb fuse trailing behind"),
    ("mech_commander_walk", "Mech Commander", "cat in oversized mech suit", "heavy mechanical stomp cycle, cat visible bouncing in cockpit"),
]

ATTACK_SHEETS = [
    ("pawdler_attack", "Pawdler", "cat with pickaxe", "reluctant swing, overhead, impact, recover"),
    ("nuisance_attack", "Nuisance", "small scrappy cat", "quick claw swipe, recoil, ready — annoying not deadly"),
    ("chonk_attack", "Chonk", "very fat cat", "lean back, belly slam forward, bounce, settle"),
    ("flying_fox_attack", "Flying Fox", "fruit bat", "wings tuck, dive, slash with claws, pull up"),
    ("hisser_attack", "Hisser", "arched-back cat", "arch back, open mouth wide, launch green acid spit glob, recoil"),
    ("yowler_attack", "Yowler", "support cat", "inhale, mouth wide open, purple sonic burst rings, echo"),
    ("mouser_attack", "Mouser", "sleek black cat", "coil in shadow, lunge forward, slash, fade back — green eyes flash"),
    ("catnapper_attack", "Catnapper", "sleeping cat", "sleep shift, dream bubble grows, launch projectile, Zzz reset — eyes stay closed"),
    ("ferret_sapper_attack", "Ferret Sapper", "wiry ferret", "wind up, throw explosive charge in arc, bomb tumbles, fuse sparks"),
    ("mech_commander_attack", "Mech Commander", "cat in mech suit", "aim cannon arms, muzzle flash, recoil, stabilize — cat bounces in cockpit"),
]


# ── AppleScript Chrome interaction ──────────────────────────────

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
        return f"ERROR: {result.stderr.strip()[:200]}"
    return result.stdout.strip()


def find_and_focus_chatgpt_tab():
    """Find the ChatGPT tab, bring its window to front, track its index."""
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
    """Navigate to a new ChatGPT chat."""
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js('document.querySelector("#prompt-textarea") ? "ready" : "loading"')
        if "ready" in r:
            return True
    return False


def check_rate_limit() -> bool:
    """Check if ChatGPT is showing a rate limit message."""
    r = applescript_js('''
(function() {
    var body = document.body ? document.body.innerText : "";
    if (body.match(/rate limit|too many|try again later|usage cap/i)) return "rate_limited";
    return "ok";
})()
''')
    return "rate_limited" in r


def upload_style_reference(ref_path: str) -> bool:
    """Upload a style reference image via DataTransfer API."""
    if not Path(ref_path).exists():
        print(f"    Style ref not found: {ref_path}")
        return False

    # Click "Add files" button first
    applescript_js('''
(function() {
    var addBtn = document.querySelector('button[aria-label="Add files and more"]');
    if (addBtn) addBtn.click();
})()
''')
    time.sleep(1)

    with open(ref_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode()

    js_inject = f'''
(function() {{
    var b64 = "{b64}";
    var byteChars = atob(b64);
    var byteArray = new Uint8Array(byteChars.length);
    for (var i = 0; i < byteChars.length; i++) {{
        byteArray[i] = byteChars.charCodeAt(i);
    }}
    var blob = new Blob([byteArray], {{type: "image/png"}});
    var file = new File([blob], "style_reference.png", {{type: "image/png"}});
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
        time.sleep(2)
        return True
    return False


def send_prompt_text(lines: list[str]) -> bool:
    """Fill the prompt textarea and click send."""
    # Escape for JS string embedding
    escaped = []
    for line in lines:
        escaped.append(line.replace("\\", "\\\\").replace('"', '\\"').replace("'", "\\'"))

    js_lines = ", ".join(f'"{l}"' for l in escaped)
    js_fill = f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    var lines = [{js_lines}];
    textarea.innerHTML = lines.map(function(l) {{ return "<p>" + (l || "<br>") + "</p>"; }}).join("");
    textarea.dispatchEvent(new Event("input", {{bubbles: true}}));
    return "ok";
}})()
'''
    r = applescript_js(js_fill)
    if "ok" not in r:
        print(f"    Fill failed: {r}")
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
    return "sent" in r


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate a fully rendered, stable image."""
    return wait_for_stable_image(applescript_js, timeout=timeout, settle_time=8)


def download_image(out_path: Path, crop_size: tuple[int, int] | None = None,
                   sprite_type: str = "idle") -> bool:
    """Download first generated image via curl, apply rembg if needed, QC gate, optionally crop/resize."""
    js_url = '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length === 0) return "no_images";
    return imgs[0].src;
})()
'''
    img_url = applescript_js(js_url)
    if "no_images" in img_url or not img_url.startswith("http"):
        print(f"    No image URL: {img_url[:80]}")
        return False

    cookies = applescript_js("document.cookie")

    dl_path = Path("/tmp/chatgpt_sprite_dl.png")
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 5000:
        print(f"    File too small: {dl_path.stat().st_size}B")
        dl_path.unlink()
        return False

    out_path.parent.mkdir(parents=True, exist_ok=True)

    img = Image.open(str(dl_path)).convert("RGBA")

    # rembg fallback for idle sprites: if fully opaque, ChatGPT didn't give transparency
    if sprite_type == "idle":
        alpha = np.array(img.split()[-1])
        if alpha.min() > 200:
            print("    Fully opaque — applying rembg...")
            img = remove_background(img)

    # Quality gate (skip for sheets — they have different structure)
    if sprite_type in ("idle", "portrait"):
        passed, reason = validate_sprite_quality(img, sprite_type=sprite_type)
        if not passed:
            print(f"    QC FAIL: {reason}")
            dl_path.unlink(missing_ok=True)
            return False
    else:
        reason = "sheet (no QC)"

    if crop_size:
        w, h = crop_size
        bbox = img.getbbox()
        if bbox:
            cropped = img.crop(bbox)
            cropped.thumbnail((w, h), Image.LANCZOS)
            canvas = Image.new("RGBA", (w, h), (0, 0, 0, 0))
            x = (w - cropped.width) // 2
            y = (h - cropped.height) // 2
            canvas.paste(cropped, (x, y))
            canvas.save(str(out_path))
            print(f"    {cropped.width}x{cropped.height} -> {w}x{h} ({reason})")
        else:
            print(f"    Empty image")
            dl_path.unlink()
            return False
    else:
        img.save(str(out_path))
        print(f"    Saved {img.size[0]}x{img.size[1]} ({reason})")

    dl_path.unlink(missing_ok=True)
    return True


# ── Prompt builders ──────────────────────────────────────────────

def build_terrain_prompt(name: str, description: str) -> list[str]:
    return [
        f"Generate a 128x128 isometric terrain tile for a 2D RTS game.",
        f"",
        f"Terrain type: {name}",
        f"Description: {description}",
        f"",
        f"Requirements:",
        f"- Isometric 2:1 perspective, 128x128 canvas, transparent background",
        f"- Diamond zone: 128px wide x 64px tall, bottom-aligned (rows 64-128) = flat tile surface",
        f"- Height zone: top 64 rows for features extending above tile plane (trees, pillars)",
        f"- Must tile seamlessly with adjacent tiles of same type in isometric grid",
        f"- Clean vector art, flat colors, bold dark outlines (2-3px)",
        f"- Into the Breach / Northgard aesthetic",
        f"- No gradients, 2-3 value steps per hue",
        f"- Consistent lighting from top-left",
        f"",
        f"Generate ONLY this one tile. Do not add extra elements.",
    ]


def build_resource_prompt(name: str, description: str, size: int) -> list[str]:
    return [
        f"Generate a {size}x{size} isometric resource deposit sprite for a 2D RTS game.",
        f"",
        f"Resource: {name}",
        f"Description: {description}",
        f"",
        f"Requirements:",
        f"- Isometric perspective (~30 degrees), facing south-east",
        f"- Transparent PNG background",
        f"- Should look like a natural deposit that workers would harvest from",
        f"- Visually distinct and recognizable even at small zoom",
        f"- Clean vector art, flat colors, bold dark outlines (2-3px)",
        f"- Into the Breach / Northgard aesthetic",
        f"",
        f"Generate ONLY this one image.",
    ]


def build_portrait_prompt(name: str, animal: str, faction: str, description: str) -> list[str]:
    return [
        f"Generate a 128x128 character portrait for a 2D RTS game.",
        f"",
        f"Character: {name}",
        f"Animal: {animal}",
        f"Faction: {faction}",
        f"Description: {description}",
        f"",
        f"Requirements:",
        f"- 128x128 square portrait, front-facing or 3/4 view",
        f"- Head and upper shoulders visible, filling most of the frame",
        f"- Expressive face — personality immediately readable",
        f"- Clean dark outline (2-3px) around the character",
        f"- Muted painterly background — single flat color or gradient suggesting faction theme",
        f"- Redwall-meets-Into-the-Breach art style",
        f"- Eyes are the focal point",
        f"- Should read clearly as a thumbnail at 64x64",
        f"",
        f"Generate ONLY this one portrait.",
    ]


def build_sheet_prompt(name: str, animal: str, anim_desc: str, sheet_type: str) -> list[str]:
    return [
        f"Generate a sprite sheet of 4 animation frames for a 2D isometric RTS unit.",
        f"",
        f"Unit: {name} ({animal})",
        f"Animation: {sheet_type} — {anim_desc}",
        f"",
        f"Requirements:",
        f"- EXACTLY 4 frames arranged in a single HORIZONTAL ROW, left to right",
        f"- Each frame is 128x128 pixels, total sheet is 512x128 pixels",
        f"- All frames show the unit facing south-east in isometric perspective (~30 degrees)",
        f"- Transparent PNG background",
        f"- Body/fur in neutral gray (#B0B0B0-#D0D0D0) — team color applied in-engine",
        f"- Clean vector art, flat colors, bold dark outlines (2-3px)",
        f"- Into the Breach / Northgard aesthetic",
        f"- Consistent proportions across all 4 frames (no drifting)",
        f"- Animation should smoothly loop from first to last frame",
        f"- Each frame fully contained within its 128x128 grid cell",
        f"",
        f"Generate ONLY this one sprite sheet. 4 frames, horizontal strip, 512x128 total.",
    ]


# ── Generation pipeline ─────────────────────────────────────────

def generate_one(slug: str, prompt_lines: list[str], out_path: Path,
                 crop_size: tuple[int, int] | None = None,
                 style_ref: str | None = None,
                 sprite_type: str = "idle") -> bool:
    """Full pipeline for one asset, with up to 2 retries on failure."""
    for attempt in range(3):
        if attempt > 0:
            print(f"    Retry {attempt}/2...")

        if attempt == 0:
            print(f"\n  [{slug}]")

        # Check for rate limiting
        if check_rate_limit():
            print("    Rate limited — pausing 5 minutes...")
            time.sleep(300)

        if not open_new_chat():
            print("    Failed to load new chat")
            continue

        if style_ref:
            upload_style_reference(style_ref)

        if not send_prompt_text(prompt_lines):
            print("    Failed to send prompt")
            continue
        print("    Prompt sent, waiting...")

        if not wait_for_image(timeout=120):
            print("    Timeout waiting for image")
            continue

        if download_image(out_path, crop_size, sprite_type=sprite_type):
            return True

    return False


def run_terrain(start: int = 0):
    """Generate terrain tiles."""
    done, failed = 0, []
    for i, (slug, name, desc) in enumerate(TERRAIN):
        if i < start:
            continue
        out = SPRITE_DIR / "terrain" / f"{slug}.png"
        if out.exists():
            print(f"  [{slug}] exists, skipping")
            done += 1
            continue
        prompt = build_terrain_prompt(name, desc)
        if generate_one(slug, prompt, out, crop_size=(128, 128)):
            done += 1
        else:
            failed.append(slug)
    print(f"\nTerrain: {done}/{len(TERRAIN)} done")
    if failed:
        print(f"Failed: {', '.join(failed)}")


def run_resources(start: int = 0):
    """Generate resource sprites."""
    done, failed = 0, []
    for i, (slug, name, desc, size) in enumerate(RESOURCES):
        if i < start:
            continue
        out = SPRITE_DIR / "resources" / f"{slug}.png"
        if out.exists():
            print(f"  [{slug}] exists, skipping")
            done += 1
            continue
        prompt = build_resource_prompt(name, desc, size)
        if generate_one(slug, prompt, out, crop_size=(size, size)):
            done += 1
        else:
            failed.append(slug)
    print(f"\nResources: {done}/{len(RESOURCES)} done")
    if failed:
        print(f"Failed: {', '.join(failed)}")


def run_portraits(start: int = 0):
    """Generate portrait sprites."""
    done, failed = 0, []
    for i, (slug, name, animal, faction, desc) in enumerate(PORTRAITS):
        if i < start:
            continue
        out = SPRITE_DIR / "portraits" / f"{slug}.png"
        if out.exists():
            print(f"  [{slug}] exists, skipping")
            done += 1
            continue
        prompt = build_portrait_prompt(name, animal, faction, desc)
        if generate_one(slug, prompt, out, crop_size=(128, 128), sprite_type="portrait"):
            done += 1
        else:
            failed.append(slug)
    print(f"\nPortraits: {done}/{len(PORTRAITS)} done")
    if failed:
        print(f"Failed: {', '.join(failed)}")


def scan_bad_portraits() -> list[str]:
    """Scan existing portraits and return slugs that fail QC."""
    portrait_dir = SPRITE_DIR / "portraits"
    bad = []
    known = {p[0] for p in PORTRAITS}
    for p in sorted(portrait_dir.glob("*.png")):
        slug = p.stem
        if slug not in known:
            continue
        img = Image.open(str(p)).convert("RGBA")
        passed, reason = validate_sprite_quality(img, sprite_type="portrait")
        if not passed:
            print(f"  BAD: {slug} — {reason}")
            bad.append(slug)
        else:
            print(f"  ok:  {slug} — {reason}")
    return bad


def regen_bad_portraits():
    """Scan portraits, delete bad ones, regenerate."""
    print("Scanning portraits for quality issues...")
    bad_slugs = scan_bad_portraits()
    if not bad_slugs:
        print("\nAll portraits pass QC!")
        return

    print(f"\n{len(bad_slugs)} bad portraits found. Deleting and regenerating...")
    portrait_dir = SPRITE_DIR / "portraits"

    for slug in bad_slugs:
        out = portrait_dir / f"{slug}.png"
        if out.exists():
            out.unlink()
            print(f"  Deleted {out.name}")

    # Build lookup
    portrait_map = {p[0]: p for p in PORTRAITS}

    done = 0
    failed = []
    for slug in bad_slugs:
        _, name, animal, faction, desc = portrait_map[slug]
        prompt = build_portrait_prompt(name, animal, faction, desc)
        out = portrait_dir / f"{slug}.png"
        if generate_one(slug, prompt, out, crop_size=(128, 128), sprite_type="portrait"):
            done += 1
            print(f"    Regenerated! ({done}/{len(bad_slugs)})")
        else:
            failed.append(slug)
            print(f"    REGEN FAILED")

    print(f"\nRegenerated: {done}/{len(bad_slugs)}")
    if failed:
        print(f"Still failed: {', '.join(failed)}")


def run_walk(start: int = 0):
    """Generate walk animation sheets."""
    ref = str(SPRITE_DIR / "units" / "chonk_idle.png")
    done, failed = 0, []
    for i, (slug, name, animal, desc) in enumerate(WALK_SHEETS):
        if i < start:
            continue
        out = SPRITE_DIR / "units" / f"{slug}.png"
        if out.exists():
            print(f"  [{slug}] exists, skipping")
            done += 1
            continue
        prompt = build_sheet_prompt(name, animal, desc, "walk cycle")
        if generate_one(slug, prompt, out, style_ref=ref, sprite_type="sheet"):
            done += 1
        else:
            failed.append(slug)
    print(f"\nWalk sheets: {done}/{len(WALK_SHEETS)} done")
    if failed:
        print(f"Failed: {', '.join(failed)}")


def run_attack(start: int = 0):
    """Generate attack animation sheets."""
    ref = str(SPRITE_DIR / "units" / "chonk_idle.png")
    done, failed = 0, []
    for i, (slug, name, animal, desc) in enumerate(ATTACK_SHEETS):
        if i < start:
            continue
        out = SPRITE_DIR / "units" / f"{slug}.png"
        if out.exists():
            print(f"  [{slug}] exists, skipping")
            done += 1
            continue
        prompt = build_sheet_prompt(name, animal, desc, "attack")
        if generate_one(slug, prompt, out, style_ref=ref, sprite_type="sheet"):
            done += 1
        else:
            failed.append(slug)
    print(f"\nAttack sheets: {done}/{len(ATTACK_SHEETS)} done")
    if failed:
        print(f"Failed: {', '.join(failed)}")


CATEGORIES = {
    "terrain": run_terrain,
    "resources": run_resources,
    "portraits": run_portraits,
    "walk": run_walk,
    "attack": run_attack,
}


def main():
    category = sys.argv[1] if len(sys.argv) > 1 else "all"

    # --regen-bad mode for portraits
    if "--regen-bad" in sys.argv:
        loc = find_and_focus_chatgpt_tab()
        print(f"ChatGPT tab: {loc}")
        if "not_found" in loc:
            print("ERROR: No ChatGPT tab found. Open chatgpt.com in Chrome first.")
            sys.exit(1)

        if category == "portraits":
            regen_bad_portraits()
        else:
            print("--regen-bad currently only supports 'portraits' category")
            print("Usage: python3 batch_remaining.py portraits --regen-bad")
            sys.exit(1)
        return

    start_idx = int(sys.argv[2]) if len(sys.argv) > 2 else 0

    loc = find_and_focus_chatgpt_tab()
    print(f"ChatGPT tab: {loc}")
    if "not_found" in loc:
        print("ERROR: No ChatGPT tab found. Open chatgpt.com in Chrome first.")
        sys.exit(1)

    if category == "all":
        for name, func in CATEGORIES.items():
            print(f"\n{'='*60}\n  Category: {name}\n{'='*60}")
            func(0)
    elif category in CATEGORIES:
        CATEGORIES[category](start_idx)
    else:
        print(f"Unknown category: {category}")
        print(f"Available: {', '.join(CATEGORIES.keys())}, all")
        sys.exit(1)


if __name__ == "__main__":
    main()
