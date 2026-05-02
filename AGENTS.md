# ClawedCommand Project Instructions

## Development Workflow
- Instead of mocking up features, mark them as a TDL and add to TDL.md

## "Write me a script" Rule
When the user says "write me a script", operate **entirely in the Lua/MCP layer**. Do NOT edit Rust source, Bevy systems, or any other codebase files. Instead:
1. **Understand the request** — clarify what game behavior the script should achieve.
2. **Survey available primitives** — determine which `ctx:` query/command methods and `ctx.behaviors:` calls are needed. Reference the Lua ScriptContext API:
   - **Queries** (budget-costed): `my_units`, `enemy_units`, `enemies_in_range`, `nearest_enemy`, `idle_units`, `wounded_units`, `units_by_state`, `count_units`, `army_supply`, `weakest_enemy_in_range`, `strongest_enemy_in_range`, `hp_pct`, `distance_squared_between`, `distance_squared_to_nearest_enemy`, `threats_to`, `targets_for`, `my_buildings`, `enemy_buildings`, `resources`, `nearest_deposit`, `terrain_at`, `elevation_at`, `cover_at`, `is_passable`, `movement_cost`, `can_reach`, `path_length`, `position_at_range`, `safe_positions`, `tick`, `map_size`
   - **Commands** (free): `move_units`, `attack_units`, `attack_move`, `stop`, `hold`, `gather`, `build`, `train`, `ability`, `research`, `cancel_queue`, `cancel_research`, `set_control_group`, `rally`
   - **Behaviors** (tier-gated): `assign_idle_workers`, `attack_move_group`, `focus_fire`, `kite_squad`, `retreat_wounded`, `defend_area`, `harass_economy`, `scout_pattern`, `focus_weakest`, `use_ability`, `split_squads`, `protect_unit`, `surround_target`, `auto_produce`, `balanced_production`, `expand_economy`, `coordinate_assault`, `research_priority`, `adaptive_defense`
   - **MCP sim-control** (if testing via harness): `spawn_unit`, `spawn_building`, `spawn_deposit`, `advance_ticks`, `reset`, `get_full_state`, `run_lua_script`, `register_script`
3. **Write the Lua script** — use annotations (`@name`, `@events`, `@interval`), stay within 500 budget, follow proven patterns (centroid focus fire, conditional kiting, terrain-aware movement via `movement_cost`).
4. **Output only Lua** — save to `training/arena/` or wherever the user specifies. No Rust edits.
