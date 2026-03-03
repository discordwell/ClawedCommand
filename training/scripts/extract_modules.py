#!/usr/bin/env python3
"""Decompose the champion Gen 63 script into annotated reusable modules.

Each module is annotated with:
- Which generation validated it
- Win rate delta
- Budget cost
- When to use vs avoid

Usage:
  python training/scripts/extract_modules.py

Output:
  training/data/champion_modules.json
"""

import json
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
OUTPUT_PATH = PROJECT_ROOT / "training" / "data" / "champion_modules.json"
CHAMPION_PATH = (
    PROJECT_ROOT / "training" / "arena" / "gen_063" / "player_1" / "combat_micro.lua"
)


def build_modules() -> list[dict]:
    """Decompose the champion script into 5 annotated modules."""
    modules = []

    # Module 1: Unit Classification
    modules.append({
        "name": "unit_classification",
        "description": "Classify own units into combat roles: tanks, ranged, workers. "
                       "Separate attackers from idle units for different behaviors.",
        "validated_by": ["P1_gen21", "P1_gen34", "P1_gen63"],
        "win_rate_context": "Foundation module — used in every winning script since Gen 21",
        "budget_cost": 1,
        "budget_notes": "One ctx:my_units() call. All classification is local table iteration.",
        "when_to_use": "Always. Every combat script needs to know its army composition.",
        "when_to_avoid": "Never skip this — without it you can't make informed decisions.",
        "dependencies": [],
        "code": (
            "local my_units = ctx:my_units()\n"
            "if not my_units then return end\n"
            "\n"
            "local combat_units = {}\n"
            "local attackers = {}\n"
            "local ranged_attackers = {}\n"
            "local all_combat_ids = {}\n"
            "local idle_tanks = {}\n"
            "local idle_ranged = {}\n"
            "for _, u in ipairs(my_units) do\n"
            "    local is_worker = (u.kind == \"Pawdler\" or u.kind == \"Scrounger\"\n"
            "        or u.kind == \"Delver\" or u.kind == \"Ponderer\")\n"
            "    if not is_worker then\n"
            "        table.insert(combat_units, u)\n"
            "        table.insert(all_combat_ids, u.id)\n"
            "        if u.attacking then\n"
            "            table.insert(attackers, u)\n"
            "            if u.kind == \"Hisser\" or u.kind == \"Yowler\"\n"
            "                or u.kind == \"FlyingFox\" or u.kind == \"Catnapper\" then\n"
            "                table.insert(ranged_attackers, u)\n"
            "            end\n"
            "        else\n"
            "            if u.kind == \"Chonk\" or u.kind == \"MechCommander\" then\n"
            "                table.insert(idle_tanks, u)\n"
            "            elseif u.kind == \"Hisser\" or u.kind == \"Yowler\"\n"
            "                or u.kind == \"FlyingFox\" or u.kind == \"Catnapper\" then\n"
            "                table.insert(idle_ranged, u)\n"
            "            end\n"
            "        end\n"
            "    end\n"
            "end\n"
            "\n"
            "local my_combat_count = #combat_units\n"
            "if my_combat_count == 0 then return end"
        ),
        "outputs": [
            "combat_units — all non-worker units",
            "attackers — combat units currently attacking",
            "ranged_attackers — ranged units currently attacking",
            "all_combat_ids — flat table of combat unit IDs (for mass commands)",
            "idle_tanks — idle Chonk/MechCommander units",
            "idle_ranged — idle Hisser/Yowler/FlyingFox/Catnapper units",
            "my_combat_count — total combat units",
        ],
    })

    # Module 2: Centroid Focus Fire
    modules.append({
        "name": "centroid_closest_focus_fire",
        "description": "All attacking units focus fire on the enemy closest to the attacker centroid. "
                       "The single most impactful tactical behavior.",
        "validated_by": ["P1_gen21", "P1_gen34", "P1_gen63"],
        "win_rate_context": "0% → 50% win rate (Gen 21). Closest > weakest: 70% → 80% (Gen 34).",
        "budget_cost": 1,
        "budget_notes": "Uses already-queried my_units and enemy_units. Only issues attack command.",
        "when_to_use": "Always when 2+ attackers are engaged. This is the #1 behavior.",
        "when_to_avoid": "Never. EVERY combat script should include this.",
        "dependencies": ["unit_classification"],
        "code": (
            "if #attackers >= 2 and enemies and #enemies > 0 then\n"
            "    local cx, cy = 0, 0\n"
            "    for _, u in ipairs(attackers) do\n"
            "        cx = cx + u.x\n"
            "        cy = cy + u.y\n"
            "    end\n"
            "    cx = cx / #attackers\n"
            "    cy = cy / #attackers\n"
            "\n"
            "    local best_target = nil\n"
            "    local best_dist = 12 * 12\n"
            "    for _, e in ipairs(enemies) do\n"
            "        local dx = e.x - cx\n"
            "        local dy = e.y - cy\n"
            "        local d = dx * dx + dy * dy\n"
            "        if d < best_dist then\n"
            "            best_dist = d\n"
            "            best_target = e\n"
            "        end\n"
            "    end\n"
            "\n"
            "    if best_target then\n"
            "        local ids = {}\n"
            "        for _, u in ipairs(attackers) do\n"
            "            table.insert(ids, u.id)\n"
            "        end\n"
            "        ctx:attack_units(ids, best_target.id)\n"
            "    end\n"
            "end"
        ),
        "critical_notes": [
            "MUST use centroid of attackers, not individual unit positions",
            "MUST target closest, not weakest or highest-DPS",
            "12-tile max range prevents targeting distant enemies",
            "ALL attackers target SAME enemy — never split targets",
        ],
    })

    # Module 3: Conditional Kiting
    modules.append({
        "name": "conditional_kiting",
        "description": "Ranged attackers kite away from closest enemy when outnumbered. "
                       "Preserves ranged units without causing stalemates.",
        "validated_by": ["P1_gen26", "P1_gen34", "P1_gen63"],
        "win_rate_context": "45% → 75% effective dominance (Gen 26). Essential for ranged survival.",
        "budget_cost": 0,
        "budget_notes": "Uses already-queried data. Only move commands (free).",
        "when_to_use": "When outnumbered and ranged units are actively engaged.",
        "when_to_avoid": "In late game (after tick 4000) — kiting conflicts with all-in push.",
        "dependencies": ["unit_classification"],
        "code": (
            "if not late_game and outnumbered and enemies and #ranged_attackers > 0 then\n"
            "    for _, r in ipairs(ranged_attackers) do\n"
            "        local closest_dist = 999999\n"
            "        local closest_ex, closest_ey = 0, 0\n"
            "        for _, e in ipairs(enemies) do\n"
            "            local dx = e.x - r.x\n"
            "            local dy = e.y - r.y\n"
            "            local d = dx * dx + dy * dy\n"
            "            if d < closest_dist then\n"
            "                closest_dist = d\n"
            "                closest_ex = e.x\n"
            "                closest_ey = e.y\n"
            "            end\n"
            "        end\n"
            "        if closest_dist < 5 then\n"
            "            local flee_x = r.x - (closest_ex - r.x)\n"
            "            local flee_y = r.y - (closest_ey - r.y)\n"
            "            flee_x = math.max(0, math.min(map_w - 1, flee_x))\n"
            "            flee_y = math.max(0, math.min(map_h - 1, flee_y))\n"
            "            ctx:move_units({r.id}, flee_x, flee_y)\n"
            "        end\n"
            "    end\n"
            "end"
        ),
        "critical_notes": [
            "MUST be conditional on outnumbered — unconditional kiting causes stalemates",
            "MUST be disabled after tick 4000 (timed push takes priority)",
            "Kite distance sqrt(5) ≈ 2.2 tiles — wider is not better",
            "Flee direction = directly away from closest enemy (no blending)",
        ],
    })

    # Module 4: Inline Formation
    modules.append({
        "name": "inline_formation",
        "description": "Pre-combat positioning: idle tanks advance 3 tiles toward enemy, "
                       "idle ranged pull 2 tiles behind centroid.",
        "validated_by": ["P1_gen53", "P1_gen63"],
        "win_rate_context": "80% → 85% (Gen 53). Combined with push: 90% → 95% (Gen 63).",
        "budget_cost": 0,
        "budget_notes": "Uses already-queried data. Only move/attack_move commands (free).",
        "when_to_use": "When enemies visible, not outnumbered, distance 4-15 tiles, pre-combat.",
        "when_to_avoid": "When outnumbered (retreat instead), in late game (push instead), "
                         "or enemies too close (<4 tiles, already in combat).",
        "dependencies": ["unit_classification"],
        "code": (
            "if enemies and #enemies > 0 and not outnumbered and not late_game then\n"
            "    local nearest_ex, nearest_ey = 0, 0\n"
            "    local nearest_d2 = 999999\n"
            "    for _, e in ipairs(enemies) do\n"
            "        local dx = e.x - army_cx\n"
            "        local dy = e.y - army_cy\n"
            "        local d2 = dx * dx + dy * dy\n"
            "        if d2 < nearest_d2 then\n"
            "            nearest_d2 = d2\n"
            "            nearest_ex = e.x\n"
            "            nearest_ey = e.y\n"
            "        end\n"
            "    end\n"
            "\n"
            "    local dx = nearest_ex - army_cx\n"
            "    local dy = nearest_ey - army_cy\n"
            "    local dist = math.sqrt(dx * dx + dy * dy)\n"
            "\n"
            "    if dist >= 4 and dist <= 15 then\n"
            "        local nx = dx / dist\n"
            "        local ny = dy / dist\n"
            "\n"
            "        for _, t in ipairs(idle_tanks) do\n"
            "            local tx = math.floor(army_cx + nx * 3)\n"
            "            local ty = math.floor(army_cy + ny * 3)\n"
            "            tx = math.max(0, math.min(map_w - 1, tx))\n"
            "            ty = math.max(0, math.min(map_h - 1, ty))\n"
            "            ctx:attack_move({t.id}, tx, ty)\n"
            "        end\n"
            "\n"
            "        for _, r in ipairs(idle_ranged) do\n"
            "            local rx = math.floor(army_cx - nx * 2)\n"
            "            local ry = math.floor(army_cy - ny * 2)\n"
            "            rx = math.max(0, math.min(map_w - 1, rx))\n"
            "            ry = math.max(0, math.min(map_h - 1, ry))\n"
            "            ctx:move_units({r.id}, rx, ry)\n"
            "        end\n"
            "    end\n"
            "end"
        ),
        "critical_notes": [
            "MUST be inline in combat_micro script, NOT a separate script",
            "Only move IDLE units — never interrupt units already fighting",
            "Distance 4-15 check prevents unnecessary moves (too close = in combat, too far = irrelevant)",
            "Tanks use attack_move (engage enemies on the way), ranged use move_units (stay safe)",
        ],
    })

    # Module 5: Timed Push with Building Targeting
    modules.append({
        "name": "timed_push_building_targeting",
        "description": "Push toward enemy buildings when advantage/late-game. "
                       "Target production buildings first, then HQ, then nearest.",
        "validated_by": ["P1_gen30", "P1_gen34", "P1_gen60", "P1_gen63"],
        "win_rate_context": "Prod targeting: +20% (Gen 30). Timed push: +15% (Gen 60). Combined: 95% (Gen 63).",
        "budget_cost": 1,
        "budget_notes": "One ctx:enemy_buildings() call. Attack commands are free.",
        "when_to_use": "When no enemies visible (clear path), strong advantage (>=3), or late game (tick >= 4000).",
        "when_to_avoid": "When outnumbered and enemies present — fight first, push second.",
        "dependencies": ["unit_classification"],
        "code": (
            "local should_push = (enemy_count == 0 and my_combat_count >= 2)\n"
            "    or strong_advantage\n"
            "    or late_game\n"
            "\n"
            "if should_push then\n"
            "    local enemy_buildings = ctx:enemy_buildings()\n"
            "    if enemy_buildings and #enemy_buildings > 0 then\n"
            "        local prod_target = nil\n"
            "        local prod_dist = 999999\n"
            "        local hq = nil\n"
            "        local nearest = nil\n"
            "        local nearest_dist = 999999\n"
            "\n"
            "        for _, b in ipairs(enemy_buildings) do\n"
            "            local dx = b.x - army_cx\n"
            "            local dy = b.y - army_cy\n"
            "            local d = dx * dx + dy * dy\n"
            "\n"
            "            -- Production buildings (all factions)\n"
            "            if b.kind == \"CatTree\" or b.kind == \"ServerRack\"\n"
            "                or b.kind == \"ScrapHeap\" or b.kind == \"JunkServer\"\n"
            "                or b.kind == \"SpawningPools\" or b.kind == \"SunkenServer\"\n"
            "                or b.kind == \"MoleHill\" or b.kind == \"DeepServer\"\n"
            "                or b.kind == \"RookeryNest\" or b.kind == \"DataCrypt\"\n"
            "                or b.kind == \"ChopShop\" or b.kind == \"TinkerBench\" then\n"
            "                if d < prod_dist then\n"
            "                    prod_dist = d\n"
            "                    prod_target = b\n"
            "                end\n"
            "            end\n"
            "\n"
            "            -- HQ buildings (all factions)\n"
            "            if b.kind == \"TheBox\" or b.kind == \"TheDumpster\"\n"
            "                or b.kind == \"TheGrotto\" or b.kind == \"TheBurrow\"\n"
            "                or b.kind == \"TheNest\" or b.kind == \"TheMound\" then\n"
            "                hq = b\n"
            "            end\n"
            "\n"
            "            if d < nearest_dist then\n"
            "                nearest_dist = d\n"
            "                nearest = b\n"
            "            end\n"
            "        end\n"
            "\n"
            "        local target = prod_target or hq or nearest\n"
            "        if target then\n"
            "            ctx:attack_move(all_combat_ids, target.x, target.y)\n"
            "        end\n"
            "    end\n"
            "end"
        ),
        "critical_notes": [
            "Priority: production buildings > HQ > nearest building",
            "Use attack_move (not move_units) to engage enemies on the way",
            "Late game (tick >= 4000) forces push regardless of army advantage",
            "All combat units push together — never split the push force",
        ],
    })

    return modules


def build_module_composition_guide() -> dict:
    """Describe how modules should be composed."""
    return {
        "recommended_composition": {
            "description": "The proven order for combining modules in a combat_micro script",
            "order": [
                "unit_classification",
                "wounded_retreat (skip if late_game)",
                "centroid_closest_focus_fire",
                "inline_formation (skip if outnumbered or late_game)",
                "conditional_kiting (skip if late_game)",
                "timed_push_building_targeting",
            ],
            "why_this_order": "Retreat first (save units) → focus fire (maximize damage) → "
                             "formation (position idle) → kite (protect ranged) → push (convert advantage). "
                             "Later modules can override earlier ones for the same unit.",
        },
        "proven_combinations": [
            {
                "modules": ["centroid_closest_focus_fire", "timed_push_building_targeting",
                           "inline_formation"],
                "win_rate": 0.95,
                "gen": "P1_gen63",
                "note": "Best ever. Formation + push are complementary.",
            },
            {
                "modules": ["centroid_closest_focus_fire", "timed_push_building_targeting"],
                "win_rate": 0.90,
                "gen": "P1_gen60",
                "note": "Timed push alone is very strong.",
            },
            {
                "modules": ["centroid_closest_focus_fire", "conditional_kiting",
                           "production_building_targeting"],
                "win_rate": 0.80,
                "gen": "P1_gen34",
                "note": "Solid without timed push.",
            },
        ],
        "forbidden_combinations": [
            {
                "modules": ["conditional_kiting", "timed_push_building_targeting",
                           "hp_threshold_kiting"],
                "win_rate": 0.70,
                "gen": "P1_gen64",
                "note": "HP kite conflicts with timed push. Don't add a third kiting mechanism.",
            },
            {
                "modules": ["centroid_closest_focus_fire", "ability_activation"],
                "win_rate": 0.0,
                "gen": "22",
                "note": "Ability commands override attack commands. Never combine.",
            },
        ],
    }


def main():
    print("=== Module Extraction ===")

    modules = build_modules()
    composition = build_module_composition_guide()

    # Read champion script for reference
    champion_source = ""
    if CHAMPION_PATH.exists():
        champion_source = CHAMPION_PATH.read_text()

    output = {
        "metadata": {
            "champion_script": str(CHAMPION_PATH.relative_to(PROJECT_ROOT)),
            "champion_win_rate": 0.95,
            "champion_gen": "P1_gen63",
            "total_modules": len(modules),
        },
        "modules": modules,
        "composition_guide": composition,
        "champion_source": champion_source,
    }

    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_PATH, "w") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"Output: {OUTPUT_PATH}")
    print(f"  Modules: {len(modules)}")
    for m in modules:
        print(f"    - {m['name']}: budget={m['budget_cost']}, "
              f"validated by {len(m['validated_by'])} gens")


if __name__ == "__main__":
    main()
