#!/usr/bin/env bash
# Cross-faction round-robin tournament runner.
# Runs all 15 unique faction pairs × 2 sides × 20 seeds = 600 matches.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
SEEDS="1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20"
MAX_TICKS="${MAX_TICKS:-6000}"

factions=("catGPT" "The Clawed" "Seekers of the Deep" "The Murder" "LLAMA" "Croak")
faction_slugs=("catgpt" "clawed" "seekers" "murder" "llama" "croak")

mkdir -p "$RESULTS_DIR"

echo "=== Cross-Faction Round-Robin Tournament ==="
echo "Factions: ${factions[*]}"
echo "Seeds: 1-20 | Max ticks: $MAX_TICKS"
echo ""

total_matchups=0
completed_matchups=0

# Count total matchups
for ((i=0; i<${#factions[@]}; i++)); do
    for ((j=i+1; j<${#factions[@]}; j++)); do
        total_matchups=$((total_matchups + 1))
    done
done

echo "Total matchups: $total_matchups (× 20 seeds = $((total_matchups * 20)) matches)"
echo ""

for ((i=0; i<${#factions[@]}; i++)); do
    for ((j=i+1; j<${#factions[@]}; j++)); do
        slug="${faction_slugs[$i]}_vs_${faction_slugs[$j]}"
        out_dir="$RESULTS_DIR/$slug"
        mkdir -p "$out_dir"

        completed_matchups=$((completed_matchups + 1))
        echo "[$completed_matchups/$total_matchups] ${factions[$i]} vs ${factions[$j]}"

        cargo run -p cc_agent --bin arena --features harness --release -- \
            --seeds "$SEEDS" \
            --p0-faction "${factions[$i]}" \
            --p1-faction "${factions[$j]}" \
            --standard-army \
            --shared-scripts "$SCRIPT_DIR/" \
            --output "$out_dir/" \
            --max-ticks "$MAX_TICKS" \
            2>&1 | tail -n 5

        echo ""
    done
done

echo "=== Tournament Complete ==="
echo "Results in: $RESULTS_DIR"
echo ""
echo "Run analysis:"
echo "  python3 $SCRIPT_DIR/analyze.py $RESULTS_DIR"
