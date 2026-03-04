#!/usr/bin/env python3
"""Batch generate walk/attack sprite sheets via ChatGPT + AppleScript download.

Uses AppleScript to execute JS in Chrome (ChatGPT tab), sends prompts,
waits for image generation, downloads via curl with browser cookies.

Requires "Allow JavaScript from Apple Events" enabled in Chrome.

Usage: python3 batch_sheets.py [start_index]
"""
import subprocess, sys, time, shutil
from pathlib import Path

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
RAW_DIR = PROJECT / "tools/asset_pipeline/raw/units"
OUT_DIR = PROJECT / "assets/sprites/units"
PROCESS_SCRIPT = PROJECT / "tools/asset_pipeline/scripts/process_sheet_simple.py"

# (slug, name, walk_description, attack_description)
UNITS = [
    ("pawdler", "Pawdler — a cute cat worker with a fish basket and hard hat",
     "Waddle-trot carrying fish basket, alternating front paws stepping, basket bouncing slightly. Frame 1: left paw forward, Frame 2: mid-stride both paws under body, Frame 3: right paw forward, Frame 4: mid-stride returning to Frame 1",
     "Swing fish from basket overhead and slam down. Frame 1: wind-up pulling fish from basket, Frame 2: fish raised overhead, Frame 3: slamming fish down hard with motion lines, Frame 4: recovery returning to neutral"),

    ("nuisance", "Nuisance — a scrappy aggressive small cat with wild fur",
     "Low scurrying sprint, legs a blur, scrappy aggressive posture. Frame 1: crouched ready, Frame 2: leaping forward front paws extended, Frame 3: mid-sprint legs stretched, Frame 4: gathering legs for next leap",
     "Quick scratch combo with visible claws. Frame 1: rear back with claws out, Frame 2: first slash right paw swiping, Frame 3: second slash left paw swiping, Frame 4: recovery crouching back"),

    ("hisser", "Hisser — a bristled angry cat with arched back and fangs showing",
     "Aggressive prowl with arched back and bristled fur. Frame 1: stalking pose left paw raised, Frame 2: stepping forward menacingly, Frame 3: right paw raised bristled, Frame 4: completing step mouth open hissing",
     "Spit attack — ranged acid spit. Frame 1: inhale expanding chest, Frame 2: rear back mouth wide open, Frame 3: spit green projectile forward with motion lines, Frame 4: recover to prowl stance"),

    ("chonk", "Chonk — a very large round fat cat, heavy and slow",
     "Heavy slow waddle, belly swaying side to side. Frame 1: standing wide, Frame 2: leaning left with left paw forward, Frame 3: upright mid-step, Frame 4: leaning right with right paw forward",
     "Body slam attack. Frame 1: rear up on hind legs, Frame 2: lean forward tipping, Frame 3: SLAM into ground with dust cloud and impact lines, Frame 4: recovering pushing back up"),

    ("mouser", "Mouser — a sleek stealthy cat with narrowed eyes, ninja-like",
     "Stealthy creep, belly low to ground, careful paw placement. Frame 1: crouched low scanning, Frame 2: one paw carefully reaching forward, Frame 3: body sliding forward, Frame 4: other paw reaching, eyes focused",
     "Pounce attack. Frame 1: crouch low tensing muscles, Frame 2: spring upward into leap, Frame 3: strike with extended claws mid-air, Frame 4: land in crouch ready"),

    ("flying_fox", "Flying Fox — a cat with large bat-like wings, airborne",
     "Wing flap cycle in flight. Frame 1: wings fully spread at top of stroke, Frame 2: wings mid-downstroke, Frame 3: wings at bottom of stroke body at highest point, Frame 4: wings mid-upstroke",
     "Dive bomb attack. Frame 1: hover with wings spread, Frame 2: tuck wings begin diving, Frame 3: full dive downward claws extended, Frame 4: pull up with wings spread wide"),

    ("yowler", "Yowler — a vocal energetic cat with mouth often open, expressive",
     "Bouncy trot with mouth open yowling. Frame 1: standing mouth open, Frame 2: bounce up front paws off ground yowling, Frame 3: landing front paws down, Frame 4: bounce up again different pose",
     "Sonic yowl blast — sound wave attack. Frame 1: deep inhale chest expanding, Frame 2: mouth wide open starting yowl, Frame 3: visible sound wave rings emanating from mouth, Frame 4: recover exhausted"),

    ("catnapper", "Catnapper — a drowsy sleepy cat with half-closed eyes",
     "Sleepy shuffle, half-closed eyes, dragging feet. Frame 1: standing drowsy eyes half shut, Frame 2: lazy step forward dragging paw, Frame 3: yawning mid-step, Frame 4: another drowsy step nodding off",
     "Surprise pounce — eyes snap open. Frame 1: drowsy standing, Frame 2: eyes snap wide open alert, Frame 3: explosive spring forward claws out, Frame 4: settle back to drowsy pose"),

    ("ferret_sapper", "Ferret Sapper — a ferret (not cat) with TNT, mining helmet, and demolition gear",
     "Quick scamper with equipment bouncing. Frame 1: running pose left legs forward, Frame 2: mid-stride airborne, Frame 3: right legs forward landing, Frame 4: mid-stride airborne other direction",
     "Throw explosive. Frame 1: pull bomb from pack, Frame 2: wind up arm back, Frame 3: throw bomb forward with arc line, Frame 4: recovery watching explosion"),

    ("mech_commander", "Mech Commander — a cat piloting a large bipedal mech suit with cannons",
     "Mechanical stomp walk, mech legs alternating. Frame 1: left mech leg forward, Frame 2: weight shifting both legs planted, Frame 3: right mech leg forward, Frame 4: weight shifting back",
     "Cannon blast. Frame 1: aim cannons forward, Frame 2: charge glow on cannon barrels, Frame 3: FIRE with muzzle flash and recoil, Frame 4: cooldown steam venting"),
]

# All sheets to generate: (slug, anim_type, description)
SHEETS = []
for slug, name, walk_desc, attack_desc in UNITS:
    SHEETS.append((slug, "walk", name, walk_desc))
    SHEETS.append((slug, "attack", name, attack_desc))

# Global: (window_index, tab_index) for ChatGPT tab
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
        return f"ERROR: {result.stderr.strip()[:200]}"
    return result.stdout.strip()


def find_chatgpt_tab():
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
    """Navigate to a new ChatGPT chat."""
    applescript_js('window.location.href = "https://chatgpt.com/"')
    for _ in range(20):
        time.sleep(1)
        r = applescript_js('document.querySelector("#prompt-textarea") ? "ready" : "loading"')
        if "ready" in r:
            return True
    return False


def send_sheet_prompt(name: str, anim_type: str, description: str) -> bool:
    """Send sprite sheet generation prompt."""
    prompt_lines = [
        f"Generate a 4-frame {anim_type} animation sprite sheet.",
        "",
        f"Character: {name}",
        f"Animation: {description}",
        "",
        "Layout: 4 frames in a single horizontal row. Total image should be wider than tall (roughly 4:1 ratio).",
        "Each frame shows the character at the same scale, evenly spaced in a perfect grid.",
        "",
        "Requirements:",
        "- Isometric view (~30 degrees from top-down), character facing south-east",
        "- Transparent PNG background",
        "- Body/fur in neutral gray (#B0B0B0-#D0D0D0) — team color is applied in-engine",
        "- Only accent colors on eyes, equipment, and special features",
        "- Clean vector art, flat colors, bold dark outlines (2-3px)",
        "- Into the Breach / Northgard aesthetic",
        "- No gradients, 2-3 value steps per hue",
        "- Each frame must be CLEARLY DIFFERENT — actual limb/body position changes",
        "- Character stays centered in each frame cell (no drifting)",
        "- Animation should loop smoothly (frame 4 transitions back to frame 1)" if anim_type == "walk" else "- 4 distinct keyframes of the action sequence",
        "",
        "Generate ONLY this one sprite sheet image. Do not add extra characters or scenes.",
    ]

    # Escape for JS string
    escaped_lines = []
    for line in prompt_lines:
        escaped_lines.append(line.replace("\\", "\\\\").replace('"', '\\"').replace("'", "\\'"))

    js_lines_array = ",\n".join([f'        "{line}"' for line in escaped_lines])

    js_fill = f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    var lines = [
{js_lines_array}
    ];
    textarea.innerHTML = lines.map(function(l) {{ return "<p>" + (l || "<br>") + "</p>"; }}).join("");
    textarea.dispatchEvent(new Event("input", {{bubbles: true}}));
    setTimeout(function() {{
        var btn = document.querySelector('button[data-testid="send-button"]') || document.querySelector('button[aria-label="Send prompt"]');
        if (btn) btn.click();
    }}, 500);
    return "ok";
}})()
'''
    r = applescript_js(js_fill)
    return "ok" in r


def wait_for_image(timeout=120) -> bool:
    """Wait for ChatGPT to generate an image."""
    start = time.time()
    while time.time() - start < timeout:
        time.sleep(5)
        r = applescript_js('document.querySelectorAll(\'img[alt="Generated image"]\').length.toString()')
        try:
            if int(r) > 0:
                return True
        except (ValueError, TypeError):
            pass
    return False


def download_image(slug: str, anim_type: str) -> Path | None:
    """Download the first generated image via curl."""
    # Get image URL
    img_url = applescript_js('''(function() {
        var imgs = document.querySelectorAll('img[alt="Generated image"]');
        if (imgs.length === 0) return "no_images";
        return imgs[0].src;
    })()''')
    if "no_images" in img_url or not img_url.startswith("http"):
        print(f"    No image URL found: {img_url[:100]}")
        return None

    # Get cookies
    cookies = applescript_js("document.cookie")

    # Download
    dl_path = Path.home() / "Downloads" / f"{slug}_{anim_type}_raw.png"
    result = subprocess.run(
        ["curl", "-s", "-o", str(dl_path), "-H", f"Cookie: {cookies}", img_url],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0 or not dl_path.exists():
        print(f"    curl failed: {result.stderr[:100]}")
        return None
    if dl_path.stat().st_size < 1000:
        print(f"    Too small: {dl_path.stat().st_size} bytes")
        dl_path.unlink()
        return None

    print(f"    Downloaded: {dl_path.stat().st_size} bytes")
    return dl_path


def process_sheet(dl_path: Path, slug: str, anim_type: str) -> bool:
    """Process downloaded image into 512x128 sheet."""
    raw_path = RAW_DIR / f"{slug}_{anim_type}_raw.png"
    shutil.copy2(dl_path, raw_path)

    out_path = OUT_DIR / f"{slug}_{anim_type}.png"
    result = subprocess.run(
        ["python3", str(PROCESS_SCRIPT),
         str(raw_path), str(out_path),
         "--columns", "4", "--rows", "1",
         "--tile-width", "128", "--tile-height", "128"],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0:
        print(f"    Process failed: {result.stderr[:200]}")
        return False
    print(result.stdout.strip())
    dl_path.unlink(missing_ok=True)
    return True


def generate_one(slug: str, anim_type: str, name: str, description: str) -> bool:
    """Full pipeline for one sprite sheet."""
    print(f"\n  [{slug}_{anim_type}]")

    if not open_new_chat():
        print("    Failed to load new chat")
        return False

    if not send_sheet_prompt(name, anim_type, description):
        print("    Failed to send prompt")
        return False
    print("    Prompt sent, waiting for generation...")

    if not wait_for_image():
        print("    Timeout waiting for image")
        return False

    dl_path = download_image(slug, anim_type)
    if not dl_path:
        return False

    return process_sheet(dl_path, slug, anim_type)


def main():
    start_idx = int(sys.argv[1]) if len(sys.argv) > 1 else 0

    # Check which sheets are already real (>60KB = ChatGPT generated, <60KB = fake)
    existing_real = set()
    for p in OUT_DIR.glob("*_walk.png"):
        if p.stat().st_size > 60000:
            existing_real.add(p.stem)
    for p in OUT_DIR.glob("*_attack.png"):
        if p.stat().st_size > 60000:
            existing_real.add(p.stem)

    loc = find_chatgpt_tab()
    print(f"ChatGPT tab: {loc}")

    done = 0
    failed = []
    for i, (slug, anim_type, name, desc) in enumerate(SHEETS):
        if i < start_idx:
            continue
        sheet_key = f"{slug}_{anim_type}"
        if sheet_key in existing_real:
            print(f"  [{sheet_key}] already exists (real), skipping")
            done += 1
            continue

        success = generate_one(slug, anim_type, name, desc)
        if success:
            done += 1
            print(f"    Done! ({done}/{len(SHEETS)})")
        else:
            failed.append(sheet_key)
            print(f"    FAILED")

    print(f"\nComplete: {done}/{len(SHEETS)} sheets")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
