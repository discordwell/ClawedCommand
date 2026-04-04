//! Dream sequence systems — Cmdr. Kell Fisher office grind + Claude of the Lake.
//!
//! Activated by the `DreamSequence` mission mutator. Two sub-scenes:
//! - **Office**: click-to-interact desk grind loop with day/night overlay.
//! - **Lake**: walk through water to meet Claude of the Lake.

use bevy::prelude::*;

use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::{HeroIdentity, Owner, Position, Selected};
use cc_core::coords::{GridPos, WorldPos, world_to_screen};
use cc_core::hero::HeroId;
use cc_core::mutator::{DreamSceneType, MissionMutator};
use cc_sim::campaign::state::{CampaignPhase, CampaignState};
use cc_sim::resources::CommandQueue;

use crate::renderer::hero_sprites::HeroSprites;
use crate::ui::ability_bar::AbilityBarRoot;
use crate::ui::build_menu::BuildMenuRoot;
use crate::ui::building_info::BuildingInfoRoot;
use crate::ui::command_card::CommandCardRoot;
use crate::ui::resource_bar::ResourceBarRoot;
use crate::ui::unit_info::UnitInfoRoot;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How many real seconds each "action" animation takes.
const ACTION_DURATION: f32 = 2.5;

/// Work sessions before passout.
const MAX_WORK_SESSIONS: u32 = 20;

/// Starting hour (08:00).
const START_HOUR: f32 = 8.0;

/// Hours per work/action session.
const HOURS_PER_SESSION: f32 = 4.0;

/// After this many sessions, forced actions begin.
const FORCED_ACTION_THRESHOLD: u32 = 6;

// ---------------------------------------------------------------------------
// Clickable office locations (grid positions)
// ---------------------------------------------------------------------------

/// Proximity radius (in grid tiles) for interaction prompt to appear.
const INTERACT_RADIUS: i32 = 2;

/// Grid positions for each interactable location on the office map.
/// Must be on passable tiles and reachable from the central hallway.
fn office_location_positions() -> Vec<(OfficeAction, GridPos, &'static str)> {
    vec![
        (OfficeAction::Work, GridPos::new(10, 7), "Work"),            // desk area center
        (OfficeAction::EnergyDrink, GridPos::new(14, 2), "Get Energy Drink"), // vending alcove (north)
        (OfficeAction::WorkOut, GridPos::new(3, 12), "Work Out"),     // gym area (south)
        (OfficeAction::CallAda, GridPos::new(18, 2), "Call Ada"),     // phone nook (north-east)
        (OfficeAction::Sleep, GridPos::new(2, 2), "Sleep"),           // cot nook (north-west)
        (OfficeAction::Eat, GridPos::new(16, 12), "Eat"),             // eat area (south-east)
        (OfficeAction::Talk, GridPos::new(8, 2), "Talk to Someone"),  // talk area (north-center)
    ]
}

/// Color and short label for each location's visible prop marker.
fn prop_appearance(action: OfficeAction) -> (Color, &'static str) {
    match action {
        OfficeAction::Work => (Color::srgb(0.3, 0.5, 0.8), "[PC]"),
        OfficeAction::EnergyDrink => (Color::srgb(0.1, 0.8, 0.3), "[VEND]"),
        OfficeAction::WorkOut => (Color::srgb(0.8, 0.4, 0.1), "[GYM]"),
        OfficeAction::CallAda => (Color::srgb(0.5, 0.5, 0.5), "[PHONE]"),
        OfficeAction::Sleep => (Color::srgb(0.4, 0.4, 0.6), "[COT]"),
        OfficeAction::Eat => (Color::srgb(0.6, 0.5, 0.3), "[FOOD]"),
        OfficeAction::Talk => (Color::srgb(0.5, 0.5, 0.5), "[PPL]"),
    }
}

/// Dismissive lines Kell says when you try a disabled action.
fn kell_refusal(action: OfficeAction) -> &'static str {
    match action {
        OfficeAction::CallAda => "Ada can wait. The intercepts won't read themselves.",
        OfficeAction::Sleep => "Sleep is for people who aren't winning a war.",
        OfficeAction::Eat => "I'll eat when the targeting data is processed.",
        OfficeAction::Talk => "I don't need a pep talk. I need another four hours.",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker for an interactable location entity (invisible, just holds data).
#[derive(Component)]
pub struct DreamLocation {
    pub action: OfficeAction,
    pub grid_pos: GridPos,
}

/// Marker for the "Press F to <action>" prompt UI node.
#[derive(Component)]
pub struct DreamPromptNode;

/// Marker for the brief refusal dialogue text.
#[derive(Component)]
pub struct DreamRefusalNode;

/// Marker for a visible prop at an interaction location.
#[derive(Component)]
pub struct DreamProp;

/// Marker for the day/night overlay node.
#[derive(Component)]
pub struct DayNightOverlayNode;

/// Marker for the session HUD text.
#[derive(Component)]
pub struct DreamSessionHud;

/// Marker for dream-specific entities (cleaned up when dream ends).
#[derive(Component)]
pub struct DreamEntity;

/// Tracks whether RTS UI was hidden by the dream system (so we can restore it).
#[derive(Resource, Default)]
pub struct DreamUiHidden(pub bool);

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Actions available in the office.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeAction {
    Work,
    EnergyDrink,
    WorkOut,
    // Visible but disabled:
    CallAda,
    Sleep,
    Eat,
    Talk,
}

impl OfficeAction {
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Work | Self::EnergyDrink | Self::WorkOut)
    }
}

/// Internal phase of the office dream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OfficePhase {
    /// Waiting for opening dialogue to finish.
    #[default]
    OpeningDialogue,
    /// Player can click locations.
    FreeRoam,
    /// Kell is walking to a location / performing an action.
    ActionInProgress,
    /// Session 20 hit — fading out.
    Passout,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// State for the office desk grind loop.
#[derive(Resource)]
pub struct DreamOfficeState {
    pub phase: OfficePhase,
    pub work_sessions: u32,
    pub sessions_since_drink: u32,
    pub sessions_since_workout: u32,
    pub current_hour: f32,
    pub day_count: u32,
    pub current_action: Option<OfficeAction>,
    pub action_timer: f32,
    pub forced_action: Option<OfficeAction>,
    pub initialized: bool,
    pub passout_timer: f32,
    /// Which action Kelpie is currently near (if any).
    pub nearby_action: Option<OfficeAction>,
    /// Timer for refusal dialogue display.
    pub refusal_timer: f32,
    /// Saved Kelpie sprite handle for restoration after dream.
    pub original_kelpie_sprite: Option<Handle<Image>>,
    /// Saved Rex sprite handle for restoration after dream.
    pub original_rex_sprite: Option<Handle<Image>>,
    /// Whether Rex has departed the office scene.
    pub rex_departed: bool,
    /// Countdown timer for Rex's departure after opening dialogue.
    pub rex_departure_timer: f32,
}

impl Default for DreamOfficeState {
    fn default() -> Self {
        Self {
            phase: OfficePhase::OpeningDialogue,
            work_sessions: 0,
            sessions_since_drink: 0,
            sessions_since_workout: 0,
            current_hour: START_HOUR,
            day_count: 1,
            current_action: None,
            action_timer: 0.0,
            forced_action: None,
            initialized: false,
            passout_timer: 0.0,
            nearby_action: None,
            refusal_timer: 0.0,
            original_kelpie_sprite: None,
            original_rex_sprite: None,
            rex_departed: false,
            rex_departure_timer: 4.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct DreamPlugin;

impl Plugin for DreamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DreamOfficeState>()
            .init_resource::<DreamUiHidden>()
            .add_systems(
            Update,
            (
                dream_init_system,
                dream_hide_rts_ui,
                dream_keep_selected.run_if(is_dream_office_active),
                dream_npc_departure.run_if(is_dream_office_active),
                dream_proximity_system
                    .after(dream_init_system)
                    .run_if(is_dream_office_active),
                dream_interact_system
                    .after(dream_proximity_system)
                    .run_if(is_dream_office_active),
                dream_office_fsm
                    .after(dream_interact_system)
                    .run_if(is_dream_office_active),
                dream_day_night_system.run_if(is_dream_office_active),
                dream_prompt_system.run_if(is_dream_office_active),
                dream_session_hud_system.run_if(is_dream_office_active),
                dream_passout_system.run_if(is_dream_office_active),
                dream_cleanup_system,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Run conditions
// ---------------------------------------------------------------------------

fn is_dream_office_active(campaign: Res<CampaignState>) -> bool {
    if campaign.phase != CampaignPhase::InMission {
        return false;
    }
    campaign.current_mission.as_ref().is_some_and(|m| {
        m.mutators.iter().any(|mt| {
            matches!(
                mt,
                MissionMutator::DreamSequence {
                    scene_type: DreamSceneType::Office,
                    ..
                }
            )
        })
    })
}

fn get_dream_scene_type(campaign: &CampaignState) -> Option<DreamSceneType> {
    campaign.current_mission.as_ref().and_then(|m| {
        m.mutators.iter().find_map(|mt| match mt {
            MissionMutator::DreamSequence { scene_type, .. } => Some(*scene_type),
            _ => None,
        })
    })
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// One-time initialization when entering a dream mission.
fn dream_init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut dream: ResMut<DreamOfficeState>,
    mut hero_sprites: Option<ResMut<HeroSprites>>,
    mut cmd_queue: ResMut<CommandQueue>,
    campaign: Res<CampaignState>,
    asset_server: Res<AssetServer>,
    heroes: Query<(Entity, &HeroIdentity, &Owner)>,
) {
    if campaign.phase != CampaignPhase::InMission {
        if dream.initialized {
            // Leaving dream — reset state
            dream.initialized = false;
            *dream = DreamOfficeState::default();
        }
        return;
    }

    let Some(scene_type) = get_dream_scene_type(&campaign) else {
        return;
    };

    if dream.initialized {
        return;
    }
    dream.initialized = true;

    // Swap Kelpie's sprite to Kell Fisher during dream
    if let Some(ref mut sprites) = hero_sprites {
        dream.original_kelpie_sprite = sprites.sprites.get(&HeroId::Kelpie).cloned();
        let kell_path = "sprites/heroes/kell_fisher_idle.png";
        if crate::renderer::asset_exists_on_disk(kell_path) {
            sprites.sprites.insert(HeroId::Kelpie, asset_server.load(kell_path));
        }

        // Swap Rex's sprite to his human form
        dream.original_rex_sprite = sprites.sprites.get(&HeroId::RexSolstice).cloned();
        let rex_path = "sprites/heroes/rex_harmon_idle.png";
        if crate::renderer::asset_exists_on_disk(rex_path) {
            sprites.sprites.insert(HeroId::RexSolstice, asset_server.load(rex_path));
        }
    }

    match scene_type {
        DreamSceneType::Office => {
            // Auto-select Kell so click-to-move works immediately
            if let Some(kelpie_entity) = heroes.iter().find_map(|(e, hi, owner)| {
                (hi.hero_id == cc_core::hero::HeroId::Kelpie && owner.player_id == 0)
                    .then_some(e)
            }) {
                commands.entity(kelpie_entity).insert(Selected);
                cmd_queue.push(GameCommand::Select {
                    unit_ids: vec![EntityId::from_entity(kelpie_entity)],
                });
            }

            // Spawn location markers with visible prop objects
            let prop_mesh = meshes.add(Rectangle::new(10.0, 10.0));
            for (action, pos, _label) in office_location_positions() {
                let (color, icon) = prop_appearance(action);
                let screen = world_to_screen(WorldPos::from_grid(pos));
                let elev_offset = 0.0; // flat map

                // Data-only location marker
                commands.spawn((
                    DreamEntity,
                    DreamLocation {
                        action,
                        grid_pos: pos,
                    },
                ));

                // Visible prop: colored square on the map
                let prop_mat = materials.add(ColorMaterial::from_color(color));
                commands.spawn((
                    DreamEntity,
                    DreamProp,
                    Mesh2d(prop_mesh.clone()),
                    MeshMaterial2d(prop_mat),
                    Transform::from_xyz(screen.x, -screen.y + elev_offset, -5.0),
                ));

                // Small label above the prop
                commands.spawn((
                    DreamEntity,
                    DreamProp,
                    Text2d::new(icon),
                    TextColor(color),
                    TextFont {
                        font_size: 9.0,
                        ..default()
                    },
                    Transform::from_xyz(screen.x, -screen.y + elev_offset + 10.0, 50.0),
                ));
            }

            // "Press F to <action>" prompt (hidden by default)
            commands.spawn((
                DreamEntity,
                DreamPromptNode,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(120.0),
                    left: Val::Percent(50.0),
                    ..default()
                },
                Text::new(""),
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                ZIndex(25),
            ));

            // Refusal dialogue text (hidden by default)
            commands.spawn((
                DreamEntity,
                DreamRefusalNode,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(80.0),
                    left: Val::Percent(50.0),
                    ..default()
                },
                Text::new(""),
                TextColor(Color::srgba(0.8, 0.7, 0.5, 0.0)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                ZIndex(25),
            ));

            // Day/night overlay (full-screen UI node)
            commands.spawn((
                DreamEntity,
                DayNightOverlayNode,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ZIndex(5), // Above tiles, below HUD
            ));

            // Session HUD
            commands.spawn((
                DreamEntity,
                DreamSessionHud,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(4.0),
                    right: Val::Px(12.0),
                    ..default()
                },
                Text::new(format_session_hud(0, START_HOUR, 1)),
                TextColor(Color::srgb(0.9, 0.9, 0.7)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                ZIndex(20),
            ));
        }
        DreamSceneType::Lake => {
            // Lake scene needs no special spawn — just the fog + map + hero
        }
    }
}

/// Hide RTS UI elements during dream missions, restore when dream ends.
fn dream_hide_rts_ui(
    campaign: Res<CampaignState>,
    mut ui_hidden: ResMut<DreamUiHidden>,
    mut rts_roots: Query<
        &mut Visibility,
        Or<(
            With<ResourceBarRoot>,
            With<BuildMenuRoot>,
            With<CommandCardRoot>,
            With<AbilityBarRoot>,
            With<UnitInfoRoot>,
            With<BuildingInfoRoot>,
        )>,
    >,
) {
    let is_dream = campaign.phase == CampaignPhase::InMission
        && campaign
            .current_mission
            .as_ref()
            .is_some_and(|m| cc_core::mutator::is_dream_mission(&m.mutators));

    if is_dream && !ui_hidden.0 {
        ui_hidden.0 = true;
        for mut vis in rts_roots.iter_mut() {
            *vis = Visibility::Hidden;
        }
    } else if !is_dream && ui_hidden.0 {
        ui_hidden.0 = false;
        for mut vis in rts_roots.iter_mut() {
            *vis = Visibility::Inherited;
        }
    }
}

/// Check Kelpie's proximity to interactable locations and update nearby_action.
fn dream_proximity_system(
    mut dream: ResMut<DreamOfficeState>,
    heroes: Query<(&HeroIdentity, &Owner, &Position)>,
    locations: Query<&DreamLocation>,
) {
    if dream.phase != OfficePhase::FreeRoam {
        dream.nearby_action = None;
        return;
    }

    // Find Kelpie's grid position
    let kelpie_pos = heroes.iter().find_map(|(hi, owner, pos)| {
        if hi.hero_id == cc_core::hero::HeroId::Kelpie && owner.player_id == 0 {
            Some(pos.world.to_grid())
        } else {
            None
        }
    });

    let Some(kelpie_grid) = kelpie_pos else {
        dream.nearby_action = None;
        return;
    };

    // Find the closest location within interact radius
    let mut best: Option<(OfficeAction, i32)> = None;
    for loc in locations.iter() {
        let dx = (kelpie_grid.x - loc.grid_pos.x).abs();
        let dy = (kelpie_grid.y - loc.grid_pos.y).abs();
        let dist = dx.max(dy); // Chebyshev distance
        if dist <= INTERACT_RADIUS {
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((loc.action, dist));
            }
        }
    }

    dream.nearby_action = best.map(|(action, _)| action);
}

/// Handle F key press to interact with nearby location.
fn dream_interact_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut dream: ResMut<DreamOfficeState>,
    mut prompt_q: Query<(&mut Text, &mut TextColor), (With<DreamRefusalNode>, Without<DreamPromptNode>)>,
) {
    // Tick down refusal timer
    if dream.refusal_timer > 0.0 {
        dream.refusal_timer -= time.delta_secs();
    }

    if dream.phase != OfficePhase::FreeRoam {
        return;
    }

    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Some(action) = dream.nearby_action else {
        return;
    };

    // Disabled actions: show refusal dialogue
    if !action.is_enabled() {
        let refusal = kell_refusal(action);
        if !refusal.is_empty() {
            // Set the refusal text
            for (mut text, mut color) in prompt_q.iter_mut() {
                text.0 = format!("Kell: \"{refusal}\"");
                color.0 = Color::srgba(0.85, 0.75, 0.5, 1.0);
            }
            dream.refusal_timer = 3.0;
        }
        return;
    }

    // Forced action check: can only do the forced action
    if let Some(forced) = dream.forced_action {
        if action != forced {
            return;
        }
    }

    // Begin the action
    dream.current_action = Some(action);
    dream.action_timer = ACTION_DURATION;
    dream.phase = OfficePhase::ActionInProgress;
}

/// Core office state machine — handles action completion, forced actions, passout.
fn dream_office_fsm(
    time: Res<Time>,
    mut dream: ResMut<DreamOfficeState>,
) {
    match dream.phase {
        OfficePhase::OpeningDialogue => {
            // Dialogue is handled by the trigger system (AtTick(1) -> ShowDialogue).
            // We transition to FreeRoam after the dialogue queue is empty.
            // For now, auto-transition after a brief delay (dialogue system handles display).
            // The dialogue trigger fires on tick 1; we give it a moment.
            dream.action_timer += time.delta_secs();
            if dream.action_timer > 1.0 {
                dream.phase = OfficePhase::FreeRoam;
                dream.action_timer = 0.0;
            }
        }
        OfficePhase::FreeRoam => {
            // Waiting for player click — handled by click system
        }
        OfficePhase::ActionInProgress => {
            dream.action_timer -= time.delta_secs();
            if dream.action_timer > 0.0 {
                return;
            }

            // Action complete — process result
            let action = dream.current_action.take().unwrap_or(OfficeAction::Work);
            match action {
                OfficeAction::Work => {
                    dream.work_sessions += 1;
                    dream.sessions_since_drink += 1;
                    dream.sessions_since_workout += 1;
                }
                OfficeAction::EnergyDrink => {
                    dream.sessions_since_drink = 0;
                }
                OfficeAction::WorkOut => {
                    dream.sessions_since_workout = 0;
                }
                _ => {}
            }

            // Advance time
            dream.current_hour += HOURS_PER_SESSION;
            if dream.current_hour >= 24.0 {
                dream.current_hour -= 24.0;
                dream.day_count += 1;
            }

            // Check passout
            if dream.work_sessions >= MAX_WORK_SESSIONS {
                dream.phase = OfficePhase::Passout;
                dream.passout_timer = 3.0;
                return;
            }

            // Check forced actions (after threshold)
            dream.forced_action = None;
            if dream.work_sessions >= FORCED_ACTION_THRESHOLD && action == OfficeAction::Work {
                if dream.sessions_since_drink >= 3 {
                    dream.forced_action = Some(OfficeAction::EnergyDrink);
                } else if dream.sessions_since_drink >= 2 {
                    // 50% chance at 2, always at 3
                    let pseudo_rand = (dream.work_sessions * 7 + dream.day_count * 13) % 2;
                    if pseudo_rand == 0 {
                        dream.forced_action = Some(OfficeAction::EnergyDrink);
                    }
                }

                if dream.forced_action.is_none() {
                    if dream.sessions_since_workout >= 4 {
                        dream.forced_action = Some(OfficeAction::WorkOut);
                    } else if dream.sessions_since_workout >= 3 {
                        let pseudo_rand =
                            (dream.work_sessions * 11 + dream.day_count * 3) % 2;
                        if pseudo_rand == 0 {
                            dream.forced_action = Some(OfficeAction::WorkOut);
                        }
                    }
                }
            }

            dream.phase = OfficePhase::FreeRoam;
        }
        OfficePhase::Passout => {
            // Handled by passout_system
        }
    }
}

/// Passout fade-to-black and mission completion trigger.
fn dream_passout_system(
    time: Res<Time>,
    mut dream: ResMut<DreamOfficeState>,
    mut campaign: ResMut<CampaignState>,
    mut overlay_q: Query<&mut BackgroundColor, With<DayNightOverlayNode>>,
) {
    if dream.phase != OfficePhase::Passout {
        return;
    }

    dream.passout_timer -= time.delta_secs();

    // Fade overlay to black
    let fade_t = (1.0 - dream.passout_timer / 3.0).clamp(0.0, 1.0);
    for mut bg in overlay_q.iter_mut() {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, fade_t);
    }

    if dream.passout_timer <= 0.0 {
        // Mark the Manual objective as complete before triggering victory
        if let Some(status) = campaign
            .objective_status
            .iter_mut()
            .find(|s| s.id == "complete_work")
        {
            status.completed = true;
        }
        campaign.last_mission_result =
            Some(cc_sim::campaign::state::MissionResult::Victory);
        campaign.phase = CampaignPhase::Debriefing;
    }
}

/// Update day/night overlay color based on current hour.
fn dream_day_night_system(
    dream: Res<DreamOfficeState>,
    mut overlay_q: Query<&mut BackgroundColor, With<DayNightOverlayNode>>,
) {
    if dream.phase == OfficePhase::Passout {
        return; // Passout system handles overlay during fadeout
    }

    let hour = dream.current_hour;
    let (r, g, b, a) = day_night_color(hour);

    for mut bg in overlay_q.iter_mut() {
        bg.0 = Color::srgba(r, g, b, a);
    }
}

/// Compute day/night overlay color from hour.
fn day_night_color(hour: f32) -> (f32, f32, f32, f32) {
    if hour >= 6.0 && hour < 18.0 {
        // Day: transparent
        (0.0, 0.0, 0.0, 0.0)
    } else if hour >= 18.0 && hour < 20.0 {
        // Dusk: warm orange fade
        let t = (hour - 18.0) / 2.0;
        (0.8, 0.4, 0.1, t * 0.15)
    } else if hour >= 20.0 || hour < 5.0 {
        // Night: dark blue
        (0.05, 0.05, 0.2, 0.3)
    } else {
        // Dawn (5:00-6:00): amber fade out
        let t = 1.0 - (hour - 5.0);
        (0.7, 0.5, 0.2, t * 0.15)
    }
}

/// Update the "Press F to <action>" prompt and refusal text visibility.
fn dream_prompt_system(
    dream: Res<DreamOfficeState>,
    mut prompt_q: Query<
        (&mut Text, &mut TextColor),
        (With<DreamPromptNode>, Without<DreamRefusalNode>),
    >,
    mut refusal_q: Query<
        &mut TextColor,
        (With<DreamRefusalNode>, Without<DreamPromptNode>),
    >,
) {
    // Update prompt
    for (mut text, mut color) in prompt_q.iter_mut() {
        if dream.phase != OfficePhase::FreeRoam {
            color.0 = Color::srgba(1.0, 1.0, 1.0, 0.0);
            continue;
        }

        match dream.nearby_action {
            Some(action) => {
                let label = office_location_positions()
                    .iter()
                    .find(|(a, _, _)| *a == action)
                    .map(|(_, _, l)| *l)
                    .unwrap_or("Interact");

                // Forced action override
                if let Some(forced) = dream.forced_action {
                    if action == forced {
                        text.0 = format!("Press F to {label}");
                        color.0 = Color::srgb(1.0, 0.9, 0.3); // Yellow for forced
                    } else if action.is_enabled() {
                        text.0 = format!("Press F to {label}");
                        color.0 = Color::srgba(0.5, 0.5, 0.5, 0.5); // Greyed — forced elsewhere
                    } else {
                        text.0 = format!("Press F to {label}");
                        color.0 = Color::srgba(0.6, 0.5, 0.4, 0.8);
                    }
                } else if action.is_enabled() {
                    text.0 = format!("Press F to {label}");
                    color.0 = Color::WHITE;
                } else {
                    text.0 = format!("Press F to {label}");
                    color.0 = Color::srgba(0.6, 0.5, 0.4, 0.8); // Muted for disabled
                }
            }
            None => {
                text.0.clear();
                color.0 = Color::srgba(1.0, 1.0, 1.0, 0.0);
            }
        }
    }

    // Fade out refusal text
    for mut color in refusal_q.iter_mut() {
        if dream.refusal_timer <= 0.0 {
            color.0 = Color::srgba(0.85, 0.75, 0.5, 0.0);
        } else if dream.refusal_timer < 1.0 {
            // Fade out in last second
            color.0 = Color::srgba(0.85, 0.75, 0.5, dream.refusal_timer);
        }
    }
}

/// Update session counter HUD.
fn dream_session_hud_system(
    dream: Res<DreamOfficeState>,
    mut hud_q: Query<&mut Text, With<DreamSessionHud>>,
) {
    for mut text in hud_q.iter_mut() {
        text.0 = format_session_hud(dream.work_sessions, dream.current_hour, dream.day_count);

        if let Some(forced) = dream.forced_action {
            let forced_name = match forced {
                OfficeAction::EnergyDrink => "Kell needs an energy drink.",
                OfficeAction::WorkOut => "Kell needs to work out.",
                _ => "",
            };
            if !forced_name.is_empty() {
                text.0.push_str(&format!("\n{forced_name}"));
            }
        }
    }
}

fn format_session_hud(sessions: u32, hour: f32, day: u32) -> String {
    let h = hour as u32 % 24;
    let m = ((hour.fract()) * 60.0) as u32;
    format!("SESSION {sessions}/{MAX_WORK_SESSIONS}  |  Day {day}  |  {:02}:{:02}", h, m)
}

/// Clean up dream entities when leaving a dream mission.
fn dream_cleanup_system(
    mut commands: Commands,
    dream: Res<DreamOfficeState>,
    mut hero_sprites: Option<ResMut<HeroSprites>>,
    campaign: Res<CampaignState>,
    dream_entities: Query<Entity, With<DreamEntity>>,
) {
    let still_dreaming = campaign.phase == CampaignPhase::InMission
        && get_dream_scene_type(&campaign).is_some();

    if still_dreaming || !dream.initialized {
        return;
    }

    // Restore original Kelpie sprite
    if let Some(ref mut sprites) = hero_sprites {
        if let Some(original) = &dream.original_kelpie_sprite {
            sprites.sprites.insert(HeroId::Kelpie, original.clone());
        }
        if let Some(original) = &dream.original_rex_sprite {
            sprites.sprites.insert(HeroId::RexSolstice, original.clone());
        }
    }

    // Despawn all dream-specific entities
    for entity in dream_entities.iter() {
        commands.entity(entity).despawn();
    }
}

/// Keep Kelpie selected during dream — re-insert Selected if removed.
fn dream_keep_selected(
    mut commands: Commands,
    dream: Res<DreamOfficeState>,
    heroes: Query<(Entity, &HeroIdentity, &Owner), Without<Selected>>,
) {
    if !dream.initialized {
        return;
    }
    for (entity, hi, owner) in heroes.iter() {
        if hi.hero_id == HeroId::Kelpie && owner.player_id == 0 {
            commands.entity(entity).insert(Selected);
        }
    }
}

/// Despawn Rex after dialogue ends.
fn dream_npc_departure(
    mut commands: Commands,
    time: Res<Time>,
    mut dream: ResMut<DreamOfficeState>,
    heroes: Query<(Entity, &HeroIdentity)>,
) {
    if dream.rex_departed || dream.phase == OfficePhase::OpeningDialogue {
        return;
    }
    dream.rex_departure_timer -= time.delta_secs();
    if dream.rex_departure_timer <= 0.0 {
        for (entity, hi) in heroes.iter() {
            if hi.hero_id == HeroId::RexSolstice {
                commands.entity(entity).despawn();
                dream.rex_departed = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_action_enabled() {
        assert!(OfficeAction::Work.is_enabled());
        assert!(OfficeAction::EnergyDrink.is_enabled());
        assert!(OfficeAction::WorkOut.is_enabled());
        assert!(!OfficeAction::CallAda.is_enabled());
        assert!(!OfficeAction::Sleep.is_enabled());
        assert!(!OfficeAction::Eat.is_enabled());
        assert!(!OfficeAction::Talk.is_enabled());
    }

    #[test]
    fn day_night_color_day() {
        let (_, _, _, a) = day_night_color(12.0);
        assert_eq!(a, 0.0); // Full day = transparent
    }

    #[test]
    fn day_night_color_night() {
        let (_, _, _, a) = day_night_color(0.0);
        assert!(a > 0.2); // Night has visible overlay
    }

    #[test]
    fn day_night_color_dusk() {
        let (_, _, _, a) = day_night_color(19.0);
        assert!(a > 0.0 && a < 0.2); // Dusk is partial overlay
    }

    #[test]
    fn session_counter_formatting() {
        let s = format_session_hud(5, 16.0, 1);
        assert!(s.contains("SESSION 5/20"));
        assert!(s.contains("16:00"));
        assert!(s.contains("Day 1"));
    }

    #[test]
    fn forced_action_after_threshold() {
        let mut state = DreamOfficeState::default();
        state.work_sessions = FORCED_ACTION_THRESHOLD;
        state.sessions_since_drink = 3;
        // At 3 sessions since drink, should always force drink
        assert!(state.sessions_since_drink >= 3);
    }

    #[test]
    fn passout_at_max_sessions() {
        let state = DreamOfficeState {
            work_sessions: MAX_WORK_SESSIONS,
            ..default()
        };
        assert!(state.work_sessions >= MAX_WORK_SESSIONS);
    }

    #[test]
    fn hour_wraps_around() {
        let mut hour = 22.0f32;
        hour += HOURS_PER_SESSION; // 26.0
        if hour >= 24.0 {
            hour -= 24.0;
        }
        assert!((hour - 2.0).abs() < 0.01);
    }
}
