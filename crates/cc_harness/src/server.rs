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

    #[tool(description = "Get all own units for a player. Returns array of unit objects with id, kind, pos, hp, state.")]
    async fn get_units(
        &self,
        Parameters(params): Parameters<PlayerOnly>,
    ) -> Result<CallToolResult, McpError> {
        let mut sim = self.sim.lock().await;
        let snap = sim.snapshot(params.player_id);
        let json = serde_json::to_string_pretty(&snap.my_units.iter().map(|u| {
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
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
        let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
            let mut ctx = ScriptContext::new(&snap, map, params.player_id, cc_core::terrain::FactionId::CatGPT);
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
    // Sim control tools (8)
    // =======================================================================

    #[tool(description = "Spawn a unit at a position for a player. Returns entity ID. Kinds: Pawdler, Nuisance, Chonk, FlyingFox, Hisser, Yowler, Mouser, Catnapper, FerretSapper, MechCommander.")]
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

    #[tool(description = "Spawn a building at a position for a player. Returns entity ID. Kinds: TheBox, CatTree, FishMarket, LitterBox, ServerRack, ScratchingPost, CatFlap, LaserPointer.")]
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
