#!/usr/bin/env bash
#
# run_generation.sh — Run one generation of the AI training loop.
#
# Usage:
#   ./run_generation.sh [generation_number]
#
# If generation_number is omitted, reads current_generation from tracker.json.
#
# Flow:
#   1. Create generation directory
#   2. Spawn Claude Opus sub-agent to write Lua scripts (reads reference docs)
#   3. Run arena matches: new scripts vs baseline (multiple seeds)
#   4. Record results in tracker.json
#   5. If win rate > best, promote to new baseline
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TRACKER="$SCRIPT_DIR/tracker.json"
ARENA_BIN="cargo run -p cc_agent --bin arena --features harness --"

# Seeds for evaluation matches
EVAL_SEEDS="1,2,3,4,5,6,7,8,9,10"

# Determine generation number
if [ $# -ge 1 ]; then
    GEN=$1
else
    GEN=$(python3 -c "import json; print(json.load(open('$TRACKER'))['current_generation'])")
fi

GEN_DIR="$SCRIPT_DIR/generations/gen_$(printf '%03d' $GEN)"
SCRIPTS_DIR="$GEN_DIR/scripts"
RESULTS_DIR="$GEN_DIR/results"
BASELINE_DIR="$SCRIPT_DIR/baselines"

echo "=== Generation $GEN ==="
echo "Directory: $GEN_DIR"

# Step 1: Create generation directory
mkdir -p "$SCRIPTS_DIR" "$RESULTS_DIR"

# Step 2: Generate scripts using Claude sub-agent
echo ""
echo "--- Step 2: Generating Lua scripts ---"

# Build the prompt for the sub-agent
PROMPT_FILE="$GEN_DIR/prompt.md"
cat > "$PROMPT_FILE" << 'PROMPT_EOF'
You are writing competitive Lua micro scripts for a real-time strategy game called ClawedCommand.

Your task: Write 1-3 Lua scripts that work together to control combat units in an arena match.
The FSM AI handles macro (economy, building, training). Your scripts handle micro (combat micro, positioning, ability usage).

Requirements:
- Each script should have a clear purpose (e.g., focus_fire.lua, kite_ranged.lua, retreat_wounded.lua)
- Use proper annotation headers (@name, @events, @interval)
- Be budget-conscious (500 compute budget per invocation)
- Handle nil returns gracefully
- Don't command Pawdler workers (FSM handles economy)

Read the following reference docs carefully:
- training/agent/script_api_reference.md — Full Lua API
- training/agent/game_mechanics.md — Unit stats, terrain, combat math
- training/agent/strategy_guide.md — Effective micro patterns and tips

Write each script to a separate .lua file in the scripts/ directory.
PROMPT_EOF

# Check if scripts already exist (allow manual script creation or re-runs)
SCRIPT_COUNT=$(find "$SCRIPTS_DIR" -name "*.lua" 2>/dev/null | wc -l | tr -d ' ')
if [ "$SCRIPT_COUNT" -eq 0 ]; then
    echo "No scripts found in $SCRIPTS_DIR"
    echo "Generate scripts by running a Claude sub-agent with the prompt in:"
    echo "  $PROMPT_FILE"
    echo ""
    echo "Example (using Claude Code):"
    echo "  claude --print 'Read the prompt at $PROMPT_FILE and the reference docs it mentions, then write Lua scripts to $SCRIPTS_DIR'"
    echo ""
    echo "Or place .lua files manually in $SCRIPTS_DIR and re-run this script."
    echo ""
    echo "After scripts are ready, re-run: $0 $GEN"
    exit 0
fi

echo "Found $SCRIPT_COUNT script(s) in $SCRIPTS_DIR"
ls -la "$SCRIPTS_DIR"/*.lua

# Step 3: Run arena matches
echo ""
echo "--- Step 3: Running arena matches ---"

# Match 1: New scripts (P0) vs FSM-only baseline (P1)
echo ""
echo "Match set A: Gen $GEN scripts vs FSM-only"
(cd "$PROJECT_ROOT" && $ARENA_BIN \
    --seeds "$EVAL_SEEDS" \
    --p0-scripts "$SCRIPTS_DIR" \
    --max-ticks 6000 \
    --output "$RESULTS_DIR/vs_fsm") || true

# Match 2: If baseline scripts exist, test against them
if [ -d "$BASELINE_DIR" ] && [ "$(find "$BASELINE_DIR" -name '*.lua' 2>/dev/null | wc -l | tr -d ' ')" -gt 0 ]; then
    echo ""
    echo "Match set B: Gen $GEN scripts vs baseline scripts"
    (cd "$PROJECT_ROOT" && $ARENA_BIN \
        --seeds "$EVAL_SEEDS" \
        --p0-scripts "$SCRIPTS_DIR" \
        --p1-scripts "$BASELINE_DIR" \
        --max-ticks 6000 \
        --output "$RESULTS_DIR/vs_baseline") || true
fi

# Step 4: Parse results and update tracker
echo ""
echo "--- Step 4: Recording results ---"

# Parse win rate from summary.json
WIN_RATE=0.0
if [ -f "$RESULTS_DIR/vs_fsm/summary.json" ]; then
    WIN_RATE=$(python3 -c "
import json
summary = json.load(open('$RESULTS_DIR/vs_fsm/summary.json'))
print(summary.get('p0_win_rate', 0.0))
")
    echo "Win rate vs FSM: $WIN_RATE"
fi

VS_BASELINE_WIN_RATE="null"
if [ -f "$RESULTS_DIR/vs_baseline/summary.json" ]; then
    VS_BASELINE_WIN_RATE=$(python3 -c "
import json
summary = json.load(open('$RESULTS_DIR/vs_baseline/summary.json'))
print(summary.get('p0_win_rate', 0.0))
")
    echo "Win rate vs baseline: $VS_BASELINE_WIN_RATE"
fi

# Update tracker.json
python3 << PYEOF
import json
from datetime import datetime, timezone

tracker = json.load(open("$TRACKER"))

gen_entry = {
    "generation": $GEN,
    "timestamp": datetime.now(timezone.utc).isoformat(),
    "scripts": [f.name for f in __import__('pathlib').Path("$SCRIPTS_DIR").glob("*.lua")],
    "win_rate_vs_fsm": $WIN_RATE,
    "win_rate_vs_baseline": $VS_BASELINE_WIN_RATE if "$VS_BASELINE_WIN_RATE" != "null" else None,
    "seeds": "$EVAL_SEEDS",
}

tracker["generations"].append(gen_entry)
tracker["current_generation"] = $GEN + 1

# Promote to baseline if best
if $WIN_RATE > tracker["best_win_rate"]:
    tracker["best_win_rate"] = $WIN_RATE
    tracker["best_generation"] = $GEN
    print(f"New best! Gen $GEN with {$WIN_RATE:.0%} win rate. Promoting to baseline.")
else:
    print(f"Gen $GEN: {$WIN_RATE:.0%} win rate (best: gen {tracker['best_generation']} at {tracker['best_win_rate']:.0%})")

json.dump(tracker, open("$TRACKER", "w"), indent=2)
PYEOF

# Step 5: Promote to baseline if best
SHOULD_PROMOTE=$(python3 -c "
import json
t = json.load(open('$TRACKER'))
print('yes' if t['best_generation'] == $GEN else 'no')
")

if [ "$SHOULD_PROMOTE" = "yes" ]; then
    echo ""
    echo "--- Promoting gen $GEN to baseline ---"
    rm -f "$BASELINE_DIR"/*.lua
    cp "$SCRIPTS_DIR"/*.lua "$BASELINE_DIR/"
    echo "Baseline updated with $(ls "$BASELINE_DIR"/*.lua | wc -l | tr -d ' ') script(s)"
fi

echo ""
echo "=== Generation $GEN complete ==="
echo "Results: $RESULTS_DIR"
echo "Next generation: $(($GEN + 1))"
