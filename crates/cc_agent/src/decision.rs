//! Agent decision trigger system.
//!
//! Periodically sends AgentRequests for AI-controlled players.
//! Runs on a timer in Update. Does not block — just fires requests
//! through the AgentBridge channel.

use std::collections::HashSet;

use bevy::prelude::*;

use cc_core::components::{
    AbilitySlots, AttackMoveTarget, AttackStats, AttackTarget, AttackTypeMarker, Building,
    ChasingTarget, Dead, Gathering, Health, MoveTarget, MovementSpeed, Owner, Path, Position,
    ProductionQueue, ResearchQueue, ResourceDeposit, UnitType, UnderConstruction,
};
use cc_core::status_effects::StatusEffects;
use cc_sim::resources::{MapResource, PlayerResources, SimClock};

use crate::agent_bridge::{AgentBridge, AgentRequest, AgentSource};
use crate::snapshot;
use crate::tool_tier::FactionToolStates;

/// How often (in seconds) the AI requests a decision.
const DECISION_INTERVAL_SECS: f32 = 2.0;

/// Resource tracking which players are AI-controlled and in-flight status.
#[derive(Resource)]
pub struct AgentDecisionState {
    pub timer: Timer,
    /// Player IDs that are AI-controlled (not human).
    pub ai_players: HashSet<u8>,
    /// Player IDs with an in-flight request (waiting for response).
    pub in_flight: HashSet<u8>,
}

impl Default for AgentDecisionState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(DECISION_INTERVAL_SECS, TimerMode::Repeating),
            ai_players: HashSet::new(),
            in_flight: HashSet::new(),
        }
    }
}

/// Bevy system: fires AgentRequests for AI players on a timer.
pub fn agent_decision_system(
    time: Res<Time>,
    sim_clock: Res<SimClock>,
    map_res: Res<MapResource>,
    player_resources: Res<PlayerResources>,
    bridge: Res<AgentBridge>,
    tool_states: Res<FactionToolStates>,
    mut decision_state: ResMut<AgentDecisionState>,
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
    decision_state.timer.tick(time.delta());
    if !decision_state.timer.just_finished() {
        return;
    }

    if decision_state.ai_players.is_empty() {
        return;
    }

    // Collect ECS data once
    let unit_data: Vec<_> = units
        .iter()
        .map(
            |(e, pos, own, ut, hp, spd, atk, atk_type, mt, at, path, gath, (chase, amove, dead, se, abs))| {
                (e, pos, own, ut, hp, spd, atk, atk_type, mt, at, path, gath, chase, amove, dead, se, abs)
            },
        )
        .collect();

    let building_data: Vec<_> = buildings
        .iter()
        .map(|(e, pos, own, bld, hp, uc, pq, rq)| (e, pos, own, bld, hp, uc, pq, rq))
        .collect();

    let deposit_data: Vec<_> = deposits
        .iter()
        .map(|(e, pos, dep)| (e, pos, dep))
        .collect();

    let ai_players: Vec<u8> = decision_state.ai_players.iter().copied().collect();
    for player_id in ai_players {
        // Skip if already waiting for a response
        if decision_state.in_flight.contains(&player_id) {
            continue;
        }

        let snap = snapshot::build_snapshot(
            sim_clock.tick,
            map_res.map.width,
            map_res.map.height,
            player_id,
            &player_resources,
            &unit_data,
            &building_data,
            &deposit_data,
        );

        let tier = tool_states.tier_for(player_id);

        let request = AgentRequest {
            player_id,
            prompt: "Assess the situation and issue commands.".into(),
            tier,
            source: AgentSource::GameLoop,
            chat_history: None,
            snapshot: Some(snap),
        };

        if bridge.request_tx.try_send(request).is_ok() {
            decision_state.in_flight.insert(player_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_decision_state() {
        let state = AgentDecisionState::default();
        assert!(state.ai_players.is_empty());
        assert!(state.in_flight.is_empty());
        assert_eq!(state.timer.duration().as_secs_f32(), DECISION_INTERVAL_SECS);
    }
}
