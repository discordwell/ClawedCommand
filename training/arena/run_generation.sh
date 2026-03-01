#!/bin/bash
# Run a generation of arena matches.
# Usage: ./run_generation.sh <gen_number> [p0_scripts_dir] [p1_scripts_dir]

set -e

GEN="${1:?Usage: run_generation.sh <gen_number> [p0_scripts_dir] [p1_scripts_dir]}"
P0_SCRIPTS="${2:-}"
P1_SCRIPTS="${3:-}"
SEEDS="42,123,7777,9999,31415"
MAX_TICKS=6000

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/results/gen_$(printf '%03d' "$GEN")"

echo "=== Generation $GEN ==="
echo "Output: $OUTPUT_DIR"

ARGS=(--seeds "$SEEDS" --max-ticks "$MAX_TICKS" --output "$OUTPUT_DIR")

if [ -n "$P0_SCRIPTS" ]; then
    echo "P0 scripts: $P0_SCRIPTS"
    ARGS+=(--p0-scripts "$P0_SCRIPTS")
else
    echo "P0 scripts: none (FSM only)"
fi

if [ -n "$P1_SCRIPTS" ]; then
    echo "P1 scripts: $P1_SCRIPTS"
    ARGS+=(--p1-scripts "$P1_SCRIPTS")
else
    echo "P1 scripts: none (FSM only)"
fi

echo ""

cd "$PROJECT_ROOT"
cargo run -p cc_agent --bin arena --features harness -- "${ARGS[@]}"
