#!/usr/bin/env python3
"""Cross-faction tournament results analyzer.

Reads match result JSONs from the tournament runner and produces:
1. Win rate matrix (6×6)
2. Per-faction effective win rate
3. Outlier detection (>70% matchup imbalance)
4. Unit efficiency (kills/losses ratio)
5. Timeout rate analysis
6. Counter-turtle effectiveness comparison

Usage: python3 analyze.py <results_dir> [--siege-dir <siege_results_dir>]
"""

import json
import os
import sys
from collections import defaultdict
from pathlib import Path

FACTIONS = ["catGPT", "The Clawed", "Seekers of the Deep", "The Murder", "LLAMA", "Croak"]
FACTION_SHORT = ["catGPT", "Clawed", "Seekers", "Murder", "LLAMA", "Croak"]


def load_results(results_dir: Path) -> list[dict]:
    """Load all match result JSONs from a results directory tree."""
    results = []
    for root, dirs, files in os.walk(results_dir):
        for f in files:
            if f.startswith("match_seed") and f.endswith(".json"):
                path = Path(root) / f
                try:
                    with open(path) as fh:
                        data = json.load(fh)
                        # Infer factions from parent directory name
                        parent = Path(root).name
                        data["_dir"] = parent
                        results.append(data)
                except (json.JSONDecodeError, OSError):
                    continue
    # Also load summary.json files for faction metadata
    for root, dirs, files in os.walk(results_dir):
        if "summary.json" in files:
            path = Path(root) / "summary.json"
            try:
                with open(path) as fh:
                    summary = json.load(fh)
                    parent = Path(root).name
                    # Attach faction info to matching results
                    for r in results:
                        if r["_dir"] == parent:
                            r["_p0_faction"] = summary.get("p0_faction", "")
                            r["_p1_faction"] = summary.get("p1_faction", "")
            except (json.JSONDecodeError, OSError):
                continue
    return results


def faction_index(name: str) -> int:
    for i, f in enumerate(FACTIONS):
        if f == name:
            return i
    return -1


def build_win_matrix(results: list[dict]) -> tuple:
    """Build 6×6 win count matrices (wins[i][j] = P0 wins when faction i is P0 vs faction j as P1)."""
    wins = [[0]*6 for _ in range(6)]
    total = [[0]*6 for _ in range(6)]
    timeouts = [[0]*6 for _ in range(6)]
    kills = [[0]*6 for _ in range(6)]
    losses = [[0]*6 for _ in range(6)]

    for r in results:
        p0f = r.get("_p0_faction", "")
        p1f = r.get("_p1_faction", "")
        i = faction_index(p0f)
        j = faction_index(p1f)
        if i < 0 or j < 0:
            continue

        total[i][j] += 1
        outcome = r.get("outcome", "")

        if "Victory" in outcome:
            if "winner: 0" in outcome:
                wins[i][j] += 1
        elif "Timeout" in outcome:
            timeouts[i][j] += 1
            # Count timeout with leading player as a "soft win"
            if "leading: Some(0)" in outcome:
                wins[i][j] += 1

        # Accumulate kills/losses
        stats = r.get("player_stats", [{}, {}])
        if len(stats) >= 2:
            k0 = stats[0].get("units_killed", 0) if isinstance(stats[0], dict) else 0
            l0 = stats[0].get("units_lost", 0) if isinstance(stats[0], dict) else 0
            kills[i][j] += k0
            losses[i][j] += l0

    return wins, total, timeouts, kills, losses


def print_win_matrix(wins, total, timeouts):
    """Print the win rate matrix as a markdown table."""
    print("\n## Win Rate Matrix (P0 win %)\n")
    header = "| P0 \\ P1 | " + " | ".join(FACTION_SHORT) + " |"
    sep = "|" + "---|" * (len(FACTIONS) + 1)
    print(header)
    print(sep)

    for i in range(6):
        row = f"| **{FACTION_SHORT[i]}** |"
        for j in range(6):
            if i == j:
                row += " - |"
            elif total[i][j] > 0:
                pct = wins[i][j] / total[i][j] * 100
                to_pct = timeouts[i][j] / total[i][j] * 100
                row += f" {pct:.0f}% ({timeouts[i][j]}T) |"
            else:
                row += " N/A |"
        print(row)


def print_effective_win_rates(wins, total):
    """Per-faction average win rate across all opponents (both as P0 and P1)."""
    print("\n## Effective Win Rate (averaged across all opponents)\n")
    print("| Faction | Win Rate | Matches |")
    print("|---|---|---|")

    for i in range(6):
        total_wins = 0
        total_matches = 0
        for j in range(6):
            if i == j:
                continue
            # As P0 vs j
            total_wins += wins[i][j]
            total_matches += total[i][j]
            # As P1 vs j (wins[j][i] are j's P0 wins, so i's P1 wins = total[j][i] - wins[j][i])
            total_wins += total[j][i] - wins[j][i]
            total_matches += total[j][i]

        if total_matches > 0:
            pct = total_wins / total_matches * 100
            print(f"| {FACTIONS[i]} | **{pct:.1f}%** | {total_matches} |")
        else:
            print(f"| {FACTIONS[i]} | N/A | 0 |")


def print_outliers(wins, total):
    """Flag matchups where one side wins > 70%."""
    print("\n## Balance Outliers (>70% win rate)\n")
    found = False
    for i in range(6):
        for j in range(i+1, 6):
            # Combine both directions
            total_matches = total[i][j] + total[j][i]
            if total_matches == 0:
                continue
            # i's total wins = P0 wins as i + P1 wins as i
            i_wins = wins[i][j] + (total[j][i] - wins[j][i])
            i_pct = i_wins / total_matches * 100
            if i_pct > 70:
                print(f"- **{FACTIONS[i]}** vs {FACTIONS[j]}: {i_pct:.0f}% ({i_wins}/{total_matches})")
                found = True
            elif i_pct < 30:
                j_pct = 100 - i_pct
                print(f"- **{FACTIONS[j]}** vs {FACTIONS[i]}: {j_pct:.0f}% ({total_matches - i_wins}/{total_matches})")
                found = True
    if not found:
        print("No matchups exceed 70% imbalance threshold.")


def print_unit_efficiency(kills, losses):
    """Kills/losses ratio per faction."""
    print("\n## Unit Efficiency (Kill/Loss Ratio)\n")
    print("| Faction | Total Kills | Total Losses | K/L Ratio |")
    print("|---|---|---|---|")

    for i in range(6):
        total_kills = sum(kills[i][j] for j in range(6) if j != i)
        total_losses = sum(losses[i][j] for j in range(6) if j != i)
        ratio = total_kills / max(total_losses, 1)
        print(f"| {FACTIONS[i]} | {total_kills} | {total_losses} | **{ratio:.2f}** |")


def print_timeout_analysis(timeouts, total):
    """Timeout rate per matchup."""
    print("\n## Timeout Rate Analysis\n")
    total_timeouts = sum(timeouts[i][j] for i in range(6) for j in range(6))
    total_matches = sum(total[i][j] for i in range(6) for j in range(6))
    if total_matches > 0:
        print(f"Overall timeout rate: {total_timeouts}/{total_matches} ({total_timeouts/total_matches*100:.1f}%)\n")

    # Per-matchup timeout rates
    print("| Matchup | Timeout Rate |")
    print("|---|---|")
    for i in range(6):
        for j in range(i+1, 6):
            t = timeouts[i][j] + timeouts[j][i]
            m = total[i][j] + total[j][i]
            if m > 0:
                pct = t / m * 100
                marker = " ⚠️" if pct > 50 else ""
                print(f"| {FACTION_SHORT[i]} vs {FACTION_SHORT[j]} | {pct:.0f}% ({t}/{m}){marker} |")


def print_recommendations(wins, total, timeouts, kills, losses):
    """Generate balance adjustment recommendations."""
    print("\n## Recommendations\n")

    # Find strongest and weakest factions
    eff_rates = []
    for i in range(6):
        total_wins = 0
        total_matches = 0
        for j in range(6):
            if i == j:
                continue
            total_wins += wins[i][j]
            total_matches += total[i][j]
            total_wins += total[j][i] - wins[j][i]
            total_matches += total[j][i]
        rate = total_wins / max(total_matches, 1)
        eff_rates.append((FACTIONS[i], rate, total_matches))

    eff_rates.sort(key=lambda x: x[1], reverse=True)
    strongest = eff_rates[0]
    weakest = eff_rates[-1]

    if strongest[1] > 0.55 and strongest[2] > 10:
        print(f"- **Nerf {strongest[0]}**: {strongest[1]*100:.1f}% effective win rate suggests overtuned stats")
    if weakest[1] < 0.45 and weakest[2] > 10:
        print(f"- **Buff {weakest[0]}**: {weakest[1]*100:.1f}% effective win rate suggests undertuned stats")

    # Check for high timeout matchups
    for i in range(6):
        for j in range(i+1, 6):
            t = timeouts[i][j] + timeouts[j][i]
            m = total[i][j] + total[j][i]
            if m > 0 and t / m > 0.5:
                print(f"- **{FACTION_SHORT[i]} vs {FACTION_SHORT[j]}**: {t/m*100:.0f}% timeout rate — counter-turtle abilities may be insufficient")

    if all(0.45 <= r[1] <= 0.55 for r in eff_rates if r[2] > 10):
        print("- All factions within 45-55% effective win rate — **balance looks good!**")


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <results_dir> [--siege-dir <siege_results_dir>]")
        sys.exit(1)

    results_dir = Path(sys.argv[1])
    siege_dir = None
    if "--siege-dir" in sys.argv:
        idx = sys.argv.index("--siege-dir")
        if idx + 1 < len(sys.argv):
            siege_dir = Path(sys.argv[idx + 1])

    if not results_dir.exists():
        print(f"Error: {results_dir} does not exist")
        sys.exit(1)

    results = load_results(results_dir)
    if not results:
        print(f"No match results found in {results_dir}")
        sys.exit(1)

    print(f"# Balance Test Report\n")
    print(f"Loaded {len(results)} match results from {results_dir}\n")

    wins, total, timeouts, kills, losses = build_win_matrix(results)

    print_win_matrix(wins, total, timeouts)
    print_effective_win_rates(wins, total)
    print_outliers(wins, total)
    print_unit_efficiency(kills, losses)
    print_timeout_analysis(timeouts, total)
    print_recommendations(wins, total, timeouts, kills, losses)

    # Counter-turtle comparison if siege results provided
    if siege_dir and siege_dir.exists():
        siege_results = load_results(siege_dir)
        if siege_results:
            print("\n## Counter-Turtle Effectiveness\n")
            _, _, siege_timeouts, _, _ = build_win_matrix(siege_results)
            siege_total_to = sum(siege_timeouts[i][j] for i in range(6) for j in range(6))
            siege_total_m = len(siege_results)
            base_total_to = sum(timeouts[i][j] for i in range(6) for j in range(6))
            base_total_m = len(results)

            base_rate = base_total_to / max(base_total_m, 1) * 100
            siege_rate = siege_total_to / max(siege_total_m, 1) * 100
            print(f"| Condition | Timeout Rate |")
            print(f"|---|---|")
            print(f"| Base (no abilities) | {base_rate:.1f}% |")
            print(f"| With siege abilities | {siege_rate:.1f}% |")
            delta = base_rate - siege_rate
            if delta > 5:
                print(f"\nSiege abilities reduce timeouts by {delta:.1f}pp — **counter-turtle mechanics working**")
            elif delta > 0:
                print(f"\nSiege abilities reduce timeouts by {delta:.1f}pp — marginal effect, may need tuning")
            else:
                print(f"\nSiege abilities did not reduce timeouts — **counter-turtle mechanics ineffective**")


if __name__ == "__main__":
    main()
