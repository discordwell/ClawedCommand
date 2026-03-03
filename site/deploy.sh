#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SSH_HOST="${DEPLOY_SSH_HOST:-ovh2}"
REMOTE_PATH="${DEPLOY_PATH:-/opt/clawed/site/}"

echo "=== ClawedCommand Deploy ==="
echo ""

# Step 1: Prepare assets
echo ">> Preparing assets..."
python3 "$SCRIPT_DIR/prepare_assets.py"
echo ""

# Step 2: Sync to VPS (exclude downloads dir — binaries uploaded separately)
echo ">> Deploying to ${SSH_HOST}:${REMOTE_PATH}"
rsync -avz --delete \
  --exclude='prepare_assets.py' \
  --exclude='deploy.sh' \
  --exclude='downloads/' \
  -e "ssh" \
  "$SCRIPT_DIR/" \
  "${SSH_HOST}:${REMOTE_PATH}"

echo ""
echo "=== Deploy complete ==="
echo "Site: https://clawedcommand.com"
