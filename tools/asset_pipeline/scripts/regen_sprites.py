#!/usr/bin/env python3
"""Regenerate broken/inconsistent sprites using v2 layered prompts.

Phase 1: Idle sprites (regenerated first as they serve as visual reference for sheets)
Phase 2: Walk/attack animation sheets (use idle as visual reference)

Uses assemble_prompt() from generate_asset.py (base_style + faction_overlay + template)
instead of hardcoded v1 prompts. Automates ChatGPT via AppleScript browser injection.

Usage:
    python regen_sprites.py --dry-run --phase 1              Preview Phase 1 idle prompts
    python regen_sprites.py --phase 1                         Regenerate all Phase 1 idles
    python regen_sprites.py --phase 2                         Regenerate all Phase 2 sheets
    python regen_sprites.py --faction llama --phase 1         Faction + phase filter
    python regen_sprites.py yowler_idle glitch_rat_idle       Regenerate specific sprites
    python regen_sprites.py --all                             Regenerate everything (Phase 1 then 2)
"""
import base64
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image

# ── Setup paths ──────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).resolve().parent
PIPELINE_ROOT = SCRIPT_DIR.parent
PROJECT_ROOT = PIPELINE_ROOT.parent.parent
RAW_DIR = PIPELINE_ROOT / "raw" / "units"
OUT_DIR = PROJECT_ROOT / "assets" / "sprites" / "units"

# Add scripts dir to path so we can import siblings
sys.path.insert(0, str(SCRIPT_DIR))

from image_utils import (
    remove_background,
    validate_sprite_quality,
    wait_for_stable_image,
)

import generate_asset


# ── Phase 1: Idle sprites to regenerate ──────────────────────

PHASE1_IDLES = {
    # Tier A — Critical (wrong species, missing design intent, score <=5)
    "catgpt": [
        "yowler_idle",           # score 4, generic blob → Siamese with sonic rings
    ],
    "llama": [
        "glitch_rat_idle",       # score 5, drawn as literal rat → raccoon kit
        "dead_drop_unit_idle",   # score 5, hooded figure not animal → raccoon
        "dumpster_diver_idle",   # not literally in dumpster → raccoon in dumpster
    ],
    "seekers": [
        "seeker_tunneler_idle",  # score 5, weak identity → mole (coalition)
    ],
    "clawed": [
        "whiskerwitch_idle",     # score 5, pixel art → shrew (coalition)
        "tunneler_idle",         # score 6 → vole (coalition)
        "sparks_idle",           # score 6 → shrew (coalition)
    ],
    "murder": [
        "murder_scrounger_idle", # score 5, generic bird → magpie (coalition)
        "sentinel_idle",         # score 7, weak identity → crow with telescope eye
        "hootseer_idle",         # score 7 → owl with zodiac runes
    ],
    "croak": [
        "ponderer_idle",         # score 6 → pink axolotl with teal spots
        "bogwhisper_idle",       # score 6 → newt (coalition)
        "croaker_idle",          # score 6 → frog (coalition)
    ],
}

# ── Phase 2: Animation sheets to regenerate ──────────────────

PHASE2_WALKS = {
    "catgpt": [
        "chonk_walk",            # blank frame(s)
        "flying_fox_walk",       # identity drift (becomes rabbit)
    ],
    "clawed": [
        "nibblet_walk",          # blank frame(s), becomes chick
        "whiskerwitch_walk",     # blank frame(s)
        "sparks_walk",           # blank frame(s)
        "shrieker_walk",         # identity drift (becomes ball)
    ],
    "seekers": [
        "warden_walk",           # blank frame(s)
        "cragback_walk",         # blank frame(s)
        "delver_walk",           # identity drift (becomes statue)
    ],
    "croak": [
        "regeneron_walk",        # blank frame(s)
    ],
    "llama": [
        "heap_titan_walk",       # blank frame(s)
        "bandit_walk",           # blank frame(s)
    ],
    "murder": [
        "murder_scrounger_walk", # blank frame(s)
    ],
}

PHASE2_ATTACKS = {
    "catgpt": [
        "hisser_attack",         # blank frame(s)
        "flying_fox_attack",     # identity drift
    ],
    "clawed": [
        "nibblet_attack",        # blank frame(s)
        "whiskerwitch_attack",   # blank frame(s)
    ],
    "seekers": [
        "warden_attack",         # blank frame(s)
    ],
    "llama": [
        "heap_titan_attack",     # blank frame(s)
    ],
}

# Legacy compat: combined dict for --all and --faction without --phase
REGEN_SPRITES = {}
for faction in set(list(PHASE1_IDLES) + list(PHASE2_WALKS) + list(PHASE2_ATTACKS)):
    combined = []
    combined.extend(PHASE1_IDLES.get(faction, []))
    combined.extend(PHASE2_WALKS.get(faction, []))
    combined.extend(PHASE2_ATTACKS.get(faction, []))
    if combined:
        REGEN_SPRITES[faction] = combined


# ── ChatGPT browser automation (from batch_sprites.py) ───────

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
        # After `set index of window w to 1`, window is now at index 1
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


def send_v2_prompt(prompt_text: str) -> bool:
    """Send a pre-assembled v2 prompt to ChatGPT.

    Uses execCommand('insertText') for ProseMirror compatibility
    instead of innerHTML (which silently fails in ChatGPT's React editor).
    """
    # Escape the prompt for JS string embedding
    escaped = json.dumps(prompt_text)

    # Focus the textarea and insert text
    js_fill = f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";

    // Focus and clear
    textarea.focus();
    document.execCommand("selectAll");
    document.execCommand("delete");

    // Insert via execCommand for ProseMirror compat
    document.execCommand("insertText", false, {escaped});

    return "ok";
}})()
'''
    r = applescript_js(js_fill)
    if "ok" not in r:
        return False

    time.sleep(0.5)

    # Click send button
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

    # Fallback: try Enter key
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
    """Upload a reference image to ChatGPT via chunked base64 + DragEvent drop.

    Used for Phase 2 sheets: uploads the idle sprite as visual reference so
    walk/attack sheets match the character design.
    """
    if not image_path.exists():
        print(f"    Reference image not found: {image_path}")
        return False

    img_data = image_path.read_bytes()
    b64 = base64.b64encode(img_data).decode("ascii")

    # Clear any previous image data
    applescript_js("window._imgB64 = '';")

    # Upload in chunks (AppleScript has payload limits)
    chunk_size = 3000
    for i in range(0, len(b64), chunk_size):
        chunk = b64[i:i + chunk_size]
        escaped_chunk = json.dumps(chunk)
        applescript_js(f"window._imgB64 += {escaped_chunk};")

    # Convert base64 to blob and drop onto textarea
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
        time.sleep(1)  # Let UI process the upload
        return True
    print(f"    Image upload failed: {r}")
    return False


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate a fully rendered, stable image."""
    return wait_for_stable_image(applescript_js, timeout=timeout, settle_time=8)


def download_image(slug: str, is_sheet: bool = False) -> bool:
    """Download generated image, process, QC gate, save to assets.

    For idle sprites: rembg + crop + resize to 128x128
    For sheets: process through process_walk_raw.py to 512x128
    """
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

    # Validate URL domain to prevent cookie exfiltration via crafted img src
    allowed_domains = ("oaiusercontent.com", "chatgpt.com", "openai.com")
    from urllib.parse import urlparse
    parsed = urlparse(img_url)
    if not any(parsed.hostname and (parsed.hostname == d or parsed.hostname.endswith("." + d)) for d in allowed_domains):
        print(f"    Untrusted image domain: {parsed.hostname}")
        return False

    cookies = applescript_js("document.cookie")

    # Download via curl with browser cookies
    dl_path = Path.home() / "Downloads" / f"sprite_{slug}.png"
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 5000:
        print(f"    Downloaded file too small: {dl_path.stat().st_size} bytes")
        dl_path.unlink()
        return False

    # Save raw
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    raw_path = RAW_DIR / f"{slug}_raw.png"
    shutil.copy2(dl_path, raw_path)

    if is_sheet:
        # Process sheet through process_walk_raw.py
        OUT_DIR.mkdir(parents=True, exist_ok=True)
        out_path = OUT_DIR / f"{slug}.png"
        proc_result = subprocess.run(
            [sys.executable, str(SCRIPT_DIR / "process_walk_raw.py"),
             str(raw_path), str(out_path)],
            capture_output=True, text=True
        )
        print(f"    {proc_result.stdout.strip()}")
        if proc_result.returncode != 0:
            print(f"    Sheet processing failed: {proc_result.stderr[:200]}")
            # Try regrid as fallback
            regrid_path = SCRIPT_DIR / "regrid_sheet.py"
            if regrid_path.exists():
                print("    Trying regrid_sheet.py fallback...")
                regrid_result = subprocess.run(
                    [sys.executable, str(regrid_path), str(raw_path), str(out_path)],
                    capture_output=True, text=True
                )
                if regrid_result.returncode != 0:
                    dl_path.unlink(missing_ok=True)
                    return False
                print(f"    Regrid: {regrid_result.stdout.strip()}")
            else:
                dl_path.unlink(missing_ok=True)
                return False

        dl_path.unlink(missing_ok=True)
        return out_path.exists()
    else:
        # Process idle: rembg + crop + resize + center on 128x128 canvas
        img = Image.open(str(raw_path)).convert("RGBA")

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

        # QC gate (runs on final 128x128)
        passed, reason = validate_sprite_quality(canvas, sprite_type="idle")
        if not passed:
            print(f"    QC FAIL: {reason}")
            dl_path.unlink(missing_ok=True)
            return False

        OUT_DIR.mkdir(parents=True, exist_ok=True)
        out_path = OUT_DIR / f"{slug}.png"
        canvas.save(str(out_path))
        print(f"    {cropped.width}x{cropped.height} -> 128x128 ({reason})")

        dl_path.unlink(missing_ok=True)
        return True


def update_catalog(slug: str):
    """Stamp style_version: 2 and status: game_ready in the catalog for this sprite."""
    catalog = generate_asset.load_catalog()
    category, key, entry = generate_asset.find_asset(catalog, slug)
    if entry is None:
        print(f"    Warning: {slug} not found in catalog")
        return
    catalog[category][key]["style_version"] = 2
    catalog[category][key]["status"] = "game_ready"
    generate_asset.save_catalog(catalog)
    print(f"    Catalog updated: {slug} -> style_version=2, game_ready")


def generate_one(slug: str, prompt: str, is_sheet: bool = False,
                 reference_image: Path | None = None) -> bool:
    """Full pipeline for one sprite: send prompt -> wait -> download -> QC -> catalog."""
    for attempt in range(3):
        if attempt > 0:
            print(f"    Retry {attempt}/2...")

        if check_rate_limit():
            print("    Rate limited — pausing 5 minutes...")
            time.sleep(300)

        if not open_new_chat():
            print("    Failed to load new chat")
            continue

        # Upload reference image for sheets
        if reference_image and reference_image.exists():
            print(f"    Uploading reference: {reference_image.name}")
            if not upload_reference_image(reference_image):
                print("    Warning: reference upload failed, continuing without it")

        if not send_v2_prompt(prompt):
            print("    Failed to send prompt")
            continue
        print("    Prompt sent, waiting for generation...")

        if not wait_for_image():
            print("    Timeout waiting for image")
            continue

        if download_image(slug, is_sheet=is_sheet):
            update_catalog(slug)
            return True

    return False


# ── Sheet prompt builders ────────────────────────────────────

def build_walk_prompt(slug: str) -> str | None:
    """Build a walk cycle sheet prompt from the catalog idle description."""
    # Strip _walk suffix to get idle slug
    idle_slug = slug.replace("_walk", "_idle")
    catalog = generate_asset.load_catalog()
    _, _, entry = generate_asset.find_asset(catalog, idle_slug)
    if entry is None:
        print(f"Error: '{idle_slug}' not found in catalog", file=sys.stderr)
        return None

    params = entry.get("params", {})
    desc = params.get("description", "")
    faction = params.get("faction", "")

    return f"""Generate a 512x128 pixel image: a 4-frame walk cycle sprite sheet for a 2D RTS game. 4 sprites in a single horizontal row, each fitting within 128x128. Transparent PNG background.

Character: {desc}
Bold black outlines (2-3px). Isometric 3/4 view facing bottom-right.

Match the EXACT character design from the attached reference image. Same species, same colors, same outfit, same proportions.

WALK CYCLE — Each frame shows a DIFFERENT step:

Frame 1: Left front paw/foot forward, right back. Weight shifting forward.
Frame 2: Both paws near center, body at lowest point of stride.
Frame 3: Right front paw/foot forward, left back. Mirror of frame 1.
Frame 4: Both paws near center, body at highest point of stride.

IMPORTANT: Paw/foot positions MUST be clearly different each frame. Body and face stay consistent. Only legs change position.

Art style: Smooth clean vector game art. NOT pixel art. Bold outlines, flat colors, 2-3 value steps. Wargroove/Advance Wars aesthetic. {faction} faction."""


def build_attack_prompt(slug: str) -> str | None:
    """Build an attack animation sheet prompt from the catalog idle description."""
    idle_slug = slug.replace("_attack", "_idle")
    catalog = generate_asset.load_catalog()
    _, _, entry = generate_asset.find_asset(catalog, idle_slug)
    if entry is None:
        print(f"Error: '{idle_slug}' not found in catalog", file=sys.stderr)
        return None

    params = entry.get("params", {})
    desc = params.get("description", "")
    faction = params.get("faction", "")

    return f"""Generate a 512x128 pixel image: a 4-frame attack animation sprite sheet for a 2D RTS game. 4 sprites in a single horizontal row, each fitting within 128x128. Transparent PNG background.

Character: {desc}
Bold black outlines (2-3px). Isometric 3/4 view facing bottom-right.

Match the EXACT character design from the attached reference image. Same species, same colors, same outfit, same proportions.

ATTACK ANIMATION — Each frame shows a stage of the attack:

Frame 1: Wind-up — body coiling back, weapon/ability charging.
Frame 2: Strike — full extension, weapon/ability at peak impact.
Frame 3: Follow-through — weapon past target, effect visible (projectile, slash, spark).
Frame 4: Recovery — returning to ready stance, weapon lowering.

IMPORTANT: Each frame must show a clearly different stage of the attack. The character design stays consistent across all frames.

Art style: Smooth clean vector game art. NOT pixel art. Bold outlines, flat colors, 2-3 value steps. Wargroove/Advance Wars aesthetic. {faction} faction."""


# ── Main ─────────────────────────────────────────────────────

def get_phase_sprites(phase: int, faction: str | None = None) -> list[str]:
    """Get sprites for a given phase, optionally filtered by faction."""
    if phase == 1:
        source = PHASE1_IDLES
    elif phase == 2:
        # Combine walks and attacks
        sprites = []
        for f in (PHASE2_WALKS, PHASE2_ATTACKS):
            for fac, slugs in f.items():
                if faction is None or fac == faction:
                    sprites.extend(slugs)
        return sprites
    else:
        return []

    result = []
    for fac, slugs in source.items():
        if faction is None or fac == faction:
            result.extend(slugs)
    return result


def build_prompt(slug: str) -> str | None:
    """Look up a sprite in the catalog and assemble its v2 prompt."""
    catalog = generate_asset.load_catalog()
    _, _, entry = generate_asset.find_asset(catalog, slug)
    if entry is None:
        print(f"Error: '{slug}' not found in catalog", file=sys.stderr)
        return None
    return generate_asset.assemble_prompt(entry)


def get_prompt_for_slug(slug: str) -> str | None:
    """Get the appropriate prompt for any slug (idle, walk, or attack)."""
    if slug.endswith("_walk"):
        return build_walk_prompt(slug)
    elif slug.endswith("_attack"):
        return build_attack_prompt(slug)
    else:
        return build_prompt(slug)


def get_reference_image(slug: str) -> Path | None:
    """Get the idle sprite path to use as reference for sheet generation."""
    if slug.endswith("_walk"):
        idle_slug = slug.replace("_walk", "_idle")
    elif slug.endswith("_attack"):
        idle_slug = slug.replace("_attack", "_idle")
    else:
        return None
    ref_path = OUT_DIR / f"{idle_slug}.png"
    return ref_path if ref_path.exists() else None


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Regenerate sprites with v2 prompts (Phase 1: idles, Phase 2: sheets)")
    parser.add_argument("sprites", nargs="*", help="Specific sprite slugs to regenerate")
    parser.add_argument("--faction", "-f", help="Filter by faction (catgpt, llama, croak, seekers, murder, clawed)")
    parser.add_argument("--phase", "-p", type=int, choices=[1, 2], help="Phase 1 = idles, Phase 2 = walk/attack sheets")
    parser.add_argument("--all", action="store_true", help="Regenerate all broken sprites (Phase 1 then 2)")
    parser.add_argument("--dry-run", action="store_true", help="Print prompts without generating")
    args = parser.parse_args()

    # Build sprite list
    if args.all:
        target_sprites = get_phase_sprites(1) + get_phase_sprites(2)
    elif args.phase:
        faction = args.faction.lower().strip() if args.faction else None
        target_sprites = get_phase_sprites(args.phase, faction)
        if not target_sprites:
            print(f"Error: no sprites for phase {args.phase}" +
                  (f" faction '{args.faction}'" if args.faction else ""))
            sys.exit(1)
    elif args.faction:
        faction = args.faction.lower().strip()
        target_sprites = REGEN_SPRITES.get(faction, [])
        if not target_sprites:
            print(f"Error: no regen sprites for faction '{args.faction}'")
            print(f"Available: {', '.join(REGEN_SPRITES.keys())}")
            sys.exit(1)
    elif args.sprites:
        target_sprites = args.sprites
    else:
        parser.print_help()
        sys.exit(1)

    print(f"Sprites to {'preview' if args.dry_run else 'regenerate'}: {len(target_sprites)}")

    # Dry-run: just print prompts
    if args.dry_run:
        for slug in target_sprites:
            prompt = get_prompt_for_slug(slug)
            if prompt:
                is_sheet = slug.endswith("_walk") or slug.endswith("_attack")
                ref = get_reference_image(slug)
                print(f"\n{'='*60}")
                print(f"  {slug}  {'[SHEET]' if is_sheet else '[IDLE]'}")
                if ref:
                    print(f"  Reference: {ref}")
                print(f"{'='*60}")
                print(prompt)
                print(f"\n  [{len(prompt)} chars]")
            else:
                print(f"\n  SKIP: {slug} — not in catalog")
        return

    # Live mode: find ChatGPT tab and generate
    loc = find_and_focus_chatgpt_tab()
    if "not_found" in loc:
        print("Error: No ChatGPT tab found. Open chatgpt.com in Chrome first.")
        sys.exit(1)
    print(f"ChatGPT tab: {loc}")

    # Delete existing bad sprites so they don't interfere
    for slug in target_sprites:
        out = OUT_DIR / f"{slug}.png"
        if out.exists():
            out.unlink()
            print(f"  Deleted old: {slug}.png")

    done = 0
    failed = []

    for i, slug in enumerate(target_sprites):
        prompt = get_prompt_for_slug(slug)
        if prompt is None:
            failed.append(slug)
            continue

        is_sheet = slug.endswith("_walk") or slug.endswith("_attack")
        ref_image = get_reference_image(slug) if is_sheet else None

        # Find faction for display
        faction = "unknown"
        for f, sprites in REGEN_SPRITES.items():
            if slug in sprites:
                faction = f
                break

        sheet_label = "sheet" if is_sheet else "idle"
        print(f"\n[{i+1}/{len(target_sprites)}] {slug} ({sheet_label}, faction={faction}, prompt={len(prompt)} chars)")

        if generate_one(slug, prompt, is_sheet=is_sheet, reference_image=ref_image):
            done += 1
            print(f"    Regenerated! ({done}/{len(target_sprites)})")
        else:
            failed.append(slug)
            print(f"    FAILED")

    print(f"\n{'='*40}")
    print(f"Complete: {done}/{len(target_sprites)} sprites regenerated")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
