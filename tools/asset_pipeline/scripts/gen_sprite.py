#!/usr/bin/env python3
"""Generate a sprite via ChatGPT and download it via AppleScript.

Usage: python3 gen_sprite.py <slug> <animal> <description>
Example: python3 gen_sprite.py jayflicker "Shimmering jay" "shimmering jay with translucent afterimage copies flanking it, dynamic motion pose"
"""
import subprocess, sys, time, os, json
from pathlib import Path

PROJECT = Path("/Users/discordwell/Projects/ClawedCommand")
RAW_DIR = PROJECT / "tools/asset_pipeline/raw/units"
OUT_DIR = PROJECT / "assets/sprites/units"

def run_chrome_js(js_code: str) -> str:
    """Execute JavaScript in Chrome's active tab via AppleScript."""
    # Write JS to temp file to avoid quoting hell
    tmp = Path("/tmp/chrome_js.js")
    tmp.write_text(js_code)

    applescript = f'''
    tell application "Google Chrome"
        tell active tab of window 1
            execute javascript (read POSIX file "/tmp/chrome_js.js" as «class utf8»)
        end tell
    end tell
    '''
    result = subprocess.run(["osascript", "-e", applescript], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"AppleScript error: {result.stderr}", file=sys.stderr)
        return ""
    return result.stdout.strip()

def open_new_chat():
    """Open a new ChatGPT chat tab and wait for it to load."""
    subprocess.run(["osascript", "-e", '''
    tell application "Google Chrome"
        activate
        tell window 1
            make new tab with properties {URL:"https://chatgpt.com/"}
        end tell
    end tell
    '''], capture_output=True)
    # Wait for textarea to appear (up to 15 seconds)
    for _ in range(15):
        time.sleep(1)
        result = run_chrome_js('document.querySelector("#prompt-textarea") ? "ready" : "loading"')
        if "ready" in result:
            print("  Page loaded")
            return
    print("  Warning: page may not be fully loaded")

def send_prompt(animal: str, description: str):
    """Send the sprite generation prompt."""
    js = f'''
    (function() {{
        var textarea = document.querySelector("#prompt-textarea");
        if (!textarea) return "no textarea";
        var lines = [
            "Generate a 128x128 isometric sprite for a 2D RTS game.",
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
        setTimeout(function() {{
            var btn = document.querySelector('button[data-testid="send-button"]') || document.querySelector('button[aria-label="Send prompt"]');
            if (btn) btn.click();
        }}, 500);
        return "prompt_sent";
    }})()
    '''
    result = run_chrome_js(js)
    print(f"  Prompt: {result}")

def wait_for_image(timeout=45):
    """Wait for image generation to complete."""
    start = time.time()
    while time.time() - start < timeout:
        time.sleep(5)
        result = run_chrome_js('document.querySelectorAll(\'img[alt="Generated image"]\').length.toString()')
        try:
            count = int(result)
            if count > 0:
                print(f"  Images ready: {count}")
                return True
        except ValueError:
            pass
    print("  Timeout waiting for image")
    return False

def download_image(slug: str):
    """Download the first generated image."""
    js = f'''
    (function() {{
        var imgs = document.querySelectorAll('img[alt="Generated image"]');
        if (imgs.length > 0) {{
            var img = imgs[0];
            var canvas = document.createElement("canvas");
            canvas.width = img.naturalWidth;
            canvas.height = img.naturalHeight;
            var ctx = canvas.getContext("2d");
            ctx.drawImage(img, 0, 0);
            canvas.toBlob(function(blob) {{
                var url = URL.createObjectURL(blob);
                var a = document.createElement("a");
                a.href = url;
                a.download = "sprite_{slug}.png";
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            }}, "image/png");
            return "ok";
        }}
        return "no_images";
    }})()
    '''
    result = run_chrome_js(js)
    time.sleep(2)

    dl_path = Path.home() / "Downloads" / f"sprite_{slug}.png"
    if dl_path.exists():
        print(f"  Downloaded: {dl_path}")
        return True
    else:
        print(f"  Download not found at {dl_path}")
        return False

def process_sprite(slug: str):
    """Process downloaded sprite to 128x128."""
    from PIL import Image

    dl_path = Path.home() / "Downloads" / f"sprite_{slug}.png"
    raw_path = RAW_DIR / f"{slug}_raw.png"
    out_path = OUT_DIR / f"{slug}_idle.png"

    # Copy raw
    import shutil
    shutil.copy2(dl_path, raw_path)

    # Process
    img = Image.open(str(raw_path)).convert("RGBA")
    bbox = img.getbbox()
    if bbox:
        cropped = img.crop(bbox)
        cropped.thumbnail((128, 128), Image.LANCZOS)
        canvas = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
        x = (128 - cropped.width) // 2
        y = (128 - cropped.height) // 2
        canvas.paste(cropped, (x, y))
        canvas.save(str(out_path))
        print(f"  Processed: {cropped.width}x{cropped.height} -> 128x128")

    # Cleanup download
    dl_path.unlink()
    return out_path.exists()

def generate_sprite(slug: str, animal: str, description: str):
    """Full pipeline: new chat -> prompt -> wait -> download -> process."""
    print(f"\n[{slug}]")
    open_new_chat()
    send_prompt(animal, description)
    if not wait_for_image():
        return False
    if not download_image(slug):
        return False
    return process_sprite(slug)

if __name__ == "__main__":
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <slug> <animal> <description>")
        sys.exit(1)

    slug = sys.argv[1]
    animal = sys.argv[2]
    description = sys.argv[3]

    success = generate_sprite(slug, animal, description)
    sys.exit(0 if success else 1)
