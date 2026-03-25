#!/bin/bash
# Quick process: copy latest ChatGPT download, process as sheet, regrid
# Usage: ./quick_process.sh <unit_name> <action>
# Example: ./quick_process.sh regeneron attack

UNIT=$1
ACTION=$2
RAW_DIR="tools/asset_pipeline/raw/units"
OUT_DIR="assets/sprites/units"
LATEST=$(ls -t ~/Downloads/ChatGPT*.png 2>/dev/null | head -1)

if [ -z "$LATEST" ]; then
    echo "ERROR: No ChatGPT download found"
    exit 1
fi

echo "Processing: ${UNIT}_${ACTION}"
echo "Source: $LATEST"

cp "$LATEST" "${RAW_DIR}/${UNIT}_${ACTION}_raw.png"

if [ "$ACTION" = "idle" ]; then
    python3 tools/asset_pipeline/scripts/process_sprite.py \
        "${RAW_DIR}/${UNIT}_${ACTION}_raw.png" \
        "${OUT_DIR}/${UNIT}_idle.png" \
        --width 128 --height 128
else
    python3 tools/asset_pipeline/scripts/process_walk_raw.py \
        "${RAW_DIR}/${UNIT}_${ACTION}_raw.png" \
        "${OUT_DIR}/${UNIT}_${ACTION}.png"
    python3 tools/asset_pipeline/scripts/regrid_sheet.py \
        "${OUT_DIR}/${UNIT}_${ACTION}.png" 2>/dev/null
fi

echo "Done: ${OUT_DIR}/${UNIT}_${ACTION}.png"
