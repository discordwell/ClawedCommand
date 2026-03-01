//! MCP server exposing headless simulation control via tools.

use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cc_core::commands::{AbilityTarget, EntityId, GameCommand};
use cc_core::components::{BuildingKind, ResourceType, UnitKind, UpgradeType};
use cc_core::coords::GridPos;
use cc_core::math::Fixed;

use cc_agent::behaviors;
use cc_agent::script_context::ScriptContext;

use crate::headless::HeadlessSim;

type McpError = rmcp::ErrorData;

/// The MCP server wrapping a HeadlessSim.
#[derive(Clone)]
pub struct HarnessServer {
    sim: Arc<Mutex<HeadlessSim>>,
    tool_router: ToolRouter<Self>,
}

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, JsonSchema)]
struct UnitIdsPos {
    unit_ids: Vec<u64>,
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct UnitIdsTarget {
    unit_ids: Vec<u64>,
    target_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct UnitIds {
    unit_ids: Vec<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PosRange {
    player_id: u8,
    x: i32,
    y: i32,
    range: f64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PosOnly {
    player_id: u8,
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PlayerOnly {
    player_id: u8,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PlayerFilterParams {
    player_id: u8,
    /// Optional filter: "idle", "wounded", "attacking", "gathering". Omit for all units.
    filter: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct GetSafePositionsParams {
    player_id: u8,
    unit_id: u64,
    /// Search radius in tiles (default 8).
    search_radius: Option<u32>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct GetKitePositionParams {
    player_id: u8,
    unit_id: u64,
    target_id: u64,
    desired_range: u32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct PathQueryParams {
    player_id: u8,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SpawnUnitParams {
    kind: String,
    x: i32,
    y: i32,
    player_id: u8,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SpawnBuildingParams {
    kind: String,
    x: i32,
    y: i32,
    player_id: u8,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SpawnDepositParams {
    resource_type: String,
    x: i32,
    y: i32,
    amount: u32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AdvanceParams {
    ticks: u32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ResetParams {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct GatherParams {
    unit_ids: Vec<u64>,
    deposit_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct BuildParams {
    builder_id: u64,
    building_kind: String,
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct TrainParams {
    building_id: u64,
    unit_kind: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AbilityParams {
    unit_id: u64,
    slot: u8,
    target_type: String,
    x: Option<i32>,
    y: Option<i32>,
    target_entity_id: Option<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ResearchParams {
    building_id: u64,
    upgrade: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct FocusFireParams {
    player_id: u8,
    attacker_ids: Vec<u64>,
    target_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct KiteSquadParams {
    player_id: u8,
    unit_ids: Vec<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct RetreatParams {
    player_id: u8,
    threshold: f64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct DefendAreaParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    x: i32,
    y: i32,
    radius: f64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct HarassParams {
    player_id: u8,
    raider_ids: Vec<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct FocusWeakestParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    range: f64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AssignIdleWorkersParams {
    player_id: u8,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AttackMoveGroupParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct UseAbilityParams {
    player_id: u8,
    unit_id: u64,
    slot: u8,
    target_type: String,
    x: Option<i32>,
    y: Option<i32>,
    target_entity_id: Option<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SplitSquadsParams {
    player_id: u8,
    unit_ids: Vec<u64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ProtectUnitParams {
    player_id: u8,
    escort_ids: Vec<u64>,
    vip_id: u64,
    guard_radius: Option<f64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct SurroundTargetParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    target_id: u64,
    ring_radius: Option<f64>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AutoProduceParams {
    player_id: u8,
    building_id: u64,
    unit_kind: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct BalancedProductionParams {
    player_id: u8,
    building_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ExpandEconomyParams {
    player_id: u8,
    builder_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct CoordinateAssaultParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    target_x: i32,
    target_y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ResearchPriorityParams {
    player_id: u8,
    building_id: u64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct AdaptiveDefenseParams {
    player_id: u8,
    unit_ids: Vec<u64>,
    center_x: i32,
    center_y: i32,
    radius: f64,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct ScoutPatternParams {
    player_id: u8,
    scout_id: u64,
    waypoints: Vec<WaypointParam>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct WaypointParam {
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct LuaScriptParams {
    player_id: u8,
    source: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct RegisterScriptParams {
    name: String,
    player_id: u8,
    source: String,
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl HarnessServer {
    pub fn new(sim: HeadlessSim) -> Self {
        Self {
            sim: Arc::new(Mutex::new(sim)),
            tool_router: Self::tool_router(),
        }
    }

    // =======================================================================
    // Query tools (11)
    // =======================================================================

    #[tool(description = "Get own units for a player. Optional filter: 'idle' (idle units), 'wounded' (below 50% HP), 'attacking' (currently attacking), 'gathering' (gathering workers). Omit filter for all units. Returns array of unit objects with id, kind, pos, hp, state.")]
    async fn get_units(
        &self,
        Parameters(params): Parameters<PlayerFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let units: Vec<_> = snap.my_units.iter().filter(|u| {
            match params.filter.as_deref() {
                Some("idle") => u.is_idle,
                Some("wounded") => {
                    let half_hp = u.health_max / Fixed::from_num(2);
                    u.health_current < half_hp
                }
                Some("attacking") => u.is_attacking,
                Some("gathering") => u.is_gathering,
                _ => true,
            }
        }).collect();
        let json = serde_json::to_string_pretty(&units.iter().map(|u| {
            serde_json::json!({
                "id": u.id.0, "kind": format!("{:?}", u.kind),
                "x": u.pos.x, "y": u.pos.y,
                "hp": u.health_current.to_num::<f64>(),
                "hp_max": u.health_max.to_num::<f64>(),
                "moving": u.is_moving, "attacking": u.is_attacking,
                "idle": u.is_idle, "gathering": u.is_gathering,
            })
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get all visible enemy units for a player.")]
    async fn get_enemies(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = serde_json::to_string_pretty(&snap.enemy_units.iter().map(|u| {
            serde_json::json!({
                "id": u.id.0, "kind": format!("{:?}", u.kind),
                "x": u.pos.x, "y": u.pos.y,
                "hp": u.health_current.to_num::<f64>(),
                "hp_max": u.health_max.to_num::<f64>(),
            })
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get enemies within a range of a position.")]
    async fn get_enemies_in_range(
        &self,
        Parameters(params): Parameters<PosRange>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let enemies = ctx.enemies_in_range(GridPos::new(params.x, params.y), Fixed::from_num(params.range));
        let json = serde_json::to_string_pretty(&enemies.iter().map(|u| {
            serde_json::json!({"id": u.id.0, "kind": format!("{:?}", u.kind), "hp": u.health_current.to_num::<f64>()})
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get the nearest enemy to a position.")]
    async fn get_nearest_enemy(
        &self,
        Parameters(params): Parameters<PosOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let result = match ctx.nearest_enemy(GridPos::new(params.x, params.y)) {
            Some(u) => serde_json::json!({"id": u.id.0, "kind": format!("{:?}", u.kind), "x": u.pos.x, "y": u.pos.y, "hp": u.health_current.to_num::<f64>()}),
            None => serde_json::json!(null),
        };
        Ok(CallToolResult::success(vec![Content::text(result.to_string())]))
    }

    #[tool(description = "Get enemies that threaten a specific unit (within their attack range).")]
    async fn get_threats(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let mut all_threats = Vec::new();
        for unit in &snap.my_units {
            let threats = ctx.threats_to(unit);
            for t in threats {
                all_threats.push(serde_json::json!({"threatened_unit": unit.id.0, "threat_id": t.id.0, "threat_kind": format!("{:?}", t.kind)}));
            }
        }
        let json = serde_json::to_string_pretty(&all_threats).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get enemies within attack range of a specific unit.")]
    async fn get_targets(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let mut all_targets = Vec::new();
        for unit in &snap.my_units {
            let targets = ctx.targets_for(unit);
            for t in targets {
                all_targets.push(serde_json::json!({"unit": unit.id.0, "target_id": t.id.0, "target_kind": format!("{:?}", t.kind)}));
            }
        }
        let json = serde_json::to_string_pretty(&all_targets).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get current resource levels for a player (food, gpu_cores, nfts, supply).")]
    async fn get_resources(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let sim = self.sim.lock().await;
        let res = sim.player_resources(params.player_id);
        let json = serde_json::json!({
            "food": res.food, "gpu_cores": res.gpu_cores, "nfts": res.nfts,
            "supply": res.supply, "supply_cap": res.supply_cap,
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Get terrain type, elevation, and cover at a position.")]
    async fn get_terrain_at(
        &self,
        Parameters(params): Parameters<PosOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let pos = GridPos::new(params.x, params.y);
        let json = serde_json::json!({
            "terrain": ctx.terrain_at(pos).map(|t| t.to_string()),
            "elevation": ctx.elevation_at(pos),
            "cover": ctx.cover_at(pos).to_string(),
            "passable": ctx.is_passable(pos),
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Get own buildings for a player.")]
    async fn get_buildings(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = serde_json::to_string_pretty(&snap.my_buildings.iter().map(|b| {
            serde_json::json!({
                "id": b.id.0, "kind": format!("{:?}", b.kind),
                "x": b.pos.x, "y": b.pos.y,
                "hp": b.health_current.to_num::<f64>(),
                "under_construction": b.under_construction,
                "queue": b.production_queue.iter().map(|k| format!("{k:?}")).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get visible enemy buildings for a player.")]
    async fn get_enemy_buildings(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = serde_json::to_string_pretty(&snap.enemy_buildings.iter().map(|b| {
            serde_json::json!({
                "id": b.id.0, "kind": format!("{:?}", b.kind),
                "x": b.pos.x, "y": b.pos.y,
                "hp": b.health_current.to_num::<f64>(),
            })
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get map dimensions, tick count, and game state.")]
    async fn get_map_info(&self) -> Result<CallToolResult, McpError> {
        let sim = self.sim.lock().await;
        let (w, h) = sim.map_size();
        let json = serde_json::json!({
            "width": w, "height": h,
            "tick": sim.tick(),
            "game_state": format!("{:?}", sim.game_state()),
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    // =======================================================================
    // Spatial / Pathfinding query tools
    // =======================================================================

    #[tool(description = "Get safe positions for a unit — passable tiles within search_radius that are outside all enemy attack ranges. Returns array of {x, y} positions.")]
    async fn get_safe_positions(
        &self,
        Parameters(params): Parameters<GetSafePositionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let unit = snap.my_units.iter().find(|u| u.id.0 == params.unit_id)
            .ok_or_else(|| McpError::invalid_params(format!("Unit {} not found", params.unit_id), None))?;
        let radius = params.search_radius.unwrap_or(8) as i32;
        let positions = ctx.safe_positions(unit, radius);
        let json = serde_json::to_string_pretty(&positions.iter().map(|p| {
            serde_json::json!({"x": p.x, "y": p.y})
        }).collect::<Vec<_>>()).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get the optimal kite position: a passable tile at exactly desired_range from target, closest to the unit. Returns {x, y} or null if none found.")]
    async fn get_kite_position(
        &self,
        Parameters(params): Parameters<GetKitePositionParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let unit = snap.my_units.iter().find(|u| u.id.0 == params.unit_id)
            .ok_or_else(|| McpError::invalid_params(format!("Unit {} not found", params.unit_id), None))?;
        let target = snap.enemy_units.iter().find(|u| u.id.0 == params.target_id)
            .ok_or_else(|| McpError::invalid_params(format!("Target {} not found", params.target_id), None))?;
        let result = ctx.position_at_range(unit.pos, target.pos, params.desired_range as i32);
        let json = match result {
            Some(pos) => serde_json::json!({"x": pos.x, "y": pos.y}),
            None => serde_json::json!(null),
        };
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Check if a path exists between two grid positions using A* pathfinding. Returns boolean.")]
    async fn can_reach(
        &self,
        Parameters(params): Parameters<PathQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let reachable = ctx.can_reach(
            GridPos::new(params.from_x, params.from_y),
            GridPos::new(params.to_x, params.to_y),
        );
        Ok(CallToolResult::success(vec![Content::text(format!("{reachable}"))]))
    }

    #[tool(description = "Get the A* path length in tiles between two grid positions. Returns integer length or null if unreachable.")]
    async fn get_path_length(
        &self,
        Parameters(params): Parameters<PathQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let map = sim.map();
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
        let length = ctx.path_length(
            GridPos::new(params.from_x, params.from_y),
            GridPos::new(params.to_x, params.to_y),
        );
        let json = match length {
            Some(len) => serde_json::json!(len),
            None => serde_json::json!(null),
        };
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    // =======================================================================
    // Command tools (10)
    // =======================================================================

    #[tool(description = "Move units to a grid position.")]
    async fn move_units(
        &self,
        Parameters(params): Parameters<UnitIdsPos>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::Move {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
            target: GridPos::new(params.x, params.y),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Attack a specific enemy unit.")]
    async fn attack(
        &self,
        Parameters(params): Parameters<UnitIdsTarget>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::Attack {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
            target: EntityId(params.target_id),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Attack-move units to a position (engage enemies on the way).")]
    async fn attack_move(
        &self,
        Parameters(params): Parameters<UnitIdsPos>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::AttackMove {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
            target: GridPos::new(params.x, params.y),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Stop units immediately.")]
    async fn stop(
        &self,
        Parameters(params): Parameters<UnitIds>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::Stop {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Hold position: attack enemies in range only, no chasing.")]
    async fn hold(
        &self,
        Parameters(params): Parameters<UnitIds>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::HoldPosition {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Send worker units to gather from a resource deposit.")]
    async fn gather(
        &self,
        Parameters(params): Parameters<GatherParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::GatherResource {
            unit_ids: params.unit_ids.into_iter().map(EntityId).collect(),
            deposit: EntityId(params.deposit_id),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Place a building at a grid position.")]
    async fn build(
        &self,
        Parameters(params): Parameters<BuildParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = params.building_kind.parse::<BuildingKind>()
            .map_err(|_| McpError::invalid_params(format!("Unknown building kind: {}", params.building_kind), None))?;
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::Build {
            builder: EntityId(params.builder_id),
            building_kind: kind,
            position: GridPos::new(params.x, params.y),
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Train a unit from a production building.")]
    async fn train(
        &self,
        Parameters(params): Parameters<TrainParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = params.unit_kind.parse::<UnitKind>()
            .map_err(|_| McpError::invalid_params(format!("Unknown unit kind: {}", params.unit_kind), None))?;
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::TrainUnit {
            building: EntityId(params.building_id),
            unit_kind: kind,
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Activate a unit's ability by slot index. target_type: 'self', 'position', or 'entity'.")]
    async fn activate_ability(
        &self,
        Parameters(params): Parameters<AbilityParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = match params.target_type.as_str() {
            "self" => AbilityTarget::SelfCast,
            "position" => {
                let x = params.x.ok_or_else(|| McpError::invalid_params("position target requires x", None))?;
                let y = params.y.ok_or_else(|| McpError::invalid_params("position target requires y", None))?;
                AbilityTarget::Position(GridPos::new(x, y))
            }
            "entity" => {
                let eid = params.target_entity_id.ok_or_else(|| McpError::invalid_params("entity target requires target_entity_id", None))?;
                AbilityTarget::Entity(EntityId(eid))
            }
            _ => return Err(McpError::invalid_params(format!("Unknown target_type: {}", params.target_type), None)),
        };
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::ActivateAbility {
            unit_id: EntityId(params.unit_id),
            slot: params.slot,
            target,
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    #[tool(description = "Queue research at a building. Upgrades: SharperClaws, ThickerFur, NimblePaws, SiegeTraining, MechPrototype.")]
    async fn research(
        &self,
        Parameters(params): Parameters<ResearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let upgrade = params.upgrade.parse::<UpgradeType>()
            .map_err(|_| McpError::invalid_params(format!("Unknown upgrade: {}", params.upgrade), None))?;
        let mut sim = self.sim.lock().await;
        sim.inject_command(GameCommand::Research {
            building: EntityId(params.building_id),
            upgrade,
        });
        Ok(CallToolResult::success(vec![Content::text("OK")]))
    }

    // =======================================================================
    // Behavior tools (6)
    // =======================================================================

    #[tool(description = "Focus fire: all attackers attack the same target.")]
    async fn focus_fire(
        &self,
        Parameters(params): Parameters<FocusFireParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.attacker_ids.into_iter().map(EntityId).collect();
            let result = behaviors::focus_fire(&mut ctx, &ids, EntityId(params.target_id));
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Kite squad: ranged units maintain attack range from nearest enemy.")]
    async fn kite_squad(
        &self,
        Parameters(params): Parameters<KiteSquadParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::kite_squad(&mut ctx, &ids);
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Retreat wounded: move units below HP% threshold to safe positions.")]
    async fn retreat_wounded(
        &self,
        Parameters(params): Parameters<RetreatParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::retreat_wounded(&mut ctx, params.threshold);
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Defend area: attack enemies inside radius, hold position otherwise.")]
    async fn defend_area(
        &self,
        Parameters(params): Parameters<DefendAreaParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::defend_area(&mut ctx, &ids, GridPos::new(params.x, params.y), Fixed::from_num(params.radius));
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Harass economy: attack enemy workers, or attack-move toward enemy buildings if no workers visible.")]
    async fn harass_economy(
        &self,
        Parameters(params): Parameters<HarassParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.raider_ids.into_iter().map(EntityId).collect();
            let result = behaviors::harass_economy(&mut ctx, &ids);
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Focus weakest: find weakest enemy in range of any unit, then focus fire all on it.")]
    async fn focus_weakest(
        &self,
        Parameters(params): Parameters<FocusWeakestParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::focus_weakest(&mut ctx, &ids, Fixed::from_num(params.range));
            (result, ctx.take_commands())
        };
        for cmd in commands {
            sim.inject_command(cmd);
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    // =======================================================================
    // New behavior tools (10)
    // =======================================================================

    #[tool(description = "Send idle Pawdlers to nearest resource deposit.")]
    async fn assign_idle_workers(
        &self,
        Parameters(params): Parameters<AssignIdleWorkersParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::assign_idle_workers(&mut ctx);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Group attack-move with ranged positioned behind melee.")]
    async fn attack_move_group(
        &self,
        Parameters(params): Parameters<AttackMoveGroupParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::attack_move_group(&mut ctx, &ids, GridPos::new(params.x, params.y));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Smart ability activation (validates unit, issues command).")]
    async fn use_ability(
        &self,
        Parameters(params): Parameters<UseAbilityParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = match params.target_type.as_str() {
            "self" => AbilityTarget::SelfCast,
            "position" => {
                let x = params.x.ok_or_else(|| McpError::invalid_params("position target requires x", None))?;
                let y = params.y.ok_or_else(|| McpError::invalid_params("position target requires y", None))?;
                AbilityTarget::Position(GridPos::new(x, y))
            }
            "entity" => {
                let eid = params.target_entity_id.ok_or_else(|| McpError::invalid_params("entity target requires target_entity_id", None))?;
                AbilityTarget::Entity(EntityId(eid))
            }
            _ => return Err(McpError::invalid_params(format!("Unknown target_type: {}", params.target_type), None)),
        };
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::use_ability(&mut ctx, EntityId(params.unit_id), params.slot, target);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Categorize units into melee/ranged/support groups. Returns group IDs.")]
    async fn split_squads(
        &self,
        Parameters(params): Parameters<SplitSquadsParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let (melee, ranged, support, result) = behaviors::split_squads(&mut ctx, &ids);
            serde_json::json!({
                "melee": melee.iter().map(|e| e.0).collect::<Vec<_>>(),
                "ranged": ranged.iter().map(|e| e.0).collect::<Vec<_>>(),
                "support": support.iter().map(|e| e.0).collect::<Vec<_>>(),
                "description": result.description,
            })
        };
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Escort units stay near a VIP and engage threats within guard radius.")]
    async fn protect_unit(
        &self,
        Parameters(params): Parameters<ProtectUnitParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.escort_ids.into_iter().map(EntityId).collect();
            let radius = Fixed::from_num(params.guard_radius.unwrap_or(5.0));
            let result = behaviors::protect_unit(&mut ctx, &ids, EntityId(params.vip_id), radius);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Position units in ring around enemy target, then attack.")]
    async fn surround_target(
        &self,
        Parameters(params): Parameters<SurroundTargetParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let radius = Fixed::from_num(params.ring_radius.unwrap_or(3.0));
            let result = behaviors::surround_target(&mut ctx, &ids, EntityId(params.target_id), radius);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Check resources and train unit if affordable.")]
    async fn auto_produce(
        &self,
        Parameters(params): Parameters<AutoProduceParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = params.unit_kind.parse::<UnitKind>()
            .map_err(|_| McpError::invalid_params(format!("Unknown unit kind: {}", params.unit_kind), None))?;
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::auto_produce(&mut ctx, EntityId(params.building_id), kind);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Analyze army comp and auto-queue the least-represented combat unit type.")]
    async fn balanced_production(
        &self,
        Parameters(params): Parameters<BalancedProductionParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::balanced_production(&mut ctx, EntityId(params.building_id));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Build economic infrastructure: FishMarkets near deposits, LitterBoxes for supply.")]
    async fn expand_economy(
        &self,
        Parameters(params): Parameters<ExpandEconomyParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::expand_economy(&mut ctx, EntityId(params.builder_id));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Split army into main force (70%) + flanking group (30%) for coordinated attack.")]
    async fn coordinate_assault(
        &self,
        Parameters(params): Parameters<CoordinateAssaultParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::coordinate_assault(&mut ctx, &ids, GridPos::new(params.target_x, params.target_y));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Auto-queue the best available research upgrade at a building.")]
    async fn research_priority(
        &self,
        Parameters(params): Parameters<ResearchPriorityParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let result = behaviors::research_priority(&mut ctx, EntityId(params.building_id));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Position defenses adaptively: melee forward, ranged back, support center.")]
    async fn adaptive_defense(
        &self,
        Parameters(params): Parameters<AdaptiveDefenseParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let ids: Vec<EntityId> = params.unit_ids.into_iter().map(EntityId).collect();
            let result = behaviors::adaptive_defense(&mut ctx, &ids, GridPos::new(params.center_x, params.center_y), Fixed::from_num(params.radius));
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    #[tool(description = "Move scout to nearest unvisited waypoint from a list.")]
    async fn scout_pattern(
        &self,
        Parameters(params): Parameters<ScoutPatternParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let (result, commands) = {
            let map = sim.map();
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::from_u8(params.player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT));
            let waypoints: Vec<GridPos> = params.waypoints.iter().map(|wp| GridPos::new(wp.x, wp.y)).collect();
            let result = behaviors::scout_pattern(&mut ctx, EntityId(params.scout_id), &waypoints);
            (result, ctx.take_commands())
        };
        for cmd in commands { sim.inject_command(cmd); }
        Ok(CallToolResult::success(vec![Content::text(format!("Issued {} commands: {}", result.commands_issued, result.description))]))
    }

    // =======================================================================
    // Sim control tools (8)
    // =======================================================================

    #[tool(description = "Spawn a unit at a position for a player. Returns entity ID. Cat: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, Catnapper, FerretSapper, MechCommander. Corvid: MurderScrounger, Sentinel, Rookclaw, Magpike, Magpyre, Jaycaller, Jayflicker, Dusktalon, Hootseer, CorvusRex. Badger: Delver, Ironhide, Cragback, Warden, Sapjaw, Wardenmother, SeekerTunneler, Embermaw, Dustclaw, Gutripper. Mouse: Nibblet, Swarmer, Gnawer, Shrieker, Tunneler, Sparks, Quillback, Whiskerwitch, Plaguetail, WarrenMarshal. Croak: Ponderer, Regeneron, Broodmother, Gulper, Eftsaber, Croaker, Leapfrog, Shellwarden, Bogwhisper, MurkCommander. Raccoon: Scrounger, Bandit, HeapTitan, GlitchRat, PatchPossum, GreaseMonkey, DeadDropUnit, Wrecker, DumpsterDiver, JunkyardKing.")]
    async fn spawn_unit(
        &self,
        Parameters(params): Parameters<SpawnUnitParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = params.kind.parse::<UnitKind>()
            .map_err(|_| McpError::invalid_params(format!("Unknown unit kind: {}", params.kind), None))?;
        let mut sim = self.sim.lock().await;
        let id = sim.spawn_unit(kind, GridPos::new(params.x, params.y), params.player_id);
        Ok(CallToolResult::success(vec![Content::text(format!("{id}"))]))
    }

    #[tool(description = "Spawn a building at a position for a player. Returns entity ID. Cat: TheBox, CatTree, FishMarket, LitterBox, ServerRack, ScratchingPost, CatFlap, LaserPointer. Corvid: TheParliament, Rookery, CarrionCache, AntennaArray, Panopticon, NestBox, ThornHedge, Watchtower. Mouse: TheBurrow, NestingBox, SeedVault, JunkTransmitter, GnawLab, WarrenExpansion, Mousehole, SqueakTower. Badger: TheSett, WarHollow, BurrowDepot, CoreTap, ClawMarks, DeepWarren, BulwarkGate, SlagThrower. Croak: TheGrotto, SpawningPools, LilyMarket, SunkenServer, FossilStones, ReedBed, TidalGate, SporeTower. Raccoon: TheDumpster, ScrapHeap, ChopShop, JunkServer, TinkerBench, TrashPile, DumpsterRelay, TetanusTower.")]
    async fn spawn_building(
        &self,
        Parameters(params): Parameters<SpawnBuildingParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = params.kind.parse::<BuildingKind>()
            .map_err(|_| McpError::invalid_params(format!("Unknown building kind: {}", params.kind), None))?;
        let mut sim = self.sim.lock().await;
        let id = sim.spawn_building(kind, GridPos::new(params.x, params.y), params.player_id);
        Ok(CallToolResult::success(vec![Content::text(format!("{id}"))]))
    }

    #[tool(description = "Spawn a resource deposit. Types: Food, GpuCores, Nft.")]
    async fn spawn_deposit(
        &self,
        Parameters(params): Parameters<SpawnDepositParams>,
    ) -> Result<CallToolResult, McpError> {
        let resource_type = params.resource_type.parse::<ResourceType>()
            .map_err(|_| McpError::invalid_params(format!("Unknown resource type: {}", params.resource_type), None))?;
        let mut sim = self.sim.lock().await;
        let id = sim.spawn_deposit(resource_type, GridPos::new(params.x, params.y), params.amount);
        Ok(CallToolResult::success(vec![Content::text(format!("{id}"))]))
    }

    #[tool(description = "Advance the simulation by N ticks.")]
    async fn advance_ticks(
        &self,
        Parameters(params): Parameters<AdvanceParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.advance(params.ticks);
        let tick = sim.tick();
        Ok(CallToolResult::success(vec![Content::text(format!("Advanced to tick {tick}"))]))
    }

    #[tool(description = "Reset the simulation with a new map size.")]
    async fn reset(
        &self,
        Parameters(params): Parameters<ResetParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        sim.reset(params.width, params.height);
        Ok(CallToolResult::success(vec![Content::text("Reset complete")]))
    }

    #[tool(description = "Get complete game state for a player: all units, buildings, resources, map info.")]
    async fn get_full_state(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = serde_json::json!({
            "tick": snap.tick,
            "map_width": snap.map_width,
            "map_height": snap.map_height,
            "my_units": snap.my_units.len(),
            "enemy_units": snap.enemy_units.len(),
            "my_buildings": snap.my_buildings.len(),
            "enemy_buildings": snap.enemy_buildings.len(),
            "resources": {
                "food": snap.my_resources.food,
                "gpu_cores": snap.my_resources.gpu_cores,
                "supply": snap.my_resources.supply,
                "supply_cap": snap.my_resources.supply_cap,
            },
        });
        Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
    }

    #[tool(description = "Execute a Lua script against the current game state. Returns commands produced.")]
    async fn run_lua_script(
        &self,
        Parameters(params): Parameters<LuaScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        match sim.run_script(params.player_id, &params.source) {
            Ok(cmds) => {
                let count = cmds.len();
                for cmd in cmds {
                    sim.inject_command(cmd);
                }
                Ok(CallToolResult::success(vec![Content::text(format!("Script produced {count} commands"))]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!("Script error: {e}"))]))
        }
    }

    #[tool(description = "Register a named Lua script for a player. The script is executed immediately and its commands injected.")]
    async fn register_script(
        &self,
        Parameters(params): Parameters<RegisterScriptParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        match sim.run_script(params.player_id, &params.source) {
            Ok(cmds) => {
                let count = cmds.len();
                for cmd in cmds {
                    sim.inject_command(cmd);
                }
                Ok(CallToolResult::success(vec![Content::text(format!("Registered '{}' -- produced {count} commands", params.name))]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!("Script error: {e}"))]))
        }
    }
}

#[tool_handler]
impl ServerHandler for HarnessServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "ClawedCommand headless simulation server. Spawn units, advance ticks, query state, \
                 issue commands, and run Lua scripts via MCP tools."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::HeadlessSim;
    use cc_core::components::UnitKind;
    use cc_core::coords::GridPos;

    // Helper: create a HarnessServer wrapping a fresh HeadlessSim.
    fn make_server(w: u32, h: u32) -> HarnessServer {
        HarnessServer::new(HeadlessSim::new(w, h))
    }

    // -----------------------------------------------------------------------
    // get_units filter tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_units_no_filter_returns_all() {
        let server = make_server(32, 32);
        {
            let mut sim = server.sim.lock().await;
            sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);
            sim.spawn_unit(UnitKind::Chonk, GridPos::new(10, 10), 0);
        }
        let result = server
            .get_units(Parameters(PlayerFilterParams {
                player_id: 0,
                filter: None,
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        assert_eq!(json.len(), 2);
    }

    #[tokio::test]
    async fn get_units_idle_filter() {
        let server = make_server(32, 32);
        {
            let mut sim = server.sim.lock().await;
            // Spawn two units — both start idle
            sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);
            let moving_id = sim.spawn_unit(UnitKind::Chonk, GridPos::new(10, 10), 0);
            // Issue a move command to make one unit non-idle
            sim.inject_command(GameCommand::Move {
                unit_ids: vec![cc_core::commands::EntityId(moving_id)],
                target: GridPos::new(20, 20),
            });
            sim.advance(1);
        }
        let result = server
            .get_units(Parameters(PlayerFilterParams {
                player_id: 0,
                filter: Some("idle".to_string()),
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        // At least the Hisser should be idle, Chonk should be moving
        assert!(
            json.len() >= 1,
            "Expected at least 1 idle unit, got {}",
            json.len()
        );
        for unit in &json {
            assert_eq!(unit["idle"], true, "Filtered units should all be idle");
        }
    }

    #[tokio::test]
    async fn get_units_wounded_filter() {
        let server = make_server(32, 32);
        {
            let mut sim = server.sim.lock().await;
            // Spawn a cat unit and an enemy near it to deal damage
            sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);
            sim.spawn_unit(UnitKind::Hisser, GridPos::new(10, 10), 0); // healthy unit far away
            // Spawn an enemy very close to damage the first unit
            sim.spawn_unit(UnitKind::MechCommander, GridPos::new(5, 6), 1);
            // Advance many ticks for combat to occur
            sim.advance(30);
        }
        let result = server
            .get_units(Parameters(PlayerFilterParams {
                player_id: 0,
                filter: Some("wounded".to_string()),
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        // All returned units should be below 50% HP
        for unit in &json {
            let hp = unit["hp"].as_f64().unwrap();
            let hp_max = unit["hp_max"].as_f64().unwrap();
            assert!(
                hp < hp_max / 2.0,
                "Wounded filter should only return units below 50% HP, got {hp}/{hp_max}"
            );
        }
    }

    #[tokio::test]
    async fn get_units_attacking_filter() {
        let server = make_server(32, 32);
        {
            let mut sim = server.sim.lock().await;
            let attacker = sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);
            sim.spawn_unit(UnitKind::Pawdler, GridPos::new(20, 20), 0); // idle unit far away
            let enemy = sim.spawn_unit(UnitKind::Chonk, GridPos::new(5, 6), 1);
            sim.inject_command(GameCommand::Attack {
                unit_ids: vec![cc_core::commands::EntityId(attacker)],
                target: cc_core::commands::EntityId(enemy),
            });
            sim.advance(5);
        }
        let result = server
            .get_units(Parameters(PlayerFilterParams {
                player_id: 0,
                filter: Some("attacking".to_string()),
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        for unit in &json {
            assert_eq!(
                unit["attacking"], true,
                "Attacking filter should only return attacking units"
            );
        }
    }

    // -----------------------------------------------------------------------
    // can_reach tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn can_reach_passable_path() {
        let server = make_server(32, 32);
        let result = server
            .can_reach(Parameters(PathQueryParams {
                player_id: 0,
                from_x: 5,
                from_y: 5,
                to_x: 10,
                to_y: 10,
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        assert_eq!(text.as_text().unwrap().text, "true");
    }

    #[tokio::test]
    async fn can_reach_same_position() {
        let server = make_server(32, 32);
        let result = server
            .can_reach(Parameters(PathQueryParams {
                player_id: 0,
                from_x: 5,
                from_y: 5,
                to_x: 5,
                to_y: 5,
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        // Same position should be reachable
        assert_eq!(text.as_text().unwrap().text, "true");
    }

    // -----------------------------------------------------------------------
    // get_path_length tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_path_length_returns_value() {
        let server = make_server(32, 32);
        let result = server
            .get_path_length(Parameters(PathQueryParams {
                player_id: 0,
                from_x: 5,
                from_y: 5,
                to_x: 10,
                to_y: 10,
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let val: serde_json::Value =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        assert!(val.is_number(), "Expected path length number, got {val}");
        let len = val.as_u64().unwrap();
        // Straight-line Chebyshev distance is 5; A* path should be >= 5
        assert!(len >= 5, "Path length should be at least 5, got {len}");
    }

    // -----------------------------------------------------------------------
    // get_safe_positions tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_safe_positions_returns_positions() {
        let server = make_server(32, 32);
        let unit_id;
        {
            let mut sim = server.sim.lock().await;
            unit_id = sim.spawn_unit(UnitKind::Hisser, GridPos::new(10, 10), 0);
            // Enemy far away — most positions should be safe
            sim.spawn_unit(UnitKind::Hisser, GridPos::new(25, 25), 1);
        }
        let result = server
            .get_safe_positions(Parameters(GetSafePositionsParams {
                player_id: 0,
                unit_id,
                search_radius: Some(3),
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        // With enemy far away, there should be many safe positions
        assert!(!json.is_empty(), "Expected safe positions, got none");
        // Each position should have x and y fields
        for pos in &json {
            assert!(pos["x"].is_number());
            assert!(pos["y"].is_number());
        }
    }

    #[tokio::test]
    async fn get_safe_positions_not_found_returns_error() {
        let server = make_server(32, 32);
        let result = server
            .get_safe_positions(Parameters(GetSafePositionsParams {
                player_id: 0,
                unit_id: 999999,
                search_radius: None,
            }))
            .await;
        // Should return an error for unit not found
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // get_kite_position tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_kite_position_finds_position() {
        let server = make_server(64, 64);
        let unit_id;
        let target_id;
        {
            let mut sim = server.sim.lock().await;
            unit_id = sim.spawn_unit(UnitKind::Hisser, GridPos::new(10, 10), 0);
            target_id = sim.spawn_unit(UnitKind::Chonk, GridPos::new(10, 15), 1);
        }
        let result = server
            .get_kite_position(Parameters(GetKitePositionParams {
                player_id: 0,
                unit_id,
                target_id,
                desired_range: 3,
            }))
            .await
            .unwrap();
        let text = &result.content[0];
        let val: serde_json::Value =
            serde_json::from_str(&text.as_text().unwrap().text).unwrap();
        assert!(!val.is_null(), "Expected a kite position, got null");
        assert!(val["x"].is_number());
        assert!(val["y"].is_number());
    }

    // -----------------------------------------------------------------------
    // spawn_unit / spawn_building all-faction tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn spawn_unit_non_cat_kinds() {
        let server = make_server(32, 32);
        // Test spawning units from each non-cat faction
        let kinds = ["Swarmer", "Rookclaw", "Delver", "Ponderer", "Scrounger"];
        for kind in &kinds {
            let result = server
                .spawn_unit(Parameters(SpawnUnitParams {
                    kind: kind.to_string(),
                    x: 5,
                    y: 5,
                    player_id: 0,
                }))
                .await
                .unwrap();
            let text = &result.content[0];
            let id_str = &text.as_text().unwrap().text;
            let id: u64 = id_str.parse().expect(&format!("Failed to parse entity ID for {kind}"));
            assert!(id > 0, "Entity ID for {kind} should be non-zero");
        }
    }

    #[tokio::test]
    async fn spawn_unit_invalid_kind_returns_error() {
        let server = make_server(32, 32);
        let result = server
            .spawn_unit(Parameters(SpawnUnitParams {
                kind: "NotARealUnit".to_string(),
                x: 5,
                y: 5,
                player_id: 0,
            }))
            .await;
        assert!(result.is_err(), "Invalid unit kind should return error");
    }

    #[tokio::test]
    async fn spawn_building_non_cat_kinds() {
        let server = make_server(32, 32);
        // Test spawning buildings from each non-cat faction
        let kinds = [
            "TheParliament",
            "TheBurrow",
            "TheSett",
            "TheGrotto",
            "TheDumpster",
        ];
        for kind in &kinds {
            let result = server
                .spawn_building(Parameters(SpawnBuildingParams {
                    kind: kind.to_string(),
                    x: 5,
                    y: 5,
                    player_id: 0,
                }))
                .await
                .unwrap();
            let text = &result.content[0];
            let id_str = &text.as_text().unwrap().text;
            let id: u64 = id_str
                .parse()
                .expect(&format!("Failed to parse entity ID for {kind}"));
            assert!(id > 0, "Entity ID for {kind} should be non-zero");
        }
    }

    #[tokio::test]
    async fn spawn_building_invalid_kind_returns_error() {
        let server = make_server(32, 32);
        let result = server
            .spawn_building(Parameters(SpawnBuildingParams {
                kind: "NotARealBuilding".to_string(),
                x: 5,
                y: 5,
                player_id: 0,
            }))
            .await;
        assert!(result.is_err(), "Invalid building kind should return error");
    }
}
