#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SSH_HOST="${DEPLOY_SSH_HOST:-ovh2}"
REMOTE_PATH="${DEPLOY_PATH:-/opt/clawed/site/}"

echo "=== ClawedCommand Deploy ==="
echo ""

# Step 1: Prepare assets
echo ">> Preparing assets..."
python3 "$SCRIPT_DIR/prepare_assets.py"
echo ""

# Step 2: Build WASM
echo ">> Building WASM..."
rustup target add wasm32-unknown-unknown 2>/dev/null || true

cargo build --manifest-path "$PROJECT_ROOT/Cargo.toml" \
  --target wasm32-unknown-unknown \
  -p cc_client \
  --no-default-features --features wasm-agent \
  --profile wasm-release

echo ">> Running wasm-bindgen..."
WASM_INPUT="$PROJECT_ROOT/target/wasm32-unknown-unknown/wasm-release/cc_client.wasm"
WASM_OUT="$SCRIPT_DIR/wasm"
mkdir -p "$WASM_OUT"

wasm-bindgen "$WASM_INPUT" \
  --out-dir "$WASM_OUT" \
  --target web \
  --no-typescript

# Optional: optimize with wasm-opt if available
if command -v wasm-opt &>/dev/null; then
  echo ">> Optimizing WASM with wasm-opt..."
  wasm-opt -Oz "$WASM_OUT/cc_client_bg.wasm" -o "$WASM_OUT/cc_client_bg.wasm"
fi

WASM_SIZE=$(du -h "$WASM_OUT/cc_client_bg.wasm" | cut -f1)
echo ">> WASM build complete: $WASM_SIZE"
echo ""

# Step 3: Sync to VPS (exclude downloads dir — binaries uploaded separately)
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
echo "Play: https://clawedcommand.com/play"
