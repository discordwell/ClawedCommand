/// Maps voice transcriptions to GameCommands.
///
/// Voice grammar: `[unit_selector]* [agent_command] [direction]?`
///
/// Whisper produces full text on PTT release (e.g. "all attack", "hisser stop",
/// "move north"). The intent system parses all words in one tick — no cross-tick
/// accumulation needed.
///
/// Default target: all of the player's on-screen units (not just selected).
use bevy::ecs::world::EntityWorldMut;
use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{Building, CursorGridPos, Dead, Faction, Owner, Position, StatModifiers, UnitKind, VoiceBuffed};
use cc_core::status_effects::{StatusEffectId, StatusEffects, StatusInstance};
use cc_core::coords::GridPos;
use cc_sim::ai::fsm::{FactionMap, faction_map};
use cc_sim::resources::MapResource;

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
    /// Units within ~10 tiles of the cursor position.
    Nearby,
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
        "nearby" => KeywordRole::Selector(SelectorKind::Nearby),

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
// Full-text voice command parsing
// ---------------------------------------------------------------------------

/// Parsed result of a full voice transcription.
#[derive(Debug, Clone, Default)]
pub struct ParsedVoiceCommand {
    pub action: Option<AgentAction>,
    pub selector: Option<SelectorKind>,
    pub unit_filter: Option<UnitKind>,
    pub direction: Option<DirectionKind>,
    pub building: Option<BuildingKind>,
}

/// Parse a full transcription text into a structured voice command.
///
/// Normalizes the text (lowercase, strips punctuation), splits into words,
/// and classifies each word. Collects the first occurrence of each role type.
pub fn parse_voice_text(text: &str) -> ParsedVoiceCommand {
    let mut result = ParsedVoiceCommand::default();

    // Normalize: strip non-alphanumeric (keep spaces), ensure lowercase.
    // Whisper already returns lowercase, but normalize defensively.
    let normalized: String = text
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();

    for word in normalized.split_whitespace() {
        let role = classify_keyword(word);
        match role {
            KeywordRole::Agent(action) => {
                if result.action.is_none() {
                    result.action = Some(action);
                }
            }
            KeywordRole::UnitName(kind) => {
                if result.unit_filter.is_none() {
                    result.unit_filter = Some(kind);
                }
            }
            KeywordRole::Selector(sel) => {
                if result.selector.is_none() {
                    result.selector = Some(sel);
                }
            }
            KeywordRole::Direction(dir) => {
                if result.direction.is_none() {
                    result.direction = Some(dir);
                }
            }
            KeywordRole::Building(bk) => {
                if result.building.is_none() {
                    result.building = Some(bk);
                }
            }
            // Meta Cancel resets everything parsed so far
            KeywordRole::Meta(MetaAction::Cancel) => {
                result = ParsedVoiceCommand::default();
            }
            // Conjunctions, group numbers, other meta, ignored, unrecognized — skip
            _ => {}
        }
    }

    result
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

        // Defensive behaviors
        AgentAction::Defend | AgentAction::Guard => {
            Some(GameCommand::HoldPosition { unit_ids: ids })
        }

        // Worker commands (still need context — gather needs resource target)
        AgentAction::Gather => {
            log::debug!("Gather agent needs resource target — not yet wired");
            None
        }
        AgentAction::Train => {
            log::debug!("Train agent needs building context — not yet wired");
            None
        }

        // Support commands (still need ally target)
        AgentAction::Follow | AgentAction::Heal => {
            log::debug!("Follow/heal agent needs ally target — not yet wired");
            None
        }

        // Position-targeted, attack, retreat, build — handled by voice_intent_system directly
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Screen-relative direction → isometric grid offsets
// ---------------------------------------------------------------------------

/// Distance in tiles to offset when a direction keyword is spoken.
const DIRECTION_OFFSET_TILES: i32 = 15;

/// Convert a screen-relative direction to an isometric grid offset.
///
/// Screen up (North) → grid NW (-x, -y)
/// Screen down (South) → grid SE (+x, +y)
/// Screen right (East) → grid NE (+x, -y)
/// Screen left (West) → grid SW (-x, +y)
pub fn direction_to_grid_offset(dir: DirectionKind) -> (i32, i32) {
    match dir {
        DirectionKind::North => (-DIRECTION_OFFSET_TILES, -DIRECTION_OFFSET_TILES),
        DirectionKind::South => (DIRECTION_OFFSET_TILES, DIRECTION_OFFSET_TILES),
        DirectionKind::East => (DIRECTION_OFFSET_TILES, -DIRECTION_OFFSET_TILES),
        DirectionKind::West => (-DIRECTION_OFFSET_TILES, DIRECTION_OFFSET_TILES),
    }
}

// ---------------------------------------------------------------------------
// Position-targeted command resolution
// ---------------------------------------------------------------------------

/// Resolve a move/patrol/scout command with optional direction offset.
///
/// If a direction is set: offset from map center by ~15 tiles in that direction.
/// Otherwise: move toward enemy centroid (same logic as attack targeting).
pub fn resolve_voice_move(
    unit_ids: &[EntityId],
    direction: Option<DirectionKind>,
    enemy_centroid: Option<GridPos>,
    map_width: u32,
    map_height: u32,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }
    let target = resolve_move_target(direction, enemy_centroid, map_width, map_height);
    Some(GameCommand::Move {
        unit_ids: unit_ids.to_vec(),
        target,
    })
}

/// Resolve a flank command: offset perpendicular to the enemy direction.
pub fn resolve_voice_flank(
    unit_ids: &[EntityId],
    direction: Option<DirectionKind>,
    enemy_centroid: Option<GridPos>,
    map_width: u32,
    map_height: u32,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }

    // Flank: if direction given, rotate 90° clockwise for the offset
    let perp_dir = direction.map(|d| match d {
        DirectionKind::North => DirectionKind::East,
        DirectionKind::East => DirectionKind::South,
        DirectionKind::South => DirectionKind::West,
        DirectionKind::West => DirectionKind::North,
    });

    let target = if perp_dir.is_some() {
        resolve_move_target(perp_dir, enemy_centroid, map_width, map_height)
    } else if let Some(ec) = enemy_centroid {
        // No direction: offset perpendicular to the line from map center to enemy
        let cx = map_width as i32 / 2;
        let cy = map_height as i32 / 2;
        let dx = ec.y - cy; // rotate 90°
        let dy = -(ec.x - cx);
        GridPos::new(
            (ec.x + dx.signum() * DIRECTION_OFFSET_TILES).clamp(0, map_width as i32 - 1),
            (ec.y + dy.signum() * DIRECTION_OFFSET_TILES).clamp(0, map_height as i32 - 1),
        )
    } else {
        GridPos::new(map_width as i32 / 2, map_height as i32 / 2)
    };

    Some(GameCommand::Move {
        unit_ids: unit_ids.to_vec(),
        target,
    })
}

// ---------------------------------------------------------------------------
// Build command resolution
// ---------------------------------------------------------------------------

/// Map a voice building keyword to a game BuildingKind using the player's faction map.
///
/// Voice keywords use generic role names (barracks, refinery, tower); the FactionMap
/// translates them to faction-specific buildings (e.g. CatTree for catGPT, Rookery for Murder).
pub fn voice_building_to_game_building(
    voice_kind: BuildingKind,
    fmap: &FactionMap,
) -> cc_core::components::BuildingKind {
    match voice_kind {
        BuildingKind::Barracks | BuildingKind::Tree => fmap.barracks,
        BuildingKind::Refinery | BuildingKind::Market => fmap.resource_depot,
        BuildingKind::Tower => fmap.defense_tower,
        BuildingKind::Box => fmap.hq,
        BuildingKind::Rack => fmap.tech,
        BuildingKind::Post => fmap.research,
    }
}

/// Infer the player's faction from their HQ building kind.
fn infer_faction_from_hq(hq_kind: cc_core::components::BuildingKind) -> Faction {
    match hq_kind {
        cc_core::components::BuildingKind::TheBox => Faction::CatGpt,
        cc_core::components::BuildingKind::TheBurrow => Faction::TheClawed,
        cc_core::components::BuildingKind::TheSett => Faction::SeekersOfTheDeep,
        cc_core::components::BuildingKind::TheParliament => Faction::TheMurder,
        cc_core::components::BuildingKind::TheDumpster => Faction::Llama,
        cc_core::components::BuildingKind::TheGrotto => Faction::Croak,
        _ => Faction::CatGpt, // fallback
    }
}

/// Find a valid build position near `center` using ring-scan expansion.
///
/// Expands in rings from radius 1..max_radius, checking each tile for
/// passability and no existing building.
pub fn find_voice_build_position(
    center: GridPos,
    map: &cc_core::map::GameMap,
    occupied: &[GridPos],
    max_radius: i32,
) -> Option<GridPos> {
    for radius in 0..=max_radius {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                // Only check the ring boundary, not interior (except radius 0)
                if radius > 0 && dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                let pos = GridPos::new(center.x + dx, center.y + dy);
                if map.is_passable(pos) && !occupied.contains(&pos) {
                    return Some(pos);
                }
            }
        }
    }
    None
}

/// Duration of the voice-command SpeedBuff (effectively permanent).
const VOICE_BUFF_DURATION: u32 = 9999;

/// Apply the golden voice-command SpeedBuff to a set of entities.
///
/// Same pattern as `voice_demo::apply_voice_buff` but operates on a batch.
pub fn apply_voice_buff_to_entities(
    commands: &mut Commands,
    entities: &[Entity],
    status_query: &Query<Option<&StatusEffects>, Without<Dead>>,
) {
    for &entity in entities {
        commands.entity(entity).insert(VoiceBuffed);

        let has_status = status_query
            .get(entity)
            .ok()
            .flatten()
            .is_some();

        let buff = StatusInstance {
            effect: StatusEffectId::SpeedBuff,
            remaining_ticks: VOICE_BUFF_DURATION,
            stacks: 1,
            source: EntityId(0),
        };

        if has_status {
            commands.entity(entity).queue(move |mut entity_world: EntityWorldMut| {
                if let Some(mut se) = entity_world.get_mut::<StatusEffects>() {
                    se.effects.push(buff);
                }
                if entity_world.get::<StatModifiers>().is_none() {
                    entity_world.insert(StatModifiers::default());
                }
            });
        } else {
            let mut effects = StatusEffects::default();
            effects.effects.push(buff);
            commands.entity(entity).insert((effects, StatModifiers::default()));
        }
    }
}

/// Bevy system: consumes VoiceCommandEvents and pushes GameCommands.
///
/// Parses full transcription text in one tick — no cross-tick accumulation.
/// Default target: all of the player's on-screen units (not just selected).
pub fn voice_intent_system(
    mut commands: Commands,
    mut voice_events: MessageReader<VoiceCommandEvent>,
    // All living units — filter by owner.player_id in code
    all_units: Query<(Entity, &cc_core::components::UnitType, &Position, &Owner), Without<Dead>>,
    // Selected units (fallback when "selected" keyword used)
    selected_units: Query<Entity, (With<cc_core::components::UnitType>, With<cc_core::components::Selected>)>,
    // Player buildings for retreat, build-site occupation, and rally
    player_buildings: Query<(Entity, &Position, &Owner, &Building)>,
    map_res: Res<MapResource>,
    cursor_grid: Res<CursorGridPos>,
    mut cmd_queue: ResMut<cc_sim::resources::CommandQueue>,
    mut voice_override: ResMut<cc_sim::resources::VoiceOverride>,
    status_query: Query<Option<&StatusEffects>, Without<Dead>>,
    restrictions: Option<Res<cc_sim::campaign::mutator_state::ControlRestrictions>>,
) {
    // Gate: skip voice commands if voice is disabled by mission mutator
    if restrictions.as_ref().is_some_and(|r| !r.voice_enabled) {
        return;
    }

    for event in voice_events.read() {
        log::info!("Voice text: \"{}\"", event.text);

        let parsed = parse_voice_text(&event.text);

        let Some(action) = parsed.action else {
            log::debug!("No action keyword found in transcription — ignoring");
            continue;
        };

        // Resolve target unit IDs based on parsed selector/filter
        // Filter to player's own living units only
        let own_units = || {
            all_units.iter().filter(|(_, _, _, owner)| owner.player_id == 0)
        };

        let (entities, unit_ids): (Vec<Entity>, Vec<EntityId>) = match parsed.selector {
            Some(SelectorKind::Selected) => {
                selected_units.iter().map(|e| (e, EntityId(e.to_bits()))).unzip()
            }
            Some(SelectorKind::Workers) => {
                own_units()
                    .filter(|(_, ut, _, _)| ut.kind == UnitKind::Pawdler)
                    .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                    .unzip()
            }
            Some(SelectorKind::Army) => {
                own_units()
                    .filter(|(_, ut, _, _)| ut.kind != UnitKind::Pawdler)
                    .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                    .unzip()
            }
            Some(SelectorKind::Nearby) => {
                // Filter to own units within 10 tiles (Chebyshev) of cursor
                if let Some(cursor) = cursor_grid.pos {
                    own_units()
                        .filter(|(_, _, pos, _)| {
                            let gp = pos.world.to_grid();
                            let dx = (gp.x - cursor.x).abs();
                            let dy = (gp.y - cursor.y).abs();
                            dx.max(dy) <= 10
                        })
                        .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                        .unzip()
                } else {
                    // No cursor position — fall back to all own units
                    own_units()
                        .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                        .unzip()
                }
            }
            _ => {
                // Default: filter by unit kind if specified, else all own units
                match parsed.unit_filter {
                    Some(kind) => {
                        own_units()
                            .filter(|(_, ut, _, _)| ut.kind == kind)
                            .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                            .unzip()
                    }
                    None => {
                        own_units()
                            .map(|(e, _, _, _)| (e, EntityId(e.to_bits())))
                            .unzip()
                    }
                }
            }
        };

        // Compute enemy centroid for position-targeted commands
        let enemy_centroid = compute_enemy_centroid(&all_units);
        let mw = map_res.map.width;
        let mh = map_res.map.height;

        let cmd = match action {
            // Attack/Siege → attack-move toward enemy centroid
            AgentAction::Attack | AgentAction::Siege => {
                resolve_voice_attack(&unit_ids, &all_units, &map_res)
            }
            // Retreat → move to base
            AgentAction::Retreat => {
                resolve_voice_retreat(&unit_ids, &player_buildings)
            }
            // Charge → attack-move toward direction or enemy centroid
            AgentAction::Charge => {
                let target = resolve_move_target(
                    parsed.direction,
                    enemy_centroid,
                    mw,
                    mh,
                );
                if !unit_ids.is_empty() {
                    Some(GameCommand::AttackMove {
                        unit_ids: unit_ids.clone(),
                        target,
                    })
                } else {
                    None
                }
            }
            // Flank → move perpendicular to enemy direction
            AgentAction::Flank => {
                resolve_voice_flank(
                    &unit_ids,
                    parsed.direction,
                    enemy_centroid,
                    mw,
                    mh,
                )
            }
            // Move/Patrol/Scout → move in direction or toward enemy
            AgentAction::Move | AgentAction::Patrol | AgentAction::Scout => {
                resolve_voice_move(
                    &unit_ids,
                    parsed.direction,
                    enemy_centroid,
                    mw,
                    mh,
                )
            }
            // Rally → set rally point on first owned building
            AgentAction::Rally => {
                let target = resolve_move_target(
                    parsed.direction,
                    cursor_grid.pos,
                    mw,
                    mh,
                );
                // Find first owned building
                let mut rally_cmd = None;
                for (entity, _, owner, _) in player_buildings.iter() {
                    if owner.player_id == 0 {
                        rally_cmd = Some(GameCommand::SetRallyPoint {
                            building: EntityId(entity.to_bits()),
                            target,
                        });
                        break;
                    }
                }
                rally_cmd
            }
            // Build → find nearest worker + build site
            AgentAction::Build => {
                if let Some(voice_bk) = parsed.building {
                    // Infer faction from player's HQ building
                    let player_faction = player_buildings
                        .iter()
                        .find(|(_, _, owner, b)| {
                            owner.player_id == 0 && b.kind.is_hq()
                        })
                        .map(|(_, _, _, b)| infer_faction_from_hq(b.kind))
                        .unwrap_or(Faction::CatGpt);

                    let fmap = faction_map(player_faction);
                    let game_bk =
                        voice_building_to_game_building(voice_bk, &fmap);

                    let center = cursor_grid.pos.unwrap_or(GridPos::new(
                        mw as i32 / 2,
                        mh as i32 / 2,
                    ));

                    // Collect all building positions (all players)
                    let occupied: Vec<GridPos> = player_buildings
                        .iter()
                        .map(|(_, pos, _, _)| pos.world.to_grid())
                        .collect();

                    // Find a valid build site
                    let build_pos = find_voice_build_position(
                        center,
                        &map_res.map,
                        &occupied,
                        20,
                    );

                    // Find nearest own worker (any faction's worker type)
                    let builder = own_units()
                        .filter(|(_, ut, _, _)| ut.kind.is_worker())
                        .min_by_key(|(_, _, pos, _)| {
                            let gp = pos.world.to_grid();
                            (gp.x - center.x).abs() + (gp.y - center.y).abs()
                        })
                        .map(|(e, _, _, _)| EntityId(e.to_bits()));

                    match (builder, build_pos) {
                        (Some(b), Some(bp)) => Some(GameCommand::Build {
                            builder: b,
                            building_kind: game_bk,
                            position: bp,
                        }),
                        _ => {
                            log::debug!(
                                "Voice build: no builder or build site found"
                            );
                            None
                        }
                    }
                } else {
                    log::debug!(
                        "Voice 'build' without a building keyword — ignoring"
                    );
                    None
                }
            }
            // Everything else (stop, hold, defend, guard, etc.)
            _ => resolve_agent_command(action, &unit_ids),
        };

        if let Some(c) = cmd {
            cmd_queue.push_sourced(Some(0), cc_core::commands::CommandSource::VoiceCommand, c);

            // Suppress script/AI commands for these units while
            // the voice command is active.
            voice_override.set(&unit_ids);

            // Apply golden speed buff for combat commands
            if matches!(action, AgentAction::Attack | AgentAction::Charge | AgentAction::Siege) {
                apply_voice_buff_to_entities(&mut commands, &entities, &status_query);
            }
        }
    }
}

/// Compute the centroid of all enemy (non-player-0) units.
fn compute_enemy_centroid(
    all_units: &Query<
        (Entity, &cc_core::components::UnitType, &Position, &Owner),
        Without<Dead>,
    >,
) -> Option<GridPos> {
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut count: i64 = 0;
    for (_, _, pos, owner) in all_units.iter() {
        if owner.player_id == 0 {
            continue;
        }
        let gp = pos.world.to_grid();
        sum_x += gp.x as i64;
        sum_y += gp.y as i64;
        count += 1;
    }
    if count > 0 {
        Some(GridPos::new(
            (sum_x / count) as i32,
            (sum_y / count) as i32,
        ))
    } else {
        None
    }
}

/// Resolve a move target from direction, enemy centroid, or map center.
fn resolve_move_target(
    direction: Option<DirectionKind>,
    fallback: Option<GridPos>,
    map_width: u32,
    map_height: u32,
) -> GridPos {
    if let Some(dir) = direction {
        let cx = map_width as i32 / 2;
        let cy = map_height as i32 / 2;
        let (dx, dy) = direction_to_grid_offset(dir);
        GridPos::new(
            (cx + dx).clamp(0, map_width as i32 - 1),
            (cy + dy).clamp(0, map_height as i32 - 1),
        )
    } else if let Some(pos) = fallback {
        pos
    } else {
        GridPos::new(map_width as i32 / 2, map_height as i32 / 2)
    }
}

/// Attack: attack-move selected units toward the centroid of visible enemies.
/// Falls back to map center if no enemies are visible.
fn resolve_voice_attack(
    unit_ids: &[EntityId],
    all_units: &Query<
        (Entity, &cc_core::components::UnitType, &Position, &Owner),
        Without<Dead>,
    >,
    map_res: &MapResource,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }
    let ids = unit_ids.to_vec();

    let centroid = compute_enemy_centroid(all_units);
    let target = centroid.unwrap_or(GridPos::new(
        map_res.map.width as i32 / 2,
        map_res.map.height as i32 / 2,
    ));

    Some(GameCommand::AttackMove {
        unit_ids: ids,
        target,
    })
}

/// Retreat: move selected units toward player's base (TheBox, or any owned building).
/// Falls back to (5,5) if no buildings found.
fn resolve_voice_retreat(
    unit_ids: &[EntityId],
    player_buildings: &Query<(Entity, &Position, &Owner, &Building)>,
) -> Option<GameCommand> {
    if unit_ids.is_empty() {
        return None;
    }
    let ids = unit_ids.to_vec();

    // Find player's TheBox first, fall back to any owned building
    let mut base_pos: Option<GridPos> = None;
    for (_, pos, owner, building) in player_buildings.iter() {
        if owner.player_id != 0 {
            continue;
        }
        let gp = pos.world.to_grid();
        if building.kind.is_hq() {
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
    fn test_position_commands_delegated_to_system() {
        // Position-targeted commands return None from resolve_agent_command
        // because they are handled directly in voice_intent_system
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
    fn test_classify_nearby() {
        assert_eq!(
            classify_keyword("nearby"),
            KeywordRole::Selector(SelectorKind::Nearby)
        );
    }

    #[test]
    fn test_voice_building_mapping_catgpt() {
        use cc_core::components::BuildingKind as GBK;

        let fmap = faction_map(Faction::CatGpt);
        assert_eq!(voice_building_to_game_building(BuildingKind::Barracks, &fmap), GBK::CatTree);
        assert_eq!(voice_building_to_game_building(BuildingKind::Tree, &fmap), GBK::CatTree);
        assert_eq!(voice_building_to_game_building(BuildingKind::Refinery, &fmap), GBK::FishMarket);
        assert_eq!(voice_building_to_game_building(BuildingKind::Market, &fmap), GBK::FishMarket);
        assert_eq!(voice_building_to_game_building(BuildingKind::Tower, &fmap), GBK::LaserPointer);
        assert_eq!(voice_building_to_game_building(BuildingKind::Box, &fmap), GBK::TheBox);
        assert_eq!(voice_building_to_game_building(BuildingKind::Rack, &fmap), GBK::ServerRack);
        assert_eq!(voice_building_to_game_building(BuildingKind::Post, &fmap), GBK::ScratchingPost);
    }

    #[test]
    fn test_voice_building_mapping_other_factions() {
        use cc_core::components::BuildingKind as GBK;

        // The Murder: "barracks" → Rookery, "tower" → Watchtower
        let murder = faction_map(Faction::TheMurder);
        assert_eq!(voice_building_to_game_building(BuildingKind::Barracks, &murder), GBK::Rookery);
        assert_eq!(voice_building_to_game_building(BuildingKind::Tower, &murder), GBK::Watchtower);

        // Croak: "barracks" → SpawningPools, "refinery" → LilyMarket
        let croak = faction_map(Faction::Croak);
        assert_eq!(voice_building_to_game_building(BuildingKind::Barracks, &croak), GBK::SpawningPools);
        assert_eq!(voice_building_to_game_building(BuildingKind::Refinery, &croak), GBK::LilyMarket);
    }

    #[test]
    fn test_infer_faction_from_hq() {
        use cc_core::components::BuildingKind as GBK;

        assert_eq!(infer_faction_from_hq(GBK::TheBox), Faction::CatGpt);
        assert_eq!(infer_faction_from_hq(GBK::TheBurrow), Faction::TheClawed);
        assert_eq!(infer_faction_from_hq(GBK::TheSett), Faction::SeekersOfTheDeep);
        assert_eq!(infer_faction_from_hq(GBK::TheParliament), Faction::TheMurder);
        assert_eq!(infer_faction_from_hq(GBK::TheDumpster), Faction::Llama);
        assert_eq!(infer_faction_from_hq(GBK::TheGrotto), Faction::Croak);
        // Non-HQ falls back to CatGpt
        assert_eq!(infer_faction_from_hq(GBK::CatTree), Faction::CatGpt);
    }

    #[test]
    fn test_direction_offsets() {
        // North (screen up) → grid (-15, -15)
        assert_eq!(direction_to_grid_offset(DirectionKind::North), (-15, -15));
        // South (screen down) → grid (+15, +15)
        assert_eq!(direction_to_grid_offset(DirectionKind::South), (15, 15));
        // East (screen right) → grid (+15, -15)
        assert_eq!(direction_to_grid_offset(DirectionKind::East), (15, -15));
        // West (screen left) → grid (-15, +15)
        assert_eq!(direction_to_grid_offset(DirectionKind::West), (-15, 15));
    }

    #[test]
    fn test_resolve_voice_move_with_direction() {
        let ids = vec![EntityId(1), EntityId(2)];

        // Move north on a 64x64 map: center (32,32) + offset (-15,-15) = (17,17)
        let cmd = resolve_voice_move(&ids, Some(DirectionKind::North), None, 64, 64);
        assert!(cmd.is_some());
        match cmd.unwrap() {
            GameCommand::Move { unit_ids, target } => {
                assert_eq!(unit_ids.len(), 2);
                assert_eq!(target, GridPos::new(17, 17));
            }
            _ => panic!("expected Move command"),
        }
    }

    #[test]
    fn test_resolve_voice_move_without_direction_uses_enemy() {
        let ids = vec![EntityId(1)];
        let enemy = Some(GridPos::new(50, 50));

        let cmd = resolve_voice_move(&ids, None, enemy, 64, 64);
        match cmd.unwrap() {
            GameCommand::Move { target, .. } => {
                assert_eq!(target, GridPos::new(50, 50));
            }
            _ => panic!("expected Move"),
        }
    }

    #[test]
    fn test_resolve_voice_move_clamps_to_map_bounds() {
        let ids = vec![EntityId(1)];
        // North on a 20x20 map: center(10,10) + (-15,-15) = (-5,-5) → clamped to (0,0)
        let cmd = resolve_voice_move(&ids, Some(DirectionKind::North), None, 20, 20);
        match cmd.unwrap() {
            GameCommand::Move { target, .. } => {
                assert_eq!(target, GridPos::new(0, 0));
            }
            _ => panic!("expected Move"),
        }
    }

    #[test]
    fn test_find_voice_build_position_center() {
        use cc_core::map::GameMap;
        let map = GameMap::new(16, 16); // all grass = passable
        let occupied = vec![];
        let pos = find_voice_build_position(GridPos::new(8, 8), &map, &occupied, 5);
        // Should return center itself since it's passable and unoccupied
        assert_eq!(pos, Some(GridPos::new(8, 8)));
    }

    #[test]
    fn test_find_voice_build_position_occupied_center() {
        use cc_core::map::GameMap;
        let map = GameMap::new(16, 16);
        let occupied = vec![GridPos::new(8, 8)];
        let pos = find_voice_build_position(GridPos::new(8, 8), &map, &occupied, 5);
        // Center is occupied, should find an adjacent tile
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert_ne!(p, GridPos::new(8, 8));
        let dist = (p.x - 8).abs().max((p.y - 8).abs());
        assert!(dist <= 1, "should find a tile at ring 1, got distance {dist}");
    }

    #[test]
    fn test_all_labels_file_keywords_classified() {
        // Load labels.txt directly — single source of truth for vocabulary.
        // This catches drift between config.yaml/labels.txt and classify_keyword.
        let labels_text = include_str!("../../../assets/voice/labels.txt");
        let labels: Vec<&str> = labels_text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(labels.len(), 119, "labels.txt should have exactly 119 entries");

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

    #[test]
    fn test_apply_voice_buff_inserts_components() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();

        // Spawn two entities — one with existing StatusEffects, one without
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn(StatusEffects::default()).id();
        let entities = vec![e1, e2];

        // Run the buff application via a one-shot system
        let entities_clone = entities.clone();
        let _ = world.run_system_once(move |mut commands: Commands, status_q: Query<Option<&StatusEffects>, Without<Dead>>| {
            apply_voice_buff_to_entities(&mut commands, &entities_clone, &status_q);
        });

        // Both should have VoiceBuffed marker
        assert!(world.get::<VoiceBuffed>(e1).is_some(), "e1 should have VoiceBuffed");
        assert!(world.get::<VoiceBuffed>(e2).is_some(), "e2 should have VoiceBuffed");

        // Both should have StatusEffects with SpeedBuff
        let se1 = world.get::<StatusEffects>(e1).expect("e1 should have StatusEffects");
        assert_eq!(se1.effects.len(), 1);
        assert_eq!(se1.effects[0].effect, StatusEffectId::SpeedBuff);
        assert_eq!(se1.effects[0].remaining_ticks, VOICE_BUFF_DURATION);

        let se2 = world.get::<StatusEffects>(e2).expect("e2 should have StatusEffects");
        assert_eq!(se2.effects.len(), 1);
        assert_eq!(se2.effects[0].effect, StatusEffectId::SpeedBuff);

        // Both should have StatModifiers
        assert!(world.get::<StatModifiers>(e1).is_some(), "e1 should have StatModifiers");
        assert!(world.get::<StatModifiers>(e2).is_some(), "e2 should have StatModifiers");
    }

    // parse_voice_text tests

    #[test]
    fn test_parse_simple_command() {
        let parsed = parse_voice_text("attack");
        assert_eq!(parsed.action, Some(AgentAction::Attack));
        assert_eq!(parsed.selector, None);
        assert_eq!(parsed.unit_filter, None);
        assert_eq!(parsed.direction, None);
    }

    #[test]
    fn test_parse_selector_and_command() {
        let parsed = parse_voice_text("all attack");
        assert_eq!(parsed.action, Some(AgentAction::Attack));
        assert_eq!(parsed.selector, Some(SelectorKind::All));
    }

    #[test]
    fn test_parse_unit_filter_and_command() {
        let parsed = parse_voice_text("hisser stop");
        assert_eq!(parsed.action, Some(AgentAction::Stop));
        assert_eq!(parsed.unit_filter, Some(UnitKind::Hisser));
    }

    #[test]
    fn test_parse_direction_command() {
        let parsed = parse_voice_text("move north");
        assert_eq!(parsed.action, Some(AgentAction::Move));
        assert_eq!(parsed.direction, Some(DirectionKind::North));
    }

    #[test]
    fn test_parse_building_build() {
        let parsed = parse_voice_text("tower build");
        assert_eq!(parsed.action, Some(AgentAction::Build));
        assert_eq!(parsed.building, Some(BuildingKind::Tower));
    }

    #[test]
    fn test_parse_strips_punctuation() {
        let parsed = parse_voice_text("All attack!");
        assert_eq!(parsed.action, Some(AgentAction::Attack));
        assert_eq!(parsed.selector, Some(SelectorKind::All));
    }

    #[test]
    fn test_parse_no_action() {
        let parsed = parse_voice_text("hisser north");
        assert_eq!(parsed.action, None);
        assert_eq!(parsed.unit_filter, Some(UnitKind::Hisser));
        assert_eq!(parsed.direction, Some(DirectionKind::North));
    }

    #[test]
    fn test_parse_whisper_noise_ignored() {
        // Whisper sometimes adds filler words
        let parsed = parse_voice_text("um attack the enemy");
        assert_eq!(parsed.action, Some(AgentAction::Attack));
    }

    #[test]
    fn test_parse_complex_command() {
        let parsed = parse_voice_text("army move south");
        assert_eq!(parsed.action, Some(AgentAction::Move));
        assert_eq!(parsed.selector, Some(SelectorKind::Army));
        assert_eq!(parsed.direction, Some(DirectionKind::South));
    }

    #[test]
    fn test_parse_cancel_resets() {
        let parsed = parse_voice_text("hisser cancel stop");
        // Cancel clears hisser, then stop is parsed fresh
        assert_eq!(parsed.action, Some(AgentAction::Stop));
        assert_eq!(parsed.unit_filter, None);
    }
}
