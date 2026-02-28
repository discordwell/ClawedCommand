use std::cell::RefCell;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{BuildingKind, ResourceType, UnitKind};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;
use mlua::prelude::*;

use crate::script_context::ScriptContext;
use crate::snapshot::{BuildingSnapshot, ResourceSnapshot, UnitSnapshot};

/// Maximum Lua instructions before termination (prevents infinite loops).
const INSTRUCTION_LIMIT: u32 = 10_000;

/// Execute a Lua script with full game state access via ScriptContext.
/// Returns the list of GameCommands the script produced.
pub fn execute_script_with_context(
    source: &str,
    ctx: &mut ScriptContext,
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
            let f = scope
                .create_function(|lua, (_self, filter): (LuaValue, Option<String>)| {
                    let mut ctx = cell.borrow_mut();
                    let kind = filter.and_then(|s| s.parse::<UnitKind>().ok());
                    let units = ctx.my_units(kind);
                    let tbl = lua.create_table()?;
                    for (i, unit) in units.iter().enumerate() {
                        tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                    }
                    Ok(tbl)
                })
                ?;
            ctx_table.set("my_units", f)?;
        }

        // ctx:enemy_units()
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, _self: LuaValue| {
                    let mut ctx = cell.borrow_mut();
                    let units = ctx.enemy_units();
                    let tbl = lua.create_table()?;
                    for (i, unit) in units.iter().enumerate() {
                        tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                    }
                    Ok(tbl)
                })
                ?;
            ctx_table.set("enemy_units", f)?;
        }

        // ctx:enemies_in_range(x, y, range)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, x, y, range): (LuaValue, i32, i32, f64)| {
                    let mut ctx = cell.borrow_mut();
                    let fixed_range = Fixed::from_num(range);
                    let units = ctx.enemies_in_range(GridPos::new(x, y), fixed_range);
                    let tbl = lua.create_table()?;
                    for (i, unit) in units.iter().enumerate() {
                        tbl.set(i + 1, unit_to_lua_table(lua, unit)?)?;
                    }
                    Ok(tbl)
                })
                ?;
            ctx_table
                .set("enemies_in_range", f)
                ?;
        }

        // ctx:nearest_enemy(x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, x, y): (LuaValue, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    match ctx.nearest_enemy(GridPos::new(x, y)) {
                        Some(unit) => Ok(LuaValue::Table(unit_to_lua_table(lua, unit)?)),
                        None => Ok(LuaValue::Nil),
                    }
                })
                ?;
            ctx_table
                .set("nearest_enemy", f)
                ?;
        }

        // ctx:threats_to(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, unit_id): (LuaValue, u64)| {
                    let mut ctx = cell.borrow_mut();
                    let unit = ctx
                        .state
                        .unit_by_id(EntityId(unit_id))
                        .cloned();
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
                })
                ?;
            ctx_table
                .set("threats_to", f)
                ?;
        }

        // ctx:targets_for(unit_id)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, unit_id): (LuaValue, u64)| {
                    let mut ctx = cell.borrow_mut();
                    let unit = ctx
                        .state
                        .unit_by_id(EntityId(unit_id))
                        .cloned();
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
                })
                ?;
            ctx_table
                .set("targets_for", f)
                ?;
        }

        // -------------------------------------------------------------------
        // Tactical query bindings
        // -------------------------------------------------------------------

        // ctx:position_at_range(from_x, from_y, target_x, target_y, desired_range)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(
                    |_, (_self, fx, fy, tx, ty, range): (LuaValue, i32, i32, i32, i32, i32)| {
                        let mut ctx = cell.borrow_mut();
                        match ctx.position_at_range(
                            GridPos::new(fx, fy),
                            GridPos::new(tx, ty),
                            range,
                        ) {
                            Some(pos) => Ok((LuaValue::Integer(pos.x), LuaValue::Integer(pos.y))),
                            None => Ok((LuaValue::Nil, LuaValue::Nil)),
                        }
                    },
                )
                ?;
            ctx_table
                .set("position_at_range", f)
                ?;
        }

        // ctx:safe_positions(unit_id, search_radius)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, unit_id, radius): (LuaValue, u64, i32)| {
                    let mut ctx = cell.borrow_mut();
                    let unit = ctx
                        .state
                        .unit_by_id(EntityId(unit_id))
                        .cloned();
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
                })
                ?;
            ctx_table
                .set("safe_positions", f)
                ?;
        }

        // -------------------------------------------------------------------
        // Terrain query bindings
        // -------------------------------------------------------------------

        // ctx:terrain_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, x, y): (LuaValue, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    match ctx.terrain_at(GridPos::new(x, y)) {
                        Some(t) => Ok(LuaValue::String(
                            lua.create_string(t.to_string())?,
                        )),
                        None => Ok(LuaValue::Nil),
                    }
                })
                ?;
            ctx_table
                .set("terrain_at", f)
                ?;
        }

        // ctx:elevation_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    Ok(ctx.elevation_at(GridPos::new(x, y)))
                })
                ?;
            ctx_table
                .set("elevation_at", f)
                ?;
        }

        // ctx:cover_at(x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    let cover = ctx.cover_at(GridPos::new(x, y));
                    Ok(cover.to_string())
                })
                ?;
            ctx_table
                .set("cover_at", f)
                ?;
        }

        // ctx:is_passable(x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, (_self, x, y): (LuaValue, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    Ok(ctx.is_passable(GridPos::new(x, y)))
                })
                ?;
            ctx_table
                .set("is_passable", f)
                ?;
        }

        // ctx:can_reach(from_x, from_y, to_x, to_y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, (_self, fx, fy, tx, ty): (LuaValue, i32, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    Ok(ctx.can_reach(GridPos::new(fx, fy), GridPos::new(tx, ty)))
                })
                ?;
            ctx_table
                .set("can_reach", f)
                ?;
        }

        // ctx:path_length(from_x, from_y, to_x, to_y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, (_self, fx, fy, tx, ty): (LuaValue, i32, i32, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    match ctx.path_length(GridPos::new(fx, fy), GridPos::new(tx, ty)) {
                        Some(len) => Ok(LuaValue::Integer(len as i32)),
                        None => Ok(LuaValue::Nil),
                    }
                })
                ?;
            ctx_table
                .set("path_length", f)
                ?;
        }

        // -------------------------------------------------------------------
        // Economy query bindings
        // -------------------------------------------------------------------

        // ctx:resources()
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, _self: LuaValue| {
                    let ctx = cell.borrow_mut();
                    let res = ctx.resources();
                    let tbl = lua.create_table()?;
                    tbl.set("food", res.food)?;
                    tbl.set("gpu_cores", res.gpu_cores)?;
                    tbl.set("nfts", res.nfts)?;
                    tbl.set("supply", res.supply)?;
                    tbl.set("supply_cap", res.supply_cap)?;
                    Ok(tbl)
                })
                ?;
            ctx_table
                .set("resources", f)
                ?;
        }

        // ctx:nearest_deposit(x, y, kind?)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, x, y, kind): (LuaValue, i32, i32, Option<String>)| {
                    let mut ctx = cell.borrow_mut();
                    let res_kind = kind.and_then(|s| s.parse::<ResourceType>().ok());
                    match ctx.nearest_deposit(GridPos::new(x, y), res_kind) {
                        Some(dep) => Ok(LuaValue::Table(deposit_to_lua_table(lua, dep)?)),
                        None => Ok(LuaValue::Nil),
                    }
                })
                ?;
            ctx_table
                .set("nearest_deposit", f)
                ?;
        }

        // ctx:my_buildings(kind_filter?)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|lua, (_self, filter): (LuaValue, Option<String>)| {
                    let mut ctx = cell.borrow_mut();
                    let kind = filter.and_then(|s| s.parse::<BuildingKind>().ok());
                    let buildings = ctx.my_buildings(kind);
                    let tbl = lua.create_table()?;
                    for (i, b) in buildings.iter().enumerate() {
                        tbl.set(i + 1, building_to_lua_table(lua, b)?)?;
                    }
                    Ok(tbl)
                })
                ?;
            ctx_table
                .set("my_buildings", f)
                ?;
        }

        // -------------------------------------------------------------------
        // Game state query bindings
        // -------------------------------------------------------------------

        // ctx:tick()
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, _self: LuaValue| {
                    let ctx = cell.borrow();
                    Ok(ctx.tick())
                })
                ?;
            ctx_table.set("tick", f)?;
        }

        // ctx:map_size()
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(|_, _self: LuaValue| {
                    let ctx = cell.borrow();
                    let (w, h) = ctx.map_size();
                    Ok((w, h))
                })
                ?;
            ctx_table
                .set("map_size", f)
                ?;
        }

        // -------------------------------------------------------------------
        // Command bindings
        // -------------------------------------------------------------------

        // ctx:move_units(unit_ids, x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_move(
                        unit_ids.into_iter().map(EntityId).collect(),
                        GridPos::new(x, y),
                    );
                    Ok(())
                })
                ?;
            ctx_table
                .set("move_units", f)
                ?;
        }

        // ctx:attack_units(unit_ids, target_id)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(move |_, (_self, unit_ids, target_id): (LuaValue, Vec<u64>, u64)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_attack(
                        unit_ids.into_iter().map(EntityId).collect(),
                        EntityId(target_id),
                    );
                    Ok(())
                })
                ?;
            ctx_table
                .set("attack_units", f)
                ?;
        }

        // ctx:attack_move(unit_ids, x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_attack_move(
                        unit_ids.into_iter().map(EntityId).collect(),
                        GridPos::new(x, y),
                    );
                    Ok(())
                })
                ?;
            ctx_table
                .set("attack_move", f)
                ?;
        }

        // ctx:stop(unit_ids)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_stop(unit_ids.into_iter().map(EntityId).collect());
                    Ok(())
                })
                ?;
            ctx_table.set("stop", f)?;
        }

        // ctx:hold(unit_ids)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                    let mut ctx = cell.borrow_mut();
                    ctx.cmd_hold(unit_ids.into_iter().map(EntityId).collect());
                    Ok(())
                })
                ?;
            ctx_table.set("hold", f)?;
        }

        // ctx:gather(unit_ids, deposit_id)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(
                    move |_, (_self, unit_ids, deposit_id): (LuaValue, Vec<u64>, u64)| {
                        let mut ctx = cell.borrow_mut();
                        ctx.cmd_gather(
                            unit_ids.into_iter().map(EntityId).collect(),
                            EntityId(deposit_id),
                        );
                        Ok(())
                    },
                )
                ?;
            ctx_table.set("gather", f)?;
        }

        // ctx:build(builder_id, building_type, x, y)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(
                    move |_, (_self, builder_id, building_type, x, y): (LuaValue, u64, String, i32, i32)| {
                        let kind = building_type.parse::<BuildingKind>()
                            .map_err(|_| mlua::Error::RuntimeError(
                                format!("Unknown building type: {building_type}"),
                            ))?;
                        let mut ctx = cell.borrow_mut();
                        ctx.cmd_build(EntityId(builder_id), kind, GridPos::new(x, y));
                        Ok(())
                    },
                )
                ?;
            ctx_table.set("build", f)?;
        }

        // ctx:train(building_id, unit_type)
        {
            let cell = &ctx_cell;
            let f = scope
                .create_function(
                    move |_, (_self, building_id, unit_type): (LuaValue, u64, String)| {
                        let kind = unit_type.parse::<UnitKind>()
                            .map_err(|_| mlua::Error::RuntimeError(
                                format!("Unknown unit type: {unit_type}"),
                            ))?;
                        let mut ctx = cell.borrow_mut();
                        ctx.cmd_train(EntityId(building_id), kind);
                        Ok(())
                    },
                )
                ?;
            ctx_table.set("train", f)?;
        }

        // Set ctx as global
        lua.globals()
            .set("ctx", ctx_table)
            ?;

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
        lua.load(source)
            .exec()
            ?;

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
    use cc_core::map::GameMap;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;
    use crate::snapshot::GameStateSnapshot;

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
    tbl.set("atk_dmg", fixed_to_f64(unit.attack_damage))?;
    tbl.set("atk_range", fixed_to_f64(unit.attack_range))?;
    tbl.set("atk_speed", unit.attack_speed)?;
    tbl.set("atk_type", unit.attack_type.to_string())?;
    tbl.set("moving", unit.is_moving)?;
    tbl.set("attacking", unit.is_attacking)?;
    tbl.set("idle", unit.is_idle)?;
    tbl.set("owner", unit.owner)?;
    Ok(tbl)
}

fn building_to_lua_table(
    lua: &Lua,
    building: &BuildingSnapshot,
) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", building.id.0)?;
    tbl.set("kind", building.kind.to_string())?;
    tbl.set("x", building.pos.x)?;
    tbl.set("y", building.pos.y)?;
    tbl.set("hp", fixed_to_f64(building.health_current))?;
    tbl.set("hp_max", fixed_to_f64(building.health_max))?;
    tbl.set("under_construction", building.under_construction)?;
    tbl.set("construction_progress", building.construction_progress)?;
    tbl.set("owner", building.owner)?;
    Ok(tbl)
}

fn deposit_to_lua_table(
    lua: &Lua,
    deposit: &ResourceSnapshot,
) -> LuaResult<LuaTable> {
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
            resource_deposits: vec![
                crate::snapshot::ResourceSnapshot {
                    id: EntityId(100),
                    resource_type: ResourceType::Food,
                    pos: GridPos::new(3, 3),
                    remaining: 200,
                },
            ],
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
}
