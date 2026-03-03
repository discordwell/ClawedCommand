#!/bin/bash
# Download the first generated image from the active ChatGPT tab via AppleScript
# Usage: download_sprite.sh <slug>
SLUG="$1"
if [ -z "$SLUG" ]; then echo "Usage: $0 <slug>"; exit 1; fi

RAW_DIR="/Users/discordwell/Projects/ClawedCommand/tools/asset_pipeline/raw/units"
OUT_DIR="/Users/discordwell/Projects/ClawedCommand/assets/sprites/units"

# Download via AppleScript (requires "Allow JavaScript from Apple Events" enabled in Chrome)
osascript -e "
tell application \"Google Chrome\"
    tell active tab of window 1
        execute javascript \"
            (function() {
                var imgs = document.querySelectorAll('img[alt=\\\"Generated image\\\"]');
                if (imgs.length > 0) {
                    var img = imgs[0];
                    var canvas = document.createElement('canvas');
                    canvas.width = img.naturalWidth;
                    canvas.height = img.naturalHeight;
                    var ctx = canvas.getContext('2d');
                    ctx.drawImage(img, 0, 0);
                    canvas.toBlob(function(blob) {
                        var url = URL.createObjectURL(blob);
                        var a = document.createElement('a');
                        a.href = url;
                        a.download = 'sprite_${SLUG}.png';
                        document.body.appendChild(a);
                        a.click();
                        document.body.removeChild(a);
                        URL.revokeObjectURL(url);
                    }, 'image/png');
                    return 'ok';
                }
                return 'no_images';
            })()
        \"
    end tell
end tell"

sleep 2

# Copy raw
if [ -f "$HOME/Downloads/sprite_${SLUG}.png" ]; then
    cp "$HOME/Downloads/sprite_${SLUG}.png" "$RAW_DIR/${SLUG}_raw.png"
    # Process to 128x128
    python3 -c "
from PIL import Image
img = Image.open('$RAW_DIR/${SLUG}_raw.png').convert('RGBA')
bbox = img.getbbox()
if bbox:
    cropped = img.crop(bbox)
    cropped.thumbnail((128, 128), Image.LANCZOS)
    canvas = Image.new('RGBA', (128, 128), (0,0,0,0))
    x = (128 - cropped.width) // 2
    y = (128 - cropped.height) // 2
    canvas.paste(cropped, (x, y))
    canvas.save('$OUT_DIR/${SLUG}_idle.png')
    print(f'${SLUG}: {cropped.width}x{cropped.height} -> 128x128')
"
    rm "$HOME/Downloads/sprite_${SLUG}.png"
    echo "Done: $SLUG"
else
    echo "FAILED: sprite_${SLUG}.png not found in Downloads"
fi
