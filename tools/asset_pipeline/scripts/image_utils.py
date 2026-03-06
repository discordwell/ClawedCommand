"""Shared image processing utilities for the asset pipeline."""

import sys
import time

import numpy as np
from PIL import Image

try:
    from rembg import remove as rembg_remove
    HAS_REMBG = True
except ImportError:
    HAS_REMBG = False


def remove_background(img: Image.Image) -> Image.Image:
    """Remove background using rembg, or skip if already has alpha."""
    if img.mode == "RGBA":
        alpha = np.array(img.split()[-1])
        if alpha.min() < 250:
            print("  Alpha channel detected, skipping bg removal")
            return img

    if not HAS_REMBG:
        print("  Warning: rembg not installed, skipping bg removal", file=sys.stderr)
        return img.convert("RGBA")

    print("  Removing background with rembg...")
    return rembg_remove(img).convert("RGBA")


def crop_to_content(img: Image.Image, padding: int = 2) -> Image.Image:
    """Crop to bounding box of non-transparent pixels with optional padding."""
    alpha = np.array(img.split()[-1])
    rows = np.any(alpha > 0, axis=1)
    cols = np.any(alpha > 0, axis=0)

    if not rows.any() or not cols.any():
        print("  Warning: image is fully transparent after bg removal", file=sys.stderr)
        return img

    rmin, rmax = np.where(rows)[0][[0, -1]]
    cmin, cmax = np.where(cols)[0][[0, -1]]

    rmin = max(0, rmin - padding)
    rmax = min(img.height - 1, rmax + padding)
    cmin = max(0, cmin - padding)
    cmax = min(img.width - 1, cmax + padding)

    cropped = img.crop((cmin, rmin, cmax + 1, rmax + 1))
    print(f"  Cropped: {img.size} → {cropped.size}")
    return cropped


def resize_to_fit(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """Resize image to fit within target dimensions, preserving aspect ratio.
    Centers the result on a transparent canvas of exact target size."""
    scale = min(target_w / img.width, target_h / img.height)

    if scale >= 1.0:
        new_w, new_h = img.width, img.height
    else:
        new_w = int(img.width * scale)
        new_h = int(img.height * scale)
        img = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
        print(f"  Resized to: {new_w}x{new_h}")

    canvas = Image.new("RGBA", (target_w, target_h), (0, 0, 0, 0))
    offset_x = (target_w - new_w) // 2
    offset_y = (target_h - new_h) // 2
    canvas.paste(img, (offset_x, offset_y), img)
    return canvas


def validate_sprite_quality(img: Image.Image, sprite_type: str = "idle") -> tuple[bool, str]:
    """Validate sprite quality: transparency, sharpness, content.

    Args:
        img: RGBA PIL Image to validate.
        sprite_type: "idle" (transparency + sharpness), "portrait" (sharpness only),
                     "sheet" (sharpness only).

    Returns:
        (passed, reason) tuple.
    """
    arr = np.array(img.convert("RGBA"))
    alpha = arr[:, :, 3]
    total_pixels = alpha.size

    # Content check — must have at least 1% opaque pixels
    opaque_count = np.count_nonzero(alpha > 10)
    if opaque_count < total_pixels * 0.01:
        return False, f"empty/invisible — only {opaque_count}/{total_pixels} opaque pixels"

    # Transparency check (idle sprites only — portraits have painted backgrounds)
    if sprite_type == "idle":
        opaque_ratio = np.count_nonzero(alpha > 200) / total_pixels
        if opaque_ratio > 0.95:
            return False, f"fully opaque ({opaque_ratio:.1%}) — likely mid-generation grab or missing bg removal"

    # Sharpness check via Laplacian variance (pure numpy, no scipy)
    gray = np.mean(arr[:, :, :3], axis=2).astype(np.float64)
    # Pad for 3x3 Laplacian kernel
    padded = np.pad(gray, 1, mode="edge")
    laplacian = (
        padded[:-2, 1:-1] + padded[2:, 1:-1] +
        padded[1:-1, :-2] + padded[1:-1, 2:] -
        4 * padded[1:-1, 1:-1]
    )
    edge_var = np.var(laplacian)
    if edge_var < 500:
        return False, f"blurry — Laplacian variance {edge_var:.0f} < 500"

    return True, f"ok (opaque={np.count_nonzero(alpha > 200) / total_pixels:.0%}, sharpness={edge_var:.0f})"


def download_and_validate(raw_path: str, out_path: str,
                          sprite_type: str = "idle",
                          crop_size: tuple[int, int] | None = None) -> tuple[bool, str]:
    """Consolidated processing chain: load -> rembg if opaque -> crop -> resize -> QC -> save.

    Args:
        raw_path: Path to the raw downloaded image.
        out_path: Path to save the processed result.
        sprite_type: "idle", "portrait", or "sheet".
        crop_size: (width, height) to fit into, or None to keep original size.

    Returns:
        (success, reason) tuple.
    """
    from pathlib import Path

    raw = Path(raw_path)
    if not raw.exists():
        return False, f"raw file not found: {raw_path}"

    if raw.stat().st_size < 5000:
        return False, f"file too small: {raw.stat().st_size}B"

    img = Image.open(str(raw)).convert("RGBA")

    # rembg for idle sprites: if fully opaque, ChatGPT/Gemini didn't give transparency
    if sprite_type == "idle":
        alpha = np.array(img.split()[-1])
        if alpha.min() > 200:
            img = remove_background(img)

    # QC gate (skip for sheets — they have different structure)
    if sprite_type in ("idle", "portrait"):
        passed, reason = validate_sprite_quality(img, sprite_type=sprite_type)
        if not passed:
            return False, f"QC fail: {reason}"
    else:
        reason = "sheet (no QC)"

    # Crop/resize to target dimensions
    if crop_size:
        w, h = crop_size
        bbox = img.getbbox()
        if not bbox:
            return False, "empty image (fully transparent)"
        cropped = img.crop(bbox)
        cropped.thumbnail((w, h), Image.Resampling.LANCZOS)
        canvas = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        x = (w - cropped.width) // 2
        y = (h - cropped.height) // 2
        canvas.paste(cropped, (x, y), cropped)
        img = canvas

    out = Path(out_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(str(out))
    return True, f"ok ({img.size[0]}x{img.size[1]}, {reason})"


def wait_for_stable_image(applescript_js_fn, timeout: int = 120,
                          settle_time: float = 8.0,
                          poll_interval: float = 3.0) -> bool:
    """Wait for a ChatGPT-generated image to fully render and stabilize.

    Phase 1: Wait for img[alt="Generated image"] to exist in DOM.
    Phase 2: Wait for img.complete && img.naturalWidth > 0.
    Phase 3: Wait settle_time after image src URL stabilizes on two consecutive polls.

    Args:
        applescript_js_fn: Callable that takes JS code string and returns result string.
        timeout: Max total wait time in seconds.
        settle_time: Seconds to wait after src URL stabilizes.
        poll_interval: Seconds between polls.

    Returns:
        True if image is stable, False on timeout.
    """
    start = time.time()

    # Phase 1: Wait for image element to exist
    while time.time() - start < timeout:
        time.sleep(poll_interval)
        r = applescript_js_fn(
            'document.querySelectorAll(\'img[alt="Generated image"]\').length.toString()'
        )
        try:
            if int(r) > 0:
                break
        except (ValueError, TypeError):
            pass
    else:
        return False

    # Phase 2: Wait for img.complete and naturalWidth > 0
    while time.time() - start < timeout:
        time.sleep(poll_interval)
        r = applescript_js_fn('''
(function() {
    var img = document.querySelector('img[alt="Generated image"]');
    if (!img) return "no_img";
    if (img.complete && img.naturalWidth > 0) return "loaded";
    return "loading";
})()
''')
        if "loaded" in r:
            break
    else:
        return False

    # Phase 3: Wait for src URL to stabilize
    last_src = None
    stable_since = None
    while time.time() - start < timeout:
        time.sleep(poll_interval)
        src = applescript_js_fn('''
(function() {
    var img = document.querySelector('img[alt="Generated image"]');
    return img ? img.src : "no_img";
})()
''')
        if src == last_src and last_src and "no_img" not in last_src:
            if stable_since is None:
                stable_since = time.time()
            elif time.time() - stable_since >= settle_time:
                return True
        else:
            last_src = src
            stable_since = None

    return False
