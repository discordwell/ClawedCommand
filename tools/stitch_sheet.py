#!/usr/bin/env python3
"""Convert a 2x2 grid (1024x1024) or 1x4 strip into a 4096x1024 horizontal sprite sheet."""
import sys
from PIL import Image

def process_sheet(input_path, output_path):
    img = Image.open(input_path).convert("RGBA")
    w, h = img.size

    if w == 4096 and h == 1024:
        # Already correct format
        img.save(output_path)
        print(f"Already 4096x1024, saved as-is: {output_path}")
        return

    if w == h:
        # Square image — assume 2x2 grid
        half = w // 2
        frames = [
            img.crop((0, 0, half, half)),         # top-left = frame 1
            img.crop((half, 0, w, half)),          # top-right = frame 2
            img.crop((0, half, half, h)),          # bottom-left = frame 3
            img.crop((half, half, w, h)),          # bottom-right = frame 4
        ]
    elif w > h:
        # Wide image — assume 1x4 horizontal strip
        fw = w // 4
        frames = [img.crop((i * fw, 0, (i + 1) * fw, h)) for i in range(4)]
    else:
        # Tall image — assume 4x1 vertical strip
        fh = h // 4
        frames = [img.crop((0, i * fh, w, (i + 1) * fh)) for i in range(4)]

    # Resize each frame to 1024x1024
    frames = [f.resize((1024, 1024), Image.LANCZOS) for f in frames]

    # Stitch horizontally
    sheet = Image.new("RGBA", (4096, 1024), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        sheet.paste(f, (i * 1024, 0))

    sheet.save(output_path)
    print(f"Processed {img.size} -> 4096x1024: {output_path}")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: stitch_sheet.py <input.png> <output.png>")
        sys.exit(1)
    process_sheet(sys.argv[1], sys.argv[2])
