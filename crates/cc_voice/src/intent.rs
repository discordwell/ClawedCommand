/// Maps recognized voice keywords to GameCommands.
///
/// Simple 1:1 mapping for keywords that don't need context (stop, hold).
/// Parameterized commands (attack, move, build) are stubbed — they need
/// selection context and target resolution which will be added later.
use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};

use crate::events::VoiceCommandEvent;

/// Maps a VoiceCommandEvent to a GameCommand, if possible.
///
/// Returns `None` for keywords that require additional context (target,
/// selection) which isn't available from audio alone.
pub fn resolve_intent(
    event: &VoiceCommandEvent,
    selected_unit_ids: &[EntityId],
) -> Option<GameCommand> {
    if selected_unit_ids.is_empty() {
        return None;
    }

    let ids = selected_unit_ids.to_vec();

    match event.keyword.as_str() {
        // Direct mappings — no target needed
        "stop" | "hold" => Some(GameCommand::Stop { unit_ids: ids }),
        "select" | "all" => Some(GameCommand::Select { unit_ids: ids }),

        // Parameterized — need target resolution (stubbed for now)
        "attack" | "move" | "retreat" | "patrol" | "gather" | "defend" => {
            log::debug!(
                "Voice keyword '{}' requires target context — not yet implemented",
                event.keyword
            );
            None
        }

        // Build commands — need building type + placement
        "build" | "train" | "barracks" | "refinery" | "tower" => {
            log::debug!(
                "Voice keyword '{}' requires build context — not yet implemented",
                event.keyword
            );
            None
        }

        // Meta / unit names — informational, not direct commands
        "north" | "south" | "east" | "west" | "base" | "workers" | "army" | "group"
        | "pawdler" | "nuisance" | "chonk" | "hisser" | "yowler" | "mouser" => {
            log::debug!("Voice keyword '{}' is a modifier, not a standalone command", event.keyword);
            None
        }

        // Special classes — ignore
        "unknown" | "silence" => None,

        other => {
            log::warn!("Unrecognized voice keyword: '{other}'");
            None
        }
    }
}

/// Bevy system: consumes VoiceCommandEvents and pushes GameCommands.
pub fn voice_intent_system(
    mut voice_events: MessageReader<VoiceCommandEvent>,
    selected_units: Query<Entity, (With<cc_core::components::UnitType>, With<cc_core::components::Selected>)>,
    mut cmd_queue: ResMut<cc_sim::resources::CommandQueue>,
) {
    let unit_ids: Vec<EntityId> = selected_units
        .iter()
        .map(|e| EntityId(e.to_bits()))
        .collect();

    for event in voice_events.read() {
        log::info!(
            "Voice: '{}' (confidence: {:.2})",
            event.keyword,
            event.confidence
        );
        if let Some(cmd) = resolve_intent(event, &unit_ids) {
            cmd_queue.push(cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_maps_to_game_command() {
        let event = VoiceCommandEvent {
            keyword: "stop".to_string(),
            confidence: 0.95,
        };
        let ids = vec![EntityId(1), EntityId(2)];
        let cmd = resolve_intent(&event, &ids);
        assert!(cmd.is_some());
        match cmd.unwrap() {
            GameCommand::Stop { unit_ids } => {
                assert_eq!(unit_ids.len(), 2);
                assert_eq!(unit_ids[0], EntityId(1));
            }
            _ => panic!("expected Stop command"),
        }
    }

    #[test]
    fn test_hold_maps_to_stop() {
        let event = VoiceCommandEvent {
            keyword: "hold".to_string(),
            confidence: 0.88,
        };
        let ids = vec![EntityId(5)];
        let cmd = resolve_intent(&event, &ids);
        assert!(matches!(cmd, Some(GameCommand::Stop { .. })));
    }

    #[test]
    fn test_no_selection_returns_none() {
        let event = VoiceCommandEvent {
            keyword: "stop".to_string(),
            confidence: 0.99,
        };
        let cmd = resolve_intent(&event, &[]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_unknown_returns_none() {
        let event = VoiceCommandEvent {
            keyword: "unknown".to_string(),
            confidence: 0.5,
        };
        let ids = vec![EntityId(1)];
        let cmd = resolve_intent(&event, &ids);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_silence_returns_none() {
        let event = VoiceCommandEvent {
            keyword: "silence".to_string(),
            confidence: 0.99,
        };
        let ids = vec![EntityId(1)];
        let cmd = resolve_intent(&event, &ids);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_parameterized_returns_none_for_now() {
        let event = VoiceCommandEvent {
            keyword: "attack".to_string(),
            confidence: 0.92,
        };
        let ids = vec![EntityId(1)];
        let cmd = resolve_intent(&event, &ids);
        assert!(cmd.is_none(), "attack needs target — should be None for now");
    }
}
