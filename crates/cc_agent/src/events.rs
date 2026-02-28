use std::collections::HashSet;

use cc_core::commands::EntityId;
use cc_core::math::Fixed;

use crate::snapshot::{GameStateSnapshot, UnitSnapshot};

/// Events detected by diffing consecutive game state snapshots.
#[derive(Debug, Clone)]
pub enum ScriptEvent {
    /// Fires every N ticks (configurable per script).
    Tick,
    /// A new enemy unit became visible.
    EnemySpotted { enemy: UnitSnapshot },
    /// An own unit took damage since last snapshot.
    UnitAttacked {
        unit_id: EntityId,
        damage_taken: Fixed,
    },
    /// An own unit has no commands — it's idle.
    UnitIdle { unit_id: EntityId },
    /// An own unit died (was in previous snapshot, now dead or missing).
    UnitDied { unit_id: EntityId },
}

/// A registered script with its event bindings.
#[derive(Debug, Clone)]
pub struct ScriptRegistration {
    pub name: String,
    pub source: String,
    pub events: Vec<String>,
    pub tick_interval: u32,
    pub player_id: u8,
    /// Internal: tick counter for on_tick scheduling.
    pub ticks_since_last_run: u32,
}

impl ScriptRegistration {
    pub fn new(
        name: String,
        source: String,
        events: Vec<String>,
        player_id: u8,
    ) -> Self {
        Self {
            name,
            source,
            events,
            tick_interval: 5, // default: run every 5 ticks (2Hz at 10Hz sim)
            player_id,
            ticks_since_last_run: 0,
        }
    }

    /// Check if this script listens for the given event name.
    pub fn listens_for(&self, event_name: &str) -> bool {
        self.events.iter().any(|e| e == event_name)
    }
}

/// Detect events by comparing the current snapshot to the previous one.
/// Returns a list of events that fired this tick.
pub fn detect_events(
    current: &GameStateSnapshot,
    previous: Option<&GameStateSnapshot>,
) -> Vec<ScriptEvent> {
    let mut events = Vec::new();

    // Tick event always fires
    events.push(ScriptEvent::Tick);

    let Some(prev) = previous else {
        return events;
    };

    // Enemy spotted: enemy in current but not in previous
    let prev_enemy_ids: HashSet<EntityId> = prev
        .enemy_units
        .iter()
        .map(|u| u.id)
        .collect();

    for enemy in &current.enemy_units {
        if !enemy.is_dead && !prev_enemy_ids.contains(&enemy.id) {
            events.push(ScriptEvent::EnemySpotted {
                enemy: enemy.clone(),
            });
        }
    }

    // Unit attacked: own unit's health decreased
    let prev_my_health: std::collections::HashMap<EntityId, Fixed> = prev
        .my_units
        .iter()
        .map(|u| (u.id, u.health_current))
        .collect();

    for unit in &current.my_units {
        if let Some(&prev_hp) = prev_my_health.get(&unit.id) {
            if unit.health_current < prev_hp && !unit.is_dead {
                events.push(ScriptEvent::UnitAttacked {
                    unit_id: unit.id,
                    damage_taken: prev_hp - unit.health_current,
                });
            }
        }
    }

    // Unit idle: own unit is idle now but wasn't in previous
    let prev_idle: HashSet<EntityId> = prev
        .my_units
        .iter()
        .filter(|u| u.is_idle)
        .map(|u| u.id)
        .collect();

    for unit in &current.my_units {
        if unit.is_idle && !unit.is_dead && !prev_idle.contains(&unit.id) {
            events.push(ScriptEvent::UnitIdle { unit_id: unit.id });
        }
    }

    // Unit died: own unit in previous but dead or missing in current
    let current_alive_ids: HashSet<EntityId> = current
        .my_units
        .iter()
        .filter(|u| !u.is_dead)
        .map(|u| u.id)
        .collect();

    for prev_unit in &prev.my_units {
        if prev_unit.is_dead {
            continue;
        }
        if !current_alive_ids.contains(&prev_unit.id) {
            events.push(ScriptEvent::UnitDied {
                unit_id: prev_unit.id,
            });
        }
    }

    events
}

/// Map a ScriptEvent to its event name string (matches what scripts register for).
pub fn event_name(event: &ScriptEvent) -> &'static str {
    match event {
        ScriptEvent::Tick => "on_tick",
        ScriptEvent::EnemySpotted { .. } => "on_enemy_spotted",
        ScriptEvent::UnitAttacked { .. } => "on_unit_attacked",
        ScriptEvent::UnitIdle { .. } => "on_unit_idle",
        ScriptEvent::UnitDied { .. } => "on_unit_died",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::components::{AttackType, UnitKind};
    use cc_core::coords::{GridPos, WorldPos};
    use cc_core::math::fixed_from_i32;
    use cc_sim::resources::PlayerResourceState;

    fn make_unit(id: u64, x: i32, y: i32, owner: u8) -> UnitSnapshot {
        UnitSnapshot {
            id: EntityId(id),
            kind: UnitKind::Hisser,
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
            is_idle: false,
            is_dead: false,
        }
    }

    fn empty_snapshot(my_units: Vec<UnitSnapshot>, enemy_units: Vec<UnitSnapshot>) -> GameStateSnapshot {
        GameStateSnapshot {
            tick: 1,
            map_width: 64,
            map_height: 64,
            player_id: 0,
            my_units,
            enemy_units,
            my_buildings: vec![],
            enemy_buildings: vec![],
            resource_deposits: vec![],
            my_resources: PlayerResourceState::default(),
        }
    }

    #[test]
    fn tick_event_always_fires() {
        let current = empty_snapshot(vec![], vec![]);
        let events = detect_events(&current, None);
        assert!(events.iter().any(|e| matches!(e, ScriptEvent::Tick)));
    }

    #[test]
    fn enemy_spotted_on_new_enemy() {
        let prev = empty_snapshot(vec![], vec![]);
        let current = empty_snapshot(vec![], vec![make_unit(10, 5, 5, 1)]);

        let events = detect_events(&current, Some(&prev));
        let spotted: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::EnemySpotted { .. }))
            .collect();
        assert_eq!(spotted.len(), 1);
    }

    #[test]
    fn no_enemy_spotted_if_already_visible() {
        let enemy = make_unit(10, 5, 5, 1);
        let prev = empty_snapshot(vec![], vec![enemy.clone()]);
        let current = empty_snapshot(vec![], vec![enemy]);

        let events = detect_events(&current, Some(&prev));
        let spotted: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::EnemySpotted { .. }))
            .collect();
        assert_eq!(spotted.len(), 0);
    }

    #[test]
    fn unit_attacked_on_health_decrease() {
        let mut healthy = make_unit(1, 5, 5, 0);
        healthy.health_current = fixed_from_i32(100);

        let mut damaged = healthy.clone();
        damaged.health_current = fixed_from_i32(80);

        let prev = empty_snapshot(vec![healthy], vec![]);
        let current = empty_snapshot(vec![damaged], vec![]);

        let events = detect_events(&current, Some(&prev));
        let attacked: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::UnitAttacked { .. }))
            .collect();
        assert_eq!(attacked.len(), 1);

        if let ScriptEvent::UnitAttacked { damage_taken, .. } = &attacked[0] {
            assert_eq!(*damage_taken, fixed_from_i32(20));
        }
    }

    #[test]
    fn unit_idle_fires_on_transition() {
        let mut busy = make_unit(1, 5, 5, 0);
        busy.is_idle = false;
        busy.is_moving = true;

        let mut idle = busy.clone();
        idle.is_idle = true;
        idle.is_moving = false;

        let prev = empty_snapshot(vec![busy], vec![]);
        let current = empty_snapshot(vec![idle], vec![]);

        let events = detect_events(&current, Some(&prev));
        let idles: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::UnitIdle { .. }))
            .collect();
        assert_eq!(idles.len(), 1);
    }

    #[test]
    fn unit_died_when_removed() {
        let alive = make_unit(1, 5, 5, 0);
        let prev = empty_snapshot(vec![alive], vec![]);
        let current = empty_snapshot(vec![], vec![]); // unit gone

        let events = detect_events(&current, Some(&prev));
        let deaths: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::UnitDied { .. }))
            .collect();
        assert_eq!(deaths.len(), 1);
    }

    #[test]
    fn unit_died_when_marked_dead() {
        let alive = make_unit(1, 5, 5, 0);
        let mut dead = alive.clone();
        dead.is_dead = true;

        let prev = empty_snapshot(vec![alive], vec![]);
        let current = empty_snapshot(vec![dead], vec![]);

        let events = detect_events(&current, Some(&prev));
        let deaths: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ScriptEvent::UnitDied { .. }))
            .collect();
        assert_eq!(deaths.len(), 1);
    }

    #[test]
    fn script_registration_listens_for() {
        let reg = ScriptRegistration::new(
            "test".into(),
            "".into(),
            vec!["on_tick".into(), "on_enemy_spotted".into()],
            0,
        );
        assert!(reg.listens_for("on_tick"));
        assert!(reg.listens_for("on_enemy_spotted"));
        assert!(!reg.listens_for("on_unit_died"));
    }
}
