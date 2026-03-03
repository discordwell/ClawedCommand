#!/usr/bin/env python3
"""Extract strategic knowledge from 48+ generations of arena play.

Parses tracker.json, reads all generation scripts, and outputs a structured
knowledge base of proven patterns and anti-patterns for training data generation.

Usage:
  python training/scripts/extract_arena_knowledge.py

Output:
  training/data/arena_knowledge.json
"""

import json
import os
import re
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
ARENA_DIR = PROJECT_ROOT / "training" / "arena"
TRACKER_PATH = ARENA_DIR / "tracker.json"
OUTPUT_PATH = PROJECT_ROOT / "training" / "data" / "arena_knowledge.json"


def load_tracker() -> dict:
    """Load and parse tracker.json."""
    with open(TRACKER_PATH) as f:
        return json.load(f)


def normalize_gen_id(gen_id) -> str:
    """Normalize generation ID to string."""
    if isinstance(gen_id, int):
        return str(gen_id)
    return str(gen_id)


def extract_win_rate(gen: dict) -> dict:
    """Extract win rate info from a generation entry (handles multiple formats)."""
    result = {
        "p0_wins": 0,
        "p1_wins": 0,
        "timeouts": 0,
        "draws": 0,
        "total_matches": 0,
        "scripted_player": gen.get("scripted_player"),
    }

    # Try different match result formats
    for key in ["matches_20", "matches_10", "matches_5", "combined_15"]:
        if key in gen:
            m = gen[key]
            result["p0_wins"] = m.get("p0_wins", 0)
            result["p1_wins"] = m.get("p1_wins", 0)
            result["timeouts"] = m.get("timeouts", 0)
            result["draws"] = m.get("draws", 0)
            result["total_matches"] = (
                result["p0_wins"] + result["p1_wins"]
                + result["timeouts"] + result["draws"]
            )
            break

    # Compute effective win rate for the scripted player
    sp = gen.get("scripted_player")
    if sp == 0:
        result["scripted_win_rate"] = gen.get(
            "p0_win_rate",
            result["p0_wins"] / max(result["total_matches"], 1),
        )
    elif sp == 1:
        result["scripted_win_rate"] = gen.get(
            "p1_win_rate",
            result["p1_wins"] / max(result["total_matches"], 1),
        )
    else:
        result["scripted_win_rate"] = None

    result["effective_dominance"] = gen.get(
        "p1_effective_dominance",
        gen.get("p0_effective_dominance"),
    )

    return result


def find_gen_scripts(gen_id_str: str) -> list[dict]:
    """Find and read all Lua scripts for a generation directory."""
    scripts = []

    # Try multiple directory name formats
    candidates = [
        ARENA_DIR / f"gen_{int(gen_id_str):03d}" if gen_id_str.isdigit() else None,
        ARENA_DIR / f"gen_{gen_id_str}",
    ]
    # Handle P1_genNN format
    if gen_id_str.startswith("P1_gen"):
        num = gen_id_str.replace("P1_gen", "")
        if num.isdigit():
            candidates.insert(0, ARENA_DIR / f"gen_{int(num):03d}")

    for gen_dir in candidates:
        if gen_dir is None or not gen_dir.exists():
            continue

        for player_dir in ["player_0", "player_1"]:
            script_dir = gen_dir / player_dir
            if not script_dir.exists():
                continue
            for lua_file in sorted(script_dir.glob("*.lua")):
                source = lua_file.read_text()
                scripts.append({
                    "path": str(lua_file.relative_to(PROJECT_ROOT)),
                    "player": int(player_dir[-1]),
                    "name": lua_file.stem,
                    "source": source,
                    "line_count": len(source.splitlines()),
                })

        # Also check for scripts in shared/ directory
        shared_dir = gen_dir / "shared"
        if shared_dir.exists():
            for lua_file in sorted(shared_dir.glob("*.lua")):
                scripts.append({
                    "path": str(lua_file.relative_to(PROJECT_ROOT)),
                    "player": None,
                    "name": lua_file.stem,
                    "source": lua_file.read_text(),
                    "line_count": len(lua_file.read_text().splitlines()),
                })

    return scripts


def extract_script_features(source: str) -> list[str]:
    """Analyze a Lua script and extract the tactical features it uses."""
    features = []

    # Focus fire detection
    if "centroid" in source.lower() or (
        "cx" in source and "cy" in source and "attack_units" in source
    ):
        features.append("centroid_focus_fire")
    if "weakest" in source.lower() and "target" in source.lower():
        features.append("weakest_targeting")
    if "nearest_enemy" in source or "closest" in source.lower():
        features.append("closest_targeting")

    # Kiting
    if "kite" in source.lower() or "flee" in source.lower():
        features.append("kiting")
    if "position_at_range" in source:
        features.append("position_at_range_kiting")
    if "outnumbered" in source.lower() and ("kite" in source.lower() or "flee" in source.lower()):
        features.append("conditional_kiting")

    # Retreat
    if "retreat" in source.lower() or ("hp" in source.lower() and "rally" in source.lower()):
        features.append("wounded_retreat")

    # Push/aggression
    if "attack_move" in source and "building" in source.lower():
        features.append("building_push")
    if "prod" in source.lower() and "target" in source.lower():
        features.append("production_building_targeting")
    if "tick" in source and ("4000" in source or "push" in source.lower()):
        features.append("timed_push")

    # Formation
    if "formation" in source.lower() or ("tank" in source.lower() and "ranged" in source.lower()):
        features.append("inline_formation")

    # Worker management
    if "gather" in source or "pawdler" in source.lower():
        features.append("worker_management")

    # Army splitting
    if "harass" in source.lower() or "split" in source.lower():
        features.append("army_splitting")

    # Abilities
    if "ability" in source:
        features.append("ability_usage")

    # Terrain awareness
    if "terrain_at" in source or "cover_at" in source or "elevation_at" in source:
        features.append("terrain_awareness")
    if "movement_cost" in source or "is_passable" in source:
        features.append("pathfinding_awareness")

    return features


def classify_generation(gen: dict, win_info: dict) -> str:
    """Classify a generation as proven, neutral, anti-pattern, or baseline."""
    wr = win_info.get("scripted_win_rate")
    if wr is None:
        return "baseline"

    if isinstance(wr, str):
        # Range like "0.0 - 0.2"
        return "anti_pattern"

    if wr >= 0.80:
        return "proven"
    elif wr >= 0.50:
        return "neutral"
    elif wr >= 0.30:
        return "weak"
    else:
        return "anti_pattern"


def build_proven_patterns(tracker: dict) -> list[dict]:
    """Extract proven tactical patterns from winning generations."""
    patterns = []

    # Manual curation based on tracker data + notes
    patterns.append({
        "name": "centroid_closest_focus_fire",
        "description": "All attacking units target the closest enemy to the attacker centroid. "
                       "Concentrates damage, kills enemies one at a time.",
        "win_rate": 0.95,
        "first_proven_gen": "P1_gen34",
        "best_gen": "P1_gen63",
        "budget_cost": "~3 per invocation (my_units + enemy_units + attack_units)",
        "code_pattern": (
            "-- Compute attacker centroid\n"
            "local cx, cy = 0, 0\n"
            "for _, u in ipairs(attackers) do cx = cx + u.x; cy = cy + u.y end\n"
            "cx = cx / #attackers; cy = cy / #attackers\n"
            "-- Find closest enemy to centroid\n"
            "local best_target, best_dist = nil, 12*12\n"
            "for _, e in ipairs(enemies) do\n"
            "    local d = (e.x-cx)^2 + (e.y-cy)^2\n"
            "    if d < best_dist then best_dist = d; best_target = e end\n"
            "end\n"
            "-- All attackers focus same target\n"
            "if best_target then ctx:attack_units(attacker_ids, best_target.id) end"
        ),
        "why_it_works": "Kills immediate threats faster than chasing distant low-HP units. "
                        "Prevents army from scattering damage across multiple targets.",
        "evidence": [
            "Gen 34: Switched from weakest to closest → 14→16 P1 wins (80% win rate)",
            "Gen 23: Per-unit targeting = 0% win rate (CATASTROPHIC)",
            "Gen 54: DPS-weighted targeting = 50/50 (broke focus fire)",
            "Gen 56: Kill-secure targeting = 50/50 (chasing wounded)",
        ],
    })

    patterns.append({
        "name": "timed_push",
        "description": "After tick 4000, disable retreat and kiting. Force all-in aggression "
                       "to convert late-game stalemates into decisive victories.",
        "win_rate": 0.95,
        "first_proven_gen": "P1_gen60",
        "best_gen": "P1_gen63",
        "budget_cost": "~1 (tick query is free, just changes behavior flags)",
        "code_pattern": (
            "local late_game = ctx:tick() >= 4000\n"
            "-- In late game: disable retreat, disable kiting, force push\n"
            "if late_game then\n"
            "    -- Skip retreat logic\n"
            "    -- Skip kiting logic\n"
            "    -- Force attack_move to enemy buildings\n"
            "end"
        ),
        "why_it_works": "Prevents endless stalemates by forcing decisive engagement in final third. "
                        "The biggest single improvement since closest-focus (16→18 P1 wins).",
        "evidence": [
            "Gen 60: Timed push → 18 P1 wins, 0 P0 wins, 2 timeouts",
            "Gen 35: Push too early (tick 3000) → lost units, regression",
            "Gen 63: Combined with formation → 19/20 wins (95%)",
        ],
    })

    patterns.append({
        "name": "inline_formation",
        "description": "Before combat engagement, position tanks 3 tiles forward toward enemy "
                       "and ranged units 2 tiles back. Only for idle units, does not interrupt combat.",
        "win_rate": 0.85,
        "first_proven_gen": "P1_gen53",
        "best_gen": "P1_gen63",
        "budget_cost": "~3 (enemy scan + per-unit moves)",
        "code_pattern": (
            "-- Only when enemies visible, not outnumbered, not late game\n"
            "if enemies and #enemies > 0 and not outnumbered and not late_game then\n"
            "    -- Find direction to nearest enemy cluster\n"
            "    local nx, ny = (normalized direction to nearest enemy)\n"
            "    -- Idle tanks: advance 3 tiles toward enemy\n"
            "    for _, t in ipairs(idle_tanks) do\n"
            "        ctx:attack_move({t.id}, army_cx + nx*3, army_cy + ny*3)\n"
            "    end\n"
            "    -- Idle ranged: position 2 tiles behind centroid\n"
            "    for _, r in ipairs(idle_ranged) do\n"
            "        ctx:move_units({r.id}, army_cx - nx*2, army_cy - ny*2)\n"
            "    end\n"
            "end"
        ),
        "why_it_works": "Better pre-combat positioning means tanks absorb damage while ranged deal "
                        "it safely. Works because it only moves idle units, not fighting ones.",
        "evidence": [
            "Gen 53: Inline formation → 17/20 wins, 0 timeouts",
            "Gen 52: Separate formation script → no impact (command conflicts)",
            "Gen 63: Formation + timed push = 19/20 wins (best ever)",
        ],
    })

    patterns.append({
        "name": "conditional_kiting",
        "description": "Ranged units kite (move away from closest enemy) only when outnumbered. "
                       "Preserves ranged units without causing stalemates.",
        "win_rate": 0.75,
        "first_proven_gen": "P1_gen26",
        "best_gen": "P1_gen34",
        "budget_cost": "~2 per ranged unit (distance checks + move)",
        "code_pattern": (
            "if outnumbered and #ranged_attackers > 0 then\n"
            "    for _, r in ipairs(ranged_attackers) do\n"
            "        -- Find closest enemy to this ranged unit\n"
            "        local closest_dist, closest_ex, closest_ey = 999999, 0, 0\n"
            "        for _, e in ipairs(enemies) do\n"
            "            local d = (e.x-r.x)^2 + (e.y-r.y)^2\n"
            "            if d < closest_dist then ... end\n"
            "        end\n"
            "        if closest_dist < 5 then\n"
            "            -- Flee in opposite direction\n"
            "            ctx:move_units({r.id}, r.x - (closest_ex-r.x), r.y - (closest_ey-r.y))\n"
            "        end\n"
            "    end\n"
            "end"
        ),
        "why_it_works": "Outnumber condition prevents stalemates from excessive kiting. "
                        "Sqrt(5) ≈ 2.2 tile kite distance is optimal.",
        "evidence": [
            "Gen 26: Conditional kite = 75% effective dominance",
            "Gen 25: No kiting = regression to 45%",
            "Gen 27: HP-threshold kite (<20%) = worse, less kiting lets P0 convert",
            "Gen 58: Centroid-bias kite = worse, blending directions reduces effectiveness",
        ],
    })

    patterns.append({
        "name": "wounded_retreat",
        "description": "Units below 30% HP retreat toward rally point when outnumbered. "
                       "Preserves damaged units for later fights.",
        "win_rate": 0.70,
        "first_proven_gen": "P1_gen21",
        "best_gen": "P1_gen34",
        "budget_cost": "~2 (HP check + move command)",
        "code_pattern": (
            "local retreat_ids = {}\n"
            "for _, u in ipairs(combat_units) do\n"
            "    local hp_pct = u.hp / math.max(u.hp_max, 1)\n"
            "    if hp_pct < 0.30 and u.attacking and outnumbered then\n"
            "        table.insert(retreat_ids, u.id)\n"
            "    end\n"
            "end\n"
            "if #retreat_ids > 0 then\n"
            "    ctx:move_units(retreat_ids, rally_x, rally_y)\n"
            "end"
        ),
        "why_it_works": "Preserves army for attrition advantage. Only retreats when outnumbered "
                        "to avoid pulling units from winning fights.",
        "evidence": [
            "Gen 21: First combat micro with retreat = 50% win rate (from 0%)",
            "Gen 33: 20% threshold = marginal improvement over 30%",
            "Gen 40: Tighter retreat neutral with closest-focus",
        ],
    })

    patterns.append({
        "name": "production_building_targeting",
        "description": "When pushing, target production buildings (CatTree, ServerRack) first, "
                       "then HQ. Starves enemy reinforcements.",
        "win_rate": 0.85,
        "first_proven_gen": "P1_gen30",
        "best_gen": "P1_gen34",
        "budget_cost": "~1 (enemy_buildings query + attack_move)",
        "code_pattern": (
            "local enemy_buildings = ctx:enemy_buildings()\n"
            "-- Priority: production > HQ > nearest\n"
            "local prod_target, hq, nearest = nil, nil, nil\n"
            "for _, b in ipairs(enemy_buildings) do\n"
            "    if b.kind == 'CatTree' or b.kind == 'ServerRack' then\n"
            "        prod_target = closest_of(b, army_centroid)\n"
            "    elseif b.kind == 'TheBox' then hq = b\n"
            "    end\n"
            "    nearest = closest_of(b, army_centroid)\n"
            "end\n"
            "local target = prod_target or hq or nearest\n"
            "ctx:attack_move(all_combat_ids, target.x, target.y)"
        ),
        "why_it_works": "Destroying production buildings prevents reinforcements, converting "
                        "timeouts (P1-leading) to decisive wins.",
        "evidence": [
            "Gen 30: Prod targeting → 14 decisive wins (from ~6 previously)",
            "Gen 21: HQ-only targeting → only 5 decisive wins",
        ],
    })

    patterns.append({
        "name": "push_on_advantage",
        "description": "Push toward enemy buildings when no enemies visible and army >= 2, "
                       "or when army advantage >= 3.",
        "win_rate": 0.80,
        "first_proven_gen": "P1_gen26",
        "best_gen": "P1_gen34",
        "budget_cost": "~1 (enemy count check + attack_move)",
        "code_pattern": (
            "local should_push = (enemy_count == 0 and my_combat_count >= 2)\n"
            "    or (my_combat_count >= enemy_count + 3)\n"
            "    or late_game\n"
            "if should_push then ... end"
        ),
        "why_it_works": "Converts army advantage into map control. >= 3 threshold avoids "
                        "pushing into marginal fights.",
        "evidence": [
            "Gen 32: Push at ANY advantage → slightly worse (marginal fights lost)",
            "Gen 37: Push at >= 2 → marginally worse",
            "Gen 34: Push at >= 3 → optimal sweet spot",
        ],
    })

    return patterns


def build_anti_patterns(tracker: dict) -> list[dict]:
    """Extract anti-patterns from failing generations."""
    anti_patterns = []

    anti_patterns.append({
        "name": "per_unit_targeting",
        "description": "Each unit independently targets the weakest enemy in its range. "
                       "Scatters damage across multiple enemies — none die quickly.",
        "win_rate": 0.0,
        "gen": "23",
        "severity": "catastrophic",
        "why_it_fails": "Without concentrated fire, no enemy dies fast enough. "
                        "Army takes full return damage while slowly whittling multiple targets.",
        "what_to_do_instead": "All attackers target the SAME enemy (closest to centroid).",
    })

    anti_patterns.append({
        "name": "army_splitting_harassment",
        "description": "Detach units from main army to harass enemy workers. "
                       "Weakens the main force fatally in decisive combat.",
        "win_rate": 0.0,
        "gen": "P1_gen29",
        "severity": "catastrophic",
        "why_it_fails": "One fewer unit in the main army flips fights from close wins to losses. "
                        "The harasser rarely kills enough workers to compensate.",
        "what_to_do_instead": "Keep army concentrated. Never split for harassment.",
    })

    anti_patterns.append({
        "name": "ability_combat_interference",
        "description": "Running ability activation scripts alongside combat micro scripts. "
                       "Ability commands override attack commands.",
        "win_rate": 0.0,
        "gen": "22",
        "severity": "catastrophic",
        "why_it_fails": "ctx:ability() commands issued after ctx:attack_units() override them. "
                        "Units try to cast instead of attacking.",
        "what_to_do_instead": "Never combine ability and combat scripts. Use one or the other.",
    })

    anti_patterns.append({
        "name": "dps_weighted_targeting",
        "description": "Target enemies by damage/HP ratio instead of proximity. "
                       "Breaks focus fire pattern.",
        "win_rate": 0.50,
        "gen": "P1_gen54",
        "severity": "major",
        "why_it_fails": "High-DPS enemies are often scattered. Army splits to chase "
                        "high-priority targets across the map.",
        "what_to_do_instead": "Always target closest to centroid. Proximity > priority.",
    })

    anti_patterns.append({
        "name": "kill_secure_targeting",
        "description": "Target enemies with lowest absolute HP to 'secure kills'. "
                       "Army chases wounded enemies instead of fighting threats.",
        "win_rate": 0.50,
        "gen": "P1_gen56",
        "severity": "major",
        "why_it_fails": "Wounded enemies retreat. Army chases them away from the real fight, "
                        "leaving remaining enemies to deal free damage.",
        "what_to_do_instead": "Target closest. Wounded enemies near the centroid will die naturally.",
    })

    anti_patterns.append({
        "name": "weakest_enemy_targeting",
        "description": "Target the weakest enemy (lowest HP%) regardless of position. "
                       "Stalls army by chasing distant wounded units.",
        "win_rate": 0.10,
        "gen": "P1_gen51",
        "severity": "catastrophic",
        "why_it_fails": "Army pivots toward distant wounded enemies, breaking formation and "
                        "exposing flanks to healthy enemies.",
        "what_to_do_instead": "Target closest to centroid. Position-based targeting always wins.",
    })

    anti_patterns.append({
        "name": "split_role_targeting",
        "description": "Melee targets closest, ranged targets weakest. "
                       "Breaks unified focus fire by splitting targets between roles.",
        "win_rate": 0.70,
        "gen": "P1_gen55",
        "severity": "moderate",
        "why_it_fails": "Two different targets mean damage is split. Neither dies as fast "
                        "as when the whole army focuses one enemy.",
        "what_to_do_instead": "ALL units target the same enemy, regardless of role.",
    })

    anti_patterns.append({
        "name": "hp_kite_with_timed_push",
        "description": "HP-threshold kiting (kite when own HP < 70%) combined with timed push. "
                       "Kiting pulls ranged units back when they should commit.",
        "win_rate": 0.80,
        "gen": "P1_gen62",
        "severity": "moderate",
        "why_it_fails": "In late game, the timed push demands all-in commitment. "
                        "HP-based kiting overrides the push, causing ranged units to retreat.",
        "what_to_do_instead": "Disable ALL retreat/kite logic during timed push (after tick 4000).",
    })

    anti_patterns.append({
        "name": "centroid_bias_kiting",
        "description": "Kite direction blended between flee (away from enemy) and toward army center. "
                       "Reduces kite effectiveness.",
        "win_rate": 0.70,
        "gen": "P1_gen58",
        "severity": "moderate",
        "why_it_fails": "Blending flee direction with army center means units don't move far enough "
                        "from enemies. Results in more timeouts.",
        "what_to_do_instead": "Kite directly away from closest enemy. Don't blend directions.",
    })

    anti_patterns.append({
        "name": "separate_formation_script",
        "description": "Formation positioning as a shared/separate script rather than inline. "
                       "No measurable impact due to command conflicts.",
        "win_rate": 0.80,
        "gen": "P1_gen52",
        "severity": "minor",
        "why_it_fails": "Separate scripts can issue conflicting commands for the same units. "
                        "Formation commands get overridden by combat script commands.",
        "what_to_do_instead": "Inline formation logic into the combat micro script.",
    })

    anti_patterns.append({
        "name": "three_feature_combo",
        "description": "Combining more than 2 tactical features (e.g., timed push + HP kite + formation). "
                       "Features interfere with each other.",
        "win_rate": 0.70,
        "gen": "P1_gen64",
        "severity": "moderate",
        "why_it_fails": "More features = more potential for conflicting commands. "
                        "HP kite fights timed push. Less is more.",
        "what_to_do_instead": "Maximum 2 complementary features per script. Test combinations carefully.",
    })

    return anti_patterns


def build_evolution_tree(tracker: dict) -> dict:
    """Build a simplified tree showing how strategies evolved."""
    tree = {
        "description": "How arena strategies evolved from baseline to 95% win rate",
        "stages": [
            {
                "stage": "Baseline",
                "gens": ["0", "baseline_v2"],
                "win_rate": "80% P0 (map advantage)",
                "key_insight": "FSM vs FSM: P0 dominates due to map-gen asymmetry",
            },
            {
                "stage": "First scripts (failed)",
                "gens": ["1-9"],
                "win_rate": "0-20% P0",
                "key_insight": "Combat scripts override FSM commands, causing worse behavior",
            },
            {
                "stage": "Production breakthrough",
                "gens": ["12"],
                "win_rate": "53% P0",
                "key_insight": "Smart production fill was the first helpful script",
            },
            {
                "stage": "Combat micro v1",
                "gens": ["21", "P1_gen21"],
                "win_rate": "50% P0 / 45% P1",
                "key_insight": "Group focus fire + retreat wounded = scripts help underdog",
            },
            {
                "stage": "Catastrophic failures",
                "gens": ["22", "23", "P1_gen29"],
                "win_rate": "0%",
                "key_insight": "Abilities interfere, per-unit targeting scatters, army splitting kills",
            },
            {
                "stage": "Refinement",
                "gens": ["P1_gen24", "P1_gen26", "P1_gen30"],
                "win_rate": "75-85% P1",
                "key_insight": "Conditional kiting, production building targeting, push on advantage",
            },
            {
                "stage": "Closest-focus breakthrough",
                "gens": ["P1_gen34"],
                "win_rate": "80% P1 (90% effective)",
                "key_insight": "Closest enemy to centroid > weakest enemy. Biggest targeting improvement.",
            },
            {
                "stage": "Targeting experiments (all failed)",
                "gens": ["P1_gen54", "P1_gen55", "P1_gen56", "P1_gen57"],
                "win_rate": "50-75% P1",
                "key_insight": "DPS-weighted, split-role, kill-secure, threat-count all worse than closest",
            },
            {
                "stage": "Timed push breakthrough",
                "gens": ["P1_gen60"],
                "win_rate": "90% P1 (100% effective)",
                "key_insight": "Force aggression after tick 4000. Converts all stalemates.",
            },
            {
                "stage": "Champion: push + formation",
                "gens": ["P1_gen63"],
                "win_rate": "95% P1",
                "key_insight": "Timed push + inline formation = best combo. 19/20 wins.",
            },
            {
                "stage": "Over-combination (failed)",
                "gens": ["P1_gen62", "P1_gen64"],
                "win_rate": "70-80% P1",
                "key_insight": "Adding HP kite to push hurts. Less is more: 2 features optimal.",
            },
        ],
    }
    return tree


def build_generation_details(tracker: dict) -> list[dict]:
    """Extract per-generation details for data generation context."""
    details = []
    for gen in tracker["generations"]:
        gen_id = normalize_gen_id(gen["id"])
        win_info = extract_win_rate(gen)
        scripts = find_gen_scripts(gen_id)
        classification = classify_generation(gen, win_info)

        details.append({
            "id": gen_id,
            "classification": classification,
            "scripted_player": gen.get("scripted_player"),
            "win_info": win_info,
            "scripts": [
                {
                    "name": s["name"],
                    "player": s["player"],
                    "line_count": s["line_count"],
                    "features": extract_script_features(s["source"]),
                }
                for s in scripts
            ],
            "notes": gen.get("notes", ""),
        })

    return details


def build_strategic_rules() -> list[dict]:
    """Compile strategic rules that should be encoded in the training system prompt."""
    return [
        {
            "rule": "ALWAYS use centroid-closest focus fire",
            "priority": 1,
            "context": "combat",
            "detail": "All attackers target the closest enemy to the attacker centroid. "
                      "Never target weakest, highest-DPS, or lowest-HP.",
        },
        {
            "rule": "NEVER split army for harassment",
            "priority": 2,
            "context": "combat",
            "detail": "Concentrated force always wins. Detaching raiders weakens the "
                      "main army fatally.",
        },
        {
            "rule": "NEVER combine ability and combat scripts",
            "priority": 3,
            "context": "script_design",
            "detail": "ctx:ability() commands override ctx:attack_units() commands. "
                      "Choose one command type per script.",
        },
        {
            "rule": "Kite ONLY when outnumbered",
            "priority": 4,
            "context": "combat",
            "detail": "Ranged units kite when my_count < enemy_count. "
                      "Unconditional kiting causes stalemates.",
        },
        {
            "rule": "Disable retreat/kiting after tick 4000",
            "priority": 5,
            "context": "combat",
            "detail": "Late-game requires all-in commitment. Retreating or kiting "
                      "in the final third prevents decisive outcomes.",
        },
        {
            "rule": "Target production buildings before HQ",
            "priority": 6,
            "context": "pushing",
            "detail": "Destroying CatTree/ServerRack starves reinforcements. "
                      "Going straight for TheBox lets enemy rebuild army.",
        },
        {
            "rule": "Push threshold is >= 3 army advantage",
            "priority": 7,
            "context": "pushing",
            "detail": "Pushing at >= 2 or any advantage leads to marginal fights. "
                      ">= 3 is the optimal sweet spot.",
        },
        {
            "rule": "Maximum 2 complementary features per script",
            "priority": 8,
            "context": "script_design",
            "detail": "Formation + timed push = best combo. Adding a third feature "
                      "(like HP kite) causes interference. Less is more.",
        },
        {
            "rule": "ALL units target the SAME enemy",
            "priority": 9,
            "context": "combat",
            "detail": "Split-role targeting (melee closest, ranged weakest) breaks "
                      "unified focus fire. Group focus always wins.",
        },
        {
            "rule": "Formation must be inline, not a separate script",
            "priority": 10,
            "context": "script_design",
            "detail": "Separate formation scripts have no impact due to command conflicts. "
                      "Integrate formation into the combat micro script.",
        },
    ]


def main():
    print("=== Arena Knowledge Extraction ===")

    tracker = load_tracker()
    generations = tracker["generations"]
    print(f"Loaded {len(generations)} generations from tracker.json")

    # Extract all components
    proven_patterns = build_proven_patterns(tracker)
    anti_patterns = build_anti_patterns(tracker)
    evolution_tree = build_evolution_tree(tracker)
    generation_details = build_generation_details(tracker)
    strategic_rules = build_strategic_rules()

    # Compute summary stats
    total_scripts = 0
    total_lines = 0
    for detail in generation_details:
        for script in detail["scripts"]:
            total_scripts += 1
            total_lines += script["line_count"]

    knowledge = {
        "metadata": {
            "source": "training/arena/tracker.json + gen_*/player_*/",
            "total_generations": len(generations),
            "total_scripts_found": total_scripts,
            "total_lua_lines": total_lines,
            "best_generation": tracker.get("best_generation"),
            "best_win_rate": tracker.get("best_p1_win_rate"),
        },
        "proven_patterns": proven_patterns,
        "anti_patterns": anti_patterns,
        "strategic_rules": strategic_rules,
        "evolution_tree": evolution_tree,
        "generation_details": generation_details,
        "key_insights": tracker.get("key_insights", []),
    }

    # Write output
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_PATH, "w") as f:
        json.dump(knowledge, f, indent=2, ensure_ascii=False)

    print(f"\nOutput: {OUTPUT_PATH}")
    print(f"  Proven patterns: {len(proven_patterns)}")
    print(f"  Anti-patterns:   {len(anti_patterns)}")
    print(f"  Strategic rules: {len(strategic_rules)}")
    print(f"  Evolution stages: {len(evolution_tree['stages'])}")
    print(f"  Generation details: {len(generation_details)}")
    print(f"  Scripts found: {total_scripts} ({total_lines} total lines)")


if __name__ == "__main__":
    main()
