#!/usr/bin/env python3
"""Regenerate broken/inconsistent idle sprites using v2 layered prompts.

Uses assemble_prompt() from generate_asset.py (base_style + faction_overlay + template)
instead of hardcoded v1 prompts. Automates ChatGPT via AppleScript browser injection.

Usage:
    python regen_sprites.py --dry-run --faction llama     Print prompts without generating
    python regen_sprites.py --faction llama                Regenerate all broken LLAMA sprites
    python regen_sprites.py scrounger_idle bandit_idle     Regenerate specific sprites
    python regen_sprites.py --all                          Regenerate all broken sprites (all factions)
"""
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


# ── Sprites to regenerate, grouped by faction ────────────────

REGEN_SPRITES = {
    "llama": [
        "scrounger_idle",
        "junkyard_king_idle",
        "glitch_rat_idle",
        "grease_monkey_idle",
        "dumpster_diver_idle",
        "heap_titan_idle",
        "dead_drop_unit_idle",
        "bandit_idle",        # style mismatch: looks like cat, not raccoon
    ],
    "croak": [
        "gulper_idle",
        "leapfrog_idle",
        "eftsaber_idle",
        "regeneron_idle",
        "shellwarden_idle",
        "murk_commander_idle",
    ],
    "seekers": [
        "dustclaw_idle",
        "gutripper_idle",
        "cragback_idle",      # style mismatch: looks like axolotl
    ],
    "murder": [
        "dusktalon_idle",     # style mismatch: looks like axolotl
        "corvus_rex_idle",    # style mismatch: too gray/metallic
        "jayflicker_idle",    # style mismatch: generic bird
    ],
    "clawed": [
        "nibblet_idle",       # pixel art inconsistency
        "swarmer_idle",       # pixel art inconsistency
        "shrieker_idle",      # pixel art inconsistency
    ],
}


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
    import json

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


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate a fully rendered, stable image."""
    return wait_for_stable_image(applescript_js, timeout=timeout, settle_time=8)


def download_image(slug: str) -> bool:
    """Download generated image, apply rembg + crop + resize, QC gate, save to assets."""
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
    if not any(parsed.hostname and parsed.hostname.endswith(d) for d in allowed_domains):
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

    # Process: rembg → crop → resize → center on 128x128 canvas
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
    print(f"    {cropped.width}x{cropped.height} → 128x128 ({reason})")

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
    print(f"    Catalog updated: {slug} → style_version=2, game_ready")


def generate_one(slug: str, prompt: str) -> bool:
    """Full pipeline for one sprite: send prompt → wait → download → QC → catalog."""
    for attempt in range(3):
        if attempt > 0:
            print(f"    Retry {attempt}/2...")

        if check_rate_limit():
            print("    Rate limited — pausing 5 minutes...")
            time.sleep(300)

        if not open_new_chat():
            print("    Failed to load new chat")
            continue

        if not send_v2_prompt(prompt):
            print("    Failed to send prompt")
            continue
        print("    Prompt sent, waiting for generation...")

        if not wait_for_image():
            print("    Timeout waiting for image")
            continue

        if download_image(slug):
            update_catalog(slug)
            return True

    return False


# ── Main ─────────────────────────────────────────────────────

def get_sprites_for_faction(faction: str) -> list[str]:
    """Get the list of sprites to regen for a faction."""
    faction_key = faction.lower().strip()
    return REGEN_SPRITES.get(faction_key, [])


def build_prompt(slug: str) -> str | None:
    """Look up a sprite in the catalog and assemble its v2 prompt."""
    catalog = generate_asset.load_catalog()
    _, _, entry = generate_asset.find_asset(catalog, slug)
    if entry is None:
        print(f"Error: '{slug}' not found in catalog", file=sys.stderr)
        return None
    return generate_asset.assemble_prompt(entry)


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Regenerate broken idle sprites with v2 prompts")
    parser.add_argument("sprites", nargs="*", help="Specific sprite slugs to regenerate (e.g. scrounger_idle)")
    parser.add_argument("--faction", "-f", help="Regenerate all broken sprites for a faction (llama, croak, seekers, murder, clawed)")
    parser.add_argument("--all", action="store_true", help="Regenerate all broken sprites across all factions")
    parser.add_argument("--dry-run", action="store_true", help="Print prompts without generating")
    args = parser.parse_args()

    # Build sprite list
    if args.all:
        # All factions in priority order (worst first)
        target_sprites = []
        for faction in ["llama", "croak", "seekers", "murder", "clawed"]:
            target_sprites.extend(REGEN_SPRITES[faction])
    elif args.faction:
        target_sprites = get_sprites_for_faction(args.faction)
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
            prompt = build_prompt(slug)
            if prompt:
                print(f"\n{'='*60}")
                print(f"  {slug}")
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
        prompt = build_prompt(slug)
        if prompt is None:
            failed.append(slug)
            continue

        faction = "unknown"
        for f, sprites in REGEN_SPRITES.items():
            if slug in sprites:
                faction = f
                break

        print(f"\n[{i+1}/{len(target_sprites)}] {slug} (faction={faction}, prompt={len(prompt)} chars)")

        if generate_one(slug, prompt):
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
