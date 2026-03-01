"""Shared constants and heuristics for ClawedCommand Lua API validation.

Used by both eval_lua.py and validate_lua_data.py to avoid duplication.
All method sets are derived from lua_runtime.rs bindings in cc_agent.
"""

import re

# ---------------------------------------------------------------------------
# Valid ctx API surface (derived from lua_runtime.rs bindings)
# ---------------------------------------------------------------------------

# Methods called as ctx:method(...)
VALID_CTX_METHODS = {
    # Unit queries
    "my_units", "enemy_units", "enemies_in_range", "nearest_enemy",
    "idle_units", "wounded_units", "units_by_state", "count_units",
    "army_supply", "threats_to", "targets_for", "weakest_enemy_in_range",
    "strongest_enemy_in_range", "hp_pct", "distance_squared_between",
    "distance_squared_to_nearest_enemy",
    # Building queries
    "my_buildings", "enemy_buildings",
    # Economy queries
    "get_resources", "resources", "resource_deposits", "nearest_deposit",
    # Terrain queries
    "terrain_at", "elevation_at", "cover_at", "is_passable",
    "can_reach", "path_length",
    # Tactical queries
    "position_at_range", "safe_positions",
    # Game state queries
    "tick", "map_size",
    # Unit commands
    "move_units", "attack_units", "attack_move", "stop", "hold",
    "gather", "build", "train", "ability", "research",
    "rally", "cancel_queue", "cancel_research", "set_control_group",
}

# Methods called as ctx.behaviors:method(...)
VALID_BEHAVIOR_METHODS = {
    # Basic
    "assign_idle_workers", "attack_move_group",
    # Tactical
    "focus_fire", "kite_squad", "retreat_wounded", "defend_area",
    "harass_economy", "scout_pattern", "focus_weakest", "use_ability",
    "split_squads", "protect_unit", "surround_target",
    # Strategic
    "auto_produce", "balanced_production", "expand_economy",
    "coordinate_assault",
    # Advanced
    "research_priority", "adaptive_defense",
}

# Commands that mutate game state (subset of ctx methods)
COMMAND_METHODS = {
    "move_units", "attack_units", "attack_move", "stop", "hold",
    "gather", "build", "train", "ability", "research",
    "rally", "cancel_queue", "cancel_research", "set_control_group",
}

# ---------------------------------------------------------------------------
# Regex patterns
# ---------------------------------------------------------------------------

CTX_METHOD_PATTERN = re.compile(r"ctx:(\w+)\s*\(")
BEHAVIOR_METHOD_PATTERN = re.compile(r"ctx\.behaviors:(\w+)\s*\(")
INTENT_PATTERN = re.compile(r"^-- Intent:\s*\S+", re.MULTILINE)
DESCRIPTION_PATTERN = re.compile(r"^-- Description:\s*\S+", re.MULTILINE)


# ---------------------------------------------------------------------------
# Shared heuristic Lua syntax checker
# ---------------------------------------------------------------------------

def check_lua_block_balance(script: str) -> tuple[int, int]:
    """Count Lua block openers vs closers.

    Returns (openers, closers).

    Rules:
    - Don't count 'do' — it's part of for/while syntax, not its own block
    - Subtract 'elseif' from 'if' count — elseif shares the outer if's 'end'
    """
    openers = len(re.findall(r"\bfunction\b|\bif\b|\bfor\b|\bwhile\b", script))
    elseifs = len(re.findall(r"\belseif\b", script))
    openers -= elseifs
    closers = len(re.findall(r"\bend\b", script))
    return openers, closers
