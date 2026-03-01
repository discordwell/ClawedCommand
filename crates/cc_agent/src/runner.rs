use bevy::prelude::*;

use cc_core::components::{
    AttackMoveTarget, AttackStats, AttackTarget, AttackTypeMarker, Building, ChasingTarget, Dead,
    Gathering, Health, MoveTarget, MovementSpeed, Owner, Path, Position, ProductionQueue,
    ResourceDeposit, UnitType, UnderConstruction,
};
use cc_core::terrain::FactionId;
use cc_sim::resources::{CommandQueue, MapResource, PlayerResources, SimClock};

use crate::events::{self, ScriptRegistration};
use crate::lua_runtime;
use crate::script_context::ScriptContext;
use crate::snapshot::{self, GameStateSnapshot};
use crate::tool_tier::FactionToolStates;

/// Resource: registered scripts that respond to game events.
#[derive(Resource, Default)]
pub struct ScriptRegistry {
    pub scripts: Vec<ScriptRegistration>,
}

impl ScriptRegistry {
    pub fn register(&mut self, script: ScriptRegistration) {
        self.scripts.push(script);
    }

    pub fn unregister(&mut self, name: &str) {
        self.scripts.retain(|s| s.name != name);
    }
}

/// Resource: previous tick's snapshot for event diffing, keyed by player_id.
#[derive(Resource, Default)]
pub struct PreviousSnapshots {
    pub snapshots: std::collections::HashMap<u8, GameStateSnapshot>,
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
    tool_states: Res<FactionToolStates>,
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
            Option<&ChasingTarget>,
            Option<&AttackMoveTarget>,
            Option<&Dead>,
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
        ),
        With<Building>,
    >,
    deposits: Query<(Entity, &Position, &ResourceDeposit)>,
) {
    // Collect query results into vecs for snapshot building
    let unit_data: Vec<_> = units
        .iter()
        .map(
            |(e, pos, own, ut, hp, spd, atk, atk_type, mt, at, path, gath, chase, amove, dead)| {
                (e, pos, own, ut, hp, spd, atk, atk_type, mt, at, path, gath, chase, amove, dead)
            },
        )
        .collect();

    let building_data: Vec<_> = buildings
        .iter()
        .map(|(e, pos, own, bld, hp, uc, pq)| (e, pos, own, bld, hp, uc, pq))
        .collect();

    let deposit_data: Vec<_> = deposits
        .iter()
        .map(|(e, pos, dep)| (e, pos, dep))
        .collect();

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
        let fired_events = events::detect_events(
            &current_snapshot,
            prev_snapshots.snapshots.get(&player_id),
        );

        let faction = FactionId::from_u8(player_id).unwrap_or(FactionId::CatGPT);

        // Run scripts that match fired events
        for script in registry.scripts.iter_mut() {
            if script.player_id != player_id {
                continue;
            }

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

            // Create ScriptContext and execute
            let mut ctx = ScriptContext::new(
                &current_snapshot,
                &map_res.map,
                player_id,
                faction,
            );

            let tier = tool_states.tier_for(player_id);
            match lua_runtime::execute_script_with_context_tiered(&script.source, &mut ctx, tier) {
                Ok(commands) => {
                    for cmd in commands {
                        cmd_queue.push(cmd);
                    }
                }
                Err(e) => {
                    log::warn!("Script '{}' error: {}", script.name, e);
                }
            }
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
            .add_systems(FixedUpdate, script_runner_system);
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

        registry.register(ScriptRegistration::new(
            "a".into(), "".into(), vec![], 0,
        ));
        registry.register(ScriptRegistration::new(
            "b".into(), "".into(), vec![], 0,
        ));
        registry.register(ScriptRegistration::new(
            "c".into(), "".into(), vec![], 0,
        ));

        registry.unregister("b");
        assert_eq!(registry.scripts.len(), 2);
        assert!(registry.scripts.iter().all(|s| s.name != "b"));
    }
}
