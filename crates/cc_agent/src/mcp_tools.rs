use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{BuildingKind, UnitKind};
use cc_core::coords::GridPos;
use cc_core::map::GameMap;
use cc_core::math::Fixed;
use cc_core::terrain::FactionId;
use serde_json::Value;

use crate::behaviors;
use crate::script_context::ScriptContext;
use crate::snapshot::GameStateSnapshot;
use crate::tool_tier::{ToolRegistry, ToolTier};

/// Return tier-filtered MCP tool definitions for the AI agent.
pub fn tool_definitions(tier: ToolTier) -> Vec<super::llm_client::ToolDef> {
    ToolRegistry::build_default().tool_definitions_for_tier(tier)
}

/// Execute a tool call, returning JSON result and any game commands to enqueue.
/// Rejects tools above the caller's tier. Read tools use the snapshot;
/// write/behavior tools produce GameCommands.
pub fn execute_tool(
    name: &str,
    args: &Value,
    player_id: u8,
    snapshot: Option<&GameStateSnapshot>,
    tier: ToolTier,
) -> (Value, Vec<GameCommand>) {
    // Tier gate: reject tools the caller hasn't unlocked
    let registry = ToolRegistry::build_default();
    if !registry.is_available(name, tier) {
        return (
            serde_json::json!({"error": format!("tool '{}' requires higher tier", name)}),
            vec![],
        );
    }

    match name {
        // -----------------------------------------------------------------
        // Read tools — return data from snapshot
        // -----------------------------------------------------------------
        "get_units" => {
            let Some(snap) = snapshot else {
                return (serde_json::json!({"error": "no game state available"}), vec![]);
            };
            let kind_filter = args.get("kind")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<UnitKind>().ok());
            let units: Vec<Value> = snap.my_units.iter()
                .filter(|u| !u.is_dead)
                .filter(|u| kind_filter.is_none_or(|k| u.kind == k))
                .map(unit_to_json)
                .collect();
            let count = units.len();
            (serde_json::json!({"units": units, "count": count}), vec![])
        }
        "get_buildings" => {
            let Some(snap) = snapshot else {
                return (serde_json::json!({"error": "no game state available"}), vec![]);
            };
            let buildings: Vec<Value> = snap.my_buildings.iter()
                .map(|b| serde_json::json!({
                    "id": b.id.0,
                    "kind": b.kind.to_string(),
                    "x": b.pos.x, "y": b.pos.y,
                    "hp": b.health_current.to_num::<f64>(),
                    "hp_max": b.health_max.to_num::<f64>(),
                    "under_construction": b.under_construction,
                }))
                .collect();
            let count = buildings.len();
            (serde_json::json!({"buildings": buildings, "count": count}), vec![])
        }
        "get_visible_enemies" => {
            let Some(snap) = snapshot else {
                return (serde_json::json!({"error": "no game state available"}), vec![]);
            };
            let enemies: Vec<Value> = snap.enemy_units.iter()
                .filter(|u| !u.is_dead)
                .map(unit_to_json)
                .collect();
            let count = enemies.len();
            (serde_json::json!({"enemies": enemies, "count": count}), vec![])
        }
        "get_resources" => {
            let Some(snap) = snapshot else {
                return (serde_json::json!({"error": "no game state available"}), vec![]);
            };
            (serde_json::json!({
                "food": snap.my_resources.food,
                "gpu_cores": snap.my_resources.gpu_cores,
                "nfts": snap.my_resources.nfts,
                "supply": snap.my_resources.supply,
                "supply_cap": snap.my_resources.supply_cap,
            }), vec![])
        }
        "get_map_info" => {
            let Some(snap) = snapshot else {
                return (serde_json::json!({"error": "no game state available"}), vec![]);
            };
            let deposits: Vec<Value> = snap.resource_deposits.iter()
                .filter(|d| d.remaining > 0)
                .map(|d| serde_json::json!({
                    "id": d.id.0,
                    "kind": d.resource_type.to_string(),
                    "x": d.pos.x, "y": d.pos.y,
                    "remaining": d.remaining,
                }))
                .collect();
            (serde_json::json!({
                "width": snap.map_width,
                "height": snap.map_height,
                "tick": snap.tick,
                "resource_deposits": deposits,
            }), vec![])
        }

        // -----------------------------------------------------------------
        // Command tools — produce GameCommands
        // -----------------------------------------------------------------
        "move_units" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            let Some(x) = args["x"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: x"}), vec![]);
            };
            let Some(y) = args["y"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: y"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::Move { unit_ids, target: GridPos::new(x as i32, y as i32) }])
        }
        "attack_units" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            let Some(target_id) = args["target_id"].as_u64() else {
                return (serde_json::json!({"error": "missing required field: target_id"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::Attack { unit_ids, target: EntityId(target_id) }])
        }
        "build" => {
            let Some(builder_id) = args["builder_id"].as_u64() else {
                return (serde_json::json!({"error": "missing required field: builder_id"}), vec![]);
            };
            let Some(building_type) = args["building_type"].as_str() else {
                return (serde_json::json!({"error": "missing required field: building_type"}), vec![]);
            };
            let Some(kind) = building_type.parse::<BuildingKind>().ok() else {
                return (serde_json::json!({"error": format!("unknown building type: {building_type}")}), vec![]);
            };
            let Some(x) = args["x"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: x"}), vec![]);
            };
            let Some(y) = args["y"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: y"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::Build {
                builder: EntityId(builder_id),
                building_kind: kind,
                position: GridPos::new(x as i32, y as i32),
            }])
        }
        "train_unit" => {
            let Some(building_id) = args["building_id"].as_u64() else {
                return (serde_json::json!({"error": "missing required field: building_id"}), vec![]);
            };
            let Some(unit_type) = args["unit_type"].as_str() else {
                return (serde_json::json!({"error": "missing required field: unit_type"}), vec![]);
            };
            let Some(kind) = unit_type.parse::<UnitKind>().ok() else {
                return (serde_json::json!({"error": format!("unknown unit type: {unit_type}")}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::TrainUnit {
                building: EntityId(building_id),
                unit_kind: kind,
            }])
        }
        "gather_resource" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            let Some(deposit_id) = args["deposit_id"].as_u64() else {
                return (serde_json::json!({"error": "missing required field: deposit_id"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::GatherResource { unit_ids, deposit: EntityId(deposit_id) }])
        }
        "patrol" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            let Some(x) = args["x"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: x"}), vec![]);
            };
            let Some(y) = args["y"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: y"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::AttackMove { unit_ids, target: GridPos::new(x as i32, y as i32) }])
        }
        "set_rally_point" => {
            let Some(building_id) = args["building_id"].as_u64() else {
                return (serde_json::json!({"error": "missing required field: building_id"}), vec![]);
            };
            let Some(x) = args["x"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: x"}), vec![]);
            };
            let Some(y) = args["y"].as_i64() else {
                return (serde_json::json!({"error": "missing required field: y"}), vec![]);
            };
            (serde_json::json!({"status": "ok"}), vec![GameCommand::SetRallyPoint { building: EntityId(building_id), target: GridPos::new(x as i32, y as i32) }])
        }
        "stop" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            (serde_json::json!({"status": "ok"}), vec![GameCommand::Stop { unit_ids }])
        }
        "hold_position" => {
            let unit_ids = parse_entity_ids(args, "unit_ids");
            (serde_json::json!({"status": "ok"}), vec![GameCommand::HoldPosition { unit_ids }])
        }
        // -----------------------------------------------------------------
        // Behavior tools — require snapshot + ScriptContext
        // -----------------------------------------------------------------
        "focus_fire" | "focus_weakest" | "kite_squad" | "retreat_wounded"
        | "defend_area" | "scout_pattern" | "harass_economy" | "auto_produce"
        | "assign_idle_workers" | "attack_move_group" | "use_ability"
        | "split_squads" | "protect_unit" | "surround_target"
        | "balanced_production" | "expand_economy" | "coordinate_assault"
        | "research_priority" | "adaptive_defense" => {
            execute_behavior(name, args, player_id, snapshot)
        }

        _ => (serde_json::json!({"error": "unknown tool"}), vec![]),
    }
}

/// Execute a behavior tool using ScriptContext, returning results + commands.
fn execute_behavior(
    name: &str,
    args: &Value,
    player_id: u8,
    snapshot: Option<&GameStateSnapshot>,
) -> (Value, Vec<GameCommand>) {
    let Some(snap) = snapshot else {
        return (serde_json::json!({"error": "no game state available"}), vec![]);
    };
    let map = GameMap::new(snap.map_width, snap.map_height);
    let faction = FactionId::for_player(player_id);
    let mut ctx = ScriptContext::new(snap, &map, player_id, faction);

    let result = match name {
        "focus_fire" => {
            let ids = parse_entity_ids(args, "attacker_ids");
            let target = args["target_id"].as_u64().unwrap_or(0);
            behaviors::focus_fire(&mut ctx, &ids, EntityId(target))
        }
        "focus_weakest" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let range = args["range"].as_f64().unwrap_or(10.0);
            behaviors::focus_weakest(&mut ctx, &ids, Fixed::from_num(range))
        }
        "kite_squad" => {
            let ids = parse_entity_ids(args, "unit_ids");
            behaviors::kite_squad(&mut ctx, &ids)
        }
        "retreat_wounded" => {
            let threshold = args["threshold"].as_f64().unwrap_or(0.3);
            behaviors::retreat_wounded(&mut ctx, threshold)
        }
        "defend_area" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let x = args["x"].as_i64().unwrap_or(0) as i32;
            let y = args["y"].as_i64().unwrap_or(0) as i32;
            let radius = args["radius"].as_f64().unwrap_or(5.0);
            behaviors::defend_area(&mut ctx, &ids, GridPos::new(x, y), Fixed::from_num(radius))
        }
        "scout_pattern" => {
            let scout_id = args["scout_id"].as_u64().unwrap_or(0);
            let waypoints: Vec<GridPos> = args["waypoints"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|wp| {
                            let x = wp["x"].as_i64()? as i32;
                            let y = wp["y"].as_i64()? as i32;
                            Some(GridPos::new(x, y))
                        })
                        .collect()
                })
                .unwrap_or_default();
            behaviors::scout_pattern(&mut ctx, EntityId(scout_id), &waypoints)
        }
        "harass_economy" => {
            let ids = parse_entity_ids(args, "raider_ids");
            behaviors::harass_economy(&mut ctx, &ids)
        }
        "auto_produce" => {
            let building_id = args["building_id"].as_u64().unwrap_or(0);
            let unit_type = args["unit_type"].as_str().unwrap_or("Pawdler");
            let kind = unit_type.parse::<UnitKind>().unwrap_or(UnitKind::Pawdler);
            behaviors::auto_produce(&mut ctx, EntityId(building_id), kind)
        }
        "assign_idle_workers" => {
            behaviors::assign_idle_workers(&mut ctx)
        }
        "attack_move_group" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let x = args["x"].as_i64().unwrap_or(0) as i32;
            let y = args["y"].as_i64().unwrap_or(0) as i32;
            behaviors::attack_move_group(&mut ctx, &ids, GridPos::new(x, y))
        }
        "use_ability" => {
            let unit_id = args["unit_id"].as_u64().unwrap_or(0);
            let slot = args["slot"].as_u64().unwrap_or(0) as u8;
            let target_type = args["target_type"].as_str().unwrap_or("self");
            let target = match target_type {
                "position" => {
                    let x = args["x"].as_i64().unwrap_or(0) as i32;
                    let y = args["y"].as_i64().unwrap_or(0) as i32;
                    AbilityTarget::Position(GridPos::new(x, y))
                }
                "entity" => {
                    let tid = args["target_id"].as_u64().unwrap_or(0);
                    AbilityTarget::Entity(EntityId(tid))
                }
                _ => AbilityTarget::SelfCast,
            };
            behaviors::use_ability(&mut ctx, EntityId(unit_id), slot, target)
        }
        "split_squads" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let (melee, ranged, support, result) = behaviors::split_squads(&mut ctx, &ids);
            let cmds = ctx.take_commands();
            return (serde_json::json!({
                "commands_issued": result.commands_issued,
                "description": result.description,
                "melee_ids": melee.iter().map(|e| e.0).collect::<Vec<_>>(),
                "ranged_ids": ranged.iter().map(|e| e.0).collect::<Vec<_>>(),
                "support_ids": support.iter().map(|e| e.0).collect::<Vec<_>>(),
            }), cmds);
        }
        "protect_unit" => {
            let escort_ids = parse_entity_ids(args, "escort_ids");
            let vip_id = args["vip_id"].as_u64().unwrap_or(0);
            let guard_radius = args["guard_radius"].as_f64().unwrap_or(5.0);
            behaviors::protect_unit(&mut ctx, &escort_ids, EntityId(vip_id), Fixed::from_num(guard_radius))
        }
        "surround_target" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let target_id = args["target_id"].as_u64().unwrap_or(0);
            let ring_radius = args["ring_radius"].as_f64().unwrap_or(3.0);
            behaviors::surround_target(&mut ctx, &ids, EntityId(target_id), Fixed::from_num(ring_radius))
        }
        "balanced_production" => {
            let building_id = args["building_id"].as_u64().unwrap_or(0);
            behaviors::balanced_production(&mut ctx, EntityId(building_id))
        }
        "expand_economy" => {
            let builder_id = args["builder_id"].as_u64().unwrap_or(0);
            behaviors::expand_economy(&mut ctx, EntityId(builder_id))
        }
        "coordinate_assault" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let x = args["target_x"].as_i64().unwrap_or(0) as i32;
            let y = args["target_y"].as_i64().unwrap_or(0) as i32;
            behaviors::coordinate_assault(&mut ctx, &ids, GridPos::new(x, y))
        }
        "research_priority" => {
            let building_id = args["building_id"].as_u64().unwrap_or(0);
            behaviors::research_priority(&mut ctx, EntityId(building_id))
        }
        "adaptive_defense" => {
            let ids = parse_entity_ids(args, "unit_ids");
            let cx = args["center_x"].as_i64().unwrap_or(0) as i32;
            let cy = args["center_y"].as_i64().unwrap_or(0) as i32;
            let radius = args["radius"].as_f64().unwrap_or(5.0);
            behaviors::adaptive_defense(&mut ctx, &ids, GridPos::new(cx, cy), Fixed::from_num(radius))
        }
        _ => unreachable!(),
    };

    let cmds = ctx.take_commands();
    (serde_json::json!({
        "commands_issued": result.commands_issued,
        "description": result.description,
    }), cmds)
}

fn parse_entity_ids(args: &Value, key: &str) -> Vec<EntityId> {
    args[key]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64())
                .map(EntityId)
                .collect()
        })
        .unwrap_or_default()
}

fn unit_to_json(unit: &crate::snapshot::UnitSnapshot) -> Value {
    serde_json::json!({
        "id": unit.id.0,
        "kind": unit.kind.to_string(),
        "x": unit.pos.x,
        "y": unit.pos.y,
        "hp": unit.health_current.to_num::<f64>(),
        "hp_max": unit.health_max.to_num::<f64>(),
        "speed": unit.speed.to_num::<f64>(),
        "atk_dmg": unit.attack_damage.to_num::<f64>(),
        "atk_range": unit.attack_range.to_num::<f64>(),
        "atk_speed": unit.attack_speed,
        "moving": unit.is_moving,
        "attacking": unit.is_attacking,
        "idle": unit.is_idle,
    })
}
