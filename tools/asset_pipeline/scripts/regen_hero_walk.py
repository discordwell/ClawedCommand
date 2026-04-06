#!/usr/bin/env python3
"""Regenerate a hero walk sprite sheet using the idle sprite as visual reference.

Uses the same AppleScript+ChatGPT pipeline as batch_remaining.py:
1. Opens new ChatGPT chat
2. Uploads idle sprite as style reference via DataTransfer API
3. Sends a walk cycle prompt
4. Downloads the raw output
5. Processes through the full-body-aware pipeline (no half-split)

Usage: python3 regen_hero_walk.py <hero_name>
  e.g. python3 regen_hero_walk.py kell_fisher
"""
import base64
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image

PROJECT = Path(__file__).resolve().parents[3]
SPRITES = PROJECT / "assets" / "sprites" / "heroes"
RAW_DIR = PROJECT / "tools" / "asset_pipeline" / "raw" / "heroes"

HEROES = {
    "kell_fisher": {
        "character": (
            "Human military officer in olive/khaki dress uniform with medals, "
            "shoulder boards, and insignia. Dark hair, stern expression. "
            "Holding a white coffee mug in left hand. Black dress shoes. "
            "Isometric 3/4 view facing bottom-right."
        ),
        "walk_notes": (
            "Measured officer's stride with coffee mug held steady. "
            "Military bearing, purposeful gait. Legs clearly visible with "
            "different foot positions each frame."
        ),
    },
    "rex_harmon": {
        "character": (
            "Human military lieutenant in olive/khaki uniform, slightly "
            "more relaxed than the commander. Short brown hair, younger face. "
            "Hands at sides or one hand gesturing. Black boots. "
            "Isometric 3/4 view facing bottom-right."
        ),
        "walk_notes": (
            "Casual military walk, slightly more relaxed than a commander. "
            "Natural stride, arms swinging."
        ),
    },
}

# ── AppleScript Chrome interaction (same as batch_remaining.py) ──

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
    '''], capture_output=True, text=True, timeout=60)
    if result.returncode != 0:
        return f"ERROR: {result.stderr.strip()[:200]}"
    return result.stdout.strip()


def find_chatgpt_tab():
    """Find and focus the ChatGPT tab."""
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
    '''], capture_output=True, text=True, timeout=60)
    loc = result.stdout.strip()
    if "," in loc:
        parts = loc.split(",")
        _target_tab = (1, int(parts[1]))
        return True
    return False


def open_new_chat():
    """Navigate to a fresh ChatGPT chat."""
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js(
            'document.querySelector("#prompt-textarea") ? "ready" : "loading"'
        )
        if "ready" in r:
            return True
    return False


def upload_reference(ref_path: str) -> bool:
    """Upload the idle sprite as a style reference via DataTransfer API."""
    p = Path(ref_path)
    if not p.exists():
        print(f"  Reference not found: {ref_path}")
        return False

    # Click "Add files" button
    applescript_js('''
(function() {
    var addBtn = document.querySelector('button[aria-label="Add files and more"]');
    if (addBtn) addBtn.click();
})()
''')
    time.sleep(1)

    with open(ref_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode()

    js = f'''
(function() {{
    var b64 = "{b64}";
    var byteChars = atob(b64);
    var byteArray = new Uint8Array(byteChars.length);
    for (var i = 0; i < byteChars.length; i++) {{
        byteArray[i] = byteChars.charCodeAt(i);
    }}
    var blob = new Blob([byteArray], {{type: "image/png"}});
    var file = new File([blob], "idle_reference.png", {{type: "image/png"}});
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
    r = applescript_js(js)
    if "uploaded" in r:
        print("  Reference uploaded")
        time.sleep(2)
        return True
    print(f"  Upload failed: {r}")
    return False


def send_prompt(text: str) -> bool:
    """Fill prompt and click send."""
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
    r = applescript_js(js)
    return "SENT" in r


def wait_for_image(timeout: int = 120) -> bool:
    """Poll until an image appears in the response."""
    start = time.time()
    while time.time() - start < timeout:
        js = """
(function() {
    var imgs = document.querySelectorAll('img');
    for (var img of imgs) {
        if (img.naturalWidth > 400 && img.naturalHeight > 400) return "FOUND";
    }
    return "WAITING";
})()
"""
        r = applescript_js(js)
        if "FOUND" in r:
            return True
        time.sleep(5)
    return False


def download_first_image(output: Path) -> bool:
    """Download first generated image via curl with browser cookies."""
    js_url = '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length === 0) {
        // Fallback: find any large image
        var all = document.querySelectorAll('img');
        for (var i of all) {
            if (i.naturalWidth > 400 && i.naturalHeight > 400) return i.src;
        }
        return "no_images";
    }
    return imgs[imgs.length - 1].src;
})()
'''
    img_url = applescript_js(js_url)
    if "no_images" in img_url or not img_url.startswith("http"):
        print(f"  No image URL: {img_url[:80]}")
        return False

    cookies = applescript_js("document.cookie")

    dl_path = Path("/tmp/chatgpt_hero_dl.png")
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"  curl failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 5000:
        print(f"  File too small: {dl_path.stat().st_size}B")
        dl_path.unlink()
        return False

    import shutil
    output.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(dl_path), str(output))
    return True


def process_full_body(raw_path: Path, output_path: Path) -> bool:
    """Process raw ChatGPT output into 512x128 sheet. No half-split."""
    img = Image.open(raw_path).convert("RGBA")
    arr = np.array(img)
    alpha = arr[:, :, 3]

    # If image is mostly opaque, use rembg for background removal
    if alpha.min() > 200:
        try:
            from rembg import remove
            print("  Removing background with rembg...")
            img = remove(img)
            arr = np.array(img)
            alpha = arr[:, :, 3]
        except ImportError:
            print("  Warning: rembg not installed, skipping bg removal")

    # Zero faint pixels
    mask = alpha < 30
    arr[mask] = [0, 0, 0, 0]
    clean = Image.fromarray(arr)
    alpha = arr[:, :, 3]

    # Find FULL content bbox - no half splitting
    cr = np.where((alpha > 30).sum(axis=1) > 3)[0]
    cc = np.where((alpha > 30).sum(axis=0) > 3)[0]
    if len(cr) == 0 or len(cc) == 0:
        print("  No content found")
        return False

    print(f"  Content: rows {cr[0]}-{cr[-1]}, cols {cc[0]}-{cc[-1]}")

    pad = 5
    cropped = clean.crop((
        max(0, cc[0] - pad), max(0, cr[0] - pad),
        min(clean.width, cc[-1] + 1 + pad),
        min(clean.height, cr[-1] + 1 + pad),
    ))
    cw, ch = cropped.size
    print(f"  Cropped: {cw}x{ch}")

    # Split into 4 frames, fit each into 128x128
    quarter = cw // 4
    frames = []
    for i in range(4):
        frame = cropped.crop((quarter * i, 0, quarter * (i + 1), ch))
        fa = np.array(frame)[:, :, 3]
        fr = np.where((fa > 20).sum(axis=1) > 2)[0]
        fc = np.where((fa > 20).sum(axis=0) > 2)[0]
        if len(fr) > 0 and len(fc) > 0:
            sprite = frame.crop((fc[0], fr[0], fc[-1] + 1, fr[-1] + 1))
            sw, sh = sprite.size
            scale = min(120.0 / sw, 120.0 / sh)
            nw, nh = int(sw * scale), int(sh * scale)
            sprite = sprite.resize((nw, nh), Image.LANCZOS)
            cell = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
            xo = (128 - nw) // 2
            yo = 128 - nh - 4  # bottom-align
            cell.paste(sprite, (xo, yo))
            frames.append(cell)
            print(f"    Frame {i}: {sw}x{sh} -> {nw}x{nh}")
        else:
            frames.append(Image.new("RGBA", (128, 128), (0, 0, 0, 0)))

    sheet = Image.new("RGBA", (512, 128), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        sheet.paste(f, (128 * i, 0))
    sheet.save(output_path)
    print(f"  Output: {output_path}")
    return True


def build_prompt(hero: dict) -> str:
    return f"""I've attached a reference image of a character standing idle (128x128). Generate a 512x128 pixel walk cycle sprite sheet that MATCHES THIS CHARACTER EXACTLY.

CRITICAL: Show the COMPLETE CHARACTER from HEAD TO FEET in every frame. Do NOT crop at the waist or knees. The ENTIRE body including shoes must be visible. Each character should fill ~80% of vertical space.

4 sprites in a single horizontal row, each 128x128 pixels. Transparent PNG background.

Character: {hero['character']}
Match the attached reference image's style, proportions, colors, and details EXACTLY.
Bold black outlines (2-3px), flat colors, vector art style.

WALK CYCLE — {hero['walk_notes']}
Frame 1: Left foot clearly forward, right foot back. Full stride.
Frame 2: Feet passing center, body at lowest point.
Frame 3: Right foot clearly forward, left foot back. Mirror of frame 1.
Frame 4: Feet passing center, body at highest point.

IMPORTANT: Foot positions MUST be clearly different each frame. Body, face, and held items stay consistent. Only legs change. Shoes visible at bottom of every frame.

Style: Smooth clean vector game art. NOT pixel art. Into the Breach / Wargroove aesthetic."""


def generate_hero(name: str) -> bool:
    hero = HEROES[name]
    idle_path = SPRITES / f"{name}_idle.png"
    raw_path = RAW_DIR / f"{name}_walk_raw.png"
    final_path = SPRITES / f"{name}_walk.png"

    if not idle_path.exists():
        print(f"Idle sprite not found: {idle_path}")
        return False

    print(f"\n=== Generating {name} walk sheet ===")
    print(f"  Idle ref: {idle_path}")

    # Find ChatGPT tab
    if not find_chatgpt_tab():
        print("  ChatGPT tab not found in Chrome")
        return False

    # Open new chat
    if not open_new_chat():
        print("  Failed to open new chat")
        return False

    # Upload idle sprite as reference
    if not upload_reference(str(idle_path)):
        print("  Failed to upload reference (continuing without it)")

    # Send prompt
    prompt = build_prompt(hero)
    if not send_prompt(prompt):
        print("  Failed to send prompt")
        return False
    print("  Prompt sent, waiting for generation...")

    # Wait for image
    if not wait_for_image(timeout=120):
        print("  Timeout waiting for image")
        return False
    print("  Image generated!")

    # Download
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    if not download_first_image(raw_path):
        print("  Download failed")
        return False
    print(f"  Raw: {raw_path}")

    # Process
    if not process_full_body(raw_path, final_path):
        print("  Processing failed")
        return False

    print(f"  Done: {final_path}")
    return True


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: python3 {sys.argv[0]} <hero_name>")
        print(f"Available: {', '.join(HEROES.keys())}")
        sys.exit(1)

    name = sys.argv[1]
    if name not in HEROES:
        print(f"Unknown hero: {name}. Available: {', '.join(HEROES.keys())}")
        sys.exit(1)

    success = generate_hero(name)
    sys.exit(0 if success else 1)
