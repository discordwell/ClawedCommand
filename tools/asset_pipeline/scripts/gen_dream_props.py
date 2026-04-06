#!/usr/bin/env python3
"""Generate dream office prop sprites via ChatGPT.

Uses the same AppleScript+ChatGPT pipeline as batch_remaining.py.
Each prop is a 128x128 isometric sprite with transparent background.

Usage: python3 gen_dream_props.py [prop_name]
  If prop_name given, generate only that one. Otherwise generate all missing.
"""
import base64
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image

PROJECT = Path(__file__).resolve().parents[3]
SPRITE_DIR = PROJECT / "assets" / "sprites" / "dream"

# slug -> (display_name, description for ChatGPT prompt)
PROPS = {
    "phone": ("Secure Phone", "Military field telephone on a desk mount, olive drab handset, coiled cord, small base unit with buttons"),
    "bed_bunk": ("Bunk Bed", "Military bunk bed, metal frame, olive green blankets tightly made, thin pillow, footlocker at base"),
    "couch": ("Break Room Couch", "Worn institutional couch, dark green/brown fabric, slightly sagging cushions, armrests"),
    "food_tray": ("Mess Hall Tray", "Cafeteria tray with plate, cup, and utensils on a long table, institutional style"),
    "exit_gate": ("Base Gate", "Military base entrance gate, chain-link with razor wire top, guard booth window, red/white barrier arm"),
    "ammo_crate": ("Ammo Crate", "Olive drab wooden ammunition crate with stenciled markings, metal clasps, stacked 2 high"),
    "bulletin_board": ("Bulletin Board", "Cork bulletin board on wall with pinned papers, safety notices, a few photos, pushpins"),
    "water_fountain": ("Water Fountain", "Stainless steel wall-mounted drinking fountain, institutional style, small basin"),
    "window_view": ("Window", "Military building window looking out to desert/base exterior, venetian blinds half-open"),
    "briefing_map": ("Briefing Room Map", "Large wall-mounted tactical map with pins and string, situation board, standing podium"),
    "office_door": ("CO's Office Door", "Heavy wooden office door with brass nameplate, 'COMMANDING OFFICER' placard"),
    "medical_cabinet": ("Medical Cabinet", "White medical supply cabinet with red cross, glass door showing bandages and supplies"),
    "locker": ("Locker Row", "Row of tall olive/gray metal lockers, some slightly ajar, combination locks, name tape strips"),
    "tv_set": ("Break Room TV", "Wall-mounted CRT television showing news, small shelf underneath, cable box"),
    "photo_frame": ("Photo Wall", "Cluster of framed unit photos on wall, group shots, some faded, small American flag"),
    "coffee_machine": ("Broken Coffee Machine", "Commercial drip coffee maker with 'OUT OF ORDER' sign taped on, stained carafe"),
    "bench_outdoor": ("Courtyard Bench", "Concrete and wood outdoor bench, slightly weathered, near a trash can"),
    "humvee": ("Humvee", "Military HMMWV (Humvee) parked, olive drab, canvas top, spare tire on back"),
    "helicopter": ("Helicopter", "UH-60 Black Hawk helicopter on pad, rotors folded, tie-down chains visible"),
    "scif_door": ("SCIF Door", "Heavy vault-like door with cipher lock, 'RESTRICTED AREA' sign, red warning light above"),
    "server_rack": ("Server Rack", "19-inch server rack with blinking LEDs, cable management, blue status lights glowing"),
    "microwave": ("Break Room Microwave", "Countertop microwave on a small table, slightly stained, paper towel roll next to it"),
    "supply_shelf": ("Supply Shelf", "Metal industrial shelving with boxes, cleaning supplies, paper reams, labeled bins"),
    "chapel_pew": ("Chapel Pew", "Simple wooden church pew/bench, small altar with cloth, subdued cross on wall"),
    "pool_table": ("Pool Table", "Green felt pool table with balls racked, two cues on wall mount, overhead lamp"),
    "arcade_cabinet": ("Arcade Cabinet", "Classic standup arcade cabinet, colorful side art, joystick and buttons, glowing screen"),
    "washing_machine": ("Washing Machine", "Industrial front-load washing machine, one with 'OUT OF ORDER' sign, detergent on top"),
    "menu_board": ("Menu Board", "Cafeteria menu whiteboard with handwritten items, 'TODAY'S SPECIAL' header, dry-erase markers"),
    "mess_table": ("Mess Hall Table", "Long cafeteria table with attached bench seats, institutional gray, tray return sign"),
    "letter_envelope": ("Letter from Home", "Handwritten letter on a footlocker, envelope with stamps, reading glasses nearby"),
    "running_track": ("Running Track Marker", "Painted lane marker on dirt track, small distance sign, orange cone"),
    "flagpole": ("Flagpole", "Tall flagpole with American flag, concrete base, small spotlight"),
}

# Style reference — use one of the existing dream sprites for consistency
STYLE_REF = str(SPRITE_DIR / "desk_pc.png")

_target_tab = (1, 1)


def applescript_js(js_code: str) -> str:
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
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js(
            'document.querySelector("#prompt-textarea") ? "ready" : "loading"'
        )
        if "ready" in r:
            return True
    return False


def upload_style_ref(ref_path: str) -> bool:
    p = Path(ref_path)
    if not p.exists():
        print(f"    Style ref not found: {ref_path}")
        return False

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
    r = applescript_js(js)
    if "uploaded" in r:
        time.sleep(2)
        return True
    return False


def send_prompt(text: str) -> bool:
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


def wait_for_image(timeout=120) -> bool:
    start = time.time()
    while time.time() - start < timeout:
        js = '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length > 0) return "FOUND";
    // Fallback
    var all = document.querySelectorAll('img');
    for (var i of all) {
        if (i.naturalWidth > 400 && i.naturalHeight > 400) return "FOUND";
    }
    return "WAITING";
})()
'''
        r = applescript_js(js)
        if "FOUND" in r:
            return True
        time.sleep(5)
    return False


def download_image(out_path: Path) -> bool:
    js_url = '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length === 0) {
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
        print(f"    No image URL: {img_url[:80]}")
        return False

    cookies = applescript_js("document.cookie")

    dl_path = Path("/tmp/chatgpt_prop_dl.png")
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl failed: {result.stderr[:100]}")
        return False

    if dl_path.stat().st_size < 5000:
        print(f"    File too small: {dl_path.stat().st_size}B")
        dl_path.unlink()
        return False

    # Process: rembg if needed, crop to content, resize to 128x128
    img = Image.open(str(dl_path)).convert("RGBA")
    arr = np.array(img)
    alpha = arr[:, :, 3]

    if alpha.min() > 200:
        try:
            from rembg import remove
            print("    Removing background...")
            img = remove(img)
            arr = np.array(img)
            alpha = arr[:, :, 3]
        except ImportError:
            print("    rembg not installed")

    # Zero faint
    mask = alpha < 30
    arr[mask] = [0, 0, 0, 0]
    clean = Image.fromarray(arr)
    alpha = arr[:, :, 3]

    cr = np.where((alpha > 30).sum(axis=1) > 3)[0]
    cc = np.where((alpha > 30).sum(axis=0) > 3)[0]
    if len(cr) == 0 or len(cc) == 0:
        print("    No content found")
        return False

    cropped = clean.crop((cc[0], cr[0], cc[-1] + 1, cr[-1] + 1))
    cw, ch = cropped.size

    # Scale to fit 120x120 preserving aspect, center in 128x128
    scale = min(120.0 / cw, 120.0 / ch)
    nw, nh = int(cw * scale), int(ch * scale)
    resized = cropped.resize((nw, nh), Image.LANCZOS)

    cell = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
    xo = (128 - nw) // 2
    yo = 128 - nh - 4  # bottom-align
    cell.paste(resized, (xo, yo))

    out_path.parent.mkdir(parents=True, exist_ok=True)
    cell.save(str(out_path))
    print(f"    Saved: {out_path} ({nw}x{nh})")
    dl_path.unlink()
    return True


def build_prompt(name: str, description: str) -> str:
    return f"""I've attached a style reference image. Generate a 128x128 isometric prop sprite matching this exact art style.

Object: {name}
Description: {description}

Requirements:
- 128x128 pixel canvas, transparent PNG background
- Isometric 3/4 perspective (~30 degrees), facing south-east
- Object should fill about 70-80% of the canvas
- Bold black outlines (2-3px), flat colors, 2-3 value steps per hue
- Clean vector game art, NOT pixel art
- Into the Breach / Wargroove / Northgard aesthetic
- Consistent top-left lighting
- Military/institutional color palette (olive, gray, tan, muted tones)
- Must be instantly recognizable as a {name.lower()} even at small size"""


def generate_prop(slug: str) -> bool:
    name, desc = PROPS[slug]
    out_path = SPRITE_DIR / f"{slug}.png"

    print(f"\n  [{slug}]")

    if not find_chatgpt_tab():
        print("    ChatGPT tab not found")
        return False

    if not open_new_chat():
        print("    Failed to open new chat")
        return False

    if Path(STYLE_REF).exists():
        upload_style_ref(STYLE_REF)

    prompt = build_prompt(name, desc)
    if not send_prompt(prompt):
        print("    Failed to send prompt")
        return False
    print("    Waiting for generation...")

    if not wait_for_image(timeout=120):
        print("    Timeout")
        return False

    time.sleep(3)  # Let image fully render
    return download_image(out_path)


def main():
    SPRITE_DIR.mkdir(parents=True, exist_ok=True)

    if len(sys.argv) > 1:
        names = sys.argv[1:]
    else:
        # Generate all missing props
        names = [slug for slug in PROPS if not (SPRITE_DIR / f"{slug}.png").exists()]
        print(f"Generating {len(names)} missing props...")

    ok, fail = 0, 0
    for slug in names:
        if slug not in PROPS:
            print(f"Unknown prop: {slug}")
            continue
        for attempt in range(2):
            if attempt > 0:
                print(f"    Retry...")
            if generate_prop(slug):
                ok += 1
                break
        else:
            fail += 1

    print(f"\nDone: {ok} OK, {fail} failed")


if __name__ == "__main__":
    main()
