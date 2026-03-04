#!/usr/bin/env python3
"""Regenerate walk sheets for units with poor animation frames.

Uses AppleScript to drive ChatGPT in Chrome, same approach as batch_remaining.py.
Downloads raw image, processes through process_walk_raw.py pipeline.

Usage: python3 regen_walks.py [unit_name]
  If unit_name provided, only regen that one. Otherwise does all 3.
"""
import subprocess
import sys
import time
import os
from pathlib import Path

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
RAW_DIR = PROJECT / "tools/asset_pipeline/raw/units"
OUT_DIR = PROJECT / "assets/sprites/units"

# Character descriptions matched to idle sprites
UNITS = {
    "hisser": {
        "character": "A spiky, aggressive gray cat with arched back and raised hackles. "
                     "Neutral light gray body (#B0B0B0-#C0C0C0). Bright green eyes, "
                     "wild spiky fur especially along the spine. Mouth slightly open showing fangs. "
                     "Low aggressive stance.",
        "walk_notes": "Stalking/prowling gait — body stays low and tense.",
    },
    "mouser": {
        "character": "A sleek dark gray (almost black) cat in a low crouching stealth pose. "
                     "Dark gray body (#555555-#707070). Yellow-green eyes, smooth fur, "
                     "long low body close to the ground. Stealthy hunter.",
        "walk_notes": "Sneaking/creeping gait — body stays very low, stalking prey.",
    },
    "nuisance": {
        "character": "A small scrappy gray cat with wild messy fur sticking out everywhere. "
                     "Neutral light gray body (#B0B0B0-#C0C0C0). Yellow eyes, "
                     "mischievous expression, compact energetic body. Looks like trouble.",
        "walk_notes": "Bouncy/energetic trot — this is a hyperactive harasser unit.",
    },
}


def build_prompt(name: str, info: dict) -> str:
    return f"""Generate a 512x128 pixel image: a 4-frame walk cycle sprite sheet for a 2D RTS game. 4 sprites in a single horizontal row, each fitting within 128x128. Transparent PNG background.

Character: {info['character']}
Bold black outlines (2-3px). Isometric 3/4 view facing bottom-right.

WALK CYCLE — {info['walk_notes']} Each frame shows a DIFFERENT step:

Frame 1: Left front paw forward, right back. Weight shifting forward.
Frame 2: Both paws near center, body at lowest point of stride.
Frame 3: Right front paw forward, left back. Mirror of frame 1.
Frame 4: Both paws near center, body at highest point of stride.

IMPORTANT: Paw positions MUST be clearly different each frame. Body and face stay consistent. Only legs change position.

Art style: Smooth clean vector game art. NOT pixel art. Bold outlines, flat colors, 2-3 value steps. Wargroove/Advance Wars aesthetic."""


def applescript_js(js_code: str) -> str:
    """Execute JS in the active Chrome tab via AppleScript."""
    escaped = js_code.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    ascript = f'''
    tell application "Google Chrome"
        set theTab to active tab of front window
        set theResult to execute theTab javascript "{escaped}"
        return theResult
    end tell
    '''
    result = subprocess.run(
        ["osascript", "-e", ascript],
        capture_output=True, text=True, timeout=30
    )
    return result.stdout.strip()


def open_new_chat():
    """Navigate to chatgpt.com for a fresh conversation."""
    ascript = '''
    tell application "Google Chrome"
        set URL of active tab of front window to "https://chatgpt.com"
    end tell
    '''
    subprocess.run(["osascript", "-e", ascript], capture_output=True, timeout=10)
    time.sleep(4)


def send_prompt(text: str) -> bool:
    """Fill prompt-textarea and click send."""
    lines = text.split("\n")
    html_parts = []
    for line in lines:
        if line.strip() == "":
            html_parts.append("<p><br></p>")
        else:
            safe = line.replace("'", "\\'").replace('"', "&quot;")
            html_parts.append(f"<p>{safe}</p>")
    html = "".join(html_parts)

    js = f"""
    (function() {{
        var ta = document.querySelector('#prompt-textarea');
        if (!ta) return 'NO_TEXTAREA';
        ta.innerHTML = '{html}';
        ta.dispatchEvent(new Event('input', {{ bubbles: true }}));
        setTimeout(function() {{
            var btn = document.querySelector('button[data-testid="send-button"]') ||
                      document.querySelector('button[aria-label="Send prompt"]');
            if (btn) btn.click();
        }}, 500);
        return 'SENT';
    }})()
    """
    result = applescript_js(js)
    return "SENT" in result


def wait_for_image(timeout: int = 120) -> str | None:
    """Poll for generated image, return its URL."""
    start = time.time()
    while time.time() - start < timeout:
        js = """
        (function() {
            var imgs = document.querySelectorAll('img[alt="Generated image"]');
            if (imgs.length > 0) {
                return imgs[imgs.length - 1].src;
            }
            return 'WAITING';
        })()
        """
        result = applescript_js(js)
        if result and result != "WAITING" and result.startswith("http"):
            return result
        time.sleep(5)
    return None


def download_image(url: str, output: Path) -> bool:
    """Download image via curl with Chrome cookies."""
    cookie_js = "document.cookie"
    cookies = applescript_js(cookie_js)

    result = subprocess.run(
        ["curl", "-sL", "-o", str(output),
         "-H", f"Cookie: {cookies}",
         "-H", "User-Agent: Mozilla/5.0",
         url],
        capture_output=True, timeout=60
    )
    return output.exists() and output.stat().st_size > 1000


def process_raw(raw_path: Path, output_path: Path) -> bool:
    """Process raw ChatGPT image to game-ready 512x128 sheet."""
    result = subprocess.run(
        [sys.executable, str(PROJECT / "tools/asset_pipeline/scripts/process_walk_raw.py"),
         str(raw_path), str(output_path)],
        capture_output=True, text=True
    )
    print(result.stdout)
    if result.returncode != 0:
        print(result.stderr)
    return result.returncode == 0


def generate_one(name: str) -> bool:
    info = UNITS[name]
    raw_path = RAW_DIR / f"{name}_walk_regen_raw.png"
    final_path = OUT_DIR / f"{name}_walk.png"
    backup_path = OUT_DIR / f"{name}_walk_old.png"

    print(f"\n  [{name}_walk]")

    # Open new chat
    open_new_chat()

    # Send prompt
    prompt = build_prompt(name, info)
    print("    Sending prompt...")
    if not send_prompt(prompt):
        print("    FAIL: couldn't send prompt")
        return False

    # Wait for image
    print("    Waiting for image...")
    url = wait_for_image(timeout=120)
    if not url:
        print("    FAIL: image generation timed out")
        return False

    # Download
    print("    Downloading...")
    if not download_image(url, raw_path):
        print("    FAIL: download failed")
        return False

    # Process
    print("    Processing...")
    temp_path = OUT_DIR / f"{name}_walk_new.png"
    if not process_raw(raw_path, temp_path):
        print("    FAIL: processing failed")
        return False

    # Backup old and replace
    if final_path.exists():
        if backup_path.exists():
            backup_path.unlink()
        final_path.rename(backup_path)
    temp_path.rename(final_path)
    print(f"    OK: {final_path}")
    return True


def main():
    RAW_DIR.mkdir(parents=True, exist_ok=True)

    if len(sys.argv) > 1:
        names = [sys.argv[1]]
    else:
        names = list(UNITS.keys())

    ok, fail = 0, 0
    for name in names:
        if name not in UNITS:
            print(f"Unknown unit: {name}")
            continue
        if generate_one(name):
            ok += 1
        else:
            fail += 1

    print(f"\nDone: {ok} OK, {fail} failed")


if __name__ == "__main__":
    main()
