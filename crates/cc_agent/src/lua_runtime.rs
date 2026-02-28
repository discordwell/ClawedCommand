use std::cell::RefCell;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{AttackType, BuildingKind, ResourceType, UnitKind};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;
use cc_core::terrain::CoverLevel;
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
                    let kind = filter.and_then(|s| parse_unit_kind(&s));
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
                            lua.create_string(terrain_type_name(t))?,
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
                    Ok(cover_level_name(cover))
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
                    let res_kind = kind.and_then(|s| parse_resource_type(&s));
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
                    let kind = filter.and_then(|s| parse_building_kind(&s));
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
                        let kind = parse_building_kind(&building_type)
                            .ok_or_else(|| mlua::Error::RuntimeError(
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
                        let kind = parse_unit_kind(&unit_type)
                            .ok_or_else(|| mlua::Error::RuntimeError(
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
/// Kept for backwards compatibility with simple scripts.
pub fn execute_script(source: &str, _player_id: u8) -> Result<Vec<GameCommand>, LuaScriptError> {
    let lua = Lua::new();

    let commands: std::sync::Arc<std::sync::Mutex<Vec<GameCommand>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let ctx = lua.create_table().map_err(LuaScriptError::Lua)?;

    // Note: All functions accept `_self: LuaValue` as first arg because
    // Luau colon syntax `ctx:method(args)` passes `ctx` as first arg.

    // ctx:move_units(unit_ids, x, y)
    {
        let cmds = commands.clone();
        let f = lua
            .create_function(move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                cmds.lock().unwrap().push(GameCommand::Move {
                    unit_ids: unit_ids.into_iter().map(EntityId).collect(),
                    target: GridPos::new(x, y),
                });
                Ok(())
            })
            .map_err(LuaScriptError::Lua)?;
        ctx.set("move_units", f).map_err(LuaScriptError::Lua)?;
    }

    // ctx:attack_units(unit_ids, target_id)
    {
        let cmds = commands.clone();
        let f = lua
            .create_function(move |_, (_self, unit_ids, target_id): (LuaValue, Vec<u64>, u64)| {
                cmds.lock().unwrap().push(GameCommand::Attack {
                    unit_ids: unit_ids.into_iter().map(EntityId).collect(),
                    target: EntityId(target_id),
                });
                Ok(())
            })
            .map_err(LuaScriptError::Lua)?;
        ctx.set("attack_units", f).map_err(LuaScriptError::Lua)?;
    }

    // ctx:attack_move(unit_ids, x, y)
    {
        let cmds = commands.clone();
        let f = lua
            .create_function(move |_, (_self, unit_ids, x, y): (LuaValue, Vec<u64>, i32, i32)| {
                cmds.lock().unwrap().push(GameCommand::AttackMove {
                    unit_ids: unit_ids.into_iter().map(EntityId).collect(),
                    target: GridPos::new(x, y),
                });
                Ok(())
            })
            .map_err(LuaScriptError::Lua)?;
        ctx.set("attack_move", f).map_err(LuaScriptError::Lua)?;
    }

    // ctx:stop(unit_ids)
    {
        let cmds = commands.clone();
        let f = lua
            .create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                cmds.lock().unwrap().push(GameCommand::Stop {
                    unit_ids: unit_ids.into_iter().map(EntityId).collect(),
                });
                Ok(())
            })
            .map_err(LuaScriptError::Lua)?;
        ctx.set("stop", f).map_err(LuaScriptError::Lua)?;
    }

    // ctx:hold(unit_ids)
    {
        let cmds = commands.clone();
        let f = lua
            .create_function(move |_, (_self, unit_ids): (LuaValue, Vec<u64>)| {
                cmds.lock().unwrap().push(GameCommand::HoldPosition {
                    unit_ids: unit_ids.into_iter().map(EntityId).collect(),
                });
                Ok(())
            })
            .map_err(LuaScriptError::Lua)?;
        ctx.set("hold", f).map_err(LuaScriptError::Lua)?;
    }

    lua.globals()
        .set("ctx", ctx)
        .map_err(LuaScriptError::Lua)?;

    // Remove os/debug libraries before sandboxing
    lua.globals()
        .set("os", LuaValue::Nil)
        .map_err(LuaScriptError::Lua)?;
    lua.globals()
        .set("debug", LuaValue::Nil)
        .map_err(LuaScriptError::Lua)?;

    lua.sandbox(true).map_err(LuaScriptError::Lua)?;

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

    lua.load(source)
        .exec()
        .map_err(LuaScriptError::Lua)?;

    let result = std::sync::Arc::try_unwrap(commands)
        .unwrap_or_else(|arc| arc.lock().unwrap().clone().into())
        .into_inner()
        .unwrap();

    Ok(result)
}

// ---------------------------------------------------------------------------
// Helpers: Rust → Lua conversion
// ---------------------------------------------------------------------------

fn unit_to_lua_table(lua: &Lua, unit: &UnitSnapshot) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    tbl.set("id", unit.id.0)?;
    tbl.set("kind", unit_kind_name(unit.kind))?;
    tbl.set("x", unit.pos.x)?;
    tbl.set("y", unit.pos.y)?;
    tbl.set("hp", fixed_to_f64(unit.health_current))?;
    tbl.set("hp_max", fixed_to_f64(unit.health_max))?;
    tbl.set("speed", fixed_to_f64(unit.speed))?;
    tbl.set("atk_dmg", fixed_to_f64(unit.attack_damage))?;
    tbl.set("atk_range", fixed_to_f64(unit.attack_range))?;
    tbl.set("atk_speed", unit.attack_speed)?;
    tbl.set(
        "atk_type",
        match unit.attack_type {
            AttackType::Melee => "Melee",
            AttackType::Ranged => "Ranged",
        },
    )?;
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
    tbl.set("kind", building_kind_name(building.kind))?;
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
    tbl.set("kind", resource_type_name(deposit.resource_type))?;
    tbl.set("x", deposit.pos.x)?;
    tbl.set("y", deposit.pos.y)?;
    tbl.set("remaining", deposit.remaining)?;
    Ok(tbl)
}

fn fixed_to_f64(v: Fixed) -> f64 {
    v.to_num::<f64>()
}

// ---------------------------------------------------------------------------
// Helpers: String ↔ enum conversion
// ---------------------------------------------------------------------------

fn unit_kind_name(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Pawdler => "Pawdler",
        UnitKind::Nuisance => "Nuisance",
        UnitKind::Chonk => "Chonk",
        UnitKind::FlyingFox => "FlyingFox",
        UnitKind::Hisser => "Hisser",
        UnitKind::Yowler => "Yowler",
        UnitKind::Mouser => "Mouser",
        UnitKind::Catnapper => "Catnapper",
        UnitKind::FerretSapper => "FerretSapper",
        UnitKind::MechCommander => "MechCommander",
    }
}

fn parse_unit_kind(s: &str) -> Option<UnitKind> {
    match s {
        "Pawdler" => Some(UnitKind::Pawdler),
        "Nuisance" => Some(UnitKind::Nuisance),
        "Chonk" => Some(UnitKind::Chonk),
        "FlyingFox" => Some(UnitKind::FlyingFox),
        "Hisser" => Some(UnitKind::Hisser),
        "Yowler" => Some(UnitKind::Yowler),
        "Mouser" => Some(UnitKind::Mouser),
        "Catnapper" => Some(UnitKind::Catnapper),
        "FerretSapper" => Some(UnitKind::FerretSapper),
        "MechCommander" => Some(UnitKind::MechCommander),
        _ => None,
    }
}

fn building_kind_name(kind: BuildingKind) -> &'static str {
    match kind {
        BuildingKind::TheBox => "TheBox",
        BuildingKind::CatTree => "CatTree",
        BuildingKind::FishMarket => "FishMarket",
        BuildingKind::LitterBox => "LitterBox",
    }
}

fn parse_building_kind(s: &str) -> Option<BuildingKind> {
    match s {
        "TheBox" => Some(BuildingKind::TheBox),
        "CatTree" => Some(BuildingKind::CatTree),
        "FishMarket" => Some(BuildingKind::FishMarket),
        "LitterBox" => Some(BuildingKind::LitterBox),
        _ => None,
    }
}

fn terrain_type_name(t: cc_core::terrain::TerrainType) -> &'static str {
    use cc_core::terrain::TerrainType;
    match t {
        TerrainType::Grass => "Grass",
        TerrainType::Dirt => "Dirt",
        TerrainType::Sand => "Sand",
        TerrainType::Forest => "Forest",
        TerrainType::Water => "Water",
        TerrainType::Shallows => "Shallows",
        TerrainType::Rock => "Rock",
        TerrainType::Ramp => "Ramp",
        TerrainType::Road => "Road",
        TerrainType::TechRuins => "TechRuins",
    }
}

fn resource_type_name(r: ResourceType) -> &'static str {
    match r {
        ResourceType::Food => "Food",
        ResourceType::GpuCores => "GpuCores",
        ResourceType::Nft => "Nft",
    }
}

fn parse_resource_type(s: &str) -> Option<ResourceType> {
    match s {
        "Food" => Some(ResourceType::Food),
        "GpuCores" => Some(ResourceType::GpuCores),
        "Nft" => Some(ResourceType::Nft),
        _ => None,
    }
}

fn cover_level_name(c: CoverLevel) -> &'static str {
    match c {
        CoverLevel::None => "None",
        CoverLevel::Light => "Light",
        CoverLevel::Heavy => "Heavy",
    }
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
    use cc_core::components::AttackType;
    use cc_core::coords::WorldPos;
    use cc_core::map::GameMap;
    use cc_core::math::fixed_from_i32;
    use cc_core::terrain::FactionId;
    use cc_sim::resources::PlayerResourceState;

    use crate::snapshot::GameStateSnapshot;

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

    fn make_unit(id: u64, kind: UnitKind, x: i32, y: i32, owner: u8) -> crate::snapshot::UnitSnapshot {
        crate::snapshot::UnitSnapshot {
            id: EntityId(id),
            kind,
            pos: GridPos::new(x, y),
            world_pos: WorldPos::from_grid(GridPos::new(x, y)),
            owner,
            health_current: fixed_from_i32(100),
            health_max: fixed_from_i32(100),
            speed: fixed_from_i32(1),
            attack_damage: fixed_from_i32(10),
            attack_range: fixed_from_i32(5),
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            is_moving: false,
            is_attacking: false,
            is_idle: true,
            is_dead: false,
        }
    }

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
