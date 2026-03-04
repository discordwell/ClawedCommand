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
//!   Escape     — quit
//!   Click sidebar item — select it
//!   Click Idle/Walk/Attack — change animation phase (units only)
//!   Click Auto — toggle auto-cycle
//!   Mouse wheel over sidebar — scroll

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use cc_client::loading::LoadingTracker;
use cc_client::renderer::anim_assets::load_anim_assets;
use cc_client::renderer::animation::{self, AnimIndices, AnimState, AnimTimer, PrevAnimState};
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
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
    phase_timer: Timer,
    auto_cycle: bool,
    resume_timer: Timer,
    /// Dirty flag to detect actual changes in swap_display.
    prev_mode: ViewerMode,
    prev_unit_index: usize,
    prev_building_index: usize,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            mode: ViewerMode::Unit,
            unit_index: 0,
            building_index: 0,
            phase: AnimPhase::Idle,
            phase_timer: Timer::from_seconds(PHASE_DURATION, TimerMode::Repeating),
            auto_cycle: true,
            resume_timer: Timer::from_seconds(RESUME_DELAY, TimerMode::Once),
            prev_mode: ViewerMode::Building, // Force first-frame swap
            prev_unit_index: usize::MAX,
            prev_building_index: usize::MAX,
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
                && self.building_index != self.prev_building_index)
    }

    fn mark_clean(&mut self) {
        self.prev_mode = self.mode;
        self.prev_unit_index = self.unit_index;
        self.prev_building_index = self.building_index;
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
            // --- Left sidebar ---
            root.spawn((
                Node {
                    width: Val::Px(SIDEBAR_WIDTH),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(SIDEBAR_BG),
            ))
            .with_children(|sidebar| {
                for (fi, (faction_name, _, faction_color)) in FACTIONS.iter().enumerate() {
                    // Faction header
                    sidebar.spawn((
                        Text::new(faction_name.to_string()),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(*faction_color),
                        Node {
                            margin: UiRect {
                                top: Val::Px(if fi == 0 { 4.0 } else { 14.0 }),
                                bottom: Val::Px(2.0),
                                left: Val::Px(4.0),
                                ..default()
                            },
                            ..default()
                        },
                    ));

                    // "Units" sub-header
                    sidebar.spawn((
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
                        spawn_sidebar_button::<UnitButton>(
                            sidebar,
                            UnitButton(unit_idx),
                            &unit_display_name(unit_idx),
                        );
                    }

                    // "Buildings" sub-header
                    sidebar.spawn((
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
                        spawn_sidebar_button::<BuildingButton>(
                            sidebar,
                            BuildingButton(building_idx),
                            &building_display_name(building_idx),
                        );
                    }
                }
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
                    });
            });
        });
}

/// Helper: spawn a sidebar button with a text child.
fn spawn_sidebar_button<M: Component>(
    parent: &mut ChildSpawnerCommands,
    marker: M,
    label: &str,
) {
    parent
        .spawn((
            marker,
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
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }

    if keys.just_pressed(KeyCode::Space) {
        state.auto_cycle = !state.auto_cycle;
    }

    let mut navigated = false;
    if keys.just_pressed(KeyCode::ArrowRight) {
        let idx = (state.flat_index() + 1) % TOTAL_ITEMS;
        state.set_flat_index(idx);
        state.phase = AnimPhase::Idle;
        state.phase_timer.reset();
        navigated = true;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        let cur = state.flat_index();
        let idx = if cur == 0 { TOTAL_ITEMS - 1 } else { cur - 1 };
        state.set_flat_index(idx);
        state.phase = AnimPhase::Idle;
        state.phase_timer.reset();
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
    // Anim buttons only affect units
    if state.mode == ViewerMode::Unit {
        for (btn, interaction) in anim_query.iter() {
            if *interaction == Interaction::Pressed {
                state.phase = btn.0;
                state.phase_timer.reset();
                state.auto_cycle = false;
                state.resume_timer.reset();
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
                // Buildings have no animation phases — just advance after one timer tick
                let idx = (state.flat_index() + 1) % TOTAL_ITEMS;
                state.set_flat_index(idx);
                state.phase = AnimPhase::Idle;
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
        (With<ViewerUnit>, Without<ViewerBuilding>),
    >,
    mut building_query: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        (With<ViewerBuilding>, Without<ViewerUnit>),
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

    let Ok((mut bld_sprite, mut bld_transform, mut bld_vis)) = building_query.single_mut() else {
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
        }
        ViewerMode::Building => {
            *unit_vis = Visibility::Hidden;
            *bld_vis = Visibility::Inherited;

            let bkind = ALL_BUILDING_KINDS[state.building_index];
            let faction_idx = state.building_index / BUILDINGS_PER_FACTION;
            let player_id = FACTIONS[faction_idx].1;
            let tint = team_color(player_id);
            let bidx = building_kind_index(bkind);
            let has_art = building_sprites.has_art[bidx];
            let scale = building_scale(bkind, has_art);

            bld_sprite.image = building_sprites.sprites[bidx].clone();
            bld_sprite.color = tint;
            bld_transform.translation = Vec3::new(0.0, 0.0, 10.0);
            bld_transform.scale = Vec3::splat(scale);
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

fn reset_viewer_transform(mut query: Query<&mut Transform, With<ViewerUnit>>) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };
    transform.translation = Vec3::new(0.0, 0.0, 10.0);
    transform.rotation = Quat::IDENTITY;
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
    let dimmed = Color::srgba(0.15, 0.15, 0.18, 0.5);
    for (btn, interaction, mut bg) in query.iter_mut() {
        if state.mode == ViewerMode::Building {
            // Dim anim buttons when showing a building
            *bg = BackgroundColor(dimmed);
        } else {
            *bg = if btn.0 == state.phase {
                BackgroundColor(BTN_SELECTED)
            } else if *interaction == Interaction::Hovered {
                BackgroundColor(BTN_HOVER)
            } else {
                BackgroundColor(BTN_NORMAL)
            };
        }
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
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.12)))
        .init_resource::<LoadingTracker>()
        .add_systems(
            PreStartup,
            (
                unit_gen::generate_unit_sprites,
                building_gen::generate_building_sprites,
                load_anim_assets,
            ),
        )
        .add_systems(Startup, setup_viewer)
        .add_systems(
            Update,
            (
                // Input
                handle_input,
                handle_sidebar_clicks.after(handle_input),
                handle_anim_clicks.after(handle_input),
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
                // UI updates
                update_labels.after(swap_display),
                update_sidebar_colors.after(swap_display),
                update_anim_button_colors.after(swap_display),
            ),
        )
        .run();
}
