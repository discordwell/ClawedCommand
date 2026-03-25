use bevy::prelude::*;

use cc_core::components::{
    AbilitySlots, AttackMoveTarget, AttackStats, AttackTarget, AttackTypeMarker, Building,
    ChasingTarget, Dead, Gathering, Health, MoveTarget, MovementSpeed, Owner, Path, Position,
    ProductionQueue, ResearchQueue, ResourceDeposit, UnderConstruction, UnitType,
};
use cc_core::status_effects::StatusEffects;
use cc_core::terrain::FactionId;
use cc_sim::resources::{CommandQueue, MapResource, PlayerResources, SimClock};

use crate::events::{self, ActivationMode};
use crate::lua_runtime;
use crate::script_context::{BlackboardValue, EnemyMemoryEntry, ScriptContext, ScriptEvent};
pub use crate::script_registry::ScriptRegistry;
use crate::snapshot::{self, GameStateSnapshot};
use crate::tool_tier::FactionToolStates;

use std::collections::HashMap;

/// Resource: previous tick's snapshot for event diffing, keyed by player_id.
#[derive(Resource, Default)]
pub struct PreviousSnapshots {
    pub snapshots: HashMap<u8, GameStateSnapshot>,
}

/// Resource: persistent per-player state for Phase 1-3 ScriptContext features.
/// Blackboard, enemy memory, events, and squads all persist across ticks.
#[derive(Resource, Default)]
pub struct PlayerScriptState {
    pub blackboards: HashMap<u8, HashMap<String, BlackboardValue>>,
    pub enemy_memories: HashMap<u8, HashMap<u64, EnemyMemoryEntry>>,
    pub events: HashMap<u8, Vec<ScriptEvent>>,
    pub squads: HashMap<u8, HashMap<String, Vec<u64>>>,
}

/// Resource: manual script triggers from the UI. Cleared each tick after processing.
#[derive(Resource, Default)]
pub struct ManualScriptTriggers {
    pub triggered: Vec<String>,
}

/// Bevy system: runs registered scripts in response to detected events.
/// Executes after all simulation systems in FixedUpdate.
pub fn script_runner_system(
    sim_clock: Res<SimClock>,
    map_res: Res<MapResource>,
    player_resources: Res<PlayerResources>,
    mut cmd_queue: ResMut<CommandQueue>,
    mut registry: ResMut<ScriptRegistry>,
    mut prev_snapshots: ResMut<PreviousSnapshots>,
    mut script_state: ResMut<PlayerScriptState>,
    mut manual_triggers: ResMut<ManualScriptTriggers>,
    tool_states: Res<FactionToolStates>,
    #[cfg(feature = "harness")] mut arena_stats: Option<ResMut<crate::arena::ArenaStats>>,
    units: Query<
        (
            Entity,
            &Position,
            &Owner,
            &UnitType,
            &Health,
            &MovementSpeed,
            Option<&AttackStats>,
            Option<&AttackTypeMarker>,
            Option<&MoveTarget>,
            Option<&AttackTarget>,
            Option<&Path>,
            Option<&Gathering>,
            (
                Option<&ChasingTarget>,
                Option<&AttackMoveTarget>,
                Option<&Dead>,
                Option<&StatusEffects>,
                Option<&AbilitySlots>,
            ),
        ),
        With<UnitType>,
    >,
    buildings: Query<
        (
            Entity,
            &Position,
            &Owner,
            &Building,
            &Health,
            Option<&UnderConstruction>,
            Option<&ProductionQueue>,
            Option<&ResearchQueue>,
        ),
        With<Building>,
    >,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
) {
    // Collect query results into vecs for snapshot building
    let unit_data: Vec<_> = units
        .iter()
        .map(
            |(
                e,
                pos,
                own,
                ut,
                hp,
                spd,
                atk,
                atk_type,
                mt,
                at,
                path,
                gath,
                (chase, amove, dead, se, abs),
            )| {
                (
                    e, pos, own, ut, hp, spd, atk, atk_type, mt, at, path, gath, chase, amove,
                    dead, se, abs,
                )
            },
        )
        .collect();

    let building_data: Vec<_> = buildings.iter().collect();

    let deposit_data: Vec<_> = deposits.iter().collect();

    // Determine which players have scripts registered
    let player_ids: Vec<u8> = registry
        .scripts
        .iter()
        .map(|s| s.player_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for player_id in player_ids {
        // Build snapshot for this player
        let current_snapshot = snapshot::build_snapshot(
            sim_clock.tick,
            map_res.map.width,
            map_res.map.height,
            player_id,
            &player_resources,
            &unit_data,
            &building_data,
            &deposit_data,
        );

        // Detect events by diffing with this player's previous snapshot
        let fired_events =
            events::detect_events(&current_snapshot, prev_snapshots.snapshots.get(&player_id));

        let faction = FactionId::from_u8(player_id).unwrap_or(FactionId::CatGPT);

        // Drain manual triggers for this player
        let manual_names: Vec<String> = manual_triggers.triggered.drain(..).collect();

        // Run scripts that match fired events
        for script in registry.scripts.iter_mut() {
            if script.player_id != player_id {
                continue;
            }

            // --- Filter 1: Enabled check ---
            if !script.enabled {
                continue;
            }

            // --- Filter 2: Manual mode ---
            if script.activation_mode == ActivationMode::Manual {
                if !manual_names.iter().any(|n| n == &script.name) {
                    continue;
                }
                // Manual scripts bypass tick/event checks — run immediately
            } else {
                // --- Normal tick/event scheduling ---
                // Check tick interval for on_tick events
                let should_run_tick = if script.listens_for("on_tick") {
                    script.ticks_since_last_run += 1;
                    if script.ticks_since_last_run >= script.tick_interval {
                        script.ticks_since_last_run = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Check if any non-tick event matches
                let has_matching_event = fired_events.iter().any(|event| {
                    let name = events::event_name(event);
                    if name == "on_tick" {
                        return false; // Handled separately above
                    }
                    script.listens_for(name)
                });

                if !should_run_tick && !has_matching_event {
                    continue;
                }
            }

            // --- Filter 3: @when condition ---
            if let Some(ref condition) = script.when_condition {
                match lua_runtime::evaluate_condition(
                    condition,
                    &current_snapshot,
                    player_id,
                    &map_res.map,
                ) {
                    Ok(true) => {} // condition met, proceed
                    Ok(false) => continue,
                    Err(e) => {
                        log::warn!("Script '{}' @when condition error: {}", script.name, e);
                        continue;
                    }
                }
            }

            // --- Filter 4: Unit/squad snapshot filtering ---
            let filtered_snapshot = if script.unit_filter.is_some() || script.squad_filter.is_some()
            {
                let mut snap = current_snapshot.clone();

                if let Some(ref kinds) = script.unit_filter {
                    snap.my_units.retain(|u| kinds.contains(&u.kind));
                }

                if let Some(ref squad_name) = script.squad_filter {
                    let squad_ids: Vec<u64> = script_state
                        .squads
                        .get(&player_id)
                        .and_then(|sq| sq.get(squad_name))
                        .cloned()
                        .unwrap_or_default();
                    if !squad_ids.is_empty() {
                        snap.my_units.retain(|u| squad_ids.contains(&u.id.0));
                    }
                }

                Some(snap)
            } else {
                None
            };

            let snapshot_ref = filtered_snapshot.as_ref().unwrap_or(&current_snapshot);

            // Get persistent per-player state for this script invocation.
            // We destructure script_state to get independent mutable borrows.
            let PlayerScriptState {
                blackboards,
                enemy_memories,
                events: event_buses,
                squads: squad_maps,
            } = &mut *script_state;

            let blackboard = blackboards.entry(player_id).or_default().clone();
            let enemy_mem = enemy_memories.entry(player_id).or_default();
            let events = event_buses.entry(player_id).or_default();
            let squads = squad_maps.entry(player_id).or_default();

            // Create ScriptContext with all optional features wired
            let mut ctx = ScriptContext::new_with_blackboard(
                snapshot_ref,
                &map_res.map,
                player_id,
                faction,
                blackboard,
            )
            .with_enemy_memory(enemy_mem)
            .with_events(events)
            .with_squads(squads);

            // Update enemy memory from current snapshot visibility
            ctx.update_enemy_memory();

            let tier = tool_states.tier_for(player_id);
            match lua_runtime::execute_script_with_context_tiered(&script.source, &mut ctx, tier) {
                Ok(commands) => {
                    for cmd in commands {
                        cmd_queue.push(cmd);
                    }
                }
                Err(e) => {
                    log::warn!("Script '{}' error: {}", script.name, e);
                    // Capture error for arena reporting if ArenaStats resource exists
                    #[cfg(feature = "harness")]
                    if let Some(ref mut stats) = arena_stats {
                        stats.script_errors.push(crate::arena::ArenaScriptError {
                            tick: sim_clock.tick,
                            script_name: script.name.clone(),
                            message: format!("{}", e),
                        });
                    }
                }
            }

            // Persist blackboard changes back to resource
            let bb = ctx.take_blackboard();
            blackboards.insert(player_id, bb);
        }

        // Store snapshot for next tick's diffing (per-player)
        prev_snapshots.snapshots.insert(player_id, current_snapshot);
    }
}

/// Plugin that adds the script runner to the Bevy app.
/// Scripts run in FixedUpdate after sim systems. Commands they emit
/// are pushed to CommandQueue and processed on the NEXT sim tick.
pub struct ScriptRunnerPlugin;

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptRegistry>()
            .init_resource::<PreviousSnapshots>()
            .init_resource::<PlayerScriptState>()
            .init_resource::<ManualScriptTriggers>()
            .add_systems(
                FixedUpdate,
                script_runner_system.run_if(|state: Res<cc_sim::resources::GameState>| {
                    *state == cc_sim::resources::GameState::Playing
                }),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ScriptRegistration;

    #[test]
    fn script_registry_add_remove() {
        let mut registry = ScriptRegistry::default();

        registry.register(ScriptRegistration::new(
            "test_script".into(),
            "ctx:move_units({1}, 10, 10)".into(),
            vec!["on_tick".into()],
            0,
        ));
        assert_eq!(registry.scripts.len(), 1);

        registry.unregister("test_script");
        assert_eq!(registry.scripts.len(), 0);
    }

    #[test]
    fn script_registry_unregister_by_name() {
        let mut registry = ScriptRegistry::default();

        registry.register(ScriptRegistration::new("a".into(), "".into(), vec![], 0));
        registry.register(ScriptRegistration::new("b".into(), "".into(), vec![], 0));
        registry.register(ScriptRegistration::new("c".into(), "".into(), vec![], 0));

        registry.unregister("b");
        assert_eq!(registry.scripts.len(), 2);
        assert!(registry.scripts.iter().all(|s| s.name != "b"));
    }

    #[test]
    fn register_lua_script_replaces_existing() {
        let mut registry = ScriptRegistry::default();

        registry.register_lua_script("kite", "local x = ctx:my_units()", 0);
        assert_eq!(registry.scripts.len(), 1);
        assert_eq!(registry.scripts[0].tick_interval, 3);
        assert_eq!(registry.scripts[0].events, vec!["on_tick"]);

        // Re-register with different source — should replace
        registry.register_lua_script("kite", "local y = ctx:enemy_units()", 0);
        assert_eq!(registry.scripts.len(), 1);
        assert!(registry.scripts[0].source.contains("enemy_units"));
    }
}
