/// Maps recognized voice keywords to GameCommands.
///
/// Voice grammar: `[agent_command] [unit_selector]* [conjunction]*`
///
/// Each keyword is recognized independently in a 1-second window.
/// The intent system accumulates keywords into a pending command,
/// then flushes when an agent command arrives with a prior target context.
///
/// Default target: all of the player's on-screen units (not just selected).
use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Building, Dead, Owner, Position, UnitKind};
use cc_core::coords::GridPos;

use crate::events::VoiceCommandEvent;

// ---------------------------------------------------------------------------
// Keyword classification
// ---------------------------------------------------------------------------

/// What kind of keyword was recognized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordRole {
    /// Agent command — triggers an action (attack, retreat, stop, etc.)
    Agent(AgentAction),
    /// Unit type name — filters target set to this unit kind
    UnitName(UnitKind),
    /// Group selector — modifies target set (all, army, workers, idle, etc.)
    Selector(SelectorKind),
    /// Conjunction — chains selectors (and, with, except, not)
    Conjunction(ConjunctionKind),
    /// Direction — cardinal direction modifier
    Direction(DirectionKind),
    /// Building name — for build/train commands
    Building(BuildingKind),
    /// Control group number
    GroupNumber(u8),
    /// Meta command (cancel, help, undo, yes, no)
    Meta(MetaAction),
    /// Intentionally ignored (unknown, silence, unimplemented faction units)
    Ignored,
    /// Unrecognized keyword — not in any match arm
    Unrecognized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAction {
    Attack,
    Retreat,
    Move,
    Defend,
    Hold,
    Patrol,
    Gather,
    Scout,
    Build,
    Train,
    Stop,
    Follow,
    Guard,
    Heal,
    Flank,
    Charge,
    Siege,
    Rally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorKind {
    All,
    Screen,
    Selected,
    Group,
    Army,
    Workers,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConjunctionKind {
    And,
    With,
    Except,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionKind {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingKind {
    Barracks,
    Refinery,
    Tower,
    Box,
    Tree,
    Market,
    Rack,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaAction {
    Base,
    Cancel,
    Help,
    Undo,
    Yes,
    No,
}

/// Classify a keyword string into its role.
pub fn classify_keyword(keyword: &str) -> KeywordRole {
    match keyword {
        // Agent commands
        "attack" => KeywordRole::Agent(AgentAction::Attack),
        "retreat" => KeywordRole::Agent(AgentAction::Retreat),
        "move" => KeywordRole::Agent(AgentAction::Move),
        "defend" => KeywordRole::Agent(AgentAction::Defend),
        "hold" => KeywordRole::Agent(AgentAction::Hold),
        "patrol" => KeywordRole::Agent(AgentAction::Patrol),
        "gather" => KeywordRole::Agent(AgentAction::Gather),
        "scout" => KeywordRole::Agent(AgentAction::Scout),
        "build" => KeywordRole::Agent(AgentAction::Build),
        "train" => KeywordRole::Agent(AgentAction::Train),
        "stop" => KeywordRole::Agent(AgentAction::Stop),
        "follow" => KeywordRole::Agent(AgentAction::Follow),
        "guard" => KeywordRole::Agent(AgentAction::Guard),
        "heal" => KeywordRole::Agent(AgentAction::Heal),
        "flank" => KeywordRole::Agent(AgentAction::Flank),
        "charge" => KeywordRole::Agent(AgentAction::Charge),
        "siege" => KeywordRole::Agent(AgentAction::Siege),
        "rally" => KeywordRole::Agent(AgentAction::Rally),

        // catGPT units (+ abbreviations)
        "pawdler" | "pawds" => KeywordRole::UnitName(UnitKind::Pawdler),
        "nuisance" => KeywordRole::UnitName(UnitKind::Nuisance),
        "chonk" => KeywordRole::UnitName(UnitKind::Chonk),
        "fox" => KeywordRole::UnitName(UnitKind::FlyingFox),
        "hisser" => KeywordRole::UnitName(UnitKind::Hisser),
        "yowler" => KeywordRole::UnitName(UnitKind::Yowler),
        "mouser" => KeywordRole::UnitName(UnitKind::Mouser),
        "catnapper" | "napper" => KeywordRole::UnitName(UnitKind::Catnapper),
        "sapper" => KeywordRole::UnitName(UnitKind::FerretSapper),
        "mech" => KeywordRole::UnitName(UnitKind::MechCommander),

        // The Clawed units (+ abbreviations)
        // Not yet in UnitKind enum — log and ignore for now
        "nibblet" | "swarmer" | "gnawer" | "shrieker" | "tunneler" | "sparks"
        | "quillback" | "whiskerwitch" | "witch" | "plaguetail" | "plague"
        | "marshal" => {
            log::debug!("Clawed unit '{keyword}' recognized but not yet in UnitKind");
            KeywordRole::Ignored
        }

        // Seekers units
        "delver" | "ironhide" | "cragback" | "warden" | "sapjaw"
        | "wardenmother" | "embermaw" | "dustclaw" | "gutripper" => {
            log::debug!("Seekers unit '{keyword}' recognized but not yet in UnitKind");
            KeywordRole::Ignored
        }

        // The Murder units
        "scrounger" | "sentinel" | "rookclaw" | "magpike" | "magpyre"
        | "jaycaller" | "jayflicker" | "dusktalon" | "hootseer" | "corvus" => {
            log::debug!("Murder unit '{keyword}' recognized but not yet in UnitKind");
            KeywordRole::Ignored
        }

        // Croak units (+ abbreviations)
        "ponderer" | "regeneron" | "regen" | "broodmother" | "brood"
        | "gulper" | "eftsaber" | "croaker" | "leapfrog" | "shellwarden"
        | "shell" | "bogwhisper" | "bog" | "murk" => {
            log::debug!("Croak unit '{keyword}' recognized but not yet in UnitKind");
            KeywordRole::Ignored
        }

        // LLAMA units
        "bandit" | "titan" | "glitch" | "patch" | "grease"
        | "drop" | "wrecker" | "diver" | "junkyard" => {
            log::debug!("LLAMA unit '{keyword}' recognized but not yet in UnitKind");
            KeywordRole::Ignored
        }

        // Selectors
        "all" | "screen" => KeywordRole::Selector(SelectorKind::All),
        "selected" => KeywordRole::Selector(SelectorKind::Selected),
        "group" => KeywordRole::Selector(SelectorKind::Group),
        "army" => KeywordRole::Selector(SelectorKind::Army),
        "workers" => KeywordRole::Selector(SelectorKind::Workers),
        "idle" => KeywordRole::Selector(SelectorKind::Idle),

        // Control group numbers
        "one" => KeywordRole::GroupNumber(1),
        "two" => KeywordRole::GroupNumber(2),
        "three" => KeywordRole::GroupNumber(3),

        // Conjunctions
        "and" => KeywordRole::Conjunction(ConjunctionKind::And),
        "with" => KeywordRole::Conjunction(ConjunctionKind::With),
        "except" => KeywordRole::Conjunction(ConjunctionKind::Except),
        "not" => KeywordRole::Conjunction(ConjunctionKind::Not),

        // Directions
        "north" => KeywordRole::Direction(DirectionKind::North),
        "south" => KeywordRole::Direction(DirectionKind::South),
        "east" => KeywordRole::Direction(DirectionKind::East),
        "west" => KeywordRole::Direction(DirectionKind::West),

        // Buildings
        "barracks" | "tree" => KeywordRole::Building(BuildingKind::Barracks), // Cat Tree = Barracks per GAME_DESIGN.md
        "refinery" | "market" => KeywordRole::Building(BuildingKind::Refinery),
        "tower" => KeywordRole::Building(BuildingKind::Tower),
        "box" => KeywordRole::Building(BuildingKind::Box),
        "post" => KeywordRole::Building(BuildingKind::Post), // Scratching Post = Research per GAME_DESIGN.md
        "rack" => KeywordRole::Building(BuildingKind::Rack),

        // Meta
        "base" => KeywordRole::Meta(MetaAction::Base),
        "cancel" => KeywordRole::Meta(MetaAction::Cancel),
        "help" => KeywordRole::Meta(MetaAction::Help),
        "undo" => KeywordRole::Meta(MetaAction::Undo),
        "yes" => KeywordRole::Meta(MetaAction::Yes),
        "no" => KeywordRole::Meta(MetaAction::No),

        // Special / unknown
        "unknown" | "silence" => KeywordRole::Ignored,

        other => {
            log::warn!("Unrecognized voice keyword: '{other}'");
            KeywordRole::Unrecognized
        }
    }
}

// ---------------------------------------------------------------------------
// Intent resolution — maps agent commands to GameCommands
// ---------------------------------------------------------------------------

/// Maps an agent action + target unit IDs to a GameCommand.
///
/// For commands that need a position target (move, patrol, attack-move),
/// we use the cursor grid position or a direction offset.
/// Commands that don't need a target (stop, hold) resolve immediately.
pub fn resolve_agent_command(
    action: AgentAction,
    unit_ids: &[EntityId],
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }

    let ids = unit_ids.to_vec();

    match action {
        // Immediate commands — no target needed
        AgentAction::Stop => Some(GameCommand::Stop { unit_ids: ids }),
        AgentAction::Hold => Some(GameCommand::HoldPosition { unit_ids: ids }),

        // Position-targeted commands — need cursor position (stubbed)
        AgentAction::Move
        | AgentAction::Patrol
        | AgentAction::Rally
        | AgentAction::Flank
        | AgentAction::Charge
        | AgentAction::Scout => {
            log::debug!(
                "Agent '{:?}' needs cursor target — not yet wired to cursor position",
                action
            );
            None
        }

        // Attack-move style — engage enemies while moving
        AgentAction::Attack | AgentAction::Siege => {
            log::debug!(
                "Agent '{:?}' needs attack-move target — not yet wired",
                action
            );
            None
        }

        // Defensive behaviors
        AgentAction::Defend | AgentAction::Guard => {
            // Defend = hold position + attack in range (same as HoldPosition for now)
            Some(GameCommand::HoldPosition { unit_ids: ids })
        }

        // Retreat — move away from nearest enemy (needs enemy positions, stubbed)
        AgentAction::Retreat => {
            log::debug!("Retreat agent needs enemy proximity data — not yet wired");
            None
        }

        // Worker commands
        AgentAction::Gather => {
            log::debug!("Gather agent needs resource target — not yet wired");
            None
        }
        AgentAction::Build | AgentAction::Train => {
            log::debug!("Build/train agent needs building context — not yet wired");
            None
        }

        // Support commands
        AgentAction::Follow | AgentAction::Heal => {
            log::debug!("Follow/heal agent needs ally target — not yet wired");
            None
        }
    }
}

/// Bevy system: consumes VoiceCommandEvents and pushes GameCommands.
///
/// Default target: all of the player's on-screen units (not just selected).
/// If a unit name keyword was recently spoken, filters to that unit type.
pub fn voice_intent_system(
    mut voice_events: MessageReader<VoiceCommandEvent>,
    // All player-owned units on screen (default target set)
    all_units: Query<(Entity, &cc_core::components::UnitType), With<Owner>>,
    // Selected units (fallback when "selected" keyword used)
    selected_units: Query<Entity, (With<cc_core::components::UnitType>, With<cc_core::components::Selected>)>,
    // Enemy units for attack targeting
    enemy_units: Query<(Entity, &Position, &Owner), Without<Dead>>,
    // Player buildings for retreat targeting
    player_buildings: Query<(&Position, &Owner, &Building)>,
    mut cmd_queue: ResMut<cc_sim::resources::CommandQueue>,
    mut pending_filter: Local<Option<UnitKind>>,
    mut pending_selector: Local<Option<SelectorKind>>,
) {
    for event in voice_events.read() {
        log::info!(
            "Voice: '{}' (confidence: {:.2})",
            event.keyword,
            event.confidence
        );

        let role = classify_keyword(&event.keyword);

        match role {
            KeywordRole::Agent(action) => {
                // Resolve target unit IDs based on pending filter/selector
                let unit_ids: Vec<EntityId> = match *pending_selector {
                    Some(SelectorKind::Selected) => {
                        selected_units.iter().map(|e| EntityId(e.to_bits())).collect()
                    }
                    Some(SelectorKind::Workers) => {
                        all_units.iter()
                            .filter(|(_, ut)| ut.kind == UnitKind::Pawdler)
                            .map(|(e, _)| EntityId(e.to_bits()))
                            .collect()
                    }
                    Some(SelectorKind::Army) => {
                        all_units.iter()
                            .filter(|(_, ut)| ut.kind != UnitKind::Pawdler)
                            .map(|(e, _)| EntityId(e.to_bits()))
                            .collect()
                    }
                    _ => {
                        // Default: filter by unit kind if specified, else all units
                        match *pending_filter {
                            Some(kind) => {
                                all_units.iter()
                                    .filter(|(_, ut)| ut.kind == kind)
                                    .map(|(e, _)| EntityId(e.to_bits()))
                                    .collect()
                            }
                            None => {
                                all_units.iter()
                                    .map(|(e, _)| EntityId(e.to_bits()))
                                    .collect()
                            }
                        }
                    }
                };

                // Attack/Retreat/Siege use ECS queries for targeting
                let cmd = match action {
                    AgentAction::Attack | AgentAction::Siege => {
                        resolve_voice_attack(&unit_ids, &enemy_units)
                    }
                    AgentAction::Retreat => {
                        resolve_voice_retreat(&unit_ids, &player_buildings)
                    }
                    _ => resolve_agent_command(action, &unit_ids),
                };
                if let Some(c) = cmd {
                    cmd_queue.push(c);
                }

                // Clear pending state after command execution
                *pending_filter = None;
                *pending_selector = None;
            }

            KeywordRole::UnitName(kind) => {
                *pending_filter = Some(kind);
                *pending_selector = None;
            }

            KeywordRole::Selector(sel) => {
                *pending_selector = Some(sel);
                *pending_filter = None;
            }

            KeywordRole::Meta(MetaAction::Cancel) => {
                // Cancel clears any pending voice state
                *pending_filter = None;
                *pending_selector = None;
            }

            // Conjunctions, directions, buildings, group numbers, meta —
            // accumulated for future multi-word command resolution
            _ => {}
        }
    }
}

/// Attack: attack-move selected units toward the centroid of visible enemies.
/// Falls back to map center (32,32) if no enemies are visible.
fn resolve_voice_attack(
    unit_ids: &[EntityId],
    enemy_units: &Query<(Entity, &Position, &Owner), Without<Dead>>,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }
    let ids = unit_ids.to_vec();

    // Compute centroid of enemy units (player_id != 0)
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut count: i64 = 0;
    for (_, pos, owner) in enemy_units.iter() {
        if owner.player_id == 0 {
            continue; // skip player's own units
        }
        let gp = pos.world.to_grid();
        sum_x += gp.x as i64;
        sum_y += gp.y as i64;
        count += 1;
    }

    let target = if count > 0 {
        GridPos::new((sum_x / count) as i32, (sum_y / count) as i32)
    } else {
        GridPos::new(32, 32) // fallback: map center
    };

    Some(GameCommand::AttackMove {
        unit_ids: ids,
        target,
    })
}

/// Retreat: move selected units toward player's base (TheBox, or any owned building).
/// Falls back to (5,5) if no buildings found.
fn resolve_voice_retreat(
    unit_ids: &[EntityId],
    player_buildings: &Query<(&Position, &Owner, &Building)>,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }
    let ids = unit_ids.to_vec();

    // Find player's TheBox first, fall back to any owned building
    let mut base_pos: Option<GridPos> = None;
    for (pos, owner, building) in player_buildings.iter() {
        if owner.player_id != 0 {
            continue;
        }
        let gp = pos.world.to_grid();
        if building.kind == cc_core::components::BuildingKind::TheBox {
            base_pos = Some(gp);
            break;
        }
        if base_pos.is_none() {
            base_pos = Some(gp);
        }
    }

    let target = base_pos.unwrap_or(GridPos::new(5, 5));
    Some(GameCommand::Move {
        unit_ids: ids,
        target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_maps_to_game_command() {
        let ids = vec![EntityId(1), EntityId(2)];
        let cmd = resolve_agent_command(AgentAction::Stop, &ids);
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
    fn test_hold_maps_to_hold_position() {
        let ids = vec![EntityId(5)];
        let cmd = resolve_agent_command(AgentAction::Hold, &ids);
        assert!(matches!(cmd, Some(GameCommand::HoldPosition { .. })));
    }

    #[test]
    fn test_defend_maps_to_hold_position() {
        let ids = vec![EntityId(3)];
        let cmd = resolve_agent_command(AgentAction::Defend, &ids);
        assert!(matches!(cmd, Some(GameCommand::HoldPosition { .. })));
    }

    #[test]
    fn test_empty_unit_ids_returns_none() {
        let cmd = resolve_agent_command(AgentAction::Stop, &[]);
        assert!(cmd.is_none());
    }

    #[test]
    fn test_position_commands_return_none_for_now() {
        let ids = vec![EntityId(1)];
        assert!(resolve_agent_command(AgentAction::Move, &ids).is_none());
        assert!(resolve_agent_command(AgentAction::Patrol, &ids).is_none());
        assert!(resolve_agent_command(AgentAction::Retreat, &ids).is_none());
        assert!(resolve_agent_command(AgentAction::Attack, &ids).is_none());
    }

    // Keyword classification tests

    #[test]
    fn test_classify_agent_commands() {
        assert_eq!(classify_keyword("attack"), KeywordRole::Agent(AgentAction::Attack));
        assert_eq!(classify_keyword("retreat"), KeywordRole::Agent(AgentAction::Retreat));
        assert_eq!(classify_keyword("stop"), KeywordRole::Agent(AgentAction::Stop));
        assert_eq!(classify_keyword("rally"), KeywordRole::Agent(AgentAction::Rally));
    }

    #[test]
    fn test_classify_catgpt_units() {
        assert_eq!(classify_keyword("chonk"), KeywordRole::UnitName(UnitKind::Chonk));
        assert_eq!(classify_keyword("fox"), KeywordRole::UnitName(UnitKind::FlyingFox));
        assert_eq!(classify_keyword("sapper"), KeywordRole::UnitName(UnitKind::FerretSapper));
        assert_eq!(classify_keyword("mech"), KeywordRole::UnitName(UnitKind::MechCommander));
    }

    #[test]
    fn test_classify_abbreviations() {
        // Abbreviations map to same UnitKind as full name
        assert_eq!(classify_keyword("pawds"), KeywordRole::UnitName(UnitKind::Pawdler));
        assert_eq!(classify_keyword("pawdler"), KeywordRole::UnitName(UnitKind::Pawdler));
        assert_eq!(classify_keyword("napper"), KeywordRole::UnitName(UnitKind::Catnapper));
        assert_eq!(classify_keyword("catnapper"), KeywordRole::UnitName(UnitKind::Catnapper));
    }

    #[test]
    fn test_classify_other_faction_units_are_ignored() {
        // Other faction units are recognized but not yet in UnitKind
        assert_eq!(classify_keyword("nibblet"), KeywordRole::Ignored);
        assert_eq!(classify_keyword("corvus"), KeywordRole::Ignored);
        assert_eq!(classify_keyword("gulper"), KeywordRole::Ignored);
        assert_eq!(classify_keyword("titan"), KeywordRole::Ignored);
    }

    #[test]
    fn test_classify_selectors() {
        assert_eq!(classify_keyword("all"), KeywordRole::Selector(SelectorKind::All));
        assert_eq!(classify_keyword("screen"), KeywordRole::Selector(SelectorKind::All));
        assert_eq!(classify_keyword("army"), KeywordRole::Selector(SelectorKind::Army));
        assert_eq!(classify_keyword("workers"), KeywordRole::Selector(SelectorKind::Workers));
    }

    #[test]
    fn test_classify_conjunctions() {
        assert_eq!(classify_keyword("and"), KeywordRole::Conjunction(ConjunctionKind::And));
        assert_eq!(classify_keyword("except"), KeywordRole::Conjunction(ConjunctionKind::Except));
    }

    #[test]
    fn test_classify_directions() {
        assert_eq!(classify_keyword("north"), KeywordRole::Direction(DirectionKind::North));
        assert_eq!(classify_keyword("west"), KeywordRole::Direction(DirectionKind::West));
    }

    #[test]
    fn test_classify_buildings() {
        assert_eq!(classify_keyword("tower"), KeywordRole::Building(BuildingKind::Tower));
        assert_eq!(classify_keyword("box"), KeywordRole::Building(BuildingKind::Box));
    }

    #[test]
    fn test_classify_meta() {
        assert_eq!(classify_keyword("cancel"), KeywordRole::Meta(MetaAction::Cancel));
        assert_eq!(classify_keyword("yes"), KeywordRole::Meta(MetaAction::Yes));
    }

    #[test]
    fn test_classify_group_numbers() {
        assert_eq!(classify_keyword("one"), KeywordRole::GroupNumber(1));
        assert_eq!(classify_keyword("three"), KeywordRole::GroupNumber(3));
    }

    #[test]
    fn test_classify_special_ignored() {
        assert_eq!(classify_keyword("unknown"), KeywordRole::Ignored);
        assert_eq!(classify_keyword("silence"), KeywordRole::Ignored);
    }

    #[test]
    fn test_all_labels_file_keywords_classified() {
        // Load labels.txt directly — single source of truth for vocabulary.
        // This catches drift between config.yaml/labels.txt and classify_keyword.
        let labels_text = include_str!("../../../assets/voice/labels.txt");
        let labels: Vec<&str> = labels_text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(labels.len(), 118, "labels.txt should have exactly 118 entries");

        for label in &labels {
            let role = classify_keyword(label);
            // Every label must have an explicit match arm. Unrecognized means
            // the label fell through to the `other` fallback — a bug.
            assert_ne!(
                role,
                KeywordRole::Unrecognized,
                "Label '{label}' hit the fallback branch — add it to classify_keyword"
            );
        }
    }
}
