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
use cc_core::components::{Building, CursorGridPos, Dead, Faction, Owner, Position, UnitKind};
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

/// Bevy system: consumes VoiceCommandEvents and pushes GameCommands.
///
/// Default target: all of the player's on-screen units (not just selected).
/// If a unit name keyword was recently spoken, filters to that unit type.
pub fn voice_intent_system(
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
    mut pending_filter: Local<Option<UnitKind>>,
    mut pending_selector: Local<Option<SelectorKind>>,
    mut pending_direction: Local<Option<DirectionKind>>,
    mut pending_building: Local<Option<BuildingKind>>,
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
                // Filter to player's own living units only
                let own_units = || {
                    all_units.iter().filter(|(_, _, _, owner)| owner.player_id == 0)
                };

                let unit_ids: Vec<EntityId> = match *pending_selector {
                    Some(SelectorKind::Selected) => {
                        selected_units.iter().map(|e| EntityId(e.to_bits())).collect()
                    }
                    Some(SelectorKind::Workers) => {
                        own_units()
                            .filter(|(_, ut, _, _)| ut.kind == UnitKind::Pawdler)
                            .map(|(e, _, _, _)| EntityId(e.to_bits()))
                            .collect()
                    }
                    Some(SelectorKind::Army) => {
                        own_units()
                            .filter(|(_, ut, _, _)| ut.kind != UnitKind::Pawdler)
                            .map(|(e, _, _, _)| EntityId(e.to_bits()))
                            .collect()
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
                                .map(|(e, _, _, _)| EntityId(e.to_bits()))
                                .collect()
                        } else {
                            // No cursor position — fall back to all own units
                            own_units()
                                .map(|(e, _, _, _)| EntityId(e.to_bits()))
                                .collect()
                        }
                    }
                    _ => {
                        // Default: filter by unit kind if specified, else all own units
                        match *pending_filter {
                            Some(kind) => {
                                own_units()
                                    .filter(|(_, ut, _, _)| ut.kind == kind)
                                    .map(|(e, _, _, _)| EntityId(e.to_bits()))
                                    .collect()
                            }
                            None => {
                                own_units()
                                    .map(|(e, _, _, _)| EntityId(e.to_bits()))
                                    .collect()
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
                            *pending_direction,
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
                            *pending_direction,
                            enemy_centroid,
                            mw,
                            mh,
                        )
                    }
                    // Move/Patrol/Scout → move in direction or toward enemy
                    AgentAction::Move | AgentAction::Patrol | AgentAction::Scout => {
                        resolve_voice_move(
                            &unit_ids,
                            *pending_direction,
                            enemy_centroid,
                            mw,
                            mh,
                        )
                    }
                    // Rally → set rally point on first owned building
                    AgentAction::Rally => {
                        let target = resolve_move_target(
                            *pending_direction,
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
                        if let Some(voice_bk) = *pending_building {
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
                    cmd_queue.push_for_player(0, c);
                }

                // Clear all pending state after command execution
                *pending_filter = None;
                *pending_selector = None;
                *pending_direction = None;
                *pending_building = None;
            }

            KeywordRole::UnitName(kind) => {
                *pending_filter = Some(kind);
                *pending_selector = None;
            }

            KeywordRole::Selector(sel) => {
                *pending_selector = Some(sel);
                *pending_filter = None;
            }

            KeywordRole::Direction(dir) => {
                *pending_direction = Some(dir);
            }

            KeywordRole::Building(bk) => {
                *pending_building = Some(bk);
            }

            KeywordRole::Meta(MetaAction::Cancel) => {
                *pending_filter = None;
                *pending_selector = None;
                *pending_direction = None;
                *pending_building = None;
            }

            // Conjunctions, group numbers, other meta — accumulated for future use
            _ => {}
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
}
