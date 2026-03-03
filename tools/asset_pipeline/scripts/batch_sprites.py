#!/usr/bin/env python3
"""Batch generate sprites via ChatGPT + AppleScript download.

Sends prompts via MCP (claude-in-chrome), downloads via AppleScript.
Requires "Allow JavaScript from Apple Events" enabled in Chrome.

Usage: python3 batch_sprites.py [start_index]
"""
import subprocess, sys, time, shutil
from pathlib import Path

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
RAW_DIR = PROJECT / "tools/asset_pipeline/raw/units"
OUT_DIR = PROJECT / "assets/sprites/units"

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


def upload_style_reference() -> bool:
    """Upload a style reference image to the ChatGPT chat."""
    ref_image = str(PROJECT / "assets/sprites/units/chonk_idle.png")

    # Use AppleScript to inject the file into the file input
    js = f'''
(function() {{
    // Find the file input (hidden) or the "Add files" button
    var fileInput = document.querySelector('input[type="file"]');
    if (!fileInput) {{
        // Click the "Add files" button to reveal file input
        var addBtn = document.querySelector('button[aria-label="Add files and more"]');
        if (addBtn) addBtn.click();
        return "clicked_add_btn";
    }}
    return "file_input_found";
}})()
'''
    r = applescript_js(js)
    if "clicked_add_btn" in r:
        time.sleep(1)

    # Use DataTransfer API to inject the file
    # First read the file as base64 via Python and inject
    import base64
    with open(ref_image, "rb") as f:
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
        print("    Style reference uploaded")
        time.sleep(2)
        return True
    print(f"    Style ref upload: {r}")
    return False


def send_prompt(animal: str, description: str, with_style_ref: bool = True) -> bool:
    """Send sprite generation prompt, optionally with style reference."""
    if with_style_ref:
        upload_style_reference()

    style_line = "Use the attached image as a STYLE REFERENCE. Match its art style exactly: clean vector art, bold dark outlines, flat colors, isometric perspective." if with_style_ref else ""

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


def wait_for_image(timeout=90) -> bool:
    """Wait for ChatGPT to generate images."""
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


def download_and_process(slug: str) -> bool:
    """Download first generated image via curl and process to 128x128."""
    from PIL import Image

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

    if dl_path.stat().st_size < 1000:
        print(f"    Downloaded file too small: {dl_path.stat().st_size} bytes")
        dl_path.unlink()
        return False

    # Copy raw
    raw_path = RAW_DIR / f"{slug}_raw.png"
    shutil.copy2(dl_path, raw_path)

    # Process to 128x128
    img = Image.open(str(raw_path)).convert("RGBA")
    bbox = img.getbbox()
    if bbox:
        cropped = img.crop(bbox)
        cropped.thumbnail((128, 128), Image.LANCZOS)
        canvas = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
        x = (128 - cropped.width) // 2
        y = (128 - cropped.height) // 2
        canvas.paste(cropped, (x, y))
        out_path = OUT_DIR / f"{slug}_idle.png"
        canvas.save(str(out_path))
        print(f"    {cropped.width}x{cropped.height} -> 128x128")

    dl_path.unlink()
    return True


def generate_one(slug: str, animal: str, description: str) -> bool:
    """Full pipeline for one sprite."""
    print(f"\n  [{slug}] {animal}")

    # Navigate to new chat
    if not open_new_chat():
        print("    Failed to load new chat")
        return False

    # Send prompt
    if not send_prompt(animal, description):
        print("    Failed to send prompt")
        return False
    print("    Prompt sent, waiting for generation...")

    # Wait for image
    if not wait_for_image():
        print("    Timeout waiting for image")
        return False

    # Download and process
    return download_and_process(slug)


def main():
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

        success = generate_one(slug, animal, desc)
        if success:
            done += 1
            print(f"    Done! ({done}/40)")
        else:
            failed.append(slug)
            print(f"    FAILED")

    print(f"\nComplete: {done}/40 sprites")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
