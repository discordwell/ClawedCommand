use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{BuildingKind, UnitKind};
use cc_core::coords::GridPos;
use serde_json::Value;

use crate::snapshot::GameStateSnapshot;

/// MCP tool definitions for the AI agent — maps to ARCHITECTURE.md's 13 tools.
pub fn tool_definitions() -> Vec<super::llm_client::ToolDef> {
    vec![
        tool_def("get_units", "Get all units owned by this player, with positions, health, type, and status", serde_json::json!({
            "type": "object",
            "properties": {
                "kind": {"type": "string", "description": "Optional unit type filter (e.g. 'Hisser', 'Chonk')"}
            }
        })),
        tool_def("get_buildings", "Get all buildings owned by this player", serde_json::json!({"type": "object", "properties": {}})),
        tool_def("get_visible_enemies", "Get all visible enemy units with positions, health, and type", serde_json::json!({"type": "object", "properties": {}})),
        tool_def("get_resources", "Get current resource amounts (food, GPU cores, NFTs, supply)", serde_json::json!({"type": "object", "properties": {}})),
        tool_def("get_map_info", "Get map dimensions, tick, and resource deposit locations", serde_json::json!({"type": "object", "properties": {}})),
        tool_def("move_units", "Move units to a position", serde_json::json!({
            "type": "object",
            "properties": {
                "unit_ids": {"type": "array", "items": {"type": "integer"}},
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["unit_ids", "x", "y"]
        })),
        tool_def("attack_units", "Attack a target unit", serde_json::json!({
            "type": "object",
            "properties": {
                "unit_ids": {"type": "array", "items": {"type": "integer"}},
                "target_id": {"type": "integer"}
            },
            "required": ["unit_ids", "target_id"]
        })),
        tool_def("build", "Build a structure", serde_json::json!({
            "type": "object",
            "properties": {
                "builder_id": {"type": "integer"},
                "building_type": {"type": "string"},
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["builder_id", "building_type", "x", "y"]
        })),
        tool_def("train_unit", "Train a unit from a building", serde_json::json!({
            "type": "object",
            "properties": {
                "building_id": {"type": "integer"},
                "unit_type": {"type": "string"}
            },
            "required": ["building_id", "unit_type"]
        })),
        tool_def("set_rally_point", "Set rally point for a building", serde_json::json!({
            "type": "object",
            "properties": {
                "building_id": {"type": "integer"},
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["building_id", "x", "y"]
        })),
        tool_def("patrol", "Patrol between two points", serde_json::json!({
            "type": "object",
            "properties": {
                "unit_ids": {"type": "array", "items": {"type": "integer"}},
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["unit_ids", "x", "y"]
        })),
        tool_def("gather_resource", "Send workers to gather from a deposit", serde_json::json!({
            "type": "object",
            "properties": {
                "unit_ids": {"type": "array", "items": {"type": "integer"}},
                "deposit_id": {"type": "integer"}
            },
            "required": ["unit_ids", "deposit_id"]
        })),
        tool_def("execute_strategy", "Execute a named strategy script", serde_json::json!({
            "type": "object",
            "properties": {
                "strategy": {"type": "string"}
            },
            "required": ["strategy"]
        })),
    ]
}

fn tool_def(name: &str, description: &str, parameters: Value) -> super::llm_client::ToolDef {
    super::llm_client::ToolDef {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
    }
}

/// Execute a tool call, returning JSON result and any game commands to enqueue.
/// Read tools use the snapshot; write tools produce GameCommands.
pub fn execute_tool(
    name: &str,
    args: &Value,
    _player_id: u8,
    snapshot: Option<&GameStateSnapshot>,
) -> (Value, Vec<GameCommand>) {
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
                .and_then(parse_unit_kind_str);
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
                    "kind": format!("{:?}", b.kind),
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
                    "kind": format!("{:?}", d.resource_type),
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
        // Write tools — produce GameCommands
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
            let Some(kind) = parse_building_kind_str(building_type) else {
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
            let Some(kind) = parse_unit_kind_str(unit_type) else {
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
        "execute_strategy" => {
            let strategy = args["strategy"].as_str().unwrap_or("");
            (serde_json::json!({"status": "strategy_lookup_required", "strategy": strategy}), vec![])
        }
        _ => (serde_json::json!({"error": "unknown tool"}), vec![]),
    }
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
        "kind": format!("{:?}", unit.kind),
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

fn parse_building_kind_str(s: &str) -> Option<BuildingKind> {
    match s {
        "TheBox" => Some(BuildingKind::TheBox),
        "CatTree" => Some(BuildingKind::CatTree),
        "FishMarket" => Some(BuildingKind::FishMarket),
        "LitterBox" => Some(BuildingKind::LitterBox),
        _ => None,
    }
}

fn parse_unit_kind_str(s: &str) -> Option<UnitKind> {
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
