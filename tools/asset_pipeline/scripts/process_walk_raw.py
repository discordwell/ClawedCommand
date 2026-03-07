#!/usr/bin/env python3
"""Process a raw ChatGPT walk sheet image into a game-ready 512x128 sprite sheet.

Takes a raw 1536x1024 ChatGPT output, cleans faint alpha, crops to content,
scales to height=128, and regrids into 4 proper 128x128 cells.

Usage: python3 process_walk_raw.py <raw_input.png> <output.png>
"""
import sys
from pathlib import Path
from PIL import Image
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from regrid_sheet import regrid_sheet, find_sprite_blobs, merge_nearby_blobs


def process_raw_walk(input_path: str, output_path: str) -> bool:
    from image_utils import remove_background

    img = Image.open(input_path).convert("RGBA")
    w, h = img.size
    arr = np.array(img)
    alpha = arr[:, :, 3]

    # If image is mostly opaque, use rembg for background removal
    if alpha.min() > 200:
        print("  Removing background with rembg...")
        img = remove_background(img)
        arr = np.array(img)
        alpha = arr[:, :, 3]

    # Zero out faint pixels
    mask = alpha < 50
    arr[mask] = [0, 0, 0, 0]
    clean = Image.fromarray(arr)
    alpha = arr[:, :, 3]

    # For 2-row layouts (e.g. 1536x1024), take only the top half
    if h > w * 0.6:
        # Likely a 2-row layout — find the row with the most content in the top half
        top_half_alpha = alpha[:h // 2, :]
        bot_half_alpha = alpha[h // 2:, :]
        top_content = (top_half_alpha > 50).sum()
        bot_content = (bot_half_alpha > 50).sum()
        if top_content > 0 and bot_content > 0:
            # Both halves have content — take the half with more
            if top_content >= bot_content:
                clean = clean.crop((0, 0, w, h // 2))
            else:
                clean = clean.crop((0, h // 2, w, h))
            arr = np.array(clean)
            alpha = arr[:, :, 3]
            print(f"  Took {'top' if top_content >= bot_content else 'bottom'} half ({top_content} vs {bot_content} px)")

    # Find content bounding box via alpha threshold
    content_rows = np.where((alpha > 50).sum(axis=1) > 10)[0]
    content_cols = np.where((alpha > 50).sum(axis=0) > 10)[0]

    if len(content_rows) == 0 or len(content_cols) == 0:
        print(f"  FAIL: no content found in {input_path}")
        return False

    bbox = (content_cols[0], content_rows[0], content_cols[-1] + 1, content_rows[-1] + 1)
    cropped = clean.crop(bbox)
    cw, ch = cropped.size
    print(f"  Cropped: {cw}x{ch}, aspect: {cw/ch:.2f}")

    # Scale to height=128
    new_h = 128
    new_w = int(cw * new_h / ch)
    resized = cropped.resize((new_w, new_h), Image.LANCZOS)

    # Place on canvas wide enough for regrid
    canvas_w = max(new_w, 512)
    canvas = Image.new("RGBA", (canvas_w, 128), (0, 0, 0, 0))
    canvas.paste(resized, (0, 0))

    # Regrid into 4x128x128 cells
    result = regrid_sheet(canvas, cell_w=128, cell_h=128, n_frames=4)
    if result is None:
        print(f"  FAIL: regrid couldn't process {input_path}")
        return False

    result.save(output_path)
    print(f"  OK: {output_path} ({result.size})")
    return True


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 process_walk_raw.py <raw_input.png> <output.png>")
        sys.exit(1)
    success = process_raw_walk(sys.argv[1], sys.argv[2])
    sys.exit(0 if success else 1)
