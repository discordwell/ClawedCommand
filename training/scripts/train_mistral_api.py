#!/usr/bin/env python3
"""Launch a fine-tuning job on the Mistral API.

Quick baseline: upload JSONL, start job on codestral-latest, poll until done.

Usage:
    export MISTRAL_API_KEY=your_key
    python train_mistral_api.py ../data/cc_train_mistral.jsonl ../data/cc_eval_mistral.jsonl

    # Custom model and steps
    python train_mistral_api.py train.jsonl eval.jsonl --model codestral-latest --steps 300
"""

import argparse
import os
import sys
import time
from pathlib import Path

from mistralai import Mistral


def main():
    parser = argparse.ArgumentParser(description="Mistral API fine-tuning")
    parser.add_argument("train_file", type=Path, help="Training JSONL")
    parser.add_argument("eval_file", type=Path, help="Evaluation JSONL")
    parser.add_argument(
        "--model", default="codestral-latest", help="Base model ID"
    )
    parser.add_argument("--steps", type=int, default=300, help="Training steps")
    parser.add_argument(
        "--lr", type=float, default=0.0001, help="Learning rate"
    )
    parser.add_argument(
        "--suffix", default="cc-v1", help="Model suffix for identification"
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Upload files but don't start training"
    )
    args = parser.parse_args()

    api_key = os.environ.get("MISTRAL_API_KEY")
    if not api_key:
        print("Error: MISTRAL_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    for f in [args.train_file, args.eval_file]:
        if not f.exists():
            print(f"Error: {f} not found", file=sys.stderr)
            sys.exit(1)

    client = Mistral(api_key=api_key)

    # Upload files
    print(f"Uploading training data: {args.train_file}")
    train_upload = client.files.upload(
        file={
            "file_name": args.train_file.name,
            "content": open(args.train_file, "rb"),
        },
        purpose="fine-tune",
    )
    print(f"  File ID: {train_upload.id}")

    print(f"Uploading eval data: {args.eval_file}")
    eval_upload = client.files.upload(
        file={
            "file_name": args.eval_file.name,
            "content": open(args.eval_file, "rb"),
        },
        purpose="fine-tune",
    )
    print(f"  File ID: {eval_upload.id}")

    if args.dry_run:
        print("\nDry run — files uploaded, skipping job creation.")
        return

    # Create fine-tuning job
    print(f"\nCreating fine-tuning job on {args.model}...")
    job = client.fine_tuning.jobs.create(
        model=args.model,
        training_files=[{"file_id": train_upload.id, "weight": 1}],
        validation_files=[eval_upload.id],
        hyperparameters={
            "training_steps": args.steps,
            "learning_rate": args.lr,
        },
        suffix=args.suffix,
        auto_start=True,
    )
    print(f"Job ID: {job.id}")
    print(f"Status: {job.status}")

    # Poll for completion
    print("\nPolling for completion (Ctrl+C to stop polling)...")
    try:
        while True:
            status = client.fine_tuning.jobs.get(job_id=job.id)
            print(f"  [{time.strftime('%H:%M:%S')}] Status: {status.status}")

            if status.status in ("SUCCESS", "FAILED", "CANCELLED"):
                break

            time.sleep(30)
    except KeyboardInterrupt:
        print(f"\nStopped polling. Job {job.id} is still running.")
        print(f"Check status: client.fine_tuning.jobs.get(job_id='{job.id}')")
        return

    if status.status == "SUCCESS":
        print(f"\nFine-tuning complete!")
        print(f"Model ID: {status.fine_tuned_model}")
        print(f"\nUse in inference:")
        print(f'  model="{status.fine_tuned_model}"')
    else:
        print(f"\nJob ended with status: {status.status}")
        if hasattr(status, "error") and status.error:
            print(f"Error: {status.error}")


if __name__ == "__main__":
    main()
