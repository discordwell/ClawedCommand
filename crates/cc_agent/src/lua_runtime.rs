use std::cell::RefCell;

use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{BuildingKind, ResourceType, UnitKind, UpgradeType};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;
use mlua::prelude::*;

use crate::script_context::{
    BlackboardValue, EnemyMemoryEntry, ScriptContext, ScriptEvent, UnitState,
};
use crate::snapshot::{BuildingSnapshot, ResourceSnapshot, UnitSnapshot};
use crate::tool_tier::ToolTier;

/// Maximum Lua instructions before termination (prevents infinite loops).
const INSTRUCTION_LIMIT: u32 = 10_000;

/// Execute a Lua script with full game state access via ScriptContext.
/// Returns the list of GameCommands the script produced.
/// The `tier` parameter controls which behavior bindings are available.
pub fn execute_script_with_context(
    source: &str,
    ctx: &mut ScriptContext,
) -> Result<Vec<GameCommand>, LuaScriptError> {
    execute_script_with_context_tiered(source, ctx, ToolTier::Advanced)
}

/// Execute a Lua script with tier-gated behavior bindings.
pub fn execute_script_with_context_tiered(
    source: &str,
    ctx: &mut ScriptContext,
    tier: ToolTier,
) -> Result<Vec<GameCommand>, LuaScriptError> {
    let lua = Lua::new();

    // Wrap ctx in RefCell for interior mutability in scope closures
    let ctx_cell = RefCell::new(ctx);

    let result: Result<(), mlua::Error> = lua.scope(|scope| {
        let ctx_table = lua.create_table()?;

        // -------------------------------------------------------------------
        // Unit query bindings
        // Note: All functions accept `_self: LuaValue` as first arg because
        // Luau colon syntax `ctx:method(args)` passes `ctx` as first arg.
        // -------------------------------------------------------------------

        // ctx:my_units(kind_filter?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, filter): (LuaValue, Option<String>)| {
                let mut ctx = cell.borrow_mut();
                let kind = filter.and_then(|s| s.parse::<UnitKind>().ok());
                let units = ctx.my_units(kind);
                let tbl = lua.create_table()?;
                for (i, unit) in units.iter().enumerate() {
                    tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("my_units", f)?;
        }

        // ctx:enemy_units()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let units = ctx.enemy_units();
                let tbl = lua.create_table()?;
                for (i, unit) in units.iter().enumerate() {
                    tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("enemy_units", f)?;
        }

        // ctx:enemies_in_range(x, y, range)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|lua, (_self, x, y, range): (LuaValue, i32, i32, f64)| {
                    let mut ctx = cell.borrow_mut();
                    let fixed_range = Fixed::from_num(range);
                    let units = ctx.enemies_in_range(GridPos::new(x, y), fixed_range);
                    let tbl = lua.create_table()?;
                    for (i, unit) in units.iter().enumerate() {
                        tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                    }
                    Ok(tbl)
                })?;
            ctx_table.set("enemies_in_range", f)?;
        }

        // ctx:allies_in_range(x, y, range)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|lua, (_self, x, y, range): (LuaValue, i32, i32, f64)| {
                    let mut ctx = cell.borrow_mut();
                    let fixed_range = Fixed::from_num(range);
                    let units = ctx.allies_in_range(GridPos::new(x, y), fixed_range);
                    let tbl = lua.create_table()?;
                    for (i, unit) in units.iter().enumerate() {
                        tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                    }
                    Ok(tbl)
                })?;
            ctx_table.set("allies_in_range", f)?;
        }

        // ctx:nearest_enemy(x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                match ctx.nearest_enemy(GridPos::new(x, y)) {
                    Some(unit) => Ok(LuaValue::Table(unit_to_lua_table(lua, unit)?)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("nearest_enemy", f)?;
        }

        // ctx:nearest_ally(x, y, kind?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |lua, (_self, x, y, kind): (LuaValue, i32, i32, Option<String>)| {
                    let mut ctx = cell.borrow_mut();
                    let unit_kind = kind.and_then(|s| s.parse::<UnitKind>().ok());
                    match ctx.nearest_ally(GridPos::new(x, y), unit_kind) {
                        Some(unit) => Ok(LuaValue::Table(unit_to_lua_table(lua, unit)?)),
                        None => Ok(LuaValue::Nil),
                    }
                },
            )?;
            ctx_table.set("nearest_ally", f)?;
        }

        // ctx:threats_to(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, unit_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                let unit = ctx.state.unit_by_id(EntityId(unit_id)).cloned();
                match unit {
                    Some(ref u) => {
                        let threats = ctx.threats_to(u);
                        let tbl = lua.create_table()?;
                        for (i, t) in threats.iter().enumerate() {
                            tbl.set(i + 1, unit_to_lua_table(lua, t)?)?;
                        }
                        Ok(tbl)
                    }
                    None => Ok(lua.create_table()?),
                }
            })?;
            ctx_table.set("threats_to", f)?;
        }

        // ctx:targets_for(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, unit_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                let unit = ctx.state.unit_by_id(EntityId(unit_id)).cloned();
                match unit {
                    Some(ref u) => {
                        let targets = ctx.targets_for(u);
                        let tbl = lua.create_table()?;
                        for (i, t) in targets.iter().enumerate() {
                            tbl.set(i + 1, unit_to_lua_table(lua, t)?)?;
                        }
                        Ok(tbl)
                    }
                    None => Ok(lua.create_table()?),
                }
            })?;
            ctx_table.set("targets_for", f)?;
        }

        // -------------------------------------------------------------------
        // Extended unit query bindings
        // -------------------------------------------------------------------

        // ctx:distance_squared_between(a_id, b_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, a_id, b_id): (LuaValue, u64, u64)| {
                let mut ctx = cell.borrow_mut();
                match ctx.distance_squared_between(EntityId(a_id), EntityId(b_id)) {
                    Some(d) => Ok(LuaValue::Number(fixed_to_f64(d))),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("distance_squared_between", f)?;
        }

        // ctx:distance_squared_to_nearest_enemy(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, unit_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                match ctx.distance_squared_to_nearest_enemy(EntityId(unit_id)) {
                    Some(d) => Ok(LuaValue::Number(fixed_to_f64(d))),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("distance_squared_to_nearest_enemy", f)?;
        }

        // ctx:idle_units(kind?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, filter): (LuaValue, Option<String>)| {
                let mut ctx = cell.borrow_mut();
                let kind = filter.and_then(|s| s.parse::<UnitKind>().ok());
                let units = ctx.idle_units(kind);
                let tbl = lua.create_table()?;
                for (i, unit) in units.iter().enumerate() {
                    tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("idle_units", f)?;
        }

        // ctx:wounded_units(threshold)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, threshold): (LuaValue, f64)| {
                let mut ctx = cell.borrow_mut();
                let units = ctx.wounded_units(threshold);
                let tbl = lua.create_table()?;
                for (i, unit) in units.iter().enumerate() {
                    tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("wounded_units", f)?;
        }

        // ctx:units_by_state(state_str)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, state_str): (LuaValue, String)| {
                let state = match state_str.as_str() {
                    "Moving" => UnitState::Moving,
                    "Attacking" => UnitState::Attacking,
                    "Idle" => UnitState::Idle,
                    "Gathering" => UnitState::Gathering,
                    _ => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "Unknown unit state: {state_str}"
                        )));
                    }
                };
                let mut ctx = cell.borrow_mut();
                let units = ctx.units_by_state(state);
                let tbl = lua.create_table()?;
                for (i, unit) in units.iter().enumerate() {
                    tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("units_by_state", f)?;
        }

        // ctx:count_units(kind?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, filter): (LuaValue, Option<String>)| {
                let mut ctx = cell.borrow_mut();
                let kind = filter.and_then(|s| s.parse::<UnitKind>().ok());
                Ok(ctx.count_units(kind))
            })?;
            ctx_table.set("count_units", f)?;
        }

        // ctx:army_supply()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                Ok(ctx.army_supply())
            })?;
            ctx_table.set("army_supply", f)?;
        }

        // ctx:enemy_buildings()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let buildings = ctx.enemy_buildings();
                let tbl = lua.create_table()?;
                for (i, b) in buildings.iter().enumerate() {
                    tbl.set(i + 1, building_to_lua_table(lua, b)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("enemy_buildings", f)?;
        }

        // ctx:weakest_enemy_in_range(x, y, range)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|lua, (_self, x, y, range): (LuaValue, i32, i32, f64)| {
                    let mut ctx = cell.borrow_mut();
                    let fixed_range = Fixed::from_num(range);
                    match ctx.weakest_enemy_in_range(GridPos::new(x, y), fixed_range) {
                        Some(unit) => Ok(LuaValue::Table(unit_to_lua_table(lua, unit)?)),
                        None => Ok(LuaValue::Nil),
                    }
                })?;
            ctx_table.set("weakest_enemy_in_range", f)?;
        }

        // ctx:strongest_enemy_in_range(x, y, range)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|lua, (_self, x, y, range): (LuaValue, i32, i32, f64)| {
                    let mut ctx = cell.borrow_mut();
                    let fixed_range = Fixed::from_num(range);
                    match ctx.strongest_enemy_in_range(GridPos::new(x, y), fixed_range) {
                        Some(unit) => Ok(LuaValue::Table(unit_to_lua_table(lua, unit)?)),
                        None => Ok(LuaValue::Nil),
                    }
                })?;
            ctx_table.set("strongest_enemy_in_range", f)?;
        }

        // ctx:hp_pct(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, unit_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                match ctx.hp_pct(EntityId(unit_id)) {
                    Some(pct) => Ok(LuaValue::Number(pct)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("hp_pct", f)?;
        }

        // -------------------------------------------------------------------
        // Tactical query bindings
        // -------------------------------------------------------------------

        // ctx:position_at_range(from_x, from_y, target_x, target_y, desired_range)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, fx, fy, tx, ty, range): (LuaValue, i32, i32, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    match ctx.position_at_range(GridPos::new(fx, fy), GridPos::new(tx, ty), range) {
                        Some(pos) => Ok((LuaValue::Integer(pos.x), LuaValue::Integer(pos.y))),
                        None => Ok((LuaValue::Nil, LuaValue::Nil)),
                    }
                },
            )?;
            ctx_table.set("position_at_range", f)?;
        }

        // ctx:safe_positions(unit_id, search_radius)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|lua, (_self, unit_id, radius): (LuaValue, u64, i32)| {
                    let mut ctx = cell.borrow_mut();
                    let unit = ctx.state.unit_by_id(EntityId(unit_id)).cloned();
                    match unit {
                        Some(ref u) => {
                            let positions = ctx.safe_positions(u, radius);
                            let tbl = lua.create_table()?;
                            for (i, pos) in positions.iter().enumerate() {
                                let pos_tbl = lua.create_table()?;
                                pos_tbl.set("x", pos.x)?;
                                pos_tbl.set("y", pos.y)?;
                                tbl.set(i + 1, pos_tbl)?;
                            }
                            Ok(tbl)
                        }
                        None => Ok(lua.create_table()?),
                    }
                })?;
            ctx_table.set("safe_positions", f)?;
        }

        // -------------------------------------------------------------------
        // Terrain query bindings
        // -------------------------------------------------------------------

        // ctx:terrain_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                match ctx.terrain_at(GridPos::new(x, y)) {
                    Some(t) => Ok(LuaValue::String(lua.create_string(t.to_string())?)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("terrain_at", f)?;
        }

        // ctx:elevation_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                Ok(ctx.elevation_at(GridPos::new(x, y)))
            })?;
            ctx_table.set("elevation_at", f)?;
        }

        // ctx:cover_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                let cover = ctx.cover_at(GridPos::new(x, y));
                Ok(cover.to_string())
            })?;
            ctx_table.set("cover_at", f)?;
        }

        // ctx:is_passable(x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                Ok(ctx.is_passable(GridPos::new(x, y)))
            })?;
            ctx_table.set("is_passable", f)?;
        }

        // ctx:movement_cost(x, y) → number (multiplier) or nil if impassable
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let mut ctx = cell.borrow_mut();
                match ctx.movement_cost(GridPos::new(x, y)) {
                    Some(cost) => Ok(LuaValue::Number(fixed_to_f64(cost))),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("movement_cost", f)?;
        }

        // ctx:can_reach(from_x, from_y, to_x, to_y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, fx, fy, tx, ty): (LuaValue, i32, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    Ok(ctx.can_reach(GridPos::new(fx, fy), GridPos::new(tx, ty)))
                },
            )?;
            ctx_table.set("can_reach", f)?;
        }

        // ctx:path_length(from_x, from_y, to_x, to_y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, fx, fy, tx, ty): (LuaValue, i32, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    match ctx.path_length(GridPos::new(fx, fy), GridPos::new(tx, ty)) {
                        Some(len) => Ok(LuaValue::Integer(len as i32)),
                        None => Ok(LuaValue::Nil),
                    }
                },
            )?;
            ctx_table.set("path_length", f)?;
        }

        // -------------------------------------------------------------------
        // Economy query bindings
        // -------------------------------------------------------------------

        // ctx:resources()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let ctx = cell.borrow_mut();
                let res = ctx.resources();
                let tbl = lua.create_table()?;
                tbl.set("food", res.food)?;
                tbl.set("gpu_cores", res.gpu_cores)?;
                tbl.set("nfts", res.nfts)?;
                tbl.set("supply", res.supply)?;
                tbl.set("supply_cap", res.supply_cap)?;
                Ok(tbl)
            })?;
            ctx_table.set("resources", f)?;
        }

        // ctx:nearest_deposit(x, y, kind?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |lua, (_self, x, y, kind): (LuaValue, i32, i32, Option<String>)| {
                    let mut ctx = cell.borrow_mut();
                    let res_kind = kind.and_then(|s| s.parse::<ResourceType>().ok());
                    match ctx.nearest_deposit(GridPos::new(x, y), res_kind) {
                        Some(dep) => Ok(LuaValue::Table(deposit_to_lua_table(lua, dep)?)),
                        None => Ok(LuaValue::Nil),
                    }
                },
            )?;
            ctx_table.set("nearest_deposit", f)?;
        }

        // ctx:my_buildings(kind_filter?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, filter): (LuaValue, Option<String>)| {
                let mut ctx = cell.borrow_mut();
                let kind = filter.and_then(|s| s.parse::<BuildingKind>().ok());
                let buildings = ctx.my_buildings(kind);
                let tbl = lua.create_table()?;
                for (i, b) in buildings.iter().enumerate() {
                    tbl.set(i + 1, building_to_lua_table(lua, b)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("my_buildings", f)?;
        }

        // -------------------------------------------------------------------
        // Game state query bindings
        // -------------------------------------------------------------------

        // ctx:tick()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let ctx = cell.borrow();
                Ok(ctx.tick())
            })?;
            ctx_table.set("tick", f)?;
        }

        // ctx:my_faction()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let ctx = cell.borrow();
                Ok(ctx.my_faction().as_str().to_string())
            })?;
            ctx_table.set("my_faction", f)?;
        }

        // ctx:map_size()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let ctx = cell.borrow();
                let (w, h) = ctx.map_size();
                Ok((w, h))
            })?;
            ctx_table.set("map_size", f)?;
        }

        // -------------------------------------------------------------------
        // Phase 2: Vision query bindings (free)
        // -------------------------------------------------------------------

        // ctx:is_visible(x, y) → bool
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let ctx = cell.borrow();
                Ok(ctx.is_visible(GridPos::new(x, y)))
            })?;
            ctx_table.set("is_visible", f)?;
        }

        // ctx:fog_state(x, y) → "visible" or "fog"
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                let ctx = cell.borrow();
                Ok(ctx.fog_state(GridPos::new(x, y)).to_string())
            })?;
            ctx_table.set("fog_state", f)?;
        }

        // -------------------------------------------------------------------
        // Phase 2: Enemy memory bindings (budget-costed)
        // -------------------------------------------------------------------

        // ctx:last_seen_enemies() → table of memory entries
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let entries = ctx.last_seen_enemies();
                let tbl = lua.create_table()?;
                for (i, entry) in entries.iter().enumerate() {
                    tbl.set(i + 1, enemy_memory_to_lua_table(lua, entry)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("last_seen_enemies", f)?;
        }

        // ctx:last_seen_at(unit_id) → memory entry or nil
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, unit_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                match ctx.last_seen_at(unit_id) {
                    Some(entry) => Ok(LuaValue::Table(enemy_memory_to_lua_table(lua, &entry)?)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("last_seen_at", f)?;
        }

        // -------------------------------------------------------------------
        // Phase 2: Threat assessment bindings (budget-costed)
        // -------------------------------------------------------------------

        // ctx:threat_level(x, y, radius) → number
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|_, (_self, x, y, radius): (LuaValue, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    Ok(ctx.threat_level(GridPos::new(x, y), radius))
                })?;
            ctx_table.set("threat_level", f)?;
        }

        // ctx:army_strength() → {total_hp, total_dps, unit_count}
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let strength = ctx.army_strength();
                let tbl = lua.create_table()?;
                tbl.set("total_hp", strength.total_hp)?;
                tbl.set("total_dps", strength.total_dps)?;
                tbl.set("unit_count", strength.unit_count)?;
                Ok(tbl)
            })?;
            ctx_table.set("army_strength", f)?;
        }

        // -------------------------------------------------------------------
        // Phase 2: Inter-script event bindings (free)
        // -------------------------------------------------------------------

        // ctx:emit_event(name, data_string)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|_, (_self, name, data): (LuaValue, String, String)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.emit_event(name, data);
                    Ok(())
                })?;
            ctx_table.set("emit_event", f)?;
        }

        // ctx:poll_events(name) → table of {name, data, tick}
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, name): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                let events = ctx.poll_events(&name);
                let tbl = lua.create_table()?;
                for (i, event) in events.iter().enumerate() {
                    tbl.set(i + 1, script_event_to_lua_table(lua, event)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("poll_events", f)?;
        }

        // ctx:drain_events(name) → table of {name, data, tick}
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, name): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                let events = ctx.drain_events(&name);
                let tbl = lua.create_table()?;
                for (i, event) in events.iter().enumerate() {
                    tbl.set(i + 1, script_event_to_lua_table(lua, event)?)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("drain_events", f)?;
        }

        // -------------------------------------------------------------------
        // Phase 3: Strategic assessment, expansion, squads, scoring
        // -------------------------------------------------------------------

        // ctx:game_phase() → "early", "mid", or "late"
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let ctx = cell.borrow();
                Ok(ctx.game_phase().to_string())
            })?;
            ctx_table.set("game_phase", f)?;
        }

        // ctx:expansion_sites() → table of {deposit_id, resource_type, x, y, remaining, distance_to_base}
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let sites = ctx.expansion_sites();
                let tbl = lua.create_table()?;
                for (i, site) in sites.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("deposit_id", site.deposit_id)?;
                    entry.set("resource_type", site.resource_type.clone())?;
                    entry.set("x", site.x)?;
                    entry.set("y", site.y)?;
                    entry.set("remaining", site.remaining)?;
                    entry.set("distance_to_base", site.distance_to_base)?;
                    tbl.set(i + 1, entry)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("expansion_sites", f)?;
        }

        // ctx:predict_engagement({my_id1, my_id2}, {enemy_id1, enemy_id2})
        // → {winner, confidence, my_survivors, enemy_survivors}
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |lua, (_self, my_ids, enemy_ids): (LuaValue, Vec<u64>, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    let my_eids: Vec<EntityId> = my_ids.into_iter().map(EntityId).collect();
                    let enemy_eids: Vec<EntityId> = enemy_ids.into_iter().map(EntityId).collect();
                    let pred = ctx.predict_engagement(&my_eids, &enemy_eids);
                    let tbl = lua.create_table()?;
                    tbl.set("winner", pred.winner)?;
                    tbl.set("confidence", pred.confidence)?;
                    tbl.set("my_survivors", pred.my_survivors)?;
                    tbl.set("enemy_survivors", pred.enemy_survivors)?;
                    Ok(tbl)
                },
            )?;
            ctx_table.set("predict_engagement", f)?;
        }

        // ctx:squad_create(name, {id1, id2, ...})
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, name, unit_ids): (LuaValue, String, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.squad_create(name, unit_ids);
                    Ok(())
                },
            )?;
            ctx_table.set("squad_create", f)?;
        }

        // ctx:squad_add(name, {id1, id2})
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, name, unit_ids): (LuaValue, String, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.squad_add(&name, unit_ids);
                    Ok(())
                },
            )?;
            ctx_table.set("squad_add", f)?;
        }

        // ctx:squad_remove(name, {id1, id2})
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                |_, (_self, name, unit_ids): (LuaValue, String, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.squad_remove(&name, &unit_ids);
                    Ok(())
                },
            )?;
            ctx_table.set("squad_remove", f)?;
        }

        // ctx:squad_units(name) → table of unit IDs (auto-prunes dead)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, name): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                let ids = ctx.squad_units(&name);
                let tbl = lua.create_table()?;
                for (i, id) in ids.iter().enumerate() {
                    tbl.set(i + 1, *id)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("squad_units", f)?;
        }

        // ctx:squad_centroid(name) → x, y (two return values) or nil, nil
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, name): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                match ctx.squad_centroid(&name) {
                    Some((x, y)) => Ok((LuaValue::Integer(x), LuaValue::Integer(y))),
                    None => Ok((LuaValue::Nil, LuaValue::Nil)),
                }
            })?;
            ctx_table.set("squad_centroid", f)?;
        }

        // ctx:squad_disband(name)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, name): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                ctx.squad_disband(&name);
                Ok(())
            })?;
            ctx_table.set("squad_disband", f)?;
        }

        // ctx:squad_list() → table of squad name strings
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let ctx = cell.borrow();
                let names = ctx.squad_list();
                let tbl = lua.create_table()?;
                for (i, name) in names.iter().enumerate() {
                    tbl.set(i + 1, name.clone())?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("squad_list", f)?;
        }

        // ctx:game_score() → number (positive = winning)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                Ok(ctx.game_score())
            })?;
            ctx_table.set("game_score", f)?;
        }

        // -------------------------------------------------------------------
        // Command bindings
        // -------------------------------------------------------------------

        // ctx:move_units(unit_ids, x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_move(
                        unit_ids.into_iter().map(EntityId).collect(),
                        GridPos::new(x, y),
                    );
                    Ok(())
                },
            )?;
            ctx_table.set("move_units", f)?;
        }

        // ctx:attack_units(unit_ids, target_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, unit_ids, target_id): (LuaValue, Vec<u64>, u64)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_attack(
                        unit_ids.into_iter().map(EntityId).collect(),
                        EntityId(target_id),
                    );
                    Ok(())
                },
            )?;
            ctx_table.set("attack_units", f)?;
        }

        // ctx:attack_move(unit_ids, x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_attack_move(
                        unit_ids.into_iter().map(EntityId).collect(),
                        GridPos::new(x, y),
                    );
                    Ok(())
                },
            )?;
            ctx_table.set("attack_move", f)?;
        }

        // ctx:stop(unit_ids)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                let mut ctx = cell.borrow_mut();
                ctx.cmd_stop(unit_ids.into_iter().map(EntityId).collect());
                Ok(())
            })?;
            ctx_table.set("stop", f)?;
        }

        // ctx:hold(unit_ids)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                let mut ctx = cell.borrow_mut();
                ctx.cmd_hold(unit_ids.into_iter().map(EntityId).collect());
                Ok(())
            })?;
            ctx_table.set("hold", f)?;
        }

        // ctx:gather(unit_ids, deposit_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, unit_ids, deposit_id): (LuaValue, Vec<u64>, u64)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_gather(
                        unit_ids.into_iter().map(EntityId).collect(),
                        EntityId(deposit_id),
                    );
                    Ok(())
                },
            )?;
            ctx_table.set("gather", f)?;
        }

        // ctx:build(builder_id, building_type, x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_,
                      (_self, builder_id, building_type, x, y): (
                    LuaValue,
                    u64,
                    String,
                    i32,
                    i32,
                )| {
                    let kind = building_type.parse::<BuildingKind>().map_err(|_| {
                        mlua::Error::RuntimeError(format!("Unknown building type: {building_type}"))
                    })?;
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_build(EntityId(builder_id), kind, GridPos::new(x, y));
                    Ok(())
                },
            )?;
            ctx_table.set("build", f)?;
        }

        // ctx:train(building_id, unit_type)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, building_id, unit_type): (LuaValue, u64, String)| {
                    let kind = unit_type.parse::<UnitKind>().map_err(|_| {
                        mlua::Error::RuntimeError(format!("Unknown unit type: {unit_type}"))
                    })?;
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_train(EntityId(building_id), kind);
                    Ok(())
                },
            )?;
            ctx_table.set("train", f)?;
        }

        // -------------------------------------------------------------------
        // Behavior bindings (ctx.behaviors sub-table)
        // Tier-gated: only registers behaviors the caller has unlocked.
        // -------------------------------------------------------------------

        {
            let behaviors_table = lua.create_table()?;

            // === Basic (Tier 0) behaviors ===

            // ctx.behaviors:assign_idle_workers()
            {
                let cell = &ctx_cell;
                let f = scope.create_function(|_, _self: LuaValue| {
                    let mut ctx = cell.borrow_mut();
                    let result = crate::behaviors::assign_idle_workers(&mut ctx);
                    Ok(result.commands_issued as u32)
                })?;
                behaviors_table.set("assign_idle_workers", f)?;
            }

            // ctx.behaviors:attack_move_group(unit_ids, x, y)
            {
                let cell = &ctx_cell;
                let f = scope.create_function(
                    |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                        let mut ctx = cell.borrow_mut();
                        let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                        let result =
                            crate::behaviors::attack_move_group(&mut ctx, &ids, GridPos::new(x, y));
                        Ok(result.commands_issued as u32)
                    },
                )?;
                behaviors_table.set("attack_move_group", f)?;
            }

            // === Tactical (Tier 1) behaviors ===
            if tier >= ToolTier::Tactical {
                // ctx.behaviors:focus_fire(attacker_ids, target_id)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_, (_self, attacker_ids, target_id): (LuaValue, Vec<u64>, u64)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> =
                                attacker_ids.into_iter().map(EntityId).collect();
                            let result =
                                crate::behaviors::focus_fire(&mut ctx, &ids, EntityId(target_id));
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("focus_fire", f)?;
                }

                // ctx.behaviors:kite_squad(unit_ids)
                {
                    let cell = &ctx_cell;
                    let f =
                        scope.create_function(|_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                            let result = crate::behaviors::kite_squad(&mut ctx, &ids);
                            Ok(result.commands_issued as u32)
                        })?;
                    behaviors_table.set("kite_squad", f)?;
                }

                // ctx.behaviors:retreat_wounded(threshold)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(|_, (_self, threshold): (LuaValue, f64)| {
                        let mut ctx = cell.borrow_mut();
                        let result = crate::behaviors::retreat_wounded(&mut ctx, threshold);
                        Ok(result.commands_issued as u32)
                    })?;
                    behaviors_table.set("retreat_wounded", f)?;
                }

                // ctx.behaviors:defend_area(unit_ids, cx, cy, radius)
                {
                    let cell = &ctx_cell;
                    let f =
                        scope.create_function(
                            |_,
                             (_self, unit_ids, cx, cy, radius): (
                                LuaValue,
                                Vec<u64>,
                                i32,
                                i32,
                                f64,
                            )| {
                                let mut ctx = cell.borrow_mut();
                                let ids: Vec<EntityId> =
                                    unit_ids.into_iter().map(EntityId).collect();
                                let result = crate::behaviors::defend_area(
                                    &mut ctx,
                                    &ids,
                                    GridPos::new(cx, cy),
                                    Fixed::from_num(radius),
                                );
                                Ok(result.commands_issued as u32)
                            },
                        )?;
                    behaviors_table.set("defend_area", f)?;
                }

                // ctx.behaviors:harass_economy(raider_ids)
                {
                    let cell = &ctx_cell;
                    let f =
                        scope.create_function(|_, (_self, raider_ids): (LuaValue, Vec<u64>)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = raider_ids.into_iter().map(EntityId).collect();
                            let result = crate::behaviors::harass_economy(&mut ctx, &ids);
                            Ok(result.commands_issued as u32)
                        })?;
                    behaviors_table.set("harass_economy", f)?;
                }

                // ctx.behaviors:scout_pattern(scout_id, waypoints)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_, (_self, scout_id, waypoints): (LuaValue, u64, Vec<LuaTable>)| {
                            let mut ctx = cell.borrow_mut();
                            let wps: Vec<GridPos> = waypoints
                                .iter()
                                .filter_map(|wp| {
                                    let x: i32 = wp.get("x").ok()?;
                                    let y: i32 = wp.get("y").ok()?;
                                    Some(GridPos::new(x, y))
                                })
                                .collect();
                            let result =
                                crate::behaviors::scout_pattern(&mut ctx, EntityId(scout_id), &wps);
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("scout_pattern", f)?;
                }

                // ctx.behaviors:focus_weakest(unit_ids, range)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_, (_self, unit_ids, range): (LuaValue, Vec<u64>, f64)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                            let result = crate::behaviors::focus_weakest(
                                &mut ctx,
                                &ids,
                                Fixed::from_num(range),
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("focus_weakest", f)?;
                }

                // ctx.behaviors:use_ability(unit_id, slot, target_type, x?, y?, entity_id?)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_,
                         (_self, unit_id, slot, target_type, x, y, entity_id): (
                            LuaValue,
                            u64,
                            u8,
                            String,
                            Option<i32>,
                            Option<i32>,
                            Option<u64>,
                        )| {
                            let target = match target_type.as_str() {
                                "self" => AbilityTarget::SelfCast,
                                "position" => {
                                    let px = x.ok_or_else(|| {
                                        mlua::Error::RuntimeError("position requires x".into())
                                    })?;
                                    let py = y.ok_or_else(|| {
                                        mlua::Error::RuntimeError("position requires y".into())
                                    })?;
                                    AbilityTarget::Position(GridPos::new(px, py))
                                }
                                "entity" => {
                                    let eid = entity_id.ok_or_else(|| {
                                        mlua::Error::RuntimeError(
                                            "entity requires entity_id".into(),
                                        )
                                    })?;
                                    AbilityTarget::Entity(EntityId(eid))
                                }
                                _ => {
                                    return Err(mlua::Error::RuntimeError(format!(
                                        "Unknown target type: {target_type}"
                                    )));
                                }
                            };
                            let mut ctx = cell.borrow_mut();
                            let result = crate::behaviors::use_ability(
                                &mut ctx,
                                EntityId(unit_id),
                                slot,
                                target,
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("use_ability", f)?;
                }

                // ctx.behaviors:split_squads(unit_ids) → returns {melee={}, ranged={}, support={}}
                {
                    let cell = &ctx_cell;
                    let f =
                        scope.create_function(|lua, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                            let (melee, ranged, support, _) =
                                crate::behaviors::split_squads(&mut ctx, &ids);
                            let tbl = lua.create_table()?;
                            tbl.set("melee", melee.iter().map(|e| e.0).collect::<Vec<_>>())?;
                            tbl.set("ranged", ranged.iter().map(|e| e.0).collect::<Vec<_>>())?;
                            tbl.set("support", support.iter().map(|e| e.0).collect::<Vec<_>>())?;
                            Ok(tbl)
                        })?;
                    behaviors_table.set("split_squads", f)?;
                }

                // ctx.behaviors:protect_unit(escort_ids, vip_id, guard_radius?)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_,
                         (_self, escort_ids, vip_id, guard_radius): (
                            LuaValue,
                            Vec<u64>,
                            u64,
                            Option<f64>,
                        )| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = escort_ids.into_iter().map(EntityId).collect();
                            let radius = Fixed::from_num(guard_radius.unwrap_or(5.0));
                            let result = crate::behaviors::protect_unit(
                                &mut ctx,
                                &ids,
                                EntityId(vip_id),
                                radius,
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("protect_unit", f)?;
                }

                // ctx.behaviors:surround_target(unit_ids, target_id, ring_radius?)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_,
                         (_self, unit_ids, target_id, ring_radius): (
                            LuaValue,
                            Vec<u64>,
                            u64,
                            Option<f64>,
                        )| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                            let radius = Fixed::from_num(ring_radius.unwrap_or(3.0));
                            let result = crate::behaviors::surround_target(
                                &mut ctx,
                                &ids,
                                EntityId(target_id),
                                radius,
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("surround_target", f)?;
                }
            }

            // === Strategic (Tier 2) behaviors ===
            if tier >= ToolTier::Strategic {
                // ctx.behaviors:auto_produce(building_id, unit_type_str)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_, (_self, building_id, unit_type): (LuaValue, u64, String)| {
                            let kind = unit_type.parse::<UnitKind>().map_err(|_| {
                                mlua::Error::RuntimeError(format!("Unknown unit type: {unit_type}"))
                            })?;
                            let mut ctx = cell.borrow_mut();
                            let result = crate::behaviors::auto_produce(
                                &mut ctx,
                                EntityId(building_id),
                                kind,
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("auto_produce", f)?;
                }

                // ctx.behaviors:balanced_production(building_id)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(|_, (_self, building_id): (LuaValue, u64)| {
                        let mut ctx = cell.borrow_mut();
                        let result =
                            crate::behaviors::balanced_production(&mut ctx, EntityId(building_id));
                        Ok(result.commands_issued as u32)
                    })?;
                    behaviors_table.set("balanced_production", f)?;
                }

                // ctx.behaviors:expand_economy(builder_id)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(|_, (_self, builder_id): (LuaValue, u64)| {
                        let mut ctx = cell.borrow_mut();
                        let result =
                            crate::behaviors::expand_economy(&mut ctx, EntityId(builder_id));
                        Ok(result.commands_issued as u32)
                    })?;
                    behaviors_table.set("expand_economy", f)?;
                }

                // ctx.behaviors:coordinate_assault(unit_ids, x, y)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(
                        |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                            let mut ctx = cell.borrow_mut();
                            let ids: Vec<EntityId> = unit_ids.into_iter().map(EntityId).collect();
                            let result = crate::behaviors::coordinate_assault(
                                &mut ctx,
                                &ids,
                                GridPos::new(x, y),
                            );
                            Ok(result.commands_issued as u32)
                        },
                    )?;
                    behaviors_table.set("coordinate_assault", f)?;
                }
            }

            // === Advanced (Tier 3) behaviors ===
            if tier >= ToolTier::Advanced {
                // ctx.behaviors:research_priority(building_id)
                {
                    let cell = &ctx_cell;
                    let f = scope.create_function(|_, (_self, building_id): (LuaValue, u64)| {
                        let mut ctx = cell.borrow_mut();
                        let result =
                            crate::behaviors::research_priority(&mut ctx, EntityId(building_id));
                        Ok(result.commands_issued as u32)
                    })?;
                    behaviors_table.set("research_priority", f)?;
                }

                // ctx.behaviors:adaptive_defense(unit_ids, cx, cy, radius)
                {
                    let cell = &ctx_cell;
                    let f =
                        scope.create_function(
                            |_,
                             (_self, unit_ids, cx, cy, radius): (
                                LuaValue,
                                Vec<u64>,
                                i32,
                                i32,
                                f64,
                            )| {
                                let mut ctx = cell.borrow_mut();
                                let ids: Vec<EntityId> =
                                    unit_ids.into_iter().map(EntityId).collect();
                                let result = crate::behaviors::adaptive_defense(
                                    &mut ctx,
                                    &ids,
                                    GridPos::new(cx, cy),
                                    Fixed::from_num(radius),
                                );
                                Ok(result.commands_issued as u32)
                            },
                        )?;
                    behaviors_table.set("adaptive_defense", f)?;
                }
            }

            ctx_table.set("behaviors", behaviors_table)?;
        }

        // -------------------------------------------------------------------
        // Extended command bindings
        // -------------------------------------------------------------------

        // ctx:ability(unit_id, slot, target_type, x?, y?, entity_id?)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_,
                      (_self, unit_id, slot, target_type, x, y, entity_id): (
                    LuaValue,
                    u64,
                    u8,
                    String,
                    Option<i32>,
                    Option<i32>,
                    Option<u64>,
                )| {
                    let target = match target_type.as_str() {
                        "self" => AbilityTarget::SelfCast,
                        "position" => {
                            let px = x.ok_or_else(|| {
                                mlua::Error::RuntimeError("position target requires x".into())
                            })?;
                            let py = y.ok_or_else(|| {
                                mlua::Error::RuntimeError("position target requires y".into())
                            })?;
                            AbilityTarget::Position(GridPos::new(px, py))
                        }
                        "entity" => {
                            let eid = entity_id.ok_or_else(|| {
                                mlua::Error::RuntimeError("entity target requires entity_id".into())
                            })?;
                            AbilityTarget::Entity(EntityId(eid))
                        }
                        _ => {
                            return Err(mlua::Error::RuntimeError(format!(
                                "Unknown target type: {target_type}"
                            )));
                        }
                    };
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_ability(EntityId(unit_id), slot, target);
                    Ok(())
                },
            )?;
            ctx_table.set("ability", f)?;
        }

        // ctx:research(building_id, upgrade_type)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, building_id, upgrade_str): (LuaValue, u64, String)| {
                    let upgrade = upgrade_str.parse::<UpgradeType>().map_err(|_| {
                        mlua::Error::RuntimeError(format!("Unknown upgrade type: {upgrade_str}"))
                    })?;
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_research(EntityId(building_id), upgrade);
                    Ok(())
                },
            )?;
            ctx_table.set("research", f)?;
        }

        // ctx:cancel_queue(building_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(move |_, (_self, building_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                ctx.cmd_cancel_queue(EntityId(building_id));
                Ok(())
            })?;
            ctx_table.set("cancel_queue", f)?;
        }

        // ctx:cancel_research(building_id)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(move |_, (_self, building_id): (LuaValue, u64)| {
                let mut ctx = cell.borrow_mut();
                ctx.cmd_cancel_research(EntityId(building_id));
                Ok(())
            })?;
            ctx_table.set("cancel_research", f)?;
        }

        // ctx:set_control_group(group, unit_ids)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, group, unit_ids): (LuaValue, u8, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_set_control_group(group, unit_ids.into_iter().map(EntityId).collect());
                    Ok(())
                },
            )?;
            ctx_table.set("set_control_group", f)?;
        }

        // ctx:rally(building_id, x, y)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(
                move |_, (_self, building_id, x, y): (LuaValue, u64, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_rally(EntityId(building_id), GridPos::new(x, y));
                    Ok(())
                },
            )?;
            ctx_table.set("rally", f)?;
        }

        // ctx:get_resources()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let ctx = cell.borrow();
                let res = ctx.resources();
                let tbl = lua.create_table()?;
                tbl.set("food", res.food as f64)?;
                tbl.set("gpu_cores", res.gpu_cores as f64)?;
                tbl.set("nfts", res.nfts as f64)?;
                tbl.set("supply", res.supply as f64)?;
                tbl.set("supply_cap", res.supply_cap as f64)?;
                Ok(tbl)
            })?;
            ctx_table.set("get_resources", f)?;
        }

        // ctx:resource_deposits()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let deposits = ctx.resource_deposits();
                let tbl = lua.create_table()?;
                for (i, dep) in deposits.iter().enumerate() {
                    let d = lua.create_table()?;
                    d.set("id", dep.id.0)?;
                    d.set("x", dep.pos.x)?;
                    d.set("y", dep.pos.y)?;
                    d.set("remaining", dep.remaining as f64)?;
                    d.set("kind", dep.resource_type.to_string())?;
                    tbl.set(i + 1, d)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("resource_deposits", f)?;
        }

        // -------------------------------------------------------------------
        // Blackboard bindings
        // -------------------------------------------------------------------

        // ctx:blackboard_get(key)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, (_self, key): (LuaValue, String)| {
                let ctx = cell.borrow();
                match ctx.blackboard_get(&key) {
                    Some(BlackboardValue::String(s)) => Ok(LuaValue::String(lua.create_string(s)?)),
                    Some(BlackboardValue::Number(n)) => Ok(LuaValue::Number(*n)),
                    Some(BlackboardValue::Bool(b)) => Ok(LuaValue::Boolean(*b)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("blackboard_get", f)?;
        }

        // ctx:blackboard_set(key, value)
        {
            let cell = &ctx_cell;
            let f =
                scope.create_function(|_, (_self, key, value): (LuaValue, String, LuaValue)| {
                    let mut ctx = cell.borrow_mut();
                    match value {
                        LuaValue::Nil => {
                            ctx.blackboard_remove(&key);
                        }
                        LuaValue::Boolean(b) => {
                            ctx.blackboard_set(key, BlackboardValue::Bool(b));
                        }
                        LuaValue::Integer(n) => {
                            ctx.blackboard_set(key, BlackboardValue::Number(n as f64));
                        }
                        LuaValue::Number(n) => {
                            ctx.blackboard_set(key, BlackboardValue::Number(n));
                        }
                        LuaValue::String(s) => {
                            let s = s.to_str().map(|s| s.to_owned()).unwrap_or_default();
                            ctx.blackboard_set(key, BlackboardValue::String(s));
                        }
                        _ => { /* Unsupported type — silently ignore */ }
                    }
                    Ok(())
                })?;
            ctx_table.set("blackboard_set", f)?;
        }

        // ctx:blackboard_keys()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let ctx = cell.borrow();
                let keys = ctx.blackboard_keys();
                let tbl = lua.create_table()?;
                for (i, key) in keys.iter().enumerate() {
                    tbl.set(i + 1, key.as_str())?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("blackboard_keys", f)?;
        }

        // -------------------------------------------------------------------
        // Budget introspection
        // -------------------------------------------------------------------

        // ctx:remaining_budget()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, _self: LuaValue| {
                let ctx = cell.borrow();
                Ok(ctx.remaining_budget())
            })?;
            ctx_table.set("remaining_budget", f)?;
        }

        // -------------------------------------------------------------------
        // Economy analysis bindings
        // -------------------------------------------------------------------

        // ctx:income_rate()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let income = ctx.income_rate();
                let tbl = lua.create_table()?;
                tbl.set("food_per_tick", income.food_per_tick)?;
                tbl.set("gpu_per_tick", income.gpu_per_tick)?;
                Ok(tbl)
            })?;
            ctx_table.set("income_rate", f)?;
        }

        // ctx:can_afford(kind_str) — tries UnitKind first, then BuildingKind
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, kind_str): (LuaValue, String)| {
                let ctx = cell.borrow();
                if let Ok(unit_kind) = kind_str.parse::<UnitKind>() {
                    Ok(ctx.can_afford_unit(unit_kind))
                } else if let Ok(building_kind) = kind_str.parse::<BuildingKind>() {
                    Ok(ctx.can_afford_building(building_kind))
                } else {
                    Ok(false) // Unknown kind
                }
            })?;
            ctx_table.set("can_afford", f)?;
        }

        // ctx:time_until_afford(kind_str)
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|_, (_self, kind_str): (LuaValue, String)| {
                let mut ctx = cell.borrow_mut();
                let result = if let Ok(unit_kind) = kind_str.parse::<UnitKind>() {
                    ctx.time_until_afford_unit(unit_kind)
                } else if let Ok(building_kind) = kind_str.parse::<BuildingKind>() {
                    ctx.time_until_afford_building(building_kind)
                } else {
                    None
                };
                match result {
                    Some(ticks) => Ok(LuaValue::Number(ticks as f64)),
                    None => Ok(LuaValue::Nil),
                }
            })?;
            ctx_table.set("time_until_afford", f)?;
        }

        // ctx:army_composition()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let comp = ctx.army_composition();
                let tbl = lua.create_table()?;
                for (kind, count) in &comp {
                    tbl.set(kind.as_str(), *count)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("army_composition", f)?;
        }

        // ctx:enemy_composition()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let comp = ctx.enemy_composition();
                let tbl = lua.create_table()?;
                for (kind, count) in &comp {
                    tbl.set(kind.as_str(), *count)?;
                }
                Ok(tbl)
            })?;
            ctx_table.set("enemy_composition", f)?;
        }

        // ctx:worker_saturation()
        {
            let cell = &ctx_cell;
            let f = scope.create_function(|lua, _self: LuaValue| {
                let mut ctx = cell.borrow_mut();
                let sat = ctx.worker_saturation();
                let tbl = lua.create_table()?;
                tbl.set("total", sat.total)?;
                tbl.set("gathering", sat.gathering)?;
                tbl.set("idle", sat.idle)?;
                Ok(tbl)
            })?;
            ctx_table.set("worker_saturation", f)?;
        }

        // Set ctx as global
        lua.globals().set("ctx", ctx_table)?;

        // Remove os/debug libraries before sandboxing
        lua.globals().set("os", LuaValue::Nil)?;
        lua.globals().set("debug", LuaValue::Nil)?;

        // Enable Luau sandbox (freezes globals, restricts environment)
        lua.sandbox(true)?;

        // Set interrupt to prevent infinite loops
        let count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        lua.set_interrupt(move |_| {
            let c = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if c >= INSTRUCTION_LIMIT {
                Err(mlua::Error::RuntimeError(
                    "Script exceeded instruction limit".into(),
                ))
            } else {
                Ok(mlua::VmState::Continue)
            }
        });

        // Execute script
        lua.load(source).exec()?;

        Ok(())
    });

    result.map_err(LuaScriptError::Lua)?;

    // Extract accumulated commands from the context
    let ctx = ctx_cell.into_inner();
    Ok(ctx.take_commands())
}

/// Execute a Lua script in a sandboxed environment (command-only, no queries).
/// Thin wrapper around `execute_script_with_context` with an empty game state.
pub fn execute_script(source: &str, player_id: u8) -> Result<Vec<GameCommand>, LuaScriptError> {
    use crate::snapshot::GameStateSnapshot;
    use cc_core::map::GameMap;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    let empty_snap = GameStateSnapshot {
        tick: 0,
        map_width: 1,
        map_height: 1,
        player_id,
        my_units: vec![],
        enemy_units: vec![],
        my_buildings: vec![],
        enemy_buildings: vec![],
        resource_deposits: vec![],
        my_resources: PlayerResourceState::default(),
    };
    let map = GameMap::new(1, 1);
    let mut ctx = ScriptContext::new(&empty_snap, &map, player_id, FactionId::CatGPT);
    execute_script_with_context(source, &mut ctx)
}

// ---------------------------------------------------------------------------
// Helpers: Rust → Lua conversion
// ---------------------------------------------------------------------------

fn unit_to_lua_table(lua: &Lua, unit: &UnitSnapshot) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", unit.id.0)?;
    tbl.set("kind", unit.kind.to_string())?;
    tbl.set("x", unit.pos.x)?;
    tbl.set("y", unit.pos.y)?;
    tbl.set("hp", fixed_to_f64(unit.health_current))?;
    tbl.set("hp_max", fixed_to_f64(unit.health_max))?;
    tbl.set("speed", fixed_to_f64(unit.speed))?;
    tbl.set("damage", fixed_to_f64(unit.attack_damage))?;
    tbl.set("range", fixed_to_f64(unit.attack_range))?;
    tbl.set("attack_speed", unit.attack_speed)?;
    tbl.set("attack_type", unit.attack_type.to_string())?;
    // Backward-compatible short aliases
    tbl.set("atk_dmg", fixed_to_f64(unit.attack_damage))?;
    tbl.set("atk_range", fixed_to_f64(unit.attack_range))?;
    tbl.set("atk_speed", unit.attack_speed)?;
    tbl.set("atk_type", unit.attack_type.to_string())?;
    tbl.set("moving", unit.is_moving)?;
    tbl.set("attacking", unit.is_attacking)?;
    tbl.set("in_combat", unit.in_combat)?;
    tbl.set("idle", unit.is_idle)?;
    tbl.set("gathering", unit.is_gathering)?;
    tbl.set("owner", unit.owner)?;
    // Phase B enrichment: status effects
    let se_tbl = lua.create_table()?;
    for (i, se) in unit.status_effects.iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("effect_type", se.effect_type.clone())?;
        entry.set("remaining_ticks", se.remaining_ticks)?;
        entry.set("stacks", se.stacks)?;
        se_tbl.set(i + 1, entry)?;
    }
    tbl.set("status_effects", se_tbl)?;
    // Phase B enrichment: abilities
    let ab_tbl = lua.create_table()?;
    for (i, ab) in unit.abilities.iter().enumerate() {
        let entry = lua.create_table()?;
        entry.set("slot", ab.slot)?;
        entry.set("id", ab.id.clone())?;
        entry.set("cooldown_remaining", ab.cooldown_remaining)?;
        entry.set("ready", ab.ready)?;
        ab_tbl.set(i + 1, entry)?;
    }
    tbl.set("abilities", ab_tbl)?;
    Ok(tbl)
}

fn building_to_lua_table(lua: &Lua, building: &BuildingSnapshot) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", building.id.0)?;
    tbl.set("kind", building.kind.to_string())?;
    tbl.set("x", building.pos.x)?;
    tbl.set("y", building.pos.y)?;
    tbl.set("hp", fixed_to_f64(building.health_current))?;
    tbl.set("hp_max", fixed_to_f64(building.health_max))?;
    tbl.set("under_construction", building.under_construction)?;
    tbl.set("construction_progress", building.construction_progress)?;
    tbl.set("producing", !building.production_queue.is_empty())?;
    tbl.set("owner", building.owner)?;
    // Phase B enrichment: research queue
    let rq_tbl = lua.create_table()?;
    for (i, rq) in building.research_queue.iter().enumerate() {
        rq_tbl.set(i + 1, rq.clone())?;
    }
    tbl.set("research_queue", rq_tbl)?;
    Ok(tbl)
}

fn deposit_to_lua_table(lua: &Lua, deposit: &ResourceSnapshot) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", deposit.id.0)?;
    tbl.set("kind", deposit.resource_type.to_string())?;
    tbl.set("x", deposit.pos.x)?;
    tbl.set("y", deposit.pos.y)?;
    tbl.set("remaining", deposit.remaining)?;
    Ok(tbl)
}

fn fixed_to_f64(v: Fixed) -> f64 {
    v.to_num::<f64>()
}

fn enemy_memory_to_lua_table(lua: &Lua, entry: &EnemyMemoryEntry) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", entry.unit_id)?;
    tbl.set("kind", entry.kind.clone())?;
    tbl.set("x", entry.x)?;
    tbl.set("y", entry.y)?;
    tbl.set("hp_pct", entry.hp_pct)?;
    tbl.set("tick_last_seen", entry.tick_last_seen)?;
    tbl.set("confirmed_dead", entry.confirmed_dead)?;
    Ok(tbl)
}

fn script_event_to_lua_table(lua: &Lua, event: &ScriptEvent) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("name", event.name.clone())?;
    tbl.set("data", event.data.clone())?;
    tbl.set("tick", event.tick)?;
    Ok(tbl)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum LuaScriptError {
    Lua(mlua::Error),
}

impl std::fmt::Display for LuaScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LuaScriptError::Lua(e) => write!(f, "Lua error: {e}"),
        }
    }
}

impl std::error::Error for LuaScriptError {}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::map::GameMap;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    use crate::snapshot::GameStateSnapshot;
    use crate::test_fixtures::make_unit;

    // Keep original tests working with execute_script (no context)
    #[test]
    fn simple_move_script() {
        let script = r#"ctx:move_units({1, 2, 3}, 10, 15)"#;
        let cmds = execute_script(script, 0).unwrap();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            GameCommand::Move { unit_ids, target } => {
                assert_eq!(unit_ids.len(), 3);
                assert_eq!(target.x, 10);
                assert_eq!(target.y, 15);
            }
            _ => panic!("Expected Move command"),
        }
    }

    #[test]
    fn script_produces_multiple_commands() {
        let script = r#"
            ctx:move_units({1}, 5, 5)
            ctx:attack_move({2, 3}, 20, 20)
            ctx:stop({4})
        "#;
        let cmds = execute_script(script, 0).unwrap();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn infinite_loop_terminates() {
        let script = r#"while true do end"#;
        let result = execute_script(script, 0);
        assert!(result.is_err(), "Infinite loop should be caught");
    }

    #[test]
    fn no_os_access() {
        let script = r#"
            if os then
                error("os should not be available")
            end
        "#;
        let result = execute_script(script, 0);
        assert!(result.is_ok(), "os should not exist in sandbox");
    }

    #[test]
    fn empty_script_produces_no_commands() {
        let cmds = execute_script("", 0).unwrap();
        assert!(cmds.is_empty());
    }

    // -------------------------------------------------------------------
    // New tests for execute_script_with_context
    // -------------------------------------------------------------------

    fn make_test_snapshot() -> GameStateSnapshot {
        GameStateSnapshot {
            tick: 42,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![
                make_unit(1, UnitKind::Hisser, 5, 5, 0),
                make_unit(2, UnitKind::Chonk, 10, 10, 0),
                make_unit(3, UnitKind::Hisser, 15, 15, 0),
            ],
            enemy_units: vec![
                make_unit(10, UnitKind::Nuisance, 7, 5, 1),
                make_unit(11, UnitKind::Chonk, 50, 50, 1),
            ],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![crate::snapshot::ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(3, 3),
                remaining: 200,
            }],
            my_resources: PlayerResourceState::default(),
        }
    }

    #[test]
    fn ctx_my_units_returns_all() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local units = ctx:my_units()
            -- Should have 3 units
            if #units ~= 3 then
                error("Expected 3 units, got " .. #units)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn ctx_my_units_filters_by_kind() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local hissers = ctx:my_units("Hisser")
            if #hissers ~= 2 then
                error("Expected 2 Hissers, got " .. #hissers)
            end
            -- Verify unit data fields
            local h = hissers[1]
            if h.kind ~= "Hisser" then error("Wrong kind: " .. h.kind) end
            if h.hp ~= 100 then error("Wrong hp: " .. h.hp) end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn ctx_enemies_in_range() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            -- Enemy at (7,5) is 2 tiles from (5,5), range 5 should find it
            local enemies = ctx:enemies_in_range(5, 5, 5)
            if #enemies ~= 1 then
                error("Expected 1 enemy, got " .. #enemies)
            end
            -- Attack it
            ctx:attack_units({1}, enemies[1].id)
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            GameCommand::Attack { unit_ids, target } => {
                assert_eq!(unit_ids[0], EntityId(1));
                assert_eq!(*target, EntityId(10));
            }
            _ => panic!("Expected Attack command"),
        }
    }

    #[test]
    fn ctx_resources() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local res = ctx:resources()
            if res.food ~= 300 then error("Wrong food: " .. res.food) end
            if res.gpu_cores ~= 50 then error("Wrong gpu: " .. res.gpu_cores) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_tick_and_map_size() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local t = ctx:tick()
            if t ~= 42 then error("Wrong tick: " .. t) end
            local w, h = ctx:map_size()
            if w ~= 64 or h ~= 64 then error("Wrong map size") end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_my_faction() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);

        // Test catGPT
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);
        let script = r#"
            local f = ctx:my_faction()
            if f ~= "catGPT" then error("Wrong faction: " .. f) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();

        // Test Croak
        let mut ctx2 = ScriptContext::new(&snap, &map, 0, FactionId::Croak);
        let script2 = r#"
            local f = ctx:my_faction()
            if f ~= "Croak" then error("Wrong faction: " .. f) end
        "#;
        execute_script_with_context(script2, &mut ctx2).unwrap();
    }

    #[test]
    fn ctx_nearest_deposit() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local dep = ctx:nearest_deposit(0, 0, "Food")
            if dep == nil then error("No deposit found") end
            if dep.kind ~= "Food" then error("Wrong type: " .. dep.kind) end
            if dep.remaining ~= 200 then error("Wrong remaining") end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_commands_through_context() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            ctx:move_units({1, 2}, 20, 20)
            ctx:stop({3})
            ctx:hold({1})
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn ctx_movement_cost() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Default map tiles are Grass (cost 1.0), should return a number
        let script = r#"
            local cost = ctx:movement_cost(5, 5)
            if cost == nil then
                error("Expected number, got nil")
            end
            if cost <= 0 then
                error("Expected positive cost, got " .. cost)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn ctx_kiting_script() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // Kiting script: find Hissers, check for enemies in range, kite
        let script = r#"
            local units = ctx:my_units("Hisser")
            for _, unit in ipairs(units) do
                local enemies = ctx:enemies_in_range(unit.x, unit.y, unit.atk_range + 2)
                if #enemies > 0 then
                    -- Sort by HP (highest first = "highest value")
                    table.sort(enemies, function(a, b) return a.hp > b.hp end)
                    local target = enemies[1]
                    local kx, ky = ctx:position_at_range(unit.x, unit.y, target.x, target.y, math.floor(unit.atk_range))
                    if kx then
                        ctx:move_units({unit.id}, kx, ky)
                        ctx:attack_units({unit.id}, target.id)
                    end
                end
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        // Unit 1 (Hisser at 5,5) should find enemy at (7,5) within range 7
        // and issue move + attack commands
        assert!(
            cmds.len() >= 2,
            "Expected at least 2 commands (move+attack), got {}",
            cmds.len()
        );
    }

    #[test]
    fn formation_orient_script_runs() {
        use cc_core::math::fixed_from_i32;

        // Build a mixed army: 1 tank, 2 melee, 2 ranged (all idle, not attacking)
        let mut chonk = make_unit(1, UnitKind::Chonk, 10, 10, 0);
        chonk.health_max = fixed_from_i32(300);
        chonk.health_current = fixed_from_i32(300);

        let nuisance1 = make_unit(2, UnitKind::Nuisance, 11, 10, 0);
        let nuisance2 = make_unit(3, UnitKind::Nuisance, 10, 11, 0);

        let hisser1 = make_unit(4, UnitKind::Hisser, 9, 10, 0);
        let hisser2 = make_unit(5, UnitKind::Hisser, 10, 9, 0);

        // Enemy far away so formation triggers (dist > 4)
        let enemy = make_unit(10, UnitKind::Nuisance, 30, 30, 1);

        let snap = GameStateSnapshot {
            tick: 10,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![chonk, nuisance1, nuisance2, hisser1, hisser2],
            enemy_units: vec![enemy],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = include_str!("../../../training/agent/baselines_clean/formation_orient.lua");
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();

        // Should produce commands: 1 for tank (front), 2 for melee (mid), 2 for ranged (back)
        assert_eq!(
            cmds.len(),
            5,
            "Expected 5 commands (1 tank + 2 melee + 2 ranged), got {}",
            cmds.len()
        );

        // Verify tank goes forward (toward enemy at 30,30) and ranged goes back
        // Tank command should be attack_move with target > army centroid (10,10)
        match &cmds[0] {
            GameCommand::AttackMove { target, .. } => {
                assert!(
                    target.x > 10 && target.y > 10,
                    "Tank should move toward enemy, got ({}, {})",
                    target.x,
                    target.y
                );
            }
            other => panic!("Expected AttackMove for tank, got {:?}", other),
        }

        // Last commands are ranged (move_units, not attack_move) behind centroid
        for cmd in &cmds[3..5] {
            match cmd {
                GameCommand::Move { target, .. } => {
                    assert!(
                        target.x < 10 || target.y < 10,
                        "Ranged should stay behind centroid, got ({}, {})",
                        target.x,
                        target.y
                    );
                }
                other => panic!("Expected Move for ranged, got {:?}", other),
            }
        }
    }

    // -------------------------------------------------------------------
    // Phase 1: Blackboard, budget, economy, composition, worker saturation
    // -------------------------------------------------------------------

    #[test]
    fn ctx_blackboard_set_and_get() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            ctx:blackboard_set("phase", "attack")
            ctx:blackboard_set("count", 42)
            ctx:blackboard_set("ready", true)
            local phase = ctx:blackboard_get("phase")
            if phase ~= "attack" then error("Expected 'attack', got: " .. tostring(phase)) end
            local count = ctx:blackboard_get("count")
            if count ~= 42 then error("Expected 42, got: " .. tostring(count)) end
            local ready = ctx:blackboard_get("ready")
            if ready ~= true then error("Expected true, got: " .. tostring(ready)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_blackboard_get_nil_for_missing() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local val = ctx:blackboard_get("nonexistent")
            if val ~= nil then error("Expected nil, got: " .. tostring(val)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_blackboard_set_nil_removes_key() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            ctx:blackboard_set("temp", "value")
            local v1 = ctx:blackboard_get("temp")
            if v1 ~= "value" then error("Expected 'value'") end
            ctx:blackboard_set("temp", nil)
            local v2 = ctx:blackboard_get("temp")
            if v2 ~= nil then error("Expected nil after delete, got: " .. tostring(v2)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_blackboard_keys() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            ctx:blackboard_set("alpha", 1)
            ctx:blackboard_set("beta", 2)
            local keys = ctx:blackboard_keys()
            if #keys ~= 2 then error("Expected 2 keys, got: " .. #keys) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_remaining_budget() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local b = ctx:remaining_budget()
            if b ~= 500 then error("Expected 500, got: " .. b) end
            -- Spend some budget with a query
            ctx:my_units()
            local b2 = ctx:remaining_budget()
            if b2 ~= 499 then error("Expected 499 after query, got: " .. b2) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_income_rate() {
        // Create a snapshot with a gathering worker near a food deposit
        let mut worker = make_unit(1, UnitKind::Pawdler, 3, 3, 0);
        worker.is_gathering = true;
        worker.is_idle = false;

        let snap = GameStateSnapshot {
            tick: 10,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![worker],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![crate::snapshot::ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(3, 4),
                remaining: 200,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local inc = ctx:income_rate()
            if inc.food_per_tick <= 0 then error("Expected positive food income, got: " .. inc.food_per_tick) end
            if inc.gpu_per_tick ~= 0 then error("Expected 0 gpu income, got: " .. inc.gpu_per_tick) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_can_afford() {
        // Default resources: food=300, gpu=50
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            -- Pawdler costs 50 food, 0 gpu — should be affordable
            local can = ctx:can_afford("Pawdler")
            if not can then error("Should afford Pawdler") end

            -- MechCommander costs 400 food, 200 gpu — should NOT be affordable
            local cant = ctx:can_afford("MechCommander")
            if cant then error("Should not afford MechCommander") end

            -- Building: LitterBox costs 75 food, 0 gpu — should be affordable
            local can_build = ctx:can_afford("LitterBox")
            if not can_build then error("Should afford LitterBox") end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_time_until_afford_already_affordable() {
        // Create a snapshot with a gathering worker so income_rate > 0
        let mut worker = make_unit(1, UnitKind::Pawdler, 3, 3, 0);
        worker.is_gathering = true;
        worker.is_idle = false;

        let snap = GameStateSnapshot {
            tick: 10,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![worker],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![crate::snapshot::ResourceSnapshot {
                id: EntityId(100),
                resource_type: ResourceType::Food,
                pos: GridPos::new(3, 4),
                remaining: 200,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            -- Pawdler costs 50 food — we have 300 — should return 0
            local t = ctx:time_until_afford("Pawdler")
            if t ~= 0 then error("Expected 0 ticks, got: " .. tostring(t)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_army_composition() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local comp = ctx:army_composition()
            -- Test snapshot has 2 Hissers and 1 Chonk
            if comp.Hisser ~= 2 then error("Expected 2 Hissers, got: " .. tostring(comp.Hisser)) end
            if comp.Chonk ~= 1 then error("Expected 1 Chonk, got: " .. tostring(comp.Chonk)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_enemy_composition() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local comp = ctx:enemy_composition()
            -- Test snapshot has 1 Nuisance and 1 Chonk as enemies
            if comp.Nuisance ~= 1 then error("Expected 1 Nuisance, got: " .. tostring(comp.Nuisance)) end
            if comp.Chonk ~= 1 then error("Expected 1 Chonk, got: " .. tostring(comp.Chonk)) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_worker_saturation() {
        // Create snapshot with workers in different states
        let mut gathering_worker = make_unit(1, UnitKind::Pawdler, 3, 3, 0);
        gathering_worker.is_gathering = true;
        gathering_worker.is_idle = false;

        let idle_worker = make_unit(2, UnitKind::Pawdler, 5, 5, 0);
        // default is_idle = true from make_unit

        let combat_unit = make_unit(3, UnitKind::Hisser, 10, 10, 0);

        let snap = GameStateSnapshot {
            tick: 10,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![gathering_worker, idle_worker, combat_unit],
            enemy_units: vec![],
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local sat = ctx:worker_saturation()
            if sat.total ~= 2 then error("Expected 2 total workers, got: " .. sat.total) end
            if sat.gathering ~= 1 then error("Expected 1 gathering, got: " .. sat.gathering) end
            if sat.idle ~= 1 then error("Expected 1 idle, got: " .. sat.idle) end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn ctx_blackboard_persists_via_take() {
        // Test that blackboard can be taken and re-injected for persistence
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);

        // First script invocation: set a value
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);
        let script1 = r#"ctx:blackboard_set("wave", 3)"#;
        execute_script_with_context(script1, &mut ctx).unwrap();
        let bb = ctx.take_blackboard();

        // Second invocation: inject the blackboard back, verify value persisted
        let mut ctx2 = ScriptContext::new_with_blackboard(&snap, &map, 0, FactionId::CatGPT, bb);
        let script2 = r#"
            local w = ctx:blackboard_get("wave")
            if w ~= 3 then error("Expected 3, got: " .. tostring(w)) end
        "#;
        execute_script_with_context(script2, &mut ctx2).unwrap();
    }

    // -------------------------------------------------------------------
    // Phase 2: Vision, Memory, Threats, Events — Lua integration tests
    // -------------------------------------------------------------------

    #[test]
    fn lua_is_visible_and_fog_state() {
        let snap = make_test_snapshot(); // unit at (5,5), (10,10), (15,15)
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            -- Unit at (5,5) has sight range 8, so (5,5) is visible
            if not ctx:is_visible(5, 5) then
                error("Expected (5,5) to be visible")
            end
            if ctx:fog_state(5, 5) ~= "visible" then
                error("Expected fog_state 'visible' at (5,5)")
            end
            -- Far tile should be in fog
            if ctx:is_visible(60, 60) then
                error("Expected (60,60) to be in fog")
            end
            if ctx:fog_state(60, 60) ~= "fog" then
                error("Expected fog_state 'fog' at (60,60)")
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_threat_level() {
        let snap = make_test_snapshot(); // enemies at (7,5) and (50,50)
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            -- Enemy at (7,5) has damage=10, within Chebyshev radius 5 from (5,5)
            local threat = ctx:threat_level(5, 5, 5)
            if threat < 9 or threat > 11 then
                error("Expected threat ~10 near enemy, got " .. threat)
            end

            -- Far from enemies
            local far_threat = ctx:threat_level(60, 60, 3)
            if far_threat ~= 0 then
                error("Expected 0 threat far from enemies, got " .. far_threat)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_army_strength() {
        let snap = make_test_snapshot(); // 3 own units
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local str = ctx:army_strength()
            if str.unit_count ~= 3 then
                error("Expected 3 units, got " .. str.unit_count)
            end
            if str.total_hp < 299 then
                error("Expected ~300 total HP, got " .. str.total_hp)
            end
            if str.total_dps < 29 then
                error("Expected ~30 total DPS, got " .. str.total_dps)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_last_seen_enemies_with_memory() {
        use std::collections::HashMap;

        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut memory = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_enemy_memory(&mut memory);
        ctx.update_enemy_memory();

        let script = r#"
            local enemies = ctx:last_seen_enemies()
            if #enemies ~= 2 then
                error("Expected 2 remembered enemies, got " .. #enemies)
            end
            -- Check fields on first entry
            local e = enemies[1]
            if e.id == nil then error("Missing id") end
            if e.kind == nil then error("Missing kind") end
            if e.x == nil then error("Missing x") end
            if e.y == nil then error("Missing y") end
            if e.hp_pct == nil then error("Missing hp_pct") end
            if e.tick_last_seen == nil then error("Missing tick_last_seen") end
            if e.confirmed_dead ~= false then error("Should not be dead") end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_last_seen_at_specific_enemy() {
        use std::collections::HashMap;

        let snap = make_test_snapshot(); // enemies: id=10 at (7,5), id=11 at (50,50)
        let map = GameMap::new(64, 64);
        let mut memory = HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_enemy_memory(&mut memory);
        ctx.update_enemy_memory();

        let script = r#"
            local e = ctx:last_seen_at(10)
            if e == nil then
                error("Expected to find enemy 10")
            end
            if e.x ~= 7 then
                error("Expected x=7 got " .. e.x)
            end
            if e.y ~= 5 then
                error("Expected y=5 got " .. e.y)
            end
            -- Nonexistent enemy
            local missing = ctx:last_seen_at(999)
            if missing ~= nil then
                error("Expected nil for missing enemy")
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_emit_and_poll_events() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut event_bus = Vec::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_events(&mut event_bus);

        let script = r#"
            -- Emit events
            ctx:emit_event("attack", "go_north")
            ctx:emit_event("retreat", "fallback")
            ctx:emit_event("attack", "charge")

            -- Poll attack events (should not remove)
            local attacks = ctx:poll_events("attack")
            if #attacks ~= 2 then
                error("Expected 2 attack events, got " .. #attacks)
            end
            if attacks[1].name ~= "attack" then
                error("Wrong name: " .. attacks[1].name)
            end
            if attacks[1].data ~= "go_north" then
                error("Wrong data: " .. attacks[1].data)
            end

            -- Poll again — still there
            local still = ctx:poll_events("attack")
            if #still ~= 2 then
                error("Events should not be consumed by poll, got " .. #still)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn lua_drain_events_removes() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut event_bus = Vec::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_events(&mut event_bus);

        let script = r#"
            ctx:emit_event("attack", "go")
            ctx:emit_event("retreat", "now")
            ctx:emit_event("attack", "charge")

            -- Drain attack events
            local attacks = ctx:drain_events("attack")
            if #attacks ~= 2 then
                error("Expected 2 drained attack events, got " .. #attacks)
            end

            -- Attack events should be gone
            local empty = ctx:poll_events("attack")
            if #empty ~= 0 then
                error("Expected 0 attack events after drain, got " .. #empty)
            end

            -- Retreat event should remain
            local retreats = ctx:poll_events("retreat")
            if #retreats ~= 1 then
                error("Expected 1 retreat event, got " .. #retreats)
            end
        "#;
        let cmds = execute_script_with_context(script, &mut ctx).unwrap();
        assert!(cmds.is_empty());
    }

    // -----------------------------------------------------------------------
    // Phase 3 Lua integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn lua_game_phase_returns_early() {
        let snap = make_test_snapshot(); // tick=42, small army, no buildings
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local phase = ctx:game_phase()
            if phase ~= "early" then
                error("Expected 'early', got '" .. phase .. "'")
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_expansion_sites_returns_table() {
        use crate::snapshot::{BuildingSnapshot, ResourceSnapshot};
        use cc_core::math::fixed_from_i32;

        let snap = GameStateSnapshot {
            tick: 0,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units: vec![],
            enemy_units: vec![],
            my_buildings: vec![BuildingSnapshot {
                id: EntityId(100),
                kind: BuildingKind::TheBox,
                pos: GridPos::new(10, 10),
                owner: 0,
                health_current: fixed_from_i32(500),
                health_max: fixed_from_i32(500),
                under_construction: false,
                construction_progress: 1.0,
                production_queue: vec![],
                research_queue: vec![],
            }],
            enemy_buildings: vec![],
            resource_deposits: vec![ResourceSnapshot {
                id: EntityId(200),
                resource_type: ResourceType::Food,
                pos: GridPos::new(40, 40),
                remaining: 200,
            }],
            my_resources: PlayerResourceState::default(),
        };
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local sites = ctx:expansion_sites()
            if #sites ~= 1 then
                error("Expected 1 site, got " .. #sites)
            end
            local s = sites[1]
            if s.deposit_id ~= 200 then
                error("Wrong deposit_id: " .. s.deposit_id)
            end
            if s.x ~= 40 or s.y ~= 40 then
                error("Wrong position")
            end
            if s.remaining ~= 200 then
                error("Wrong remaining: " .. s.remaining)
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_predict_engagement_returns_result() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        // My units: ids 1, 2, 3. Enemy: ids 10, 11.
        // My: 3 units (100hp each, 10dmg/10ticks = 1 DPS each) = 300hp, 3 DPS
        // Enemy: 2 units (100hp each, 10dmg/10ticks = 1 DPS each) = 200hp, 2 DPS
        // I should win
        let script = r#"
            local result = ctx:predict_engagement({1, 2, 3}, {10, 11})
            if result.winner ~= "self" then
                error("Expected 'self', got '" .. result.winner .. "'")
            end
            if result.confidence <= 0 then
                error("Expected positive confidence")
            end
            if result.my_survivors <= 0 then
                error("Expected some survivors")
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_squad_create_and_units() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut squads = std::collections::HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        let script = r#"
            ctx:squad_create("alpha", {1, 2})
            local units = ctx:squad_units("alpha")
            if #units ~= 2 then
                error("Expected 2 units, got " .. #units)
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_squad_centroid_returns_coords() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut squads = std::collections::HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        // Units 1 at (5,5) and 2 at (10,10). Centroid = (7, 7)
        let script = r#"
            ctx:squad_create("alpha", {1, 2})
            local x, y = ctx:squad_centroid("alpha")
            if x == nil then
                error("Expected centroid, got nil")
            end
            if x ~= 7 or y ~= 7 then
                error("Expected (7, 7), got (" .. x .. ", " .. y .. ")")
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_squad_disband_removes_squad() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut squads = std::collections::HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        let script = r#"
            ctx:squad_create("alpha", {1, 2})
            ctx:squad_disband("alpha")
            local units = ctx:squad_units("alpha")
            if #units ~= 0 then
                error("Expected 0 units after disband, got " .. #units)
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_squad_list_returns_names() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut squads = std::collections::HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        let script = r#"
            ctx:squad_create("alpha", {1})
            ctx:squad_create("bravo", {2})
            local names = ctx:squad_list()
            if #names ~= 2 then
                error("Expected 2 squads, got " .. #names)
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_squad_add_and_remove() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut squads = std::collections::HashMap::new();
        let mut ctx =
            ScriptContext::new(&snap, &map, 0, FactionId::CatGPT).with_squads(&mut squads);

        let script = r#"
            ctx:squad_create("alpha", {1})
            ctx:squad_add("alpha", {2, 3})
            local units = ctx:squad_units("alpha")
            if #units ~= 3 then
                error("Expected 3 after add, got " .. #units)
            end
            ctx:squad_remove("alpha", {2})
            local units2 = ctx:squad_units("alpha")
            if #units2 ~= 2 then
                error("Expected 2 after remove, got " .. #units2)
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }

    #[test]
    fn lua_game_score_returns_number() {
        let snap = make_test_snapshot();
        let map = GameMap::new(64, 64);
        let mut ctx = ScriptContext::new(&snap, &map, 0, FactionId::CatGPT);

        let script = r#"
            local score = ctx:game_score()
            -- Should be a number (positive since we have more army)
            if type(score) ~= "number" then
                error("Expected number, got " .. type(score))
            end
        "#;
        execute_script_with_context(script, &mut ctx).unwrap();
    }
}
