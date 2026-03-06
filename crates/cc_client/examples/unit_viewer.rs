//! Unit & Building Viewer — browse all 60 unit sprites and 48 building sprites
//! with live Bevy animation pipeline.
//!
//! NOTE: For quick asset browsing, prefer the HTML viewer instead:
//!   python3 -m http.server 8899 & open http://127.0.0.1:8899/tools/asset_pipeline/gallery.html
//! The HTML viewer covers ALL 144 assets (units, buildings, terrain, resources,
//! projectiles, portraits) with animation, search, and filtering — no compilation needed.
//! This Rust viewer is for testing animations through the actual Bevy pipeline.
//!
//! Controls:
//!   Left/Right — navigate items (pauses auto-cycle 5s)
//!   Space      — toggle auto-cycle
//!   C          — toggle compare mode (when candidates exist)
//!   [ / ]      — cycle candidates (in compare mode)
//!   P          — promote candidate to game-ready sprite (in compare mode)
//!   Escape     — quit
//!   Click sidebar item — select it
//!   Click Idle/Walk/Attack — change animation phase (units)
//!   Click Static/Construct/Ambient — change building animation phase (buildings)
//!   Click Auto — toggle auto-cycle
//!   Click Compare (C) — toggle compare mode
//!   Mouse wheel over sidebar — scroll

use std::collections::{HashMap, HashSet};

use bevy::asset::AssetPlugin;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{
    ControlOrientation, CoreScrollbarDragState, CoreScrollbarThumb, Scrollbar, ScrollbarPlugin,
};

use cc_client::loading::LoadingTracker;
use cc_client::renderer::anim_assets::load_anim_assets;
use cc_client::renderer::animation::{self, AnimIndices, AnimState, AnimTimer, PrevAnimState};
use cc_client::renderer::building_anim_assets::{BuildingAnimSheets, load_building_anim_assets};
use cc_client::renderer::building_gen::{
    self, ALL_BUILDING_KINDS, BuildingSprites, building_kind_index, building_scale, building_slug,
};
use cc_client::renderer::tweens::{self, TweenState};
use cc_client::renderer::unit_gen::{self, ALL_KINDS, UnitSprites, kind_index, unit_slug};
use cc_client::setup::{UnitMesh, team_color, unit_scale};
use cc_core::components::{AttackStats, Health, UnitType, Velocity};
use cc_core::math::{FIXED_ZERO, fixed_from_f32};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PHASE_DURATION: f32 = 2.0;
const RESUME_DELAY: f32 = 5.0;
const SIDEBAR_WIDTH: f32 = 210.0;
const UNITS_PER_FACTION: usize = 10;
const BUILDINGS_PER_FACTION: usize = 8;

/// Faction info: (name, player_id, label color).
const FACTIONS: [(&str, u8, Color); 6] = [
    ("catGPT", 0, Color::srgb(0.55, 0.7, 1.0)),
    ("The Clawed", 2, Color::srgb(1.0, 0.82, 0.45)),
    ("The Murder", 1, Color::srgb(1.0, 0.55, 0.55)),
    ("Seekers of the Deep", 3, Color::srgb(0.5, 0.9, 0.6)),
    ("Croak", 4, Color::srgb(0.45, 0.9, 0.9)),
    ("LLAMA", 5, Color::srgb(1.0, 0.7, 0.35)),
];

// ---------------------------------------------------------------------------
// Animation phase enum
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
enum AnimPhase {
    #[default]
    Idle,
    Walk,
    Attack,
}

impl AnimPhase {
    fn label(self) -> &'static str {
        match self {
            AnimPhase::Idle => "Idle",
            AnimPhase::Walk => "Walk",
            AnimPhase::Attack => "Attack",
        }
    }

    fn next(self) -> Self {
        match self {
            AnimPhase::Idle => AnimPhase::Walk,
            AnimPhase::Walk => AnimPhase::Attack,
            AnimPhase::Attack => AnimPhase::Idle,
        }
    }
}

// ---------------------------------------------------------------------------
// Building animation phase enum
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum BuildingAnimPhase {
    #[default]
    Static,
    Construct,
    Ambient,
}

impl BuildingAnimPhase {
    fn label(self) -> &'static str {
        match self {
            BuildingAnimPhase::Static => "Static",
            BuildingAnimPhase::Construct => "Construct",
            BuildingAnimPhase::Ambient => "Ambient",
        }
    }

    fn next(self) -> Self {
        match self {
            BuildingAnimPhase::Static => BuildingAnimPhase::Construct,
            BuildingAnimPhase::Construct => BuildingAnimPhase::Ambient,
            BuildingAnimPhase::Ambient => BuildingAnimPhase::Static,
        }
    }
}

// ---------------------------------------------------------------------------
// Viewer mode — unit or building
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ViewerMode {
    #[default]
    Unit,
    Building,
}

// ---------------------------------------------------------------------------
// Viewer state resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct ViewerState {
    mode: ViewerMode,
    unit_index: usize,
    building_index: usize,
    phase: AnimPhase,
    building_phase: BuildingAnimPhase,
    phase_timer: Timer,
    auto_cycle: bool,
    resume_timer: Timer,
    /// Dirty flag to detect actual changes in swap_display.
    prev_mode: ViewerMode,
    prev_unit_index: usize,
    prev_building_index: usize,
    prev_building_phase: BuildingAnimPhase,
    /// Compare mode state.
    compare_mode: bool,
    candidate_index: usize,
    prev_compare_mode: bool,
    prev_candidate_index: usize,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            mode: ViewerMode::Unit,
            unit_index: 0,
            building_index: 0,
            phase: AnimPhase::Idle,
            building_phase: BuildingAnimPhase::Static,
            phase_timer: Timer::from_seconds(PHASE_DURATION, TimerMode::Repeating),
            auto_cycle: true,
            resume_timer: Timer::from_seconds(RESUME_DELAY, TimerMode::Once),
            prev_mode: ViewerMode::Unit,
            prev_unit_index: usize::MAX,
            prev_building_index: usize::MAX,
            prev_building_phase: BuildingAnimPhase::Ambient,
            compare_mode: false,
            candidate_index: 0,
            prev_compare_mode: false,
            prev_candidate_index: usize::MAX,
        }
    }
}

/// Total items across all factions (units + buildings).
const TOTAL_ITEMS: usize = 60 + 48;

impl ViewerState {
    /// Flat index for the current item (0..107). Units 0..59, buildings 60..107.
    fn flat_index(&self) -> usize {
        match self.mode {
            ViewerMode::Unit => self.unit_index,
            ViewerMode::Building => 60 + self.building_index,
        }
    }

    /// Set from flat index.
    fn set_flat_index(&mut self, idx: usize) {
        if idx < 60 {
            self.mode = ViewerMode::Unit;
            self.unit_index = idx;
        } else {
            self.mode = ViewerMode::Building;
            self.building_index = idx - 60;
        }
    }

    /// Whether the display needs updating.
    fn display_changed(&self) -> bool {
        self.mode != self.prev_mode
            || (self.mode == ViewerMode::Unit && self.unit_index != self.prev_unit_index)
            || (self.mode == ViewerMode::Building
                && (self.building_index != self.prev_building_index
                    || self.building_phase != self.prev_building_phase))
            || self.compare_mode != self.prev_compare_mode
            || (self.compare_mode && self.candidate_index != self.prev_candidate_index)
    }

    fn mark_clean(&mut self) {
        self.prev_mode = self.mode;
        self.prev_unit_index = self.unit_index;
        self.prev_building_index = self.building_index;
        self.prev_building_phase = self.building_phase;
        self.prev_compare_mode = self.compare_mode;
        self.prev_candidate_index = self.candidate_index;
    }
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct ViewerUnit;

/// Separate entity for building display (no animation components).
#[derive(Component)]
struct ViewerBuilding;

#[derive(Component)]
struct UnitNameLabel;

#[derive(Component)]
struct FactionLabel;

/// Sidebar button for a unit (index into ALL_KINDS).
#[derive(Component)]
struct UnitButton(usize);

/// Sidebar button for a building (index into ALL_BUILDING_KINDS).
#[derive(Component)]
struct BuildingButton(usize);

/// Animation phase button.
#[derive(Component)]
struct AnimButton(AnimPhase);

/// Auto-cycle toggle button.
#[derive(Component)]
struct AutoToggle;

/// Timer for building animation frame cycling in the viewer.
#[derive(Component, Deref, DerefMut)]
struct BuildingViewerAnimTimer(Timer);

/// Faction header button (click to collapse/expand).
#[derive(Component)]
struct FactionHeader(usize);

/// Child of a faction section (hidden when collapsed).
#[derive(Component)]
struct FactionChild(usize);

/// The ▼/▶ indicator text inside a faction header.
#[derive(Component)]
struct CollapseIndicator(usize);

/// Tracks which faction sections are collapsed.
#[derive(Resource, Default)]
struct CollapsedFactions(HashSet<usize>);

// ---------------------------------------------------------------------------
// Candidate comparison
// ---------------------------------------------------------------------------

/// Candidate sprites for comparison. Keyed by (unit_index, phase).
#[derive(Resource, Default)]
struct CandidateSprites {
    entries: HashMap<(usize, AnimPhase), Vec<(String, Handle<Image>)>>,
}

/// Marker for the comparison sprite entity.
#[derive(Component)]
struct CompareUnit;

/// Timer for cycling candidate sheet animation frames.
#[derive(Component, Deref, DerefMut)]
struct CompareAnimTimer(Timer);

/// UI label showing current candidate name and index.
#[derive(Component)]
struct CandidateLabel;

/// Compare (C) button in control row.
#[derive(Component)]
struct CompareButton;

/// Status text (e.g. "Promoted!") shown briefly.
#[derive(Component)]
struct StatusText;

/// Timer to auto-hide status text.
#[derive(Resource)]
struct StatusTimer(Timer);

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const SIDEBAR_BG: Color = Color::srgba(0.1, 0.1, 0.14, 0.95);
const BTN_NORMAL: Color = Color::srgba(0.18, 0.18, 0.22, 1.0);
const BTN_HOVER: Color = Color::srgba(0.25, 0.25, 0.30, 1.0);
const BTN_SELECTED: Color = Color::srgba(0.35, 0.35, 0.50, 1.0);
const BTN_AUTO_ON: Color = Color::srgba(0.2, 0.45, 0.2, 1.0);
const BTN_AUTO_OFF: Color = Color::srgba(0.45, 0.2, 0.2, 1.0);
const SECTION_HEADER: Color = Color::srgba(0.5, 0.5, 0.55, 1.0);

// ---------------------------------------------------------------------------
// Display name helpers
// ---------------------------------------------------------------------------

fn slug_to_display(slug: &str) -> String {
    slug.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + c.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn unit_display_name(index: usize) -> String {
    slug_to_display(unit_slug(ALL_KINDS[index]))
}

fn building_display_name(index: usize) -> String {
    slug_to_display(building_slug(ALL_BUILDING_KINDS[index]))
}

// ---------------------------------------------------------------------------
// Candidate loading
// ---------------------------------------------------------------------------

fn load_candidates(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut entries: HashMap<(usize, AnimPhase), Vec<(String, Handle<Image>)>> = HashMap::new();

    // Build slug → unit_index lookup
    let mut slug_to_index: HashMap<&str, usize> = HashMap::new();
    for (i, &kind) in ALL_KINDS.iter().enumerate() {
        slug_to_index.insert(unit_slug(kind), i);
    }

    let phases = [
        ("_idle_", AnimPhase::Idle),
        ("_walk_", AnimPhase::Walk),
        ("_attack_", AnimPhase::Attack),
    ];
    // Also match files ending with _idle.png, _walk.png, _attack.png (no label → "candidate")
    let phase_suffixes = [
        ("_idle", AnimPhase::Idle),
        ("_walk", AnimPhase::Walk),
        ("_attack", AnimPhase::Attack),
    ];

    // Scan candidates directory on disk
    let candidates_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/sprites/units/candidates");
    let Ok(dir_entries) = std::fs::read_dir(&candidates_dir) else {
        commands.insert_resource(CandidateSprites { entries });
        return;
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };

        // Try to match {slug}_{phase}_{label} or {slug}_{phase}
        let mut matched = false;
        for &(phase_mid, phase) in &phases {
            if let Some(pos) = stem.find(phase_mid) {
                let slug = &stem[..pos];
                let label = &stem[pos + phase_mid.len()..];
                if let Some(&unit_idx) = slug_to_index.get(slug) {
                    let asset_path = format!("sprites/units/candidates/{}.png", stem);
                    let handle = asset_server.load::<Image>(&asset_path);
                    entries
                        .entry((unit_idx, phase))
                        .or_default()
                        .push((label.to_string(), handle));
                    matched = true;
                    break;
                }
            }
        }
        if matched {
            continue;
        }
        // Try suffix-only match (no label)
        for &(suffix, phase) in &phase_suffixes {
            if stem.ends_with(suffix) {
                let slug = &stem[..stem.len() - suffix.len()];
                if let Some(&unit_idx) = slug_to_index.get(slug) {
                    let asset_path = format!("sprites/units/candidates/{}.png", stem);
                    let handle = asset_server.load::<Image>(&asset_path);
                    entries
                        .entry((unit_idx, phase))
                        .or_default()
                        .push(("candidate".to_string(), handle));
                    break;
                }
            }
        }
    }

    let count: usize = entries.values().map(|v| v.len()).sum();
    if count > 0 {
        info!("Loaded {} candidate sprites for {} unit/phase combos", count, entries.len());
    }

    commands.insert_resource(CandidateSprites { entries });
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup_viewer(
    mut commands: Commands,
    unit_sprites: Res<UnitSprites>,
    building_sprites: Res<BuildingSprites>,
) {
    // Camera — tight zoom, offset right to account for sidebar
    commands.spawn((
        Camera2d,
        Transform::from_xyz(SIDEBAR_WIDTH * 0.15, 0.0, 999.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.3,
            ..OrthographicProjection::default_2d()
        }),
    ));

    commands.init_resource::<ViewerState>();

    // Spawn the viewer unit entity (visible when mode == Unit)
    let kind = ALL_KINDS[0];
    let idx = kind_index(kind);
    let scale = unit_scale(kind);
    let tint = team_color(FACTIONS[0].1);

    commands.spawn((
        ViewerUnit,
        UnitMesh,
        UnitType { kind },
        Velocity {
            dx: FIXED_ZERO,
            dy: FIXED_ZERO,
        },
        Health {
            current: fixed_from_f32(100.0),
            max: fixed_from_f32(100.0),
        },
        AttackStats {
            damage: fixed_from_f32(10.0),
            range: fixed_from_f32(64.0),
            attack_speed: 10,
            cooldown_remaining: 0,
        },
        Sprite {
            image: unit_sprites.sprites[idx].clone(),
            color: tint,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 10.0).with_scale(Vec3::splat(scale)),
        AnimState::default(),
        PrevAnimState::default(),
        AnimIndices::default(),
        AnimTimer::default(),
        TweenState::new(kind),
    ));

    // Spawn the viewer building entity (visible when mode == Building)
    commands.spawn((
        ViewerBuilding,
        Sprite {
            image: building_sprites.sprites[0].clone(),
            color: tint,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 10.0)
            .with_scale(Vec3::splat(building_scale(ALL_BUILDING_KINDS[0], building_sprites.has_art[0]))),
        Visibility::Hidden,
        BuildingViewerAnimTimer(Timer::from_seconds(0.6, TimerMode::Repeating)),
    ));

    // Spawn compare unit entity (hidden until compare mode active)
    commands.spawn((
        CompareUnit,
        Sprite {
            image: unit_sprites.sprites[0].clone(),
            ..default()
        },
        Transform::from_xyz(80.0, 0.0, 10.0).with_scale(Vec3::splat(scale)),
        Visibility::Hidden,
        CompareAnimTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
    ));

    build_ui(&mut commands);
}

fn build_ui(commands: &mut Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .with_children(|root| {
            // --- Left sidebar (flex row: scroll area + scrollbar track) ---
            root.spawn((
                Node {
                    width: Val::Px(SIDEBAR_WIDTH),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                BackgroundColor(SIDEBAR_BG),
            ))
            .with_children(|sidebar_frame| {
                // Scroll area
                let mut scroll_cmd = sidebar_frame.spawn(Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                });
                let scroll_id = scroll_cmd.id();
                scroll_cmd.with_children(|scroll_area| {
                    for (fi, (faction_name, _, faction_color)) in FACTIONS.iter().enumerate() {
                        // Faction header (clickable, toggles collapse)
                        scroll_area
                            .spawn((
                                FactionHeader(fi),
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    margin: UiRect {
                                        top: Val::Px(if fi == 0 { 4.0 } else { 14.0 }),
                                        bottom: Val::Px(2.0),
                                        ..default()
                                    },
                                    padding: UiRect::horizontal(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_child((
                                CollapseIndicator(fi),
                                Text::new(format!("\u{25BC} {}", faction_name)),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(*faction_color),
                            ));

                        // "Units" sub-header
                        scroll_area.spawn((
                            FactionChild(fi),
                            Text::new("Units"),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(SECTION_HEADER),
                            Node {
                                margin: UiRect {
                                    top: Val::Px(2.0),
                                    bottom: Val::Px(2.0),
                                    left: Val::Px(8.0),
                                    ..default()
                                },
                                ..default()
                            },
                        ));

                        // 10 unit buttons per faction
                        for ui in 0..UNITS_PER_FACTION {
                            let unit_idx = fi * UNITS_PER_FACTION + ui;
                            spawn_sidebar_button(
                                scroll_area,
                                UnitButton(unit_idx),
                                &unit_display_name(unit_idx),
                                fi,
                            );
                        }

                        // "Buildings" sub-header
                        scroll_area.spawn((
                            FactionChild(fi),
                            Text::new("Buildings"),
                            TextFont {
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(SECTION_HEADER),
                            Node {
                                margin: UiRect {
                                    top: Val::Px(6.0),
                                    bottom: Val::Px(2.0),
                                    left: Val::Px(8.0),
                                    ..default()
                                },
                                ..default()
                            },
                        ));

                        // 8 building buttons per faction
                        for bi in 0..BUILDINGS_PER_FACTION {
                            let building_idx = fi * BUILDINGS_PER_FACTION + bi;
                            spawn_sidebar_button(
                                scroll_area,
                                BuildingButton(building_idx),
                                &building_display_name(building_idx),
                                fi,
                            );
                        }
                    }
                });

                // Scrollbar track
                sidebar_frame
                    .spawn((
                        Scrollbar::new(scroll_id, ControlOrientation::Vertical, 20.0),
                        Node {
                            width: Val::Px(8.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.5)),
                    ))
                    .with_child((
                        CoreScrollbarThumb,
                        Hovered(false),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(8.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.7)),
                    ));
            });

            // --- Center area ---
            root.spawn(Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|center| {
                // Item name label
                center.spawn((
                    UnitNameLabel,
                    Text::new(unit_display_name(0)),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                ));

                // Faction label
                center.spawn((
                    FactionLabel,
                    Text::new(FACTIONS[0].0.to_string()),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(FACTIONS[0].2),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                // Animation control row
                center
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::bottom(Val::Px(20.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        for phase in [AnimPhase::Idle, AnimPhase::Walk, AnimPhase::Attack] {
                            let is_active = phase == AnimPhase::Idle;
                            row.spawn((
                                AnimButton(phase),
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(if is_active { BTN_SELECTED } else { BTN_NORMAL }),
                            ))
                            .with_child((
                                Text::new(phase.label()),
                                TextFont {
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        }

                        // Auto toggle
                        row.spawn((
                            AutoToggle,
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                                margin: UiRect::left(Val::Px(12.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(BTN_AUTO_ON),
                        ))
                        .with_child((
                            Text::new("Auto"),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));

                        // Compare toggle
                        row.spawn((
                            CompareButton,
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                                margin: UiRect::left(Val::Px(4.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(BTN_NORMAL),
                            Visibility::Hidden,
                        ))
                        .with_child((
                            Text::new("Compare (C)"),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });

                // Candidate label (below control row)
                center.spawn((
                    CandidateLabel,
                    Text::new(""),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.8, 0.4)),
                    Node {
                        margin: UiRect::bottom(Val::Px(4.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ));

                // Status text (promote feedback)
                center.spawn((
                    StatusText,
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.4, 0.9, 0.4)),
                    Node {
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ));
            });
        });
}

/// Helper: spawn a sidebar button with a text child.
fn spawn_sidebar_button<M: Component>(
    parent: &mut ChildSpawnerCommands,
    marker: M,
    label: &str,
    faction_idx: usize,
) {
    parent
        .spawn((
            marker,
            FactionChild(faction_idx),
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                margin: UiRect::vertical(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(BTN_NORMAL),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
        ));
}

// ---------------------------------------------------------------------------
// Input systems
// ---------------------------------------------------------------------------

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewerState>,
    candidates: Res<CandidateSprites>,
    mut exit: MessageWriter<AppExit>,
    mut status_timer: ResMut<StatusTimer>,
    mut status_q: Query<(&mut Text, &mut Visibility), With<StatusText>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }

    if keys.just_pressed(KeyCode::Space) {
        state.auto_cycle = !state.auto_cycle;
    }

    // Compare mode toggle (C key) — only for units with candidates
    if keys.just_pressed(KeyCode::KeyC) && state.mode == ViewerMode::Unit {
        let key = (state.unit_index, state.phase);
        if candidates.entries.contains_key(&key) {
            state.compare_mode = !state.compare_mode;
            state.candidate_index = 0;
        }
    }

    // Cycle candidates with [/] or Up/Down in compare mode
    if state.compare_mode && state.mode == ViewerMode::Unit {
        let key = (state.unit_index, state.phase);
        if let Some(list) = candidates.entries.get(&key) {
            let count = list.len();
            if count > 0 {
                if keys.just_pressed(KeyCode::BracketLeft) || keys.just_pressed(KeyCode::ArrowUp) {
                    state.candidate_index = if state.candidate_index == 0 {
                        count - 1
                    } else {
                        state.candidate_index - 1
                    };
                }
                if keys.just_pressed(KeyCode::BracketRight) || keys.just_pressed(KeyCode::ArrowDown) {
                    state.candidate_index = (state.candidate_index + 1) % count;
                }
            }
        }
    }

    // Promote candidate (P key)
    if keys.just_pressed(KeyCode::KeyP) && state.compare_mode && state.mode == ViewerMode::Unit {
        let key = (state.unit_index, state.phase);
        if let Some(list) = candidates.entries.get(&key) {
            if let Some((label, _handle)) = list.get(state.candidate_index) {
                let slug = unit_slug(ALL_KINDS[state.unit_index]);
                let phase_str = match state.phase {
                    AnimPhase::Idle => "idle",
                    AnimPhase::Walk => "walk",
                    AnimPhase::Attack => "attack",
                };
                let candidates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets/sprites/units/candidates");
                // Reconstruct candidate filename
                let candidate_file = if label == "candidate" {
                    format!("{}_{}.png", slug, phase_str)
                } else {
                    format!("{}_{}_{}.png", slug, phase_str, label)
                };
                let src = candidates_dir.join(&candidate_file);
                let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("../../assets/sprites/units/{}_{}.png", slug, phase_str));

                if src.exists() {
                    if let Err(e) = std::fs::copy(&src, &dest) {
                        warn!("Failed to promote candidate: {}", e);
                    } else {
                        info!("Promoted {} → {}", candidate_file, dest.display());
                        // Show status text
                        if let Ok((mut text, mut vis)) = status_q.single_mut() {
                            **text = format!("Promoted: {}", label);
                            *vis = Visibility::Inherited;
                            status_timer.0.reset();
                        }
                        state.compare_mode = false;
                    }
                }
            }
        }
    }

    let mut navigated = false;
    if keys.just_pressed(KeyCode::ArrowRight) && !state.compare_mode {
        let idx = (state.flat_index() + 1) % TOTAL_ITEMS;
        state.set_flat_index(idx);
        state.phase = AnimPhase::Idle;
        state.phase_timer.reset();
        state.compare_mode = false;
        state.candidate_index = 0;
        navigated = true;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) && !state.compare_mode {
        let cur = state.flat_index();
        let idx = if cur == 0 { TOTAL_ITEMS - 1 } else { cur - 1 };
        state.set_flat_index(idx);
        state.phase = AnimPhase::Idle;
        state.phase_timer.reset();
        state.compare_mode = false;
        state.candidate_index = 0;
        navigated = true;
    }

    if navigated {
        state.auto_cycle = false;
        state.resume_timer.reset();
    }
}

fn handle_sidebar_clicks(
    mut state: ResMut<ViewerState>,
    unit_query: Query<(&UnitButton, &Interaction), Changed<Interaction>>,
    building_query: Query<(&BuildingButton, &Interaction), Changed<Interaction>>,
) {
    for (btn, interaction) in unit_query.iter() {
        if *interaction == Interaction::Pressed {
            state.mode = ViewerMode::Unit;
            state.unit_index = btn.0;
            state.phase = AnimPhase::Idle;
            state.phase_timer.reset();
            state.auto_cycle = false;
            state.resume_timer.reset();
        }
    }
    for (btn, interaction) in building_query.iter() {
        if *interaction == Interaction::Pressed {
            state.mode = ViewerMode::Building;
            state.building_index = btn.0;
            state.phase = AnimPhase::Idle;
            state.building_phase = BuildingAnimPhase::Static;
            state.phase_timer.reset();
            state.auto_cycle = false;
            state.resume_timer.reset();
        }
    }
}

fn handle_anim_clicks(
    mut state: ResMut<ViewerState>,
    anim_query: Query<(&AnimButton, &Interaction), (Changed<Interaction>, Without<AutoToggle>)>,
    auto_query: Query<&Interaction, (Changed<Interaction>, With<AutoToggle>)>,
) {
    for (btn, interaction) in anim_query.iter() {
        if *interaction == Interaction::Pressed {
            match state.mode {
                ViewerMode::Unit => {
                    state.phase = btn.0;
                    state.candidate_index = 0;
                    state.phase_timer.reset();
                    state.auto_cycle = false;
                    state.resume_timer.reset();
                }
                ViewerMode::Building => {
                    // Map AnimPhase buttons to BuildingAnimPhase
                    state.building_phase = match btn.0 {
                        AnimPhase::Idle => BuildingAnimPhase::Static,
                        AnimPhase::Walk => BuildingAnimPhase::Construct,
                        AnimPhase::Attack => BuildingAnimPhase::Ambient,
                    };
                    state.phase_timer.reset();
                    state.auto_cycle = false;
                    state.resume_timer.reset();
                }
            }
        }
    }

    for interaction in auto_query.iter() {
        if *interaction == Interaction::Pressed {
            state.auto_cycle = !state.auto_cycle;
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-cycle
// ---------------------------------------------------------------------------

fn cycle_viewer(time: Res<Time>, mut state: ResMut<ViewerState>) {
    // Never auto-cycle while in compare mode
    if state.compare_mode {
        return;
    }
    if !state.auto_cycle {
        state.resume_timer.tick(time.delta());
        if state.resume_timer.is_finished() {
            state.auto_cycle = true;
        }
        return;
    }

    state.phase_timer.tick(time.delta());
    if state.phase_timer.just_finished() {
        match state.mode {
            ViewerMode::Unit => {
                let next_phase = state.phase.next();
                if next_phase == AnimPhase::Idle {
                    // Advance to next item
                    let idx = (state.flat_index() + 1) % TOTAL_ITEMS;
                    state.set_flat_index(idx);
                }
                state.phase = next_phase;
            }
            ViewerMode::Building => {
                // Cycle through Static → Construct → Ambient → next building
                let next_phase = state.building_phase.next();
                if next_phase == BuildingAnimPhase::Static {
                    // Completed all phases, advance to next item
                    let idx = (state.flat_index() + 1) % TOTAL_ITEMS;
                    state.set_flat_index(idx);
                    state.phase = AnimPhase::Idle;
                }
                state.building_phase = next_phase;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Display swap — update the viewer entities when selection changes
// ---------------------------------------------------------------------------

fn swap_display(
    mut state: ResMut<ViewerState>,
    unit_sprites: Res<UnitSprites>,
    building_sprites: Res<BuildingSprites>,
    candidates: Res<CandidateSprites>,
    anim_sheets: Option<Res<BuildingAnimSheets>>,
    mut unit_query: Query<
        (
            &mut UnitType,
            &mut Sprite,
            &mut Transform,
            &mut AnimState,
            &mut PrevAnimState,
            &mut TweenState,
            &mut Velocity,
            &mut AttackStats,
            &mut Visibility,
        ),
        (With<ViewerUnit>, Without<ViewerBuilding>, Without<CompareUnit>),
    >,
    mut building_query: Query<
        (&mut Sprite, &mut Transform, &mut Visibility, &mut BuildingViewerAnimTimer),
        (With<ViewerBuilding>, Without<ViewerUnit>, Without<CompareUnit>),
    >,
    mut compare_query: Query<
        (&mut Sprite, &mut Transform, &mut Visibility, &mut CompareAnimTimer),
        (With<CompareUnit>, Without<ViewerUnit>, Without<ViewerBuilding>),
    >,
) {
    if !state.display_changed() {
        return;
    }

    let Ok((
        mut unit_type,
        mut unit_sprite,
        mut unit_transform,
        mut anim_state,
        mut prev_state,
        mut tween,
        _velocity,
        _attack_stats,
        mut unit_vis,
    )) = unit_query.single_mut()
    else {
        return;
    };

    let Ok((mut bld_sprite, mut bld_transform, mut bld_vis, mut bld_timer)) =
        building_query.single_mut()
    else {
        return;
    };

    let Ok((mut cmp_sprite, mut cmp_transform, mut cmp_vis, mut cmp_timer)) =
        compare_query.single_mut()
    else {
        return;
    };

    match state.mode {
        ViewerMode::Unit => {
            *unit_vis = Visibility::Inherited;
            *bld_vis = Visibility::Hidden;

            let kind = ALL_KINDS[state.unit_index];
            let faction_idx = state.unit_index / UNITS_PER_FACTION;
            let player_id = FACTIONS[faction_idx].1;
            let tint = team_color(player_id);
            let scale = unit_scale(kind);

            unit_type.kind = kind;
            unit_sprite.image = unit_sprites.sprites[kind_index(kind)].clone();
            unit_sprite.texture_atlas = None;
            unit_sprite.color = tint;
            unit_transform.scale = Vec3::splat(scale);
            *tween = TweenState::new(kind);
            tween.spawn_timer = 0.0;

            *anim_state = AnimState::Idle;
            *prev_state = PrevAnimState(AnimState::Idle);

            // Compare mode: offset main sprite left, show candidate right
            if state.compare_mode {
                // Main sprite shifts left
                // (reset_viewer_transform will set x=0, so we track offset there)

                let key = (state.unit_index, state.phase);
                if let Some(list) = candidates.entries.get(&key) {
                    let idx = state.candidate_index.min(list.len().saturating_sub(1));
                    let (_label, handle) = &list[idx];

                    cmp_sprite.image = handle.clone();
                    cmp_sprite.color = tint;
                    cmp_transform.scale = Vec3::splat(scale);

                    // For walk/attack sheets (512x128), the compare anim system
                    // auto-detects and sets up TextureAtlas from image dimensions
                    if state.phase != AnimPhase::Idle {
                        cmp_sprite.texture_atlas = None;

                        // Set timer rate based on phase
                        let rate = match state.phase {
                            AnimPhase::Walk => 0.15,
                            AnimPhase::Attack => 0.1,
                            AnimPhase::Idle => 0.15,
                        };
                        cmp_timer.set_duration(std::time::Duration::from_secs_f32(rate));
                        cmp_timer.reset();
                    } else {
                        cmp_sprite.texture_atlas = None;
                    }

                    *cmp_vis = Visibility::Inherited;
                } else {
                    *cmp_vis = Visibility::Hidden;
                }
            } else {
                *cmp_vis = Visibility::Hidden;
            }
        }
        ViewerMode::Building => {
            *unit_vis = Visibility::Hidden;
            *bld_vis = Visibility::Inherited;
            *cmp_vis = Visibility::Hidden;

            let bkind = ALL_BUILDING_KINDS[state.building_index];
            let faction_idx = state.building_index / BUILDINGS_PER_FACTION;
            let player_id = FACTIONS[faction_idx].1;
            let tint = team_color(player_id);
            let bidx = building_kind_index(bkind);
            let has_art = building_sprites.has_art[bidx];
            let scale = building_scale(bkind, has_art);

            bld_sprite.color = tint;
            bld_transform.translation = Vec3::new(0.0, 0.0, 10.0);
            bld_transform.scale = Vec3::splat(scale);

            // Apply building animation phase
            match state.building_phase {
                BuildingAnimPhase::Static => {
                    bld_sprite.image = building_sprites.sprites[bidx].clone();
                    bld_sprite.texture_atlas = None;
                }
                BuildingAnimPhase::Construct => {
                    let sheet = anim_sheets
                        .as_ref()
                        .and_then(|s| s.construct[bidx].as_ref());
                    if let Some((img, layout)) = sheet {
                        bld_sprite.image = img.clone();
                        bld_sprite.texture_atlas = Some(TextureAtlas {
                            layout: layout.clone(),
                            index: 0,
                        });
                        bld_timer.set_duration(std::time::Duration::from_secs_f32(1.0));
                        bld_timer.reset();
                    } else {
                        bld_sprite.image = building_sprites.sprites[bidx].clone();
                        bld_sprite.texture_atlas = None;
                    }
                }
                BuildingAnimPhase::Ambient => {
                    let sheet = anim_sheets
                        .as_ref()
                        .and_then(|s| s.ambient[bidx].as_ref());
                    if let Some((img, layout)) = sheet {
                        bld_sprite.image = img.clone();
                        bld_sprite.texture_atlas = Some(TextureAtlas {
                            layout: layout.clone(),
                            index: 0,
                        });
                        bld_timer.set_duration(std::time::Duration::from_secs_f32(0.6));
                        bld_timer.reset();
                    } else {
                        bld_sprite.image = building_sprites.sprites[bidx].clone();
                        bld_sprite.texture_atlas = None;
                    }
                }
            }
        }
    }

    state.mark_clean();
}

// ---------------------------------------------------------------------------
// Drive animation phase from viewer state (units only)
// ---------------------------------------------------------------------------

fn drive_anim_phase(
    state: Res<ViewerState>,
    mut query: Query<(&mut Velocity, &mut AttackStats), With<ViewerUnit>>,
) {
    if state.mode != ViewerMode::Unit {
        return;
    }
    let Ok((mut velocity, mut attack_stats)) = query.single_mut() else {
        return;
    };

    match state.phase {
        AnimPhase::Idle => {
            velocity.dx = FIXED_ZERO;
            velocity.dy = FIXED_ZERO;
            attack_stats.cooldown_remaining = 0;
        }
        AnimPhase::Walk => {
            velocity.dx = fixed_from_f32(1.0);
            velocity.dy = FIXED_ZERO;
            attack_stats.cooldown_remaining = 0;
        }
        AnimPhase::Attack => {
            velocity.dx = FIXED_ZERO;
            velocity.dy = FIXED_ZERO;
            attack_stats.cooldown_remaining = attack_stats.attack_speed;
        }
    }
}

// ---------------------------------------------------------------------------
// Reset viewer transform before tweens apply additive offsets
// ---------------------------------------------------------------------------

fn reset_viewer_transform(
    state: Res<ViewerState>,
    mut query: Query<&mut Transform, (With<ViewerUnit>, Without<CompareUnit>)>,
    mut compare_query: Query<&mut Transform, (With<CompareUnit>, Without<ViewerUnit>)>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };
    let x_offset = if state.compare_mode { -80.0 } else { 0.0 };
    transform.translation = Vec3::new(x_offset, 0.0, 10.0);
    transform.rotation = Quat::IDENTITY;

    if let Ok(mut cmp_transform) = compare_query.single_mut() {
        cmp_transform.translation = Vec3::new(if state.compare_mode { 80.0 } else { 0.0 }, 0.0, 10.0);
    }
}

// ---------------------------------------------------------------------------
// Update UI labels
// ---------------------------------------------------------------------------

fn update_labels(
    state: Res<ViewerState>,
    mut name_q: Query<&mut Text, (With<UnitNameLabel>, Without<FactionLabel>)>,
    mut faction_q: Query<(&mut Text, &mut TextColor), (With<FactionLabel>, Without<UnitNameLabel>)>,
) {
    if !state.is_changed() {
        return;
    }

    let (name, faction_idx) = match state.mode {
        ViewerMode::Unit => (
            unit_display_name(state.unit_index),
            state.unit_index / UNITS_PER_FACTION,
        ),
        ViewerMode::Building => (
            building_display_name(state.building_index),
            state.building_index / BUILDINGS_PER_FACTION,
        ),
    };
    let (faction_name, _, faction_color) = FACTIONS[faction_idx];

    if let Ok(mut text) = name_q.single_mut() {
        **text = name;
    }
    if let Ok((mut text, mut color)) = faction_q.single_mut() {
        **text = faction_name.to_string();
        *color = TextColor(faction_color);
    }
}

// ---------------------------------------------------------------------------
// Update button highlight colors
// ---------------------------------------------------------------------------

fn update_sidebar_colors(
    state: Res<ViewerState>,
    mut unit_query: Query<
        (&UnitButton, &Interaction, &mut BackgroundColor),
        Without<BuildingButton>,
    >,
    mut building_query: Query<
        (&BuildingButton, &Interaction, &mut BackgroundColor),
        Without<UnitButton>,
    >,
) {
    let is_unit = state.mode == ViewerMode::Unit;
    for (btn, interaction, mut bg) in unit_query.iter_mut() {
        *bg = if is_unit && btn.0 == state.unit_index {
            BackgroundColor(BTN_SELECTED)
        } else if *interaction == Interaction::Hovered {
            BackgroundColor(BTN_HOVER)
        } else {
            BackgroundColor(BTN_NORMAL)
        };
    }

    let is_building = state.mode == ViewerMode::Building;
    for (btn, interaction, mut bg) in building_query.iter_mut() {
        *bg = if is_building && btn.0 == state.building_index {
            BackgroundColor(BTN_SELECTED)
        } else if *interaction == Interaction::Hovered {
            BackgroundColor(BTN_HOVER)
        } else {
            BackgroundColor(BTN_NORMAL)
        };
    }
}

fn update_anim_button_colors(
    state: Res<ViewerState>,
    mut query: Query<(&AnimButton, &Interaction, &mut BackgroundColor), Without<AutoToggle>>,
    mut auto_query: Query<&mut BackgroundColor, With<AutoToggle>>,
) {
    for (btn, interaction, mut bg) in query.iter_mut() {
        let is_active = match state.mode {
            ViewerMode::Unit => btn.0 == state.phase,
            ViewerMode::Building => {
                // Map AnimPhase buttons to BuildingAnimPhase for highlight
                match btn.0 {
                    AnimPhase::Idle => state.building_phase == BuildingAnimPhase::Static,
                    AnimPhase::Walk => state.building_phase == BuildingAnimPhase::Construct,
                    AnimPhase::Attack => state.building_phase == BuildingAnimPhase::Ambient,
                }
            }
        };
        *bg = if is_active {
            BackgroundColor(BTN_SELECTED)
        } else if *interaction == Interaction::Hovered {
            BackgroundColor(BTN_HOVER)
        } else {
            BackgroundColor(BTN_NORMAL)
        };
    }

    if let Ok(mut bg) = auto_query.single_mut() {
        *bg = if state.auto_cycle {
            BackgroundColor(BTN_AUTO_ON)
        } else {
            BackgroundColor(BTN_AUTO_OFF)
        };
    }
}

// ---------------------------------------------------------------------------
// Update anim button labels based on mode (Unit vs Building)
// ---------------------------------------------------------------------------

fn update_anim_button_labels(
    state: Res<ViewerState>,
    anim_buttons: Query<(&AnimButton, &Children), Without<AutoToggle>>,
    mut texts: Query<&mut Text>,
) {
    if !state.is_changed() {
        return;
    }
    for (btn, children) in anim_buttons.iter() {
        let label = match state.mode {
            ViewerMode::Unit => btn.0.label(),
            ViewerMode::Building => match btn.0 {
                AnimPhase::Idle => BuildingAnimPhase::Static.label(),
                AnimPhase::Walk => BuildingAnimPhase::Construct.label(),
                AnimPhase::Attack => BuildingAnimPhase::Ambient.label(),
            },
        };
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = label.to_string();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Compare UI updates
// ---------------------------------------------------------------------------

fn update_compare_ui(
    state: Res<ViewerState>,
    candidates: Res<CandidateSprites>,
    mut compare_btn_q: Query<(&mut BackgroundColor, &mut Visibility), (With<CompareButton>, Without<CandidateLabel>)>,
    mut label_q: Query<(&mut Text, &mut Visibility), (With<CandidateLabel>, Without<CompareButton>)>,
) {
    if !state.is_changed() && !candidates.is_changed() {
        return;
    }

    // Show/hide compare button based on whether candidates exist
    let has_candidates = state.mode == ViewerMode::Unit
        && candidates
            .entries
            .contains_key(&(state.unit_index, state.phase));

    if let Ok((mut bg, mut vis)) = compare_btn_q.single_mut() {
        *vis = if has_candidates {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *bg = if state.compare_mode {
            BackgroundColor(BTN_SELECTED)
        } else {
            BackgroundColor(BTN_NORMAL)
        };
    }

    // Update candidate label
    if let Ok((mut text, mut vis)) = label_q.single_mut() {
        if state.compare_mode && has_candidates {
            let key = (state.unit_index, state.phase);
            if let Some(list) = candidates.entries.get(&key) {
                let idx = state.candidate_index.min(list.len().saturating_sub(1));
                let (label, _) = &list[idx];
                **text = format!("{} ({}/{})\u{2003}[P] Promote", label, idx + 1, list.len());
                *vis = Visibility::Inherited;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn handle_compare_click(
    mut state: ResMut<ViewerState>,
    candidates: Res<CandidateSprites>,
    query: Query<&Interaction, (Changed<Interaction>, With<CompareButton>)>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed && state.mode == ViewerMode::Unit {
            let key = (state.unit_index, state.phase);
            if candidates.entries.contains_key(&key) {
                state.compare_mode = !state.compare_mode;
                state.candidate_index = 0;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Building animation frame advancement
// ---------------------------------------------------------------------------

fn advance_building_viewer_anim(
    time: Res<Time>,
    state: Res<ViewerState>,
    mut query: Query<(&mut Sprite, &mut BuildingViewerAnimTimer), With<ViewerBuilding>>,
) {
    if state.mode != ViewerMode::Building {
        return;
    }
    // Only animate when in Construct or Ambient phase
    if state.building_phase == BuildingAnimPhase::Static {
        return;
    }
    let Ok((mut sprite, mut timer)) = query.single_mut() else {
        return;
    };
    timer.tick(time.delta());
    if timer.just_finished() {
        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = (atlas.index + 1) % 4;
        }
    }
}

// ---------------------------------------------------------------------------
// Collapsible faction sections
// ---------------------------------------------------------------------------

fn handle_faction_collapse(
    mut collapsed: ResMut<CollapsedFactions>,
    query: Query<(&FactionHeader, &Interaction), Changed<Interaction>>,
) {
    for (header, interaction) in query.iter() {
        if *interaction == Interaction::Pressed {
            if collapsed.0.contains(&header.0) {
                collapsed.0.remove(&header.0);
            } else {
                collapsed.0.insert(header.0);
            }
        }
    }
}

fn update_faction_visibility(
    collapsed: Res<CollapsedFactions>,
    mut child_query: Query<(&FactionChild, &mut Node), Without<CollapseIndicator>>,
    mut indicator_query: Query<(&CollapseIndicator, &mut Text)>,
) {
    if !collapsed.is_changed() {
        return;
    }

    for (fc, mut node) in child_query.iter_mut() {
        node.display = if collapsed.0.contains(&fc.0) {
            Display::None
        } else {
            Display::Flex
        };
    }

    for (ci, mut text) in indicator_query.iter_mut() {
        let faction_name = FACTIONS[ci.0].0;
        if collapsed.0.contains(&ci.0) {
            **text = format!("\u{25B6} {}", faction_name);
        } else {
            **text = format!("\u{25BC} {}", faction_name);
        }
    }
}

// ---------------------------------------------------------------------------
// Scrollbar thumb hover/drag styling
// ---------------------------------------------------------------------------

fn style_scrollbar_thumb(
    mut query: Query<
        (&mut BackgroundColor, Option<&Hovered>, &CoreScrollbarDragState),
        (
            With<CoreScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<CoreScrollbarDragState>)>,
        ),
    >,
) {
    for (mut bg, hovered, drag_state) in query.iter_mut() {
        let is_hovered = hovered.is_some_and(|h| h.0);
        *bg = if drag_state.dragging {
            BackgroundColor(Color::srgba(0.7, 0.7, 0.8, 0.9))
        } else if is_hovered {
            BackgroundColor(Color::srgba(0.55, 0.55, 0.65, 0.85))
        } else {
            BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.7))
        };
    }
}

// ---------------------------------------------------------------------------
// Compare animation frame cycling (for walk/attack sheet candidates)
// ---------------------------------------------------------------------------

fn cycle_compare_anim(
    time: Res<Time>,
    state: Res<ViewerState>,
    images: Res<Assets<Image>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut query: Query<(&mut Sprite, &mut CompareAnimTimer), With<CompareUnit>>,
) {
    if !state.compare_mode || state.phase == AnimPhase::Idle {
        return;
    }
    let Ok((mut sprite, mut timer)) = query.single_mut() else {
        return;
    };
    timer.tick(time.delta());

    // Ensure atlas is set up for sheet sprites
    if sprite.texture_atlas.is_none() {
        // Check if the image is a sheet (width > height means multi-frame)
        if let Some(img) = images.get(&sprite.image) {
            let w = img.width();
            let h = img.height();
            if w > h {
                let frames = (w / h).max(1);
                let layout =
                    TextureAtlasLayout::from_grid(UVec2::new(h, h), frames, 1, None, None);
                let layout_handle = atlas_layouts.add(layout);
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: layout_handle,
                    index: 0,
                });
            }
        }
    }

    if timer.just_finished() {
        if let Some(ref mut atlas) = sprite.texture_atlas {
            if let Some(layout) = atlas_layouts.get(&atlas.layout) {
                let frame_count = layout.textures.len();
                atlas.index = (atlas.index + 1) % frame_count;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Status text auto-hide
// ---------------------------------------------------------------------------

fn hide_status_text(
    time: Res<Time>,
    mut timer: ResMut<StatusTimer>,
    mut query: Query<&mut Visibility, With<StatusText>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        if let Ok(mut vis) = query.single_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Unit Viewer — ClawedCommand".into(),
                        resolution: (1100u32, 720u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "../../assets".to_string(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(ScrollbarPlugin)
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.12)))
        .init_resource::<LoadingTracker>()
        .init_resource::<CollapsedFactions>()
        .insert_resource(StatusTimer(Timer::from_seconds(2.0, TimerMode::Once)))
        .add_systems(
            PreStartup,
            (
                unit_gen::generate_unit_sprites,
                building_gen::generate_building_sprites,
                load_anim_assets,
                load_building_anim_assets,
            ),
        )
        .add_systems(Startup, (setup_viewer, load_candidates).chain())
        .add_systems(
            Update,
            (
                // Input
                handle_input,
                handle_sidebar_clicks.after(handle_input),
                handle_anim_clicks.after(handle_input),
                handle_compare_click.after(handle_input),
                // Auto-cycle
                cycle_viewer
                    .after(handle_sidebar_clicks)
                    .after(handle_anim_clicks),
                // Display swap + animation drive
                swap_display.after(cycle_viewer),
                drive_anim_phase.after(swap_display),
                // Reset transform before tweens add offsets
                reset_viewer_transform.after(drive_anim_phase),
                // Core animation pipeline
                animation::derive_anim_state.after(reset_viewer_transform),
                animation::advance_animation.after(animation::derive_anim_state),
                tweens::apply_unit_tweens.after(animation::advance_animation),
                // Building + compare animation
                advance_building_viewer_anim.after(swap_display),
                cycle_compare_anim.after(swap_display),
                // Collapse & scrollbar
                handle_faction_collapse.after(handle_input),
                update_faction_visibility.after(handle_faction_collapse),
                style_scrollbar_thumb,
            ),
        )
        .add_systems(
            Update,
            (
                update_labels.after(swap_display),
                update_sidebar_colors.after(swap_display),
                update_anim_button_colors.after(swap_display),
                update_anim_button_labels.after(swap_display),
                update_compare_ui.after(swap_display),
                hide_status_text,
            ),
        )
        .run();
}
