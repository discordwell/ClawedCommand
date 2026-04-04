use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{
    Building, CursorGridPos, HeroIdentity, Owner, Position, Producer, ResourceDeposit, Selected,
    UnitType,
};
use cc_core::coords::{ScreenPos, screen_to_world};
use cc_sim::campaign::state::{CampaignPhase, CampaignState};
use cc_sim::resources::{CommandQueue, MapResource};

use crate::renderer::minimap::MinimapClickConsumed;

use super::{DoubleClickState, DragSelectState, InputMode, PlacementPreview};

/// Local player ID (TODO: make configurable for multiplayer)
const LOCAL_PLAYER: u8 = 0;

/// Minimum drag distance (pixels) before box select activates.
const DRAG_THRESHOLD: f32 = 5.0;

/// Maximum time between clicks for double-click (seconds).
const DOUBLE_CLICK_WINDOW: f64 = 0.3;

/// Bundled mutable input state to stay under Bevy's 16-param limit.
#[derive(SystemParam)]
pub struct InputState<'w> {
    cmd_queue: ResMut<'w, CommandQueue>,
    drag_state: ResMut<'w, DragSelectState>,
    input_mode: ResMut<'w, InputMode>,
    placement_preview: ResMut<'w, PlacementPreview>,
    dbl_click: ResMut<'w, DoubleClickState>,
}

pub fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    units: Query<(Entity, &Position, &Owner, Option<&Selected>, &UnitType)>,
    buildings_q: Query<(Entity, &Position, &Owner), With<Building>>,
    selected_buildings_q: Query<
        (Entity, &Owner, Option<&Producer>),
        (With<Building>, With<Selected>),
    >,
    deposits: Query<(Entity, &Position), With<ResourceDeposit>>,
    map_res: Res<MapResource>,
    mut state: InputState,
    minimap_consumed: Res<MinimapClickConsumed>,
    restrictions: Option<Res<cc_sim::campaign::mutator_state::ControlRestrictions>>,
    campaign: Res<CampaignState>,
    hero_q: Query<(Entity, &HeroIdentity, &Owner)>,
) {
    // Gate: skip all mouse commands (except camera, handled separately) if restricted
    if restrictions
        .as_ref()
        .is_some_and(|r| !r.mouse_keyboard_enabled)
    {
        return;
    }

    // Block mouse input while prompt overlay is open
    if *state.input_mode == InputMode::Prompt {
        return;
    }

    // Skip if minimap consumed this click
    if minimap_consumed.0 {
        return;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera_q;

    // Convert cursor to world coordinates via camera
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Bevy world has Y-up, our isometric has Y-down, so flip Y for screen_to_world
    let iso_world = screen_to_world(ScreenPos {
        x: world_pos.x,
        y: -world_pos.y,
    });

    let cursor_grid = iso_world.to_grid();

    // --- Dream sequence override: left-click moves Kell, suppress all other RTS mouse logic ---
    let is_dream = campaign.phase == CampaignPhase::InMission
        && campaign
            .current_mission
            .as_ref()
            .is_some_and(|m| cc_core::mutator::is_dream_mission(&m.mutators));

    if is_dream {
        if mouse_button.just_pressed(MouseButton::Left) {
            let target = iso_world.to_grid();
            // Find the player hero (Kell Fisher in dream)
            if let Some((kelpie_entity, _, _)) = hero_q.iter().find(|(_, _hi, owner)| {
                owner.player_id == LOCAL_PLAYER
            }) {
                state.cmd_queue.push(GameCommand::Move {
                    unit_ids: vec![EntityId::from_entity(kelpie_entity)],
                    target,
                });
            }
        }
        // Suppress all normal RTS mouse behavior during dream
        return;
    }

    // --- BuildPlacement mode ---
    if let InputMode::BuildPlacement { kind } = *state.input_mode {
        let valid = map_res.map.is_passable(cursor_grid)
            && !buildings_q
                .iter()
                .any(|(_, pos, _)| pos.world.to_grid() == cursor_grid);
        state.placement_preview.grid_pos = Some(cursor_grid);
        state.placement_preview.valid = valid;

        if mouse_button.just_pressed(MouseButton::Left) && valid {
            // Find a selected unit to be the builder
            let builder = units
                .iter()
                .find(|(_, _, owner, sel, _)| sel.is_some() && owner.player_id == LOCAL_PLAYER);
            if let Some((builder_entity, _, _, _, _)) = builder {
                state.cmd_queue.push(GameCommand::Build {
                    builder: EntityId(builder_entity.to_bits()),
                    building_kind: kind,
                    position: cursor_grid,
                });
            }
            *state.input_mode = InputMode::Normal;
            state.placement_preview.grid_pos = None;
            return;
        }

        if mouse_button.just_pressed(MouseButton::Right) {
            *state.input_mode = InputMode::Normal;
            state.placement_preview.grid_pos = None;
            return;
        }

        return;
    }

    // Clear placement preview when not in placement mode
    state.placement_preview.grid_pos = None;

    // --- Left click down: start drag ---
    if mouse_button.just_pressed(MouseButton::Left) {
        state.drag_state.start = Some(cursor_pos);
        state.drag_state.active = false;

        // In AttackMove mode, left-click issues attack-move and reverts
        if *state.input_mode == InputMode::AttackMove {
            let selected_ids: Vec<EntityId> = units
                .iter()
                .filter(|(_, _, _, sel, _)| sel.is_some())
                .map(|(entity, _, _, _, _)| EntityId(entity.to_bits()))
                .collect();
            if !selected_ids.is_empty() {
                let target = iso_world.to_grid();
                state.cmd_queue.push(GameCommand::AttackMove {
                    unit_ids: selected_ids,
                    target,
                });
            }
            *state.input_mode = InputMode::Normal;
            state.drag_state.start = None;
            return;
        }

        // In Move mode, left-click issues explicit move and reverts
        if *state.input_mode == InputMode::Move {
            let selected_ids: Vec<EntityId> = units
                .iter()
                .filter(|(_, _, _, sel, _)| sel.is_some())
                .map(|(entity, _, _, _, _)| EntityId(entity.to_bits()))
                .collect();
            if !selected_ids.is_empty() {
                let target = iso_world.to_grid();
                state.cmd_queue.push(GameCommand::Move {
                    unit_ids: selected_ids,
                    target,
                });
            }
            *state.input_mode = InputMode::Normal;
            state.drag_state.start = None;
            return;
        }
    }

    // --- Left click held: check drag threshold ---
    if mouse_button.pressed(MouseButton::Left)
        && let Some(start) = state.drag_state.start
    {
        let delta = cursor_pos - start;
        if delta.length() > DRAG_THRESHOLD {
            state.drag_state.active = true;
        }
    }

    // --- Left click released ---
    if mouse_button.just_released(MouseButton::Left) {
        let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        if state.drag_state.active {
            // Box select: select all own units within the screen-space rectangle
            if let Some(start) = state.drag_state.start {
                let min_x = start.x.min(cursor_pos.x);
                let max_x = start.x.max(cursor_pos.x);
                let min_y = start.y.min(cursor_pos.y);
                let max_y = start.y.max(cursor_pos.y);

                if !shift {
                    state.cmd_queue.push(GameCommand::Deselect);
                }

                let mut box_selected = Vec::new();
                for (entity, pos, owner, _, _) in units.iter() {
                    if owner.player_id != LOCAL_PLAYER {
                        continue;
                    }
                    let unit_screen = unit_to_viewport(pos, camera, camera_transform);
                    if let Some(sp) = unit_screen
                        && sp.x >= min_x
                        && sp.x <= max_x
                        && sp.y >= min_y
                        && sp.y <= max_y
                    {
                        box_selected.push(EntityId(entity.to_bits()));
                    }
                }
                if !box_selected.is_empty() {
                    state.cmd_queue.push(GameCommand::Select {
                        unit_ids: box_selected,
                    });
                }
            }
        } else {
            // Click select: pick nearest unit or building
            let mut clicked_entity = None;
            let mut clicked_unit_kind = None;
            let mut best_dist = f32::MAX;

            // Check units (0.8 radius)
            for (entity, pos, _owner, _, unit_type) in units.iter() {
                let dist = world_dist(pos, &iso_world);
                if dist < 0.8 && dist < best_dist {
                    best_dist = dist;
                    clicked_entity = Some(entity);
                    clicked_unit_kind = Some(unit_type.kind);
                }
            }

            // Check buildings (1.2 radius, only if no closer unit)
            for (entity, pos, _owner) in buildings_q.iter() {
                let dist = world_dist(pos, &iso_world);
                if dist < 1.2 && dist < best_dist {
                    best_dist = dist;
                    clicked_entity = Some(entity);
                    clicked_unit_kind = None; // buildings don't have UnitKind
                }
            }

            if let Some(entity) = clicked_entity {
                let now = time.elapsed_secs_f64();

                // Double-click detection: if same unit kind within window, select all of type
                if let Some(kind) = clicked_unit_kind {
                    if state.dbl_click.last_click_kind == Some(kind)
                        && (now - state.dbl_click.last_click_time) < DOUBLE_CLICK_WINDOW
                    {
                        // Double-click: select all on-screen own units of this type
                        state.cmd_queue.push(GameCommand::Deselect);
                        let win_w = window.width();
                        let win_h = window.height();
                        let mut all_of_type = Vec::new();
                        for (e, pos, owner, _, ut) in units.iter() {
                            if owner.player_id != LOCAL_PLAYER || ut.kind != kind {
                                continue;
                            }
                            // Only units actually visible in the viewport
                            if let Some(vp) = unit_to_viewport(pos, camera, camera_transform)
                                && vp.x >= 0.0
                                && vp.x <= win_w
                                && vp.y >= 0.0
                                && vp.y <= win_h
                            {
                                all_of_type.push(EntityId(e.to_bits()));
                            }
                        }
                        if !all_of_type.is_empty() {
                            state.cmd_queue.push(GameCommand::Select {
                                unit_ids: all_of_type,
                            });
                        }
                        state.dbl_click.last_click_kind = None;
                        state.dbl_click.last_click_time = 0.0;
                    } else {
                        // Single click
                        state.dbl_click.last_click_time = now;
                        state.dbl_click.last_click_kind = Some(kind);

                        if !shift {
                            state.cmd_queue.push(GameCommand::Deselect);
                        }
                        state.cmd_queue.push(GameCommand::Select {
                            unit_ids: vec![EntityId(entity.to_bits())],
                        });
                    }
                } else {
                    // Clicked a building — no double-click behavior
                    state.dbl_click.last_click_kind = None;
                    if !shift {
                        state.cmd_queue.push(GameCommand::Deselect);
                    }
                    state.cmd_queue.push(GameCommand::Select {
                        unit_ids: vec![EntityId(entity.to_bits())],
                    });
                }
            } else {
                state.dbl_click.last_click_kind = None;
                if !shift {
                    state.cmd_queue.push(GameCommand::Deselect);
                }
            }
        }

        // Reset drag state
        state.drag_state.start = None;
        state.drag_state.active = false;
    }

    // --- Right click: smart command ---
    if mouse_button.just_pressed(MouseButton::Right) {
        // Cancel special modes on right-click
        if *state.input_mode != InputMode::Normal {
            *state.input_mode = InputMode::Normal;
            state.placement_preview.grid_pos = None;
            return;
        }

        let selected_ids: Vec<EntityId> = units
            .iter()
            .filter(|(_, _, _, sel, _)| sel.is_some())
            .map(|(entity, _, _, _, _)| EntityId(entity.to_bits()))
            .collect();

        // If no units selected, check for building rally point
        if selected_ids.is_empty() {
            // Set rally point for selected producer buildings
            for (entity, owner, producer) in selected_buildings_q.iter() {
                if owner.player_id != LOCAL_PLAYER || producer.is_none() {
                    continue;
                }
                state.cmd_queue.push(GameCommand::SetRallyPoint {
                    building: EntityId(entity.to_bits()),
                    target: cursor_grid,
                });
            }
            return;
        }

        // Check: right-click on enemy unit → Attack
        let mut clicked_enemy = None;
        let mut best_dist = f32::MAX;

        for (entity, pos, owner, _, _) in units.iter() {
            if owner.player_id == LOCAL_PLAYER {
                continue;
            }
            let dist = world_dist(pos, &iso_world);
            if dist < 0.8 && dist < best_dist {
                best_dist = dist;
                clicked_enemy = Some(entity);
            }
        }

        if let Some(enemy) = clicked_enemy {
            state.cmd_queue.push(GameCommand::Attack {
                unit_ids: selected_ids,
                target: EntityId(enemy.to_bits()),
            });
            return;
        }

        // Check: right-click on enemy building → Attack
        for (entity, pos, owner) in buildings_q.iter() {
            if owner.player_id == LOCAL_PLAYER {
                continue;
            }
            let dist = world_dist(pos, &iso_world);
            if dist < 1.2 && dist < best_dist {
                best_dist = dist;
                clicked_enemy = Some(entity);
            }
        }

        if let Some(enemy) = clicked_enemy {
            state.cmd_queue.push(GameCommand::Attack {
                unit_ids: selected_ids,
                target: EntityId(enemy.to_bits()),
            });
            return;
        }

        // Check: right-click on resource deposit → GatherResource
        let mut clicked_deposit = None;
        best_dist = f32::MAX;

        for (entity, pos) in deposits.iter() {
            let dist = world_dist(pos, &iso_world);
            if dist < 1.0 && dist < best_dist {
                best_dist = dist;
                clicked_deposit = Some(entity);
            }
        }

        if let Some(deposit) = clicked_deposit {
            state.cmd_queue.push(GameCommand::GatherResource {
                unit_ids: selected_ids,
                deposit: EntityId(deposit.to_bits()),
            });
            return;
        }

        // Default: right-click on ground → Move
        let target = iso_world.to_grid();
        state.cmd_queue.push(GameCommand::Move {
            unit_ids: selected_ids,
            target,
        });
    }
}

/// Updates the shared CursorGridPos resource each frame.
///
/// Runs before voice systems so they can read the cursor position.
pub fn update_cursor_grid_pos(
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor_grid: ResMut<CursorGridPos>,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        cursor_grid.pos = None;
        return;
    };
    let (camera, camera_transform) = *camera_q;
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        cursor_grid.pos = None;
        return;
    };
    let iso_world = screen_to_world(ScreenPos {
        x: world_pos.x,
        y: -world_pos.y,
    });
    cursor_grid.pos = Some(iso_world.to_grid());
}

/// Distance between a Position and a WorldPos in world units.
fn world_dist(pos: &Position, iso_world: &cc_core::coords::WorldPos) -> f32 {
    let ux: f32 = pos.world.x.to_num();
    let uy: f32 = pos.world.y.to_num();
    let iso_x: f32 = iso_world.x.to_num();
    let iso_y: f32 = iso_world.y.to_num();
    let dx = ux - iso_x;
    let dy = uy - iso_y;
    (dx * dx + dy * dy).sqrt()
}

/// Convert a unit's world position to viewport (screen) coordinates.
fn unit_to_viewport(
    pos: &Position,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Vec2> {
    use cc_core::coords::world_to_screen;

    let screen = world_to_screen(pos.world);
    let bevy_world = Vec3::new(screen.x, -screen.y, 0.0);
    camera.world_to_viewport(camera_transform, bevy_world).ok()
}
