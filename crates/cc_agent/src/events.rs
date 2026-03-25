use std::collections::HashSet;

use cc_core::commands::EntityId;
use cc_core::components::UnitKind;
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

/// How a script is activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationMode {
    /// Runs automatically on matching events/ticks.
    Auto,
    /// Triggered by game events only (not on_tick).
    EventTriggered,
    /// Only runs when manually triggered from UI.
    Manual,
}

impl Default for ActivationMode {
    fn default() -> Self {
        Self::Auto
    }
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
    /// Whether this script is enabled. Disabled scripts are skipped entirely.
    pub enabled: bool,
    /// Filter: only run for these unit kinds (from `@units: Hisser, Nuisance`).
    pub unit_filter: Option<Vec<UnitKind>>,
    /// Filter: only run for this squad (from `@squad: skirmishers`).
    pub squad_filter: Option<String>,
    /// Condition expression (from `@when: ctx:enemy_units() > 0`).
    pub when_condition: Option<String>,
    /// How this script is activated.
    pub activation_mode: ActivationMode,
}

impl ScriptRegistration {
    pub fn new(name: String, source: String, events: Vec<String>, player_id: u8) -> Self {
        Self {
            name,
            source,
            events,
            tick_interval: 5, // default: run every 5 ticks (2Hz at 10Hz sim)
            player_id,
            ticks_since_last_run: 0,
            enabled: true,
            unit_filter: None,
            squad_filter: None,
            when_condition: None,
            activation_mode: ActivationMode::Auto,
        }
    }

    /// Create a ScriptRegistration from raw source using annotation parsing.
    pub fn from_source(source: &str, fallback_name: &str, player_id: u8) -> Self {
        let ann = parse_annotations(source, fallback_name);
        let mut reg = Self::new(ann.name, source.to_string(), ann.events, player_id);
        reg.tick_interval = ann.interval;
        reg.unit_filter = ann.unit_filter;
        reg.squad_filter = ann.squad_filter;
        reg.when_condition = ann.when_condition;
        reg.activation_mode = ann.activation_mode;
        reg
    }

    /// Check if this script listens for the given event name.
    pub fn listens_for(&self, event_name: &str) -> bool {
        self.events.iter().any(|e| e == event_name)
    }
}

/// Result of parsing script annotation comments.
#[derive(Debug, Clone)]
pub struct ParsedAnnotations {
    pub name: String,
    pub events: Vec<String>,
    pub interval: u32,
    pub unit_filter: Option<Vec<UnitKind>>,
    pub squad_filter: Option<String>,
    pub when_condition: Option<String>,
    pub activation_mode: ActivationMode,
}

/// Parse annotation comments from a Lua script source.
///
/// Recognized annotations:
/// - `@name: script_name`
/// - `@events: on_tick, on_enemy_spotted`
/// - `@interval: 5`
/// - `@units: Hisser, Nuisance`
/// - `@squad: skirmishers`
/// - `@when: #ctx:enemy_units() > 0`
/// - `@manual`
pub fn parse_annotations(source: &str, fallback_name: &str) -> ParsedAnnotations {
    let mut name = fallback_name.to_string();
    let mut events = vec!["on_tick".to_string()];
    let mut interval = 5u32;
    let mut unit_filter = None;
    let mut squad_filter = None;
    let mut when_condition = None;
    let mut activation_mode = ActivationMode::Auto;

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("--") {
            if !trimmed.is_empty() {
                break;
            }
            continue;
        }
        let comment = trimmed.trim_start_matches("--").trim();
        if let Some(val) = comment.strip_prefix("@name:") {
            name = val.trim().to_string();
        } else if let Some(val) = comment.strip_prefix("@events:") {
            events = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(val) = comment.strip_prefix("@interval:") {
            if let Ok(n) = val.trim().parse::<u32>() {
                interval = n;
            }
        } else if let Some(val) = comment.strip_prefix("@units:") {
            let kinds: Vec<UnitKind> = val
                .split(',')
                .filter_map(|s| s.trim().parse::<UnitKind>().ok())
                .collect();
            if !kinds.is_empty() {
                unit_filter = Some(kinds);
            }
        } else if let Some(val) = comment.strip_prefix("@squad:") {
            let sq = val.trim().to_string();
            if !sq.is_empty() {
                squad_filter = Some(sq);
            }
        } else if let Some(val) = comment.strip_prefix("@when:") {
            let cond = val.trim().to_string();
            if !cond.is_empty() {
                when_condition = Some(cond);
            }
        } else if comment == "@manual" {
            activation_mode = ActivationMode::Manual;
        }
    }

    ParsedAnnotations {
        name,
        events,
        interval,
        unit_filter,
        squad_filter,
        when_condition,
        activation_mode,
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
    let prev_enemy_ids: HashSet<EntityId> = prev.enemy_units.iter().map(|u| u.id).collect();

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
        if let Some(&prev_hp) = prev_my_health.get(&unit.id)
            && unit.health_current < prev_hp
            && !unit.is_dead
        {
            events.push(ScriptEvent::UnitAttacked {
                unit_id: unit.id,
                damage_taken: prev_hp - unit.health_current,
            });
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
    use crate::test_fixtures::{make_snapshot as empty_snapshot, make_unit_owned as make_unit};
    use cc_core::math::fixed_from_i32;

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

    #[test]
    fn new_defaults_backward_compatible() {
        let reg = ScriptRegistration::new("test".into(), "".into(), vec![], 0);
        assert!(reg.enabled);
        assert!(reg.unit_filter.is_none());
        assert!(reg.squad_filter.is_none());
        assert!(reg.when_condition.is_none());
        assert_eq!(reg.activation_mode, ActivationMode::Auto);
    }

    #[test]
    fn parse_annotations_defaults() {
        let source = "local x = 1";
        let ann = parse_annotations(source, "fallback");
        assert_eq!(ann.name, "fallback");
        assert_eq!(ann.events, vec!["on_tick"]);
        assert_eq!(ann.interval, 5);
        assert!(ann.unit_filter.is_none());
        assert!(ann.squad_filter.is_none());
        assert!(ann.when_condition.is_none());
        assert_eq!(ann.activation_mode, ActivationMode::Auto);
    }

    #[test]
    fn parse_annotations_all_fields() {
        let source = r#"-- @name: kite_script
-- @events: on_tick, on_enemy_spotted
-- @interval: 3
-- @units: Hisser, Nuisance
-- @squad: skirmishers
-- @when: #ctx:enemy_units() > 0
-- @manual

local x = ctx:my_units()
"#;
        let ann = parse_annotations(source, "fallback");
        assert_eq!(ann.name, "kite_script");
        assert_eq!(ann.events, vec!["on_tick", "on_enemy_spotted"]);
        assert_eq!(ann.interval, 3);
        assert_eq!(
            ann.unit_filter,
            Some(vec![UnitKind::Hisser, UnitKind::Nuisance])
        );
        assert_eq!(ann.squad_filter, Some("skirmishers".to_string()));
        assert_eq!(
            ann.when_condition,
            Some("#ctx:enemy_units() > 0".to_string())
        );
        assert_eq!(ann.activation_mode, ActivationMode::Manual);
    }

    #[test]
    fn parse_annotations_partial() {
        let source = "-- @name: gather\n-- @interval: 10\nlocal w = ctx:idle_units()";
        let ann = parse_annotations(source, "x");
        assert_eq!(ann.name, "gather");
        assert_eq!(ann.interval, 10);
        assert_eq!(ann.events, vec!["on_tick"]); // default preserved
        assert!(ann.unit_filter.is_none());
        assert_eq!(ann.activation_mode, ActivationMode::Auto);
    }

    #[test]
    fn parse_annotations_invalid_unit_kind_skipped() {
        let source = "-- @units: Hisser, InvalidUnit, Chonk\nlocal x = 1";
        let ann = parse_annotations(source, "test");
        assert_eq!(
            ann.unit_filter,
            Some(vec![UnitKind::Hisser, UnitKind::Chonk])
        );
    }

    #[test]
    fn from_source_creates_registration() {
        let source = "-- @name: test_script\n-- @interval: 3\n-- @manual\nlocal x = 1";
        let reg = ScriptRegistration::from_source(source, "fallback", 0);
        assert_eq!(reg.name, "test_script");
        assert_eq!(reg.tick_interval, 3);
        assert_eq!(reg.activation_mode, ActivationMode::Manual);
        assert_eq!(reg.player_id, 0);
        assert!(reg.enabled);
    }
}
