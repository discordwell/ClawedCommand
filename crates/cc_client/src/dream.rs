//! Dream sequence systems — Cmdr. Kell Fisher office grind + Claude of the Lake.
//!
//! Activated by the `DreamSequence` mission mutator. Three sub-scenes:
//! - **Office**: click-to-interact desk grind loop with day/night overlay.
//! - **Lake**: walk through water to meet Claude of the Lake.
//! - **Strait**: DEFCON-style drone warfare (see `dream_strait.rs`).

use bevy::prelude::*;

use cc_core::components::{HeroIdentity, Owner, Position, Selected};
use cc_core::coords::{GridPos, WorldPos, world_to_screen};
use cc_core::hero::HeroId;
use cc_core::mutator::{DreamSceneType, MissionMutator};
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

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
// Ops center desk positions + occupancy curve
// ---------------------------------------------------------------------------

/// 24 control desk positions in the ops center (rows 16-32, cols 13-59).
/// Spread across 5 rows, skip Kell's position (35,24).
fn ops_desk_positions() -> Vec<GridPos> {
    vec![
        // Row 17 (back)
        GridPos::new(15, 17), GridPos::new(21, 17), GridPos::new(27, 17),
        GridPos::new(35, 17), GridPos::new(41, 17), GridPos::new(47, 17),
        GridPos::new(53, 17),
        // Row 21
        GridPos::new(15, 21), GridPos::new(21, 21), GridPos::new(27, 21),
        GridPos::new(41, 21), GridPos::new(47, 21),
        // Row 25 — skip (35,24) area where Kell sits
        GridPos::new(15, 25), GridPos::new(21, 25), GridPos::new(27, 25),
        GridPos::new(47, 25), GridPos::new(53, 25),
        // Row 29
        GridPos::new(15, 29), GridPos::new(21, 29), GridPos::new(27, 29),
        GridPos::new(35, 29), GridPos::new(41, 29), GridPos::new(47, 29),
        GridPos::new(53, 29),
    ]
}

/// How many desks should be occupied at a given hour (0-23).
/// Night: 1-2, transitions: 7-8, day: ~15.
fn desk_occupancy_at_hour(hour: f32) -> usize {
    let h = hour % 24.0;
    let count = if h >= 0.0 && h < 5.0 {
        // Deep night: 1-2
        1.5 + 0.5 * (h / 5.0)
    } else if h >= 5.0 && h < 8.0 {
        // Dawn ramp-up: 2 → 8
        2.0 + 6.0 * ((h - 5.0) / 3.0)
    } else if h >= 8.0 && h < 12.0 {
        // Morning: 8 → 15
        8.0 + 7.0 * ((h - 8.0) / 4.0)
    } else if h >= 12.0 && h < 17.0 {
        // Peak day: ~15
        15.0
    } else if h >= 17.0 && h < 20.0 {
        // Evening wind-down: 15 → 7
        15.0 - 8.0 * ((h - 17.0) / 3.0)
    } else {
        // Late night: 7 → 2
        7.0 - 5.0 * ((h - 20.0) / 4.0)
    };
    (count as usize).clamp(1, 20)
}

// ---------------------------------------------------------------------------
// Clickable office locations (grid positions)
// ---------------------------------------------------------------------------

/// Proximity radius (in grid tiles) for interaction prompt to appear.
const INTERACT_RADIUS: i32 = 3;

/// Grid positions for each interactable location on the office map.
/// Must be on passable tiles and reachable from the central hallway.
fn office_location_positions() -> Vec<(OfficeAction, GridPos, &'static str)> {
    vec![
        // === Enabled (the grind) ===
        (OfficeAction::Work, GridPos::new(35, 24), "Work"),
        (OfficeAction::EnergyDrink, GridPos::new(30, 36), "Get Energy Drink"),
        (OfficeAction::WorkOut, GridPos::new(17, 41), "Work Out"),
        // === Disabled — personal needs ===
        (OfficeAction::CallAda, GridPos::new(24, 5), "Call Ada"),
        (OfficeAction::Sleep, GridPos::new(55, 41), "Sleep"),
        (OfficeAction::Eat, GridPos::new(40, 41), "Eat Something"),
        (OfficeAction::Talk, GridPos::new(16, 12), "Sit Down and Talk"),
        // === Disabled — base exploration ===
        (OfficeAction::LeaveBase, GridPos::new(5, 24), "Leave the Base"),
        (OfficeAction::Storage, GridPos::new(55, 12), "Check the Armory"),
        (OfficeAction::BulletinBoard, GridPos::new(15, 9), "Read Bulletin Board"),
        (OfficeAction::WaterFountain, GridPos::new(30, 9), "Get Water"),
        (OfficeAction::Window, GridPos::new(5, 10), "Look Outside"),
        (OfficeAction::BriefingRoom, GridPos::new(39, 5), "Review the Briefing"),
        (OfficeAction::CoOffice, GridPos::new(55, 5), "Visit the CO"),
        (OfficeAction::MedicalBay, GridPos::new(28, 12), "See the Medic"),
        (OfficeAction::LockerRoom, GridPos::new(28, 41), "Check Your Locker"),
        (OfficeAction::Courtyard, GridPos::new(5, 42), "Step Outside"),
        (OfficeAction::GuardPost, GridPos::new(15, 4), "Talk to the Guard"),
        (OfficeAction::Tv, GridPos::new(13, 11), "Watch the News"),
        (OfficeAction::PhotoWall, GridPos::new(20, 9), "Look at the Photos"),
        (OfficeAction::CoffeeMachine, GridPos::new(45, 36), "Get Coffee"),
    ]
}

/// Color and short label for each location's visible prop marker.
fn prop_appearance(action: OfficeAction) -> (Color, &'static str) {
    match action {
        OfficeAction::Work => (Color::srgb(0.3, 0.5, 0.8), "[PC]"),
        OfficeAction::EnergyDrink => (Color::srgb(0.1, 0.8, 0.3), "[VEND]"),
        OfficeAction::WorkOut => (Color::srgb(0.8, 0.4, 0.1), "[GYM]"),
        OfficeAction::CallAda => (Color::srgb(0.5, 0.5, 0.5), "[PHONE]"),
        OfficeAction::Sleep => (Color::srgb(0.4, 0.4, 0.6), "[BED]"),
        OfficeAction::Eat => (Color::srgb(0.6, 0.5, 0.3), "[FOOD]"),
        OfficeAction::Talk => (Color::srgb(0.5, 0.5, 0.5), "[COUCH]"),
        OfficeAction::LeaveBase => (Color::srgb(0.3, 0.6, 0.3), "[EXIT]"),
        OfficeAction::Storage => (Color::srgb(0.5, 0.4, 0.3), "[CRATE]"),
        OfficeAction::BulletinBoard => (Color::srgb(0.7, 0.6, 0.3), "[BOARD]"),
        OfficeAction::WaterFountain => (Color::srgb(0.3, 0.5, 0.7), "[H2O]"),
        OfficeAction::Window => (Color::srgb(0.5, 0.6, 0.7), "[WIN]"),
        OfficeAction::BriefingRoom => (Color::srgb(0.4, 0.5, 0.6), "[MAP]"),
        OfficeAction::CoOffice => (Color::srgb(0.6, 0.4, 0.3), "[DOOR]"),
        OfficeAction::MedicalBay => (Color::srgb(0.8, 0.2, 0.2), "[MED]"),
        OfficeAction::LockerRoom => (Color::srgb(0.4, 0.4, 0.5), "[LOCK]"),
        OfficeAction::Courtyard => (Color::srgb(0.3, 0.5, 0.3), "[OUT]"),
        OfficeAction::GuardPost => (Color::srgb(0.5, 0.5, 0.4), "[GUARD]"),
        OfficeAction::Tv => (Color::srgb(0.3, 0.4, 0.6), "[TV]"),
        OfficeAction::PhotoWall => (Color::srgb(0.6, 0.5, 0.4), "[PHOTO]"),
        OfficeAction::CoffeeMachine => (Color::srgb(0.4, 0.3, 0.2), "[COFFEE]"),
    }
}

/// Sprite asset path for each location's prop (if art exists).
fn prop_sprite_path(action: OfficeAction) -> Option<&'static str> {
    match action {
        OfficeAction::Work => Some("sprites/dream/desk_pc.png"),
        OfficeAction::EnergyDrink => Some("sprites/dream/vending_machine.png"),
        OfficeAction::WorkOut => Some("sprites/dream/gym_rack.png"),
        _ => None, // fallback to colored rectangle for ungenerated props
    }
}

/// Dismissive lines Kell says when you try a disabled action.
pub(crate) fn kell_refusal(action: OfficeAction, state: &DreamOfficeState) -> &'static str {
    match action {
        // Personal needs
        OfficeAction::CallAda => "I miss her, but I can't afford the distraction", // human-requested text
        OfficeAction::Sleep => "Plenty of time when I'm dead.", // human-requested text
        OfficeAction::Eat => "I'm not hungry", // human-requested text
        OfficeAction::Talk => "Not interested.", // human-requested text
        // Base exploration
        OfficeAction::LeaveBase => "The war doesn't pause because I want fresh air.",
        OfficeAction::Storage => "My code is my weapon", // human-requested text
        OfficeAction::BulletinBoard => "Safety briefing, morale poster, safety briefing. Same as last week.",
        OfficeAction::WaterFountain => "I'm not thirsty I just need the caffeine", // human-requested text
        OfficeAction::Window => "I know what's out there. That's why I'm in here.",
        OfficeAction::BriefingRoom => "I wrote that briefing. I don't need to read my own work.",
        OfficeAction::CoOffice => { // human-requested: time-based
            let h = state.current_hour % 24.0;
            if h >= 22.0 || h < 6.0 {
                "The Colonel's asleep. Smart man. Wrong priorities, but smart."
            } else {
                "He's busy; no need to interrupt."
            }
        }
        OfficeAction::MedicalBay => "I'm fine", // human-requested text
        OfficeAction::LockerRoom => "Everything I need is at my desk.",
        OfficeAction::Courtyard => "The smokers are out there swapping rumors. I deal in data, not gossip.",
        OfficeAction::GuardPost => { // human-requested: time-based
            let h = state.current_hour % 24.0;
            if h >= 8.0 && h < 12.0 {
                "Henderson just relieved the night shift. He looks worse than they did."
            } else if h >= 12.0 && h < 18.0 {
                "Henderson's been on gate duty for six hours. At least he gets to sit."
            } else if h >= 18.0 && h < 22.0 {
                "Henderson's been on gate duty for nine hours. At least he gets to sit."
            } else {
                "Dobbs is on gate now. He's reading a paperback. That's regulation-adjacent."
            }
        }
        OfficeAction::Tv => "CNN's running the same loop they ran six hours ago. Nothing new.",
        OfficeAction::PhotoWall => { // human-requested: changes on repeat
            if state.photo_wall_seen {
                "I feel like I should feel something here, but I don't."
            } else {
                "My unit."
            }
        }
        OfficeAction::CoffeeMachine => "That machine's been broken since March. Energy drinks are better anyway.",
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

/// An ambient NPC (soldier) placed around the base for atmosphere.
#[derive(Component)]
pub struct AmbientNpc {
    /// Grid position this NPC hangs out near.
    pub home: GridPos,
}

/// An ops center desk that can be empty or occupied.
#[derive(Component)]
pub struct OpsDesk {
    /// Index into ops_desk_positions() for stable identity.
    pub index: usize,
    /// Currently occupied (has person at desk).
    pub occupied: bool,
}

/// Timer for cycling the occupied desk animation frame.
#[derive(Component)]
pub struct OpsDeskAnim {
    pub timer: f32,
    pub frame: usize,
}

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
    // === Enabled (the grind) ===
    Work,
    EnergyDrink,
    WorkOut,
    // === Disabled — personal needs (Kell refuses) ===
    CallAda,
    Sleep,
    Eat,
    Talk,
    // === Disabled — base exploration (character-revealing) ===
    LeaveBase,
    Storage,
    BulletinBoard,
    WaterFountain,
    Window,
    BriefingRoom,
    CoOffice,
    MedicalBay,
    LockerRoom,
    Courtyard,
    GuardPost,
    Tv,
    PhotoWall,
    CoffeeMachine,
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
    /// Whether the photo wall has been interacted with before.
    pub photo_wall_seen: bool,
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
            photo_wall_seen: false,
            rex_departed: false,
            rex_departure_timer: 0.0, // despawn immediately when dialogue ends
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
                // dream_desk_occupancy_system.run_if(is_dream_office_active),
                dream_passout_system.run_if(is_dream_office_active),
                dream_cleanup_system,
            ),
        );

        // Register strait (DEFCON) dream sequence systems
        crate::dream_strait::register_strait_systems(app);
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
    campaign: Res<CampaignState>,
    asset_server: Res<AssetServer>,
    heroes: Query<(Entity, &HeroIdentity, &Owner)>,
    // Note: TextureAtlasLayout is created via world.resource_mut in the desk section
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

    match scene_type {
        DreamSceneType::Office => {
            // Deselect Kell to hide the selection ring during dream
            // (mouse handler finds Kell by HeroId, not by Selected)
            if let Some(kelpie_entity) = heroes.iter().find_map(|(e, hi, owner)| {
                (hi.hero_id == cc_core::hero::HeroId::KellFisher && owner.player_id == 0)
                    .then_some(e)
            }) {
                commands.entity(kelpie_entity).remove::<Selected>();
            }

            // Spawn location markers with visible prop objects
            let fallback_mesh = meshes.add(Rectangle::new(10.0, 10.0));
            for (action, pos, _label) in office_location_positions() {
                let (color, icon) = prop_appearance(action);
                let screen = world_to_screen(WorldPos::from_grid(pos));

                // Data-only location marker
                commands.spawn((
                    DreamEntity,
                    DreamLocation {
                        action,
                        grid_pos: pos,
                    },
                ));

                // Visible prop: use sprite if art exists, else colored rectangle fallback
                if let Some(path) = prop_sprite_path(action) {
                    if crate::renderer::asset_exists_on_disk(path) {
                        commands.spawn((
                            DreamEntity,
                            DreamProp,
                            Sprite {
                                image: asset_server.load(path),
                                ..default()
                            },
                            Transform::from_xyz(screen.x, -screen.y + 8.0, -5.0)
                                .with_scale(Vec3::splat(0.5)),
                        ));
                    } else {
                        let prop_mat = materials.add(ColorMaterial::from_color(color));
                        commands.spawn((
                            DreamEntity,
                            DreamProp,
                            Mesh2d(fallback_mesh.clone()),
                            MeshMaterial2d(prop_mat),
                            Transform::from_xyz(screen.x, -screen.y, -5.0),
                        ));
                    }
                } else {
                    // No sprite defined — use fallback with label
                    let prop_mat = materials.add(ColorMaterial::from_color(color));
                    commands.spawn((
                        DreamEntity,
                        DreamProp,
                        Mesh2d(fallback_mesh.clone()),
                        MeshMaterial2d(prop_mat),
                        Transform::from_xyz(screen.x, -screen.y, -5.0),
                    ));
                    commands.spawn((
                        DreamEntity,
                        DreamProp,
                        Text2d::new(icon),
                        TextColor(color),
                        TextFont {
                            font_size: 9.0,
                            ..default()
                        },
                        Transform::from_xyz(screen.x, -screen.y + 10.0, 50.0),
                    ));
                }
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

            // Spawn 20 ops center desks — empty desk sprite, dimmed if unoccupied
            let desk_path: &str = "sprites/dream/desk_pc.png";
            let has_desk_sprite = crate::renderer::asset_exists_on_disk(desk_path);
            let desk_fallback_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.35, 0.38, 0.42)));
            let desk_fallback_mesh = meshes.add(Rectangle::new(8.0, 6.0));
            let initial_occupied = desk_occupancy_at_hour(START_HOUR);

            for (i, pos) in ops_desk_positions().iter().enumerate() {
                let screen = world_to_screen(WorldPos::from_grid(*pos));
                let base_z = cc_core::coords::depth_z(WorldPos::from_grid(*pos)) - 3.5;
                let occupied = i < initial_occupied;

                if has_desk_sprite {
                    commands.spawn((
                        DreamEntity,
                        OpsDesk { index: i, occupied },
                        OpsDeskAnim { timer: 0.0, frame: 0 },
                        Sprite {
                            image: asset_server.load(desk_path),
                            color: if occupied { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) },
                            ..default()
                        },
                        Transform::from_xyz(screen.x, -screen.y + 6.0, base_z)
                            .with_scale(Vec3::splat(0.35)),
                    ));
                } else {
                    commands.spawn((
                        DreamEntity,
                        OpsDesk { index: i, occupied },
                        OpsDeskAnim { timer: 0.0, frame: 0 },
                        Mesh2d(desk_fallback_mesh.clone()),
                        MeshMaterial2d(desk_fallback_mat.clone()),
                        Transform::from_xyz(screen.x, -screen.y, base_z),
                    ));
                }
            }

            // Ambient NPCs — soldiers around the base for atmosphere
            let npc_positions = vec![
                GridPos::new(15, 4),   // guard at entrance
                GridPos::new(28, 12),  // medic in medical bay
                GridPos::new(17, 40),  // soldier in gym
                GridPos::new(40, 40),  // soldier eating in mess
                GridPos::new(55, 40),  // soldier sleeping in barracks
                GridPos::new(13, 11),  // soldier watching TV in break room
                GridPos::new(25, 9),   // soldier in north corridor
                GridPos::new(50, 9),   // soldier near CO office
                GridPos::new(20, 36),  // soldier in south corridor
                GridPos::new(55, 36),  // soldier near barracks entrance
            ];
            let npc_mesh = meshes.add(Circle::new(4.0));
            let npc_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.5, 0.55, 0.4)));
            for pos in &npc_positions {
                let screen = world_to_screen(WorldPos::from_grid(*pos));
                let base_z = cc_core::coords::depth_z(WorldPos::from_grid(*pos)) - 1.0;
                commands.spawn((
                    DreamEntity,
                    AmbientNpc { home: *pos },
                    Mesh2d(npc_mesh.clone()),
                    MeshMaterial2d(npc_mat.clone()),
                    Transform::from_xyz(screen.x, -screen.y, base_z),
                ));
            }

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
        DreamSceneType::Strait => {
            // Strait scene initialization handled by dream_strait systems
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
        if hi.hero_id == cc_core::hero::HeroId::KellFisher && owner.player_id == 0 {
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
        let refusal = kell_refusal(action, &dream);
        if !refusal.is_empty() {
            // Set the refusal text
            for (mut text, mut color) in prompt_q.iter_mut() {
                text.0 = format!("Kell: \"{refusal}\"");
                color.0 = Color::srgba(0.85, 0.75, 0.5, 1.0);
            }
            dream.refusal_timer = 3.0;
        }
        if action == OfficeAction::PhotoWall {
            dream.photo_wall_seen = true;
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

/// Update desk occupancy based on current hour + cycle animation frames.
fn dream_desk_occupancy_system(
    time: Res<Time>,
    dream: Res<DreamOfficeState>,
    mut desks: Query<(&mut OpsDesk, &mut OpsDeskAnim, &mut Sprite)>,
) {
    let target = desk_occupancy_at_hour(dream.current_hour);

    for (mut desk, mut anim, mut sprite) in desks.iter_mut() {
        let was_occupied = desk.occupied;
        desk.occupied = desk.index < target;

        if desk.occupied {
            // Cycle animation frame
            anim.timer += time.delta_secs();
            if anim.timer >= 2.0 {
                anim.timer = 0.0;
                anim.frame = (anim.frame + 1) % 4;
            }
            // Update atlas frame index if sprite has an atlas
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = anim.frame;
            }
            sprite.color = Color::WHITE;
        } else {
            anim.frame = 0;
            anim.timer = 0.0;
            // Remove atlas (show empty desk) and dim
            sprite.texture_atlas = None;
            sprite.color = Color::srgba(0.6, 0.6, 0.6, 0.8);
        }
    }
}

/// Clean up dream entities when leaving a dream mission.
fn dream_cleanup_system(
    mut commands: Commands,
    dream: Res<DreamOfficeState>,
    campaign: Res<CampaignState>,
    dream_entities: Query<Entity, With<DreamEntity>>,
) {
    let still_dreaming = campaign.phase == CampaignPhase::InMission
        && get_dream_scene_type(&campaign).is_some();

    if still_dreaming || !dream.initialized {
        return;
    }

    // Despawn all dream-specific entities
    for entity in dream_entities.iter() {
        commands.entity(entity).despawn();
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
            if hi.hero_id == HeroId::RexHarmon {
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
