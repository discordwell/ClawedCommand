#!/usr/bin/env python3
"""Generate 3D GLB models from 2D sprites via the Tripo API.

Usage:
    python generate_3d_model.py \
        --input assets/sprites/units/pawdler_idle.png \
        --output assets/models/units/pawdler.glb \
        --face-limit 5000
"""

import argparse
import os
import sys
import time
from pathlib import Path

import requests
from dotenv import load_dotenv

API_BASE = "https://api.tripo3d.ai/v2/openapi"
POLL_INTERVAL = 5  # seconds
MAX_POLLS = 120  # 10 minutes max


def get_api_key() -> str:
    """Load TRIPO_API_KEY from project root .env file."""
    project_root = Path(__file__).resolve().parents[3]
    env_path = project_root / ".env"
    load_dotenv(env_path)
    key = os.getenv("TRIPO_API_KEY")
    if not key:
        print(f"Error: TRIPO_API_KEY not found. Set it in {env_path}", file=sys.stderr)
        sys.exit(1)
    return key


def upload_image(api_key: str, image_path: Path) -> str:
    """Upload an image file to Tripo and return the file token."""
    print(f"Uploading {image_path.name}...")
    with open(image_path, "rb") as f:
        resp = requests.post(
            f"{API_BASE}/upload",
            headers={"Authorization": f"Bearer {api_key}"},
            files={"file": (image_path.name, f, "image/png")},
        )
    resp.raise_for_status()
    data = resp.json()
    if data.get("code") != 0:
        print(f"Upload failed: {data}", file=sys.stderr)
        sys.exit(1)
    token = data["data"]["image_token"]
    print(f"Upload complete. Token: {token[:20]}...")
    return token


def create_task(api_key: str, image_token: str, face_limit: int) -> str:
    """Create an image-to-model task and return the task ID."""
    print(f"Creating image-to-model task (face_limit={face_limit})...")
    resp = requests.post(
        f"{API_BASE}/task",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        json={
            "type": "image_to_model",
            "file": {"type": "image", "file_token": image_token},
            "face_limit": face_limit,
            "texture": True,
            "pbr": True,
        },
    )
    resp.raise_for_status()
    data = resp.json()
    if data.get("code") != 0:
        print(f"Task creation failed: {data}", file=sys.stderr)
        sys.exit(1)
    task_id = data["data"]["task_id"]
    print(f"Task created: {task_id}")
    return task_id


def poll_task(api_key: str, task_id: str) -> dict:
    """Poll task status until completion. Returns the task result data."""
    for i in range(MAX_POLLS):
        resp = requests.get(
            f"{API_BASE}/task/{task_id}",
            headers={"Authorization": f"Bearer {api_key}"},
        )
        resp.raise_for_status()
        data = resp.json()["data"]
        status = data.get("status")
        progress = data.get("progress", 0)

        if status == "success":
            print(f"\nTask complete!")
            return data
        elif status in ("failed", "cancelled", "unknown"):
            print(f"\nTask {status}: {data}", file=sys.stderr)
            sys.exit(1)
        else:
            print(f"\r  Status: {status} ({progress}%)", end="", flush=True)
            time.sleep(POLL_INTERVAL)

    print(f"\nTimeout after {MAX_POLLS * POLL_INTERVAL}s", file=sys.stderr)
    sys.exit(1)


def download_glb(api_key: str, task_data: dict, output_path: Path):
    """Download the GLB model from the completed task."""
    model_url = task_data["output"]["model"]
    print(f"Downloading GLB from {model_url[:60]}...")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    resp = requests.get(model_url, stream=True)
    resp.raise_for_status()
    with open(output_path, "wb") as f:
        for chunk in resp.iter_content(chunk_size=8192):
            f.write(chunk)
    size_kb = output_path.stat().st_size / 1024
    print(f"Saved to {output_path} ({size_kb:.0f} KB)")


def main():
    parser = argparse.ArgumentParser(
        description="Generate 3D GLB models from 2D sprites via Tripo API"
    )
    parser.add_argument(
        "--input", required=True, type=Path, help="Input sprite image path"
    )
    parser.add_argument(
        "--output", required=True, type=Path, help="Output GLB model path"
    )
    parser.add_argument(
        "--face-limit",
        type=int,
        default=5000,
        help="Max polygon faces (default: 5000)",
    )
    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: Input file not found: {args.input}", file=sys.stderr)
        sys.exit(1)

    api_key = get_api_key()
    image_token = upload_image(api_key, args.input)
    task_id = create_task(api_key, image_token, args.face_limit)
    task_data = poll_task(api_key, task_id)
    download_glb(api_key, task_data, args.output)
    print("Done!")


if __name__ == "__main__":
    main()
