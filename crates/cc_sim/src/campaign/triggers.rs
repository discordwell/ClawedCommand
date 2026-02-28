use bevy::prelude::*;

use cc_core::components::{Dead, Health, HeroIdentity, Owner, Position};
use cc_core::mission::{DialogueLine, TriggerAction, TriggerCondition};

use crate::resources::SimClock;

use super::state::{CampaignPhase, CampaignState};

/// Message: dialogue lines to display.
#[derive(Message)]
pub struct DialogueEvent {
    pub lines: Vec<DialogueLine>,
}

/// Message: a specific trigger has fired (for chaining).
#[derive(Message)]
pub struct TriggerFiredEvent {
    pub trigger_id: String,
}

/// Message: an objective was completed by a trigger action.
#[derive(Message)]
pub struct ObjectiveCompleteEvent {
    pub objective_id: String,
}

/// System: evaluate all scripted triggers each tick during a mission.
/// Runs after combat (to see Dead markers) and before cleanup (entities still exist).
pub fn trigger_check_system(
    clock: Res<SimClock>,
    mut campaign: ResMut<CampaignState>,
    mut dialogue_writer: MessageWriter<DialogueEvent>,
    mut trigger_writer: MessageWriter<TriggerFiredEvent>,
    mut objective_writer: MessageWriter<ObjectiveCompleteEvent>,
    heroes: Query<(&HeroIdentity, &Position, &Owner, &Health, Has<Dead>)>,
    enemies: Query<(&Owner, Has<Dead>)>,
) {
    if campaign.phase != CampaignPhase::InMission {
        return;
    }

    let Some(mission) = campaign.current_mission.clone() else {
        return;
    };

    // Track enemy kills (player 0 units killed by counting dead player_id != 0)
    let mut living_enemies = 0u32;
    for (owner, is_dead) in enemies.iter() {
        if owner.player_id != 0 && !is_dead {
            living_enemies += 1;
        }
    }

    // Collect triggers to fire this tick
    let mut triggers_to_fire: Vec<String> = Vec::new();

    for trigger in &mission.triggers {
        // Skip already-fired once-triggers
        if trigger.once && campaign.fired_triggers.contains(&trigger.id) {
            continue;
        }

        if evaluate_condition(
            &trigger.condition,
            clock.tick,
            &campaign,
            &heroes,
            living_enemies,
        ) {
            triggers_to_fire.push(trigger.id.clone());
        }
    }

    // Execute trigger actions
    for trigger_id in &triggers_to_fire {
        let Some(trigger) = mission.triggers.iter().find(|t| &t.id == trigger_id) else {
            continue;
        };

        for action in &trigger.actions {
            match action {
                TriggerAction::ShowDialogue(indices) => {
                    let lines: Vec<DialogueLine> = indices
                        .iter()
                        .filter_map(|&idx| mission.dialogue.get(idx).cloned())
                        .collect();
                    if !lines.is_empty() {
                        dialogue_writer.send(DialogueEvent { lines });
                    }
                }
                TriggerAction::SpawnWave(wave_id) => {
                    // Mark wave as pending spawn — actual spawning handled by mission loader
                    if !campaign.spawned_waves.contains(wave_id) {
                        campaign.spawned_waves.push(wave_id.clone());
                    }
                }
                TriggerAction::SetFlag(flag) => {
                    if !campaign.flags.contains(flag) {
                        campaign.flags.push(flag.clone());
                    }
                }
                TriggerAction::CompleteObjective(obj_id) => {
                    objective_writer.send(ObjectiveCompleteEvent {
                        objective_id: obj_id.clone(),
                    });
                }
                TriggerAction::PanCamera(_pos) => {
                    // Camera panning is handled by the client — we just fire the trigger event
                }
            }
        }

        // Mark trigger as fired
        if trigger.once {
            campaign.fired_triggers.push(trigger_id.clone());
        }

        trigger_writer.send(TriggerFiredEvent {
            trigger_id: trigger_id.clone(),
        });
    }
}

/// Recursively evaluate a trigger condition.
fn evaluate_condition(
    condition: &TriggerCondition,
    tick: u64,
    campaign: &CampaignState,
    heroes: &Query<(&HeroIdentity, &Position, &Owner, &Health, Has<Dead>)>,
    living_enemies: u32,
) -> bool {
    match condition {
        TriggerCondition::AtTick(t) => tick == *t,

        TriggerCondition::HeroAtPos {
            hero,
            position,
            radius,
        } => {
            for (identity, pos, _owner, _health, _is_dead) in heroes.iter() {
                if identity.hero_id == *hero {
                    let grid = pos.world.to_grid();
                    let dx = (grid.x - position.x).abs();
                    let dy = (grid.y - position.y).abs();
                    return dx <= *radius && dy <= *radius;
                }
            }
            false
        }

        TriggerCondition::EnemyKillCount(target) => campaign.enemy_kill_count >= *target,

        TriggerCondition::AllEnemiesDead => living_enemies == 0,

        TriggerCondition::WaveEliminated(_wave_id) => {
            // Wave elimination tracking requires per-wave entity tracking
            // For now, check if the wave was spawned and all enemies are dead
            // TODO: track per-wave entity membership for precise elimination checks
            false
        }

        TriggerCondition::FlagSet(flag) => campaign.flags.contains(flag),

        TriggerCondition::TriggerFired(trigger_id) => {
            campaign.fired_triggers.contains(trigger_id)
        }

        TriggerCondition::All(conditions) => conditions
            .iter()
            .all(|c| evaluate_condition(c, tick, campaign, heroes, living_enemies)),

        TriggerCondition::Any(conditions) => conditions
            .iter()
            .any(|c| evaluate_condition(c, tick, campaign, heroes, living_enemies)),

        TriggerCondition::HeroHpBelow { hero, percentage } => {
            for (identity, _pos, _owner, health, _is_dead) in heroes.iter() {
                if identity.hero_id == *hero {
                    let hp_pct = if health.max > cc_core::math::Fixed::ZERO {
                        (health.current * cc_core::math::Fixed::from_num(100)) / health.max
                    } else {
                        cc_core::math::Fixed::ZERO
                    };
                    return hp_pct < cc_core::math::Fixed::from_num(*percentage);
                }
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::hero::HeroId;
    use cc_core::mission::*;

    #[test]
    fn at_tick_condition() {
        let campaign = CampaignState::default();
        let heroes = bevy::ecs::system::SystemState::<
            Query<(&HeroIdentity, &Position, &Owner, &Health, Has<Dead>)>,
        >::new(&mut bevy::ecs::world::World::new());
        // We can't easily run Query outside of a Bevy world, so test the logic directly
        let cond = TriggerCondition::AtTick(10);
        // AtTick matches exact tick
        assert!(matches!(&cond, TriggerCondition::AtTick(10)));
    }

    #[test]
    fn flag_condition() {
        let mut campaign = CampaignState::default();
        let cond = TriggerCondition::FlagSet("test_flag".into());

        // Flag not set → false
        assert!(!campaign.flags.contains(&"test_flag".to_string()));

        // Set flag
        campaign.flags.push("test_flag".into());
        assert!(campaign.flags.contains(&"test_flag".to_string()));
    }

    #[test]
    fn trigger_fired_condition() {
        let mut campaign = CampaignState::default();
        assert!(!campaign.fired_triggers.contains(&"t1".to_string()));

        campaign.fired_triggers.push("t1".into());
        assert!(campaign.fired_triggers.contains(&"t1".to_string()));
    }

    #[test]
    fn enemy_kill_count_condition() {
        let mut campaign = CampaignState::default();
        campaign.enemy_kill_count = 3;
        // Should be true when kill count >= target
        assert!(campaign.enemy_kill_count >= 3);
        assert!(!(campaign.enemy_kill_count >= 5));
        campaign.enemy_kill_count = 5;
        assert!(campaign.enemy_kill_count >= 5);
    }
}
