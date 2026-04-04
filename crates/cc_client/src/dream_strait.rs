//! DEFCON-style drone warfare dream sequence — "The Strait".
//!
//! Activated by `DreamSequence { scene_type: Strait }` mission mutator.
//! Commander Kell Fisher protects oil tankers transiting a narrow strait
//! from hostile coastal launchers using patrol drones, satellite vision,
//! interceptors, and zero-day exploits.

use bevy::prelude::*;

use cc_core::coords::GridPos;
use cc_core::mutator::{DreamSceneType, MissionMutator};
use cc_core::strait::*;
use cc_sim::campaign::state::{CampaignPhase, CampaignState};

use crate::dream::DreamEntity;

// ---------------------------------------------------------------------------
// Constants (rendering)
// ---------------------------------------------------------------------------

/// Terminal green for HUD text.
const TERM_GREEN: Color = Color::srgb(0.2, 0.9, 0.3);
/// Dim terminal green for secondary info.
const TERM_GREEN_DIM: Color = Color::srgb(0.15, 0.6, 0.2);
/// Hostile red for enemy elements.
const HOSTILE_RED: Color = Color::srgb(0.9, 0.15, 0.1);
/// Warning amber for missiles and alerts.
const WARNING_AMBER: Color = Color::srgb(0.95, 0.7, 0.1);
/// Friendly blue for tankers.
const TANKER_BLUE: Color = Color::srgb(0.3, 0.5, 0.9);
/// Dark background for DEFCON overlay.
const DEFCON_BG: Color = Color::srgba(0.02, 0.03, 0.08, 0.95);

/// Size of the radar sweep mesh (in pixels, radius).
#[allow(dead_code)]
const RADAR_SWEEP_RADIUS: f32 = 48.0;
/// Rotation speed of radar sweep (radians per second).
const RADAR_SWEEP_SPEED: f32 = std::f32::consts::TAU / 2.0;

// ---------------------------------------------------------------------------
// Bevy Components
// ---------------------------------------------------------------------------

/// Marker for a friendly patrol drone entity.
#[derive(Component)]
pub struct StraitDrone {
    pub patrol_waypoints: Vec<GridPos>,
    pub current_wp_index: usize,
    pub alive: bool,
    pub drone_id: u32,
}

/// Marker for a tanker entity in the convoy.
#[derive(Component)]
pub struct StraitTanker {
    pub hp: u32,
    pub lane_y: i32,
    pub world_x: f32,
    pub target_x: f32,
    pub arrived: bool,
    pub destroyed: bool,
    pub tanker_index: u32,
}

/// An in-flight anti-ship missile.
#[derive(Component)]
pub struct StraitMissile {
    pub state: MissileState,
    pub target_tanker: Option<Entity>,
    pub age_ticks: u32,
}

/// An enemy mobile launcher on the hostile coast.
#[derive(Component)]
pub struct StraitLauncher {
    pub phase: LauncherPhase,
    pub phase_timer: f32,
    pub hidden_pos: GridPos,
    pub firing_pos: GridPos,
    pub is_decoy: bool,
    pub salvo_count: u32,
    pub has_fired_this_phase: bool,
}

/// An enemy AA drone that hunts player patrol drones.
#[derive(Component)]
pub struct StraitAaDrone {
    pub target_drone: Option<Entity>,
    pub world_x: f32,
    pub world_y: f32,
    pub alive: bool,
}

/// A temporary satellite scan overlay.
#[derive(Component)]
pub struct StraitSatelliteScan {
    pub center: GridPos,
    pub remaining_ticks: u32,
}

/// Rotating radar sweep visual on a drone.
#[derive(Component)]
pub struct StraitRadarSweep {
    pub angle: f32,
}

/// Pre-cached coastline contour lines for DEFCON rendering.
#[derive(Resource, Default)]
pub struct StraitContourCache {
    /// Line segments for coastline contours (start, end in screen space).
    pub coastline_segments: Vec<(Vec2, Vec2)>,
    /// Shipping lane line (start, end).
    pub shipping_lane: Option<(Vec2, Vec2)>,
}

/// Marker for a selected drone (visual feedback).
#[derive(Component)]
pub struct StraitDroneSelected;

// ---------------------------------------------------------------------------
// Input resources
// ---------------------------------------------------------------------------

/// Whether the strait input system consumed this frame's mouse click.
/// Mirrors the `MinimapClickConsumed` pattern in `input/mouse.rs`.
#[derive(Resource, Default)]
pub struct StraitMouseConsumed(pub bool);

/// Current input mode for the strait mission.
#[derive(Default, PartialEq, Debug)]
pub enum StraitInputMode {
    #[default]
    Normal,
    SatelliteScan,
    ZeroDayDeploy(ZeroDayType),
}

/// Player input state for the strait dream.
#[derive(Resource, Default)]
pub struct StraitInputState {
    pub selected_drones: Vec<Entity>,
    pub mode: StraitInputMode,
}

// ---------------------------------------------------------------------------
// HUD marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct StraitComputeHud;

#[derive(Component)]
pub struct StraitInterceptorHud;

#[derive(Component)]
pub struct StraitTankerHud;

#[derive(Component)]
pub struct StraitZeroDayHud;

#[derive(Component)]
pub struct StraitStatusHud;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Master state for the strait mission.
#[derive(Resource)]
pub struct StraitState {
    pub initialized: bool,

    // Compute budget
    pub compute: f32,
    pub max_compute: f32,
    pub allocation: ComputeAllocation,

    // Interceptors
    pub interceptor_count: u32,
    pub interceptor_regen_timer: u32,

    // Tanker tracking
    pub tankers_spawned: u32,
    pub tankers_arrived: u32,
    pub tankers_destroyed: u32,
    pub tanker_spawn_timer: u32,

    // Drone tracking
    pub next_drone_id: u32,
    pub drones_alive: u32,

    // Zero-day pipeline
    pub zero_day_slot: ZeroDayState,
    pub zero_days_deployed: [bool; 4], // indexed by ZeroDayType ordinal

    // Enemy escalation
    pub current_wave: u32,
    pub wave_config: EnemyWaveConfig,
    pub enemy_launchers_spawned: bool,

    // Mission phase
    pub mission_tick: u64,
    pub mission_complete: bool,

    // Scripts registered for execution each tick
    pub active_scripts: Vec<StraitScript>,
}

/// A Lua script registered for execution during the strait dream.
pub struct StraitScript {
    pub name: String,
    pub source: String,
    /// Ticks between executions.
    pub interval: u64,
    /// Tick counter since last run.
    pub ticks_since_run: u64,
}

impl Default for StraitState {
    fn default() -> Self {
        Self {
            initialized: false,
            compute: INITIAL_COMPUTE,
            max_compute: INITIAL_COMPUTE,
            allocation: ComputeAllocation::default(),
            interceptor_count: INITIAL_INTERCEPTORS,
            interceptor_regen_timer: 0,
            tankers_spawned: 0,
            tankers_arrived: 0,
            tankers_destroyed: 0,
            tanker_spawn_timer: 0,
            next_drone_id: 0,
            drones_alive: 0,
            zero_day_slot: ZeroDayState::default(),
            zero_days_deployed: [false; 4],
            current_wave: 1,
            wave_config: EnemyWaveConfig::wave_1(),
            enemy_launchers_spawned: false,
            mission_tick: 0,
            mission_complete: false,
            active_scripts: Vec::new(),
        }
    }
}

/// Per-tile vision state for the strait.
#[derive(Resource)]
pub struct StraitVision {
    pub width: usize,
    pub height: usize,
    /// true = visible this frame.
    pub visible: Vec<bool>,
}

impl StraitVision {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            width: w,
            height: h,
            visible: vec![false; w * h],
        }
    }

    pub fn clear(&mut self) {
        self.visible.fill(false);
    }

    pub fn reveal(&mut self, cx: i32, cy: i32, radius: i32) {
        let r2 = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= r2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                        self.visible[y as usize * self.width + x as usize] = true;
                    }
                }
            }
        }
    }

    pub fn is_visible(&self, x: i32, y: i32) -> bool {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.visible[y as usize * self.width + x as usize]
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Run condition
// ---------------------------------------------------------------------------

pub fn is_dream_strait_active(campaign: Res<CampaignState>) -> bool {
    if campaign.phase != CampaignPhase::InMission {
        return false;
    }
    campaign.current_mission.as_ref().is_some_and(|m| {
        m.mutators.iter().any(|mt| {
            matches!(
                mt,
                MissionMutator::DreamSequence {
                    scene_type: DreamSceneType::Strait,
                    ..
                }
            )
        })
    })
}

// ---------------------------------------------------------------------------
// Plugin registration (called from DreamPlugin)
// ---------------------------------------------------------------------------

/// Register all strait systems. Called from `DreamPlugin::build()`.
pub fn register_strait_systems(app: &mut App) {
    app.init_resource::<StraitState>()
        .init_resource::<StraitInputState>()
        .init_resource::<StraitMouseConsumed>()
        .insert_resource(StraitVision::new(300, 60))
        .init_resource::<StraitContourCache>()
        .add_systems(
            PreUpdate,
            (
                strait_reset_consumed.run_if(is_dream_strait_active),
                strait_input_system
                    .after(strait_reset_consumed)
                    .run_if(is_dream_strait_active),
            ),
        )
        // Simulation systems (group 1: init, core loop, drones, tankers)
        .add_systems(
            Update,
            (
                strait_init_system.run_if(is_dream_strait_active),
                strait_tick_system
                    .after(strait_init_system)
                    .run_if(is_dream_strait_active),
                strait_compute_regen
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_interceptor_regen
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_spawn_tankers
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_move_tankers
                    .after(strait_spawn_tankers)
                    .run_if(is_dream_strait_active),
                strait_move_drones
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_update_vision
                    .after(strait_move_drones)
                    .run_if(is_dream_strait_active),
                strait_enemy_director
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_spawn_wave_entities
                    .after(strait_enemy_director)
                    .run_if(is_dream_strait_active),
            ),
        )
        // Simulation systems (group 2: combat, rendering, win/lose)
        .add_systems(
            Update,
            (
                strait_launcher_fsm
                    .run_if(is_dream_strait_active),
                strait_spawn_missiles
                    .after(strait_launcher_fsm)
                    .run_if(is_dream_strait_active),
                strait_missile_flight
                    .after(strait_spawn_missiles)
                    .run_if(is_dream_strait_active),
                strait_missile_interception
                    .after(strait_missile_flight)
                    .run_if(is_dream_strait_active),
                strait_missile_impact
                    .after(strait_missile_interception)
                    .run_if(is_dream_strait_active),
                strait_enemy_aa
                    .run_if(is_dream_strait_active),
                strait_render_defcon
                    .run_if(is_dream_strait_active),
                strait_radar_sweep
                    .run_if(is_dream_strait_active),
                strait_update_hud
                    .run_if(is_dream_strait_active),
                strait_satellite_decay
                    .run_if(is_dream_strait_active),
                strait_check_win_lose
                    .run_if(is_dream_strait_active),
            ),
        );

    // Script runner on FixedUpdate (native only — mlua not available on WASM)
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(
        FixedUpdate,
        strait_script_runner.run_if(is_dream_strait_active),
    );
}

// ---------------------------------------------------------------------------
// Helper: world position from grid for the strait map
// ---------------------------------------------------------------------------

/// Convert fractional world coords to screen position (isometric).
fn strait_screen_from_world(wx: f32, wy: f32) -> Vec3 {
    use cc_core::coords::{TILE_HALF_WIDTH, TILE_HALF_HEIGHT};
    let sx = (wx - wy) * TILE_HALF_WIDTH;
    let sy = (wx + wy) * TILE_HALF_HEIGHT;
    Vec3::new(sx, -sy, 5.0) // Bevy Y is up, isometric Y is down
}

// ===========================================================================
// PHASE 2: DEFCON VISUAL LAYER
// ===========================================================================

// ---------------------------------------------------------------------------
// Initialization system
// ---------------------------------------------------------------------------

fn strait_init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state: ResMut<StraitState>,
    _campaign: Res<CampaignState>,
    mut clear_color: ResMut<ClearColor>,
    // Hide all normal world-space rendering
    mut tile_sprites: Query<&mut Visibility, (With<crate::renderer::tilemap::TileSprite>, Without<crate::renderer::fog::FogOverlay>)>,
    mut fog_overlays: Query<&mut Visibility, (With<crate::renderer::fog::FogOverlay>, Without<crate::renderer::tilemap::TileSprite>)>,
) {
    if state.initialized {
        return;
    }
    state.initialized = true;

    // -- DEFCON aesthetic: pitch black background, hide all terrain --
    *clear_color = ClearColor(Color::srgb(0.01, 0.01, 0.02));

    // Hide all terrain tiles — DEFCON shows lines, not tiles
    for mut vis in tile_sprites.iter_mut() {
        *vis = Visibility::Hidden;
    }

    // Hide fog overlays — strait has its own vision system
    for mut vis in fog_overlays.iter_mut() {
        *vis = Visibility::Hidden;
    }

    // -- Spawn HUD --
    spawn_strait_hud(&mut commands);

    // -- Spawn patrol drones --
    let map_width = 300;
    let drone_count = INITIAL_PATROL_DRONES;
    let spacing = map_width as f32 / drone_count as f32;

    for i in 0..drone_count {
        let base_x = (i as f32 * spacing + spacing * 0.5) as i32;
        let patrol_y = 25; // patrol zone between hostile shallows and shipping lane
        let sector_w = (spacing * 0.33) as i32;
        let waypoints = vec![
            GridPos::new(base_x, patrol_y - 10),
            GridPos::new((base_x + sector_w).min(299), patrol_y),
            GridPos::new(base_x, patrol_y + 10),
            GridPos::new((base_x - sector_w).max(1), patrol_y),
        ];

        let pos = strait_screen_from_world(base_x as f32, patrol_y as f32);
        let drone_mesh = meshes.add(Circle::new(4.0));
        let drone_mat = materials.add(ColorMaterial::from_color(TERM_GREEN));

        let drone_id = state.next_drone_id;
        state.next_drone_id += 1;
        state.drones_alive += 1;

        commands.spawn((
            DreamEntity,
            StraitDrone {
                patrol_waypoints: waypoints,
                current_wp_index: 0,
                alive: true,
                drone_id,
            },
            Mesh2d(drone_mesh),
            MeshMaterial2d(drone_mat),
            Transform::from_translation(pos),
            StraitRadarSweep { angle: 0.0 },
        ));
    }

    // -- Spawn initial enemy launchers for wave 1 --
    spawn_enemy_wave(&mut commands, &mut meshes, &mut materials, &state.wave_config, map_width);
    state.enemy_launchers_spawned = true;

    // -- Build coastline contour cache --
    // Scan the map for Water↔Rock and Shallows↔Rock transitions to draw DEFCON-style vector coastlines.
    // We check 4 cardinal neighbors of each tile; if a neighbor is a different "zone" we draw an edge.
    let mut contour = StraitContourCache::default();
    let map_w = 300usize;
    let map_h = 60usize;
    if let Some(mission) = _campaign.current_mission.as_ref() {
        if let cc_core::mission::MissionMap::Inline { tiles, .. } = &mission.map {
            for y in 0..map_h {
                for x in 0..map_w {
                    let t = tiles[y * map_w + x];
                    let is_water = matches!(t, cc_core::terrain::TerrainType::Water | cc_core::terrain::TerrainType::Shallows);
                    let is_rock = matches!(t, cc_core::terrain::TerrainType::Rock);

                    // Check right neighbor
                    if x + 1 < map_w {
                        let r = tiles[y * map_w + x + 1];
                        let r_water = matches!(r, cc_core::terrain::TerrainType::Water | cc_core::terrain::TerrainType::Shallows);
                        let r_rock = matches!(r, cc_core::terrain::TerrainType::Rock);
                        if (is_water && r_rock) || (is_rock && r_water) {
                            let edge_x = x as f32 + 0.5;
                            let a = strait_screen_from_world(edge_x, y as f32).truncate();
                            let b = strait_screen_from_world(edge_x, y as f32 + 1.0).truncate();
                            contour.coastline_segments.push((a, b));
                        }
                    }
                    // Check bottom neighbor
                    if y + 1 < map_h {
                        let d = tiles[(y + 1) * map_w + x];
                        let d_water = matches!(d, cc_core::terrain::TerrainType::Water | cc_core::terrain::TerrainType::Shallows);
                        let d_rock = matches!(d, cc_core::terrain::TerrainType::Rock);
                        if (is_water && d_rock) || (is_rock && d_water) {
                            let edge_y = y as f32 + 0.5;
                            let a = strait_screen_from_world(x as f32, edge_y).truncate();
                            let b = strait_screen_from_world(x as f32 + 1.0, edge_y).truncate();
                            contour.coastline_segments.push((a, b));
                        }
                    }
                }
            }
        }
    }

    // Shipping lane (horizontal line across the full strait at y=30)
    let lane_start = strait_screen_from_world(0.0, 30.0).truncate();
    let lane_end = strait_screen_from_world(299.0, 30.0).truncate();
    contour.shipping_lane = Some((lane_start, lane_end));

    info!("DEFCON contour cache: {} coastline segments", contour.coastline_segments.len());
    commands.insert_resource(contour);

    info!("Strait dream sequence initialized: {} patrol drones, {} interceptors",
        drone_count, state.interceptor_count);
}

fn spawn_strait_hud(commands: &mut Commands) {
    // Main HUD container — top of screen
    commands
        .spawn((
            DreamEntity,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                right: Val::Px(8.0),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.02, 0.05, 0.85)),
            ZIndex(25),
        ))
        .with_children(|parent| {
            // Compute budget
            parent.spawn((
                StraitComputeHud,
                Text::new("COMPUTE: 100/100"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TERM_GREEN),
            ));

            // Interceptors
            parent.spawn((
                StraitInterceptorHud,
                Text::new("INTERCEPTORS: 15"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TERM_GREEN),
            ));

            // Tanker status
            parent.spawn((
                StraitTankerHud,
                Text::new("TANKERS: 0/12 SAFE | 0 LOST"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TANKER_BLUE),
            ));

            // Zero-day status
            parent.spawn((
                StraitZeroDayHud,
                Text::new("0-DAY: IDLE"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TERM_GREEN_DIM),
            ));
        });

    // Status line — bottom of screen
    commands.spawn((
        DreamEntity,
        StraitStatusHud,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        Text::new("WAVE 1 | STRAIT DEFENSE ACTIVE"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(TERM_GREEN_DIM),
        ZIndex(25),
    ));
}

fn spawn_enemy_wave(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    config: &EnemyWaveConfig,
    map_width: i32,
) {
    let launcher_mesh = meshes.add(RegularPolygon::new(6.0, 3));
    let launcher_mat = materials.add(ColorMaterial::from_color(HOSTILE_RED));
    let decoy_mat = materials.add(ColorMaterial::from_color(Color::srgba(0.9, 0.15, 0.1, 0.5)));

    let total = config.launcher_count + config.decoy_count;
    let spacing = map_width as f32 / (total + 1) as f32;

    for i in 0..total {
        let is_decoy = i >= config.launcher_count;
        let x = ((i + 1) as f32 * spacing) as i32;
        let hidden_y = 4; // deep in hostile shore
        let firing_y = 9; // at the shallows edge

        let pos = strait_screen_from_world(x as f32, hidden_y as f32);

        commands.spawn((
            DreamEntity,
            StraitLauncher {
                phase: LauncherPhase::Hidden,
                phase_timer: 30.0 + (i as f32 * 15.0), // stagger
                hidden_pos: GridPos::new(x, hidden_y),
                firing_pos: GridPos::new(x, firing_y),
                is_decoy,
                salvo_count: 0,
                has_fired_this_phase: false,
            },
            Mesh2d(launcher_mesh.clone()),
            MeshMaterial2d(if is_decoy { decoy_mat.clone() } else { launcher_mat.clone() }),
            Transform::from_translation(pos),
            Visibility::Hidden, // hidden until in vision
        ));
    }

    // AA drones
    let aa_mesh = meshes.add(Circle::new(3.5));
    let aa_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.8, 0.2, 0.2)));

    for i in 0..config.aa_drone_count {
        let x = (map_width as f32 * (i as f32 + 1.0) / (config.aa_drone_count as f32 + 1.0)) as f32;
        let y = 14.0; // patrol in the upper strait
        let pos = strait_screen_from_world(x, y);

        commands.spawn((
            DreamEntity,
            StraitAaDrone {
                target_drone: None,
                world_x: x,
                world_y: y,
                alive: true,
            },
            Mesh2d(aa_mesh.clone()),
            MeshMaterial2d(aa_mat.clone()),
            Transform::from_translation(pos),
        ));
    }
}

// ===========================================================================
// PHASE 3: TANKER CONVOY + CORE SIMULATION
// ===========================================================================

fn strait_tick_system(mut state: ResMut<StraitState>) {
    if state.mission_complete {
        return;
    }
    state.mission_tick += 1;
}

fn strait_compute_regen(mut state: ResMut<StraitState>) {
    if state.mission_complete {
        return;
    }

    // Regen
    state.compute = (state.compute + COMPUTE_REGEN_PER_TICK).min(state.max_compute);

    // Drain from drone vision (proportional to alive drones)
    let drone_cost = state.drones_alive as f32 * 0.1 * state.allocation.drone_vision;
    state.compute = (state.compute - drone_cost).max(0.0);

    // Progress zero-day build if allocated
    let zd_rate = state.allocation.zero_day * 0.3;
    if let ZeroDayState::Building { exploit_type, progress, required } = &mut state.zero_day_slot {
        *progress += zd_rate;
        if *progress >= *required {
            let zt = *exploit_type;
            state.zero_day_slot = ZeroDayState::Ready(zt);
        }
    }
}

fn strait_interceptor_regen(mut state: ResMut<StraitState>) {
    if state.mission_complete {
        return;
    }
    state.interceptor_regen_timer += 1;
    if state.interceptor_regen_timer >= INTERCEPTOR_REGEN_TICKS {
        state.interceptor_regen_timer = 0;
        if state.interceptor_count < MAX_INTERCEPTORS {
            state.interceptor_count += 1;
        }
    }
}

fn strait_spawn_tankers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete || state.tankers_spawned >= TOTAL_TANKERS {
        return;
    }

    // Wait for initial delay
    if state.mission_tick < 200 {
        return;
    }

    state.tanker_spawn_timer += 1;
    if state.tanker_spawn_timer < TANKER_SPAWN_INTERVAL {
        return;
    }
    state.tanker_spawn_timer = 0;

    let tanker_index = state.tankers_spawned;
    state.tankers_spawned += 1;

    // Tankers enter at the west (x=0) and travel east along the shipping lane (y=10)
    let lane_y = 29 + (tanker_index % 3) as i32; // slight lane variation around shipping lane center
    let pos = strait_screen_from_world(0.0, lane_y as f32);
    let tanker_mesh = meshes.add(Rectangle::new(10.0, 5.0));
    let tanker_mat = materials.add(ColorMaterial::from_color(TANKER_BLUE));

    commands.spawn((
        DreamEntity,
        StraitTanker {
            hp: TANKER_HP,
            lane_y,
            world_x: 0.0,
            target_x: 299.0,
            arrived: false,
            destroyed: false,
            tanker_index,
        },
        Mesh2d(tanker_mesh),
        MeshMaterial2d(tanker_mat),
        Transform::from_translation(pos),
    ));
}

fn strait_move_tankers(
    mut state: ResMut<StraitState>,
    mut tankers: Query<(&mut StraitTanker, &mut Transform)>,
) {
    if state.mission_complete {
        return;
    }

    for (mut tanker, mut xform) in tankers.iter_mut() {
        if tanker.arrived || tanker.destroyed {
            continue;
        }

        tanker.world_x += TANKER_SPEED;

        if tanker.world_x >= tanker.target_x {
            tanker.arrived = true;
            state.tankers_arrived += 1;
        }

        let new_pos = strait_screen_from_world(tanker.world_x, tanker.lane_y as f32);
        xform.translation = new_pos;
    }
}

// ===========================================================================
// PHASE 4: DRONE PATROL + VISION
// ===========================================================================

fn strait_move_drones(
    mut drones: Query<(&mut StraitDrone, &mut Transform)>,
    state: Res<StraitState>,
) {
    if state.mission_complete {
        return;
    }

    for (mut drone, mut xform) in drones.iter_mut() {
        if !drone.alive || drone.patrol_waypoints.is_empty() {
            continue;
        }

        let target = drone.patrol_waypoints[drone.current_wp_index];
        let target_pos = strait_screen_from_world(target.x as f32, target.y as f32);

        let dir = (target_pos - xform.translation).truncate();
        let dist = dir.length();

        if dist < 2.0 {
            // Reached waypoint, advance to next
            drone.current_wp_index = (drone.current_wp_index + 1) % drone.patrol_waypoints.len();
        } else {
            let move_speed = 1.5; // pixels per frame
            let step = dir.normalize() * move_speed;
            xform.translation.x += step.x;
            xform.translation.y += step.y;
        }
    }
}

fn strait_update_vision(
    mut vision: ResMut<StraitVision>,
    drones: Query<(&StraitDrone, &Transform)>,
    satellites: Query<&StraitSatelliteScan>,
) {
    vision.clear();

    // Reveal around alive drones using actual screen position → approximate grid
    for (drone, xform) in drones.iter() {
        if !drone.alive {
            continue;
        }
        // Reverse the isometric projection to get approximate grid coords
        use cc_core::coords::{TILE_HALF_WIDTH, TILE_HALF_HEIGHT};
        let sx = xform.translation.x;
        let sy = -xform.translation.y; // un-flip Bevy Y
        let wx = (sx / TILE_HALF_WIDTH + sy / TILE_HALF_HEIGHT) / 2.0;
        let wy = (sy / TILE_HALF_HEIGHT - sx / TILE_HALF_WIDTH) / 2.0;
        vision.reveal(wx as i32, wy as i32, DRONE_VISION_RADIUS);
    }

    // Reveal around satellite scans
    for sat in satellites.iter() {
        vision.reveal(sat.center.x, sat.center.y, SATELLITE_VISION_RADIUS);
    }
}

// ===========================================================================
// PHASE 5: ENEMY AI
// ===========================================================================

fn strait_enemy_director(
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete {
        return;
    }

    // Escalate based on tankers spawned
    let new_wave = match state.tankers_spawned {
        0..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 4,
    };

    if new_wave != state.current_wave {
        state.current_wave = new_wave;
        state.wave_config = match new_wave {
            1 => EnemyWaveConfig::wave_1(),
            2 => EnemyWaveConfig::wave_2(),
            3 => EnemyWaveConfig::wave_3(),
            _ => EnemyWaveConfig::wave_4(),
        };
        state.enemy_launchers_spawned = false;
    }
}

fn strait_spawn_wave_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete || state.enemy_launchers_spawned {
        return;
    }
    state.enemy_launchers_spawned = true;

    let map_width = 60;
    spawn_enemy_wave(&mut commands, &mut meshes, &mut materials, &state.wave_config, map_width);
    info!("Spawned enemy wave {} entities", state.current_wave);
}

fn strait_launcher_fsm(
    mut launchers: Query<(&mut StraitLauncher, &mut Transform, &mut Visibility)>,
    time: Res<Time>,
    vision: Res<StraitVision>,
) {
    let dt = time.delta_secs();

    for (mut launcher, mut xform, mut vis) in launchers.iter_mut() {
        launcher.phase_timer -= dt;

        // Update visibility based on player vision
        let pos = match launcher.phase {
            LauncherPhase::Hidden => launcher.hidden_pos,
            LauncherPhase::Setting | LauncherPhase::Firing => launcher.firing_pos,
            LauncherPhase::Retreating => launcher.hidden_pos,
        };
        *vis = if vision.is_visible(pos.x, pos.y) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if launcher.phase_timer > 0.0 {
            continue;
        }

        // Phase transitions
        match launcher.phase {
            LauncherPhase::Hidden => {
                launcher.phase = LauncherPhase::Setting;
                launcher.phase_timer = 3.0;
                launcher.has_fired_this_phase = false;
                let new_pos = strait_screen_from_world(launcher.firing_pos.x as f32, launcher.firing_pos.y as f32);
                xform.translation = new_pos;
            }
            LauncherPhase::Setting => {
                launcher.phase = LauncherPhase::Firing;
                launcher.phase_timer = 2.0; // fires for 2 seconds
            }
            LauncherPhase::Firing => {
                launcher.phase = LauncherPhase::Retreating;
                launcher.phase_timer = 2.0;
                let new_pos = strait_screen_from_world(launcher.hidden_pos.x as f32, launcher.hidden_pos.y as f32);
                xform.translation = new_pos;
            }
            LauncherPhase::Retreating => {
                launcher.phase = LauncherPhase::Hidden;
                // Relocate hidden position slightly
                let jitter = (launcher.salvo_count as i32 * 3) % 7;
                launcher.hidden_pos.x = (launcher.hidden_pos.x + jitter).clamp(1, 298);
                launcher.phase_timer = 8.0 + launcher.salvo_count as f32 * 2.0; // longer gaps as game progresses
                launcher.salvo_count += 1;
                let new_pos = strait_screen_from_world(launcher.hidden_pos.x as f32, launcher.hidden_pos.y as f32);
                xform.translation = new_pos;
            }
        }
    }
}

fn strait_spawn_missiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut launchers: Query<&mut StraitLauncher>,
    tankers: Query<(Entity, &StraitTanker)>,
) {
    let missile_mesh = meshes.add(Circle::new(2.5));
    let missile_mat = materials.add(ColorMaterial::from_color(WARNING_AMBER));

    for mut launcher in launchers.iter_mut() {
        if launcher.phase != LauncherPhase::Firing || launcher.is_decoy {
            continue;
        }

        // Fire exactly once per firing phase
        if launcher.has_fired_this_phase {
            continue;
        }
        launcher.has_fired_this_phase = true;

        // Find nearest active tanker
        let fx = launcher.firing_pos.x as f32;
        let fy = launcher.firing_pos.y as f32;

        let target = tankers.iter()
            .filter(|(_, t)| !t.arrived && !t.destroyed)
            .min_by(|(_, a), (_, b)| {
                let da = (a.world_x - fx).powi(2) + (a.lane_y as f32 - fy).powi(2);
                let db = (b.world_x - fx).powi(2) + (b.lane_y as f32 - fy).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((target_entity, target_tanker)) = target {
            let pos = strait_screen_from_world(fx, fy);

            commands.spawn((
                DreamEntity,
                StraitMissile {
                    state: MissileState::InFlight {
                        origin_x: fx,
                        origin_y: fy,
                        target_x: target_tanker.world_x,
                        target_y: target_tanker.lane_y as f32,
                        progress: 0.0,
                    },
                    target_tanker: Some(target_entity),
                    age_ticks: 0,
                },
                Mesh2d(missile_mesh.clone()),
                MeshMaterial2d(missile_mat.clone()),
                Transform::from_translation(pos),
            ));
        }
    }
}

fn strait_missile_flight(
    mut missiles: Query<(&mut StraitMissile, &mut Transform)>,
    state: Res<StraitState>,
) {
    if state.mission_complete {
        return;
    }

    let progress_per_tick = 1.0 / MISSILE_FLIGHT_TICKS as f32;

    for (mut missile, mut xform) in missiles.iter_mut() {
        if let MissileState::InFlight { origin_x, origin_y, target_x, target_y, progress } = &mut missile.state {
            *progress += progress_per_tick;

            // Lerp position
            let cur_x = *origin_x + (*target_x - *origin_x) * *progress;
            let cur_y = *origin_y + (*target_y - *origin_y) * *progress;
            xform.translation = strait_screen_from_world(cur_x, cur_y);
        }
        missile.age_ticks += 1;
    }
}

fn strait_missile_interception(
    mut commands: Commands,
    mut missiles: Query<(Entity, &mut StraitMissile, &Transform)>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete {
        return;
    }

    for (entity, mut missile, _xform) in missiles.iter_mut() {
        if !matches!(missile.state, MissileState::InFlight { .. }) {
            continue;
        }

        if let MissileState::InFlight { origin_y, target_y, progress, .. } = missile.state {
            // Check if in interceptor range (missiles in the middle third of the strait)
            let cur_y = origin_y + (target_y - origin_y) * progress;

            // Interceptors cover the shipping lane area (y 7-13)
            if cur_y >= 20.0 && cur_y <= 40.0 && state.interceptor_count > 0 {
                // Higher chance to intercept when missile is further along
                if progress > 0.3 {
                    state.interceptor_count -= 1;
                    missile.state = MissileState::Intercepted;
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn strait_missile_impact(
    mut commands: Commands,
    mut missiles: Query<(Entity, &mut StraitMissile)>,
    mut tankers: Query<(Entity, &mut StraitTanker)>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete {
        return;
    }

    for (missile_entity, mut missile) in missiles.iter_mut() {
        // Skip already-resolved missiles
        if matches!(missile.state, MissileState::Intercepted | MissileState::Impact) {
            continue;
        }

        if let MissileState::InFlight { progress, .. } = missile.state {
            if progress < 1.0 {
                continue;
            }

            // Missile reached target — transition to Impact and deal damage once
            missile.state = MissileState::Impact;

            if let Some(target_entity) = missile.target_tanker {
                if let Ok((_, mut tanker)) = tankers.get_mut(target_entity) {
                    if !tanker.destroyed && !tanker.arrived {
                        tanker.hp = tanker.hp.saturating_sub(1);
                        if tanker.hp == 0 {
                            tanker.destroyed = true;
                            state.tankers_destroyed += 1;
                            commands.entity(target_entity).insert(Visibility::Hidden);
                        }
                    }
                }
            }

            // Despawn resolved missile entity
            commands.entity(missile_entity).despawn();
        }
    }
}

fn strait_enemy_aa(
    mut aa_drones: Query<(&mut StraitAaDrone, &mut Transform), Without<StraitDrone>>,
    mut patrol_drones: Query<(Entity, &mut StraitDrone, &Transform), Without<StraitAaDrone>>,
    mut state: ResMut<StraitState>,
    time: Res<Time>,
) {
    if state.mission_complete {
        return;
    }

    let dt = time.delta_secs();

    for (mut aa, mut aa_xform) in aa_drones.iter_mut() {
        if !aa.alive {
            continue;
        }

        // Find nearest alive player drone
        let mut nearest: Option<(Entity, f32)> = None;
        for (entity, drone, drone_xform) in patrol_drones.iter() {
            if !drone.alive {
                continue;
            }
            let dx = aa_xform.translation.x - drone_xform.translation.x;
            let dy = aa_xform.translation.y - drone_xform.translation.y;
            let dist_sq = dx * dx + dy * dy;
            if nearest.is_none() || dist_sq < nearest.unwrap().1 {
                nearest = Some((entity, dist_sq));
            }
        }

        if let Some((target_entity, dist_sq)) = nearest {
            aa.target_drone = Some(target_entity);

            // Move toward target
            if let Ok((_, _, drone_xform)) = patrol_drones.get(target_entity) {
                let dir = (drone_xform.translation - aa_xform.translation).truncate();
                let dist = dir.length();
                if dist > 0.0 {
                    let speed = 1.2 * dt * 60.0; // slightly slower than patrol drones
                    let step = dir.normalize() * speed;
                    aa_xform.translation.x += step.x;
                    aa_xform.translation.y += step.y;
                }

                // Kill drone if close enough
                if dist_sq < 400.0 { // ~20 pixel kill range
                    if let Ok((_, mut drone, _)) = patrol_drones.get_mut(target_entity) {
                        drone.alive = false;
                        state.drones_alive = state.drones_alive.saturating_sub(1);
                    }
                }
            }
        }
    }
}

// ===========================================================================
// VISUALS: RADAR SWEEP + HUD
// ===========================================================================

fn strait_radar_sweep(
    mut sweeps: Query<&mut StraitRadarSweep>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for mut sweep in sweeps.iter_mut() {
        sweep.angle += RADAR_SWEEP_SPEED * dt;
        if sweep.angle > std::f32::consts::TAU {
            sweep.angle -= std::f32::consts::TAU;
        }
    }
}

/// DEFCON-style rendering using Gizmos: coastline contours, shipping lane,
/// radar sweeps, and missile trails. Runs every frame.
fn strait_render_defcon(
    mut gizmos: Gizmos,
    contour: Option<Res<StraitContourCache>>,
    drones: Query<(&StraitDrone, &Transform, &StraitRadarSweep)>,
    missiles: Query<(&StraitMissile, &Transform)>,
    tankers: Query<(&StraitTanker, &Transform)>,
    launchers: Query<(&StraitLauncher, &Transform, &Visibility)>,
    aa_drones: Query<(&StraitAaDrone, &Transform)>,
    input_state: Res<StraitInputState>,
    state: Res<StraitState>,
    vision: Res<StraitVision>,
) {
    let Some(contour) = contour else { return; };

    // -- Coastline contours (glowing green lines) --
    let coast_color = Color::srgba(0.1, 0.8, 0.3, 0.7);
    for (a, b) in &contour.coastline_segments {
        gizmos.line_2d(*a, *b, coast_color);
    }

    // -- Shipping lane (dim blue dashed) --
    if let Some((start, end)) = contour.shipping_lane {
        let lane_color = Color::srgba(0.2, 0.3, 0.7, 0.25);
        gizmos.line_2d(start, end, lane_color);
    }

    // -- Radar sweep circles around alive drones --
    let sweep_color = Color::srgba(0.15, 0.7, 0.25, 0.3);
    let drone_vision_px = DRONE_VISION_RADIUS as f32 * 28.0; // approximate tile size in pixels
    for (drone, xform, sweep) in drones.iter() {
        if !drone.alive {
            continue;
        }
        let center = xform.translation.truncate();

        // Vision circle
        gizmos.circle_2d(center, drone_vision_px, sweep_color);

        // Radar sweep line (rotating)
        let sweep_end = center + Vec2::new(sweep.angle.cos(), sweep.angle.sin()) * drone_vision_px;
        gizmos.line_2d(center, sweep_end, Color::srgba(0.2, 0.9, 0.3, 0.5));
    }

    // -- Selection rings on selected drones --
    let select_color = Color::srgba(0.3, 1.0, 0.4, 0.9);
    for &entity in &input_state.selected_drones {
        if let Ok((_, xform, _)) = drones.get(entity) {
            let center = xform.translation.truncate();
            gizmos.circle_2d(center, 12.0, select_color);
        }
    }

    // -- Missile trails (amber arcs) --
    let missile_color = Color::srgba(0.95, 0.6, 0.1, 0.8);
    for (missile, xform) in missiles.iter() {
        if matches!(missile.state, MissileState::InFlight { .. }) {
            let pos = xform.translation.truncate();
            // Draw a small bright dot at missile head
            gizmos.circle_2d(pos, 3.0, missile_color);
            // Trail line back toward origin
            if let MissileState::InFlight { origin_x, origin_y, progress, .. } = missile.state {
                if progress > 0.1 {
                    let trail_start = strait_screen_from_world(origin_x, origin_y).truncate();
                    gizmos.line_2d(trail_start, pos, Color::srgba(0.95, 0.5, 0.1, 0.3));
                }
            }
        }
    }

    // -- Tanker indicators (blue dots with direction) --
    let tanker_color = Color::srgba(0.3, 0.5, 0.95, 0.8);
    for (tanker, xform) in tankers.iter() {
        if tanker.arrived || tanker.destroyed {
            continue;
        }
        let pos = xform.translation.truncate();
        gizmos.circle_2d(pos, 5.0, tanker_color);
        // Direction indicator (small line pointing east)
        gizmos.line_2d(pos, pos + Vec2::new(8.0, 0.0), tanker_color);
    }

    // -- Mode indicator text would go here in a real implementation --
    // (Gizmos can't render text, so mode is shown in HUD)

    // -- Enemy launchers (red triangles when visible) --
    let launcher_color = Color::srgba(0.9, 0.15, 0.1, 0.9);
    let launcher_setup_color = Color::srgba(0.9, 0.5, 0.1, 0.7);
    for (launcher, xform, vis) in launchers.iter() {
        // Only draw if in player vision
        let (gx, gy) = screen_to_grid(xform.translation.x, xform.translation.y);
        if !vision.is_visible(gx, gy) && *vis == Visibility::Hidden {
            continue;
        }
        let pos = xform.translation.truncate();
        let color = if launcher.phase == LauncherPhase::Firing {
            launcher_color
        } else {
            launcher_setup_color
        };
        let label = if launcher.is_decoy { "?" } else { "!" };
        // Draw triangle pointing up
        let size = 8.0;
        let top = pos + Vec2::new(0.0, size);
        let bl = pos + Vec2::new(-size * 0.7, -size * 0.5);
        let br = pos + Vec2::new(size * 0.7, -size * 0.5);
        gizmos.line_2d(top, bl, color);
        gizmos.line_2d(bl, br, color);
        gizmos.line_2d(br, top, color);
        // Firing indicator: pulsing circle
        if launcher.phase == LauncherPhase::Firing {
            gizmos.circle_2d(pos, 12.0, launcher_color);
        }
    }

    // -- Enemy AA drones (small red dots) --
    let aa_color = Color::srgba(0.8, 0.2, 0.2, 0.8);
    for (aa, xform) in aa_drones.iter() {
        if !aa.alive {
            continue;
        }
        let pos = xform.translation.truncate();
        gizmos.circle_2d(pos, 3.0, aa_color);
        // Movement direction indicator
        if let Some(target) = aa.target_drone {
            if let Ok((_, target_xform, _)) = drones.get(target) {
                let dir = (target_xform.translation.truncate() - pos).normalize_or_zero() * 10.0;
                gizmos.line_2d(pos, pos + dir, Color::srgba(0.8, 0.2, 0.2, 0.4));
            }
        }
    }

    // -- Sector grid (very faint vertical lines) --
    if state.drones_alive > 0 {
        let sector_width = 300.0 / state.drones_alive as f32;
        let grid_color = Color::srgba(0.1, 0.3, 0.15, 0.1);
        for i in 1..state.drones_alive {
            let x = i as f32 * sector_width;
            let top = strait_screen_from_world(x, 0.0).truncate();
            let bottom = strait_screen_from_world(x, 59.0).truncate();
            gizmos.line_2d(top, bottom, grid_color);
        }
    }
}

fn strait_update_hud(
    state: Res<StraitState>,
    mut compute_hud: Query<&mut Text, (With<StraitComputeHud>, Without<StraitInterceptorHud>, Without<StraitTankerHud>, Without<StraitZeroDayHud>, Without<StraitStatusHud>)>,
    mut interceptor_hud: Query<&mut Text, (With<StraitInterceptorHud>, Without<StraitComputeHud>, Without<StraitTankerHud>, Without<StraitZeroDayHud>, Without<StraitStatusHud>)>,
    mut tanker_hud: Query<&mut Text, (With<StraitTankerHud>, Without<StraitComputeHud>, Without<StraitInterceptorHud>, Without<StraitZeroDayHud>, Without<StraitStatusHud>)>,
    mut zd_hud: Query<&mut Text, (With<StraitZeroDayHud>, Without<StraitComputeHud>, Without<StraitInterceptorHud>, Without<StraitTankerHud>, Without<StraitStatusHud>)>,
    mut status_hud: Query<&mut Text, (With<StraitStatusHud>, Without<StraitComputeHud>, Without<StraitInterceptorHud>, Without<StraitTankerHud>, Without<StraitZeroDayHud>)>,
) {
    if let Ok(mut text) = compute_hud.single_mut() {
        **text = format!("COMPUTE: {:.0}/{:.0}", state.compute, state.max_compute);
    }

    if let Ok(mut text) = interceptor_hud.single_mut() {
        **text = format!("INTERCEPTORS: {}", state.interceptor_count);
    }

    if let Ok(mut text) = tanker_hud.single_mut() {
        **text = format!(
            "TANKERS: {}/{} SAFE | {} LOST",
            state.tankers_arrived, TOTAL_TANKERS, state.tankers_destroyed
        );
    }

    if let Ok(mut text) = zd_hud.single_mut() {
        **text = match &state.zero_day_slot {
            ZeroDayState::Idle => "0-DAY: IDLE".to_string(),
            ZeroDayState::Building { exploit_type, progress, required } => {
                let pct = (progress / required * 100.0).min(100.0);
                format!("0-DAY: {:?} {:.0}%", exploit_type, pct)
            }
            ZeroDayState::Ready(zt) => format!("0-DAY: {:?} READY", zt),
        };
    }

    if let Ok(mut text) = status_hud.single_mut() {
        if state.mission_complete {
            let outcome = if state.tankers_arrived >= MIN_TANKERS_WIN {
                "MISSION COMPLETE"
            } else {
                "MISSION FAILED"
            };
            **text = format!("WAVE {} | {}", state.current_wave, outcome);
        } else {
            **text = format!(
                "WAVE {} | DRONES: {} | TICK {}",
                state.current_wave, state.drones_alive, state.mission_tick
            );
        }
    }
}

// ===========================================================================
// WIN/LOSE
// ===========================================================================

fn strait_check_win_lose(
    mut state: ResMut<StraitState>,
    mut campaign: ResMut<CampaignState>,
) {
    if state.mission_complete {
        return;
    }

    let all_resolved = state.tankers_arrived + state.tankers_destroyed >= state.tankers_spawned
        && state.tankers_spawned >= TOTAL_TANKERS;

    if !all_resolved {
        // Early fail check: if too many already lost
        if state.tankers_destroyed >= MAX_TANKERS_LOST {
            state.mission_complete = true;
            // Mission failure handled by campaign system
            info!("Strait mission FAILED: {} tankers destroyed", state.tankers_destroyed);
        }
        return;
    }

    state.mission_complete = true;

    if state.tankers_arrived >= MIN_TANKERS_WIN {
        info!("Strait mission WON: {}/{} tankers arrived safely", state.tankers_arrived, TOTAL_TANKERS);
        campaign.complete_objective("protect_convoy");
    } else {
        info!("Strait mission FAILED: only {}/{} tankers arrived", state.tankers_arrived, MIN_TANKERS_WIN);
    }
}

// ===========================================================================
// STRAIT SCRIPT RUNNER (Phase E)
// ===========================================================================

/// Build a StraitSnapshot from current ECS state for Lua consumption.
fn build_strait_snapshot(
    state: &StraitState,
    drones: &Query<(Entity, &mut StraitDrone, &Transform), Without<StraitAaDrone>>,
    tankers: &Query<&StraitTanker>,
    launchers: &Query<(&StraitLauncher, &Transform, &Visibility)>,
    _vision: &StraitVision,
) -> cc_agent::strait_bindings::StraitSnapshot {
    use cc_agent::strait_bindings::*;

    let drone_positions: Vec<DroneInfo> = drones
        .iter()
        .map(|(_, drone, xform)| {
            let (gx, gy) = screen_to_grid(xform.translation.x, xform.translation.y);
            DroneInfo {
                id: drone.drone_id,
                x: gx as f32,
                y: gy as f32,
                alive: drone.alive,
            }
        })
        .collect();

    let tanker_positions: Vec<TankerInfo> = tankers
        .iter()
        .map(|t| TankerInfo {
            index: t.tanker_index,
            x: t.world_x,
            y: t.lane_y,
            hp: t.hp,
            arrived: t.arrived,
            destroyed: t.destroyed,
        })
        .collect();

    let visible_enemies: Vec<EnemyInfo> = launchers
        .iter()
        .filter(|(launcher, _, vis)| {
            !launcher.is_decoy
                && *vis != &Visibility::Hidden
                && launcher.phase != LauncherPhase::Hidden
        })
        .map(|(_launcher, xform, _)| {
            let (gx, gy) = screen_to_grid(xform.translation.x, xform.translation.y);
            EnemyInfo {
                kind: "launcher".to_string(),
                x: gx as f32,
                y: gy as f32,
            }
        })
        .collect();

    StraitSnapshot {
        compute: state.compute,
        max_compute: state.max_compute,
        allocation: state.allocation,
        interceptor_count: state.interceptor_count,
        tankers_arrived: state.tankers_arrived,
        tankers_destroyed: state.tankers_destroyed,
        tankers_spawned: state.tankers_spawned,
        drones_alive: state.drones_alive,
        zero_day_slot: state.zero_day_slot,
        mission_tick: state.mission_tick,
        drone_positions,
        tanker_positions,
        visible_enemies,
    }
}

/// Execute registered Lua scripts and apply their strait commands.
#[cfg(not(target_arch = "wasm32"))]
fn strait_script_runner(
    mut state: ResMut<StraitState>,
    map_res: Res<cc_sim::resources::MapResource>,
    tankers_q: Query<&StraitTanker>,
    launchers_q: Query<(&StraitLauncher, &Transform, &Visibility)>,
    vision: Res<StraitVision>,
    mut drone_mut: Query<(Entity, &mut StraitDrone, &Transform), Without<StraitAaDrone>>,
    mut commands: Commands,
) {
    if state.mission_complete || state.active_scripts.is_empty() {
        return;
    }

    let snapshot = build_strait_snapshot(&state, &drone_mut, &tankers_q, &launchers_q, &vision);

    // Build a minimal GameStateSnapshot (empty — strait doesn't use standard units)
    let empty_snapshot = cc_agent::snapshot::GameStateSnapshot {
        tick: state.mission_tick,
        map_width: 300,
        map_height: 60,
        player_id: 0,
        my_units: Vec::new(),
        enemy_units: Vec::new(),
        my_buildings: Vec::new(),
        enemy_buildings: Vec::new(),
        resource_deposits: Vec::new(),
        my_resources: cc_sim::resources::PlayerResourceState::default(),
    };

    // Collect scripts to run this tick
    let mut scripts_to_run: Vec<(usize, String)> = Vec::new();
    for (i, script) in state.active_scripts.iter_mut().enumerate() {
        script.ticks_since_run += 1;
        if script.ticks_since_run >= script.interval {
            script.ticks_since_run = 0;
            scripts_to_run.push((i, script.source.clone()));
        }
    }

    for (_idx, source) in &scripts_to_run {
        let mut ctx = cc_agent::script_context::ScriptContext::new(
            &empty_snapshot,
            &map_res.map,
            0,
            cc_core::terrain::FactionId::CatGPT, // placeholder faction
        )
        .with_strait_snapshot(snapshot.clone());

        match cc_agent::lua_runtime::execute_script_with_context_tiered(
            source,
            &mut ctx,
            cc_agent::tool_tier::ToolTier::Advanced,
        ) {
            Ok(_game_commands) => {
                // Apply strait-specific commands
                for cmd in std::mem::take(&mut ctx.strait_commands) {
                    apply_strait_command(cmd, &mut state, &mut drone_mut, &mut commands);
                }
            }
            Err(e) => {
                warn!("Strait script error: {}", e);
            }
        }
    }
}

/// Apply a single StraitCommand to the ECS.
fn apply_strait_command(
    cmd: cc_agent::strait_bindings::StraitCommand,
    state: &mut StraitState,
    drone_mut: &mut Query<(Entity, &mut StraitDrone, &Transform), Without<StraitAaDrone>>,
    commands: &mut Commands,
) {
    use cc_agent::strait_bindings::StraitCommand;

    match cmd {
        StraitCommand::SetPatrol { drone_id, waypoints } => {
            for (_, mut drone, _) in drone_mut.iter_mut() {
                if drone.drone_id == drone_id && drone.alive {
                    drone.patrol_waypoints = waypoints
                        .iter()
                        .map(|&(x, y)| GridPos::new(x, y))
                        .collect();
                    drone.current_wp_index = 0;
                    break;
                }
            }
        }
        StraitCommand::SatelliteScan { x, y } => {
            let scan_cost = 15.0;
            if state.compute >= scan_cost {
                state.compute -= scan_cost;
                commands.spawn((
                    DreamEntity,
                    StraitSatelliteScan {
                        center: GridPos::new(x, y),
                        remaining_ticks: SATELLITE_SCAN_DURATION,
                    },
                ));
            }
        }
        StraitCommand::AllocateCompute(alloc) => {
            state.allocation = alloc;
        }
        StraitCommand::BuildZeroDay(zd_type) => {
            if matches!(state.zero_day_slot, ZeroDayState::Idle) {
                state.zero_day_slot = ZeroDayState::Building {
                    exploit_type: zd_type,
                    progress: 0.0,
                    required: zero_day_build_cost(zd_type),
                };
            }
        }
        StraitCommand::DeployZeroDay { exploit_type, target_x: _, target_y: _ } => {
            if matches!(state.zero_day_slot, ZeroDayState::Ready(_)) {
                state.zero_day_slot = ZeroDayState::Idle;
                let idx = match exploit_type {
                    ZeroDayType::Spoof => 0,
                    ZeroDayType::Blind => 1,
                    ZeroDayType::Hijack => 2,
                    ZeroDayType::Brick => 3,
                };
                state.zero_days_deployed[idx] = true;
                info!("Script deployed zero-day {:?}", exploit_type);
            }
        }
    }
}

// ===========================================================================
// PLAYER INPUT (Phase B + C)
// ===========================================================================

/// Reset the consumed flag each frame.
fn strait_reset_consumed(mut consumed: ResMut<StraitMouseConsumed>) {
    consumed.0 = false;
}

/// Reverse isometric projection: screen pixel → approximate grid coordinates.
fn screen_to_grid(screen_x: f32, screen_y: f32) -> (i32, i32) {
    use cc_core::coords::{TILE_HALF_WIDTH, TILE_HALF_HEIGHT};
    let sx = screen_x;
    let sy = -screen_y; // un-flip Bevy Y
    let wx = (sx / TILE_HALF_WIDTH + sy / TILE_HALF_HEIGHT) / 2.0;
    let wy = (sy / TILE_HALF_HEIGHT - sx / TILE_HALF_WIDTH) / 2.0;
    (wx as i32, wy as i32)
}

/// Main input system for the strait dream. Handles drone selection, movement,
/// satellite scans, zero-day deployment, and compute allocation.
fn strait_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Single<&Window>,
    camera_q: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut input_state: ResMut<StraitInputState>,
    mut state: ResMut<StraitState>,
    mut consumed: ResMut<StraitMouseConsumed>,
    mut drones: Query<(Entity, &mut StraitDrone, &Transform)>,
    mut commands: Commands,
) {
    if state.mission_complete {
        return;
    }

    // --- Keyboard shortcuts ---

    // Tab: cycle compute allocation preset
    if keyboard.just_pressed(KeyCode::Tab) {
        let alloc = &mut state.allocation;
        if alloc.drone_vision > 0.5 {
            // 60/20/20 → 20/60/20 (satellite focus)
            *alloc = ComputeAllocation { drone_vision: 0.2, satellite: 0.6, zero_day: 0.2 };
        } else if alloc.satellite > 0.5 {
            // 20/60/20 → 20/20/60 (zero-day focus)
            *alloc = ComputeAllocation { drone_vision: 0.2, satellite: 0.2, zero_day: 0.6 };
        } else {
            // 20/20/60 → 60/20/20 (drone focus)
            *alloc = ComputeAllocation { drone_vision: 0.6, satellite: 0.2, zero_day: 0.2 };
        }
    }

    // V: toggle satellite scan mode
    if keyboard.just_pressed(KeyCode::KeyV) {
        input_state.mode = if input_state.mode == StraitInputMode::SatelliteScan {
            StraitInputMode::Normal
        } else {
            StraitInputMode::SatelliteScan
        };
    }

    // 1-4: zero-day build or deploy mode
    for (key, zd_type) in [
        (KeyCode::Digit1, ZeroDayType::Spoof),
        (KeyCode::Digit2, ZeroDayType::Blind),
        (KeyCode::Digit3, ZeroDayType::Hijack),
        (KeyCode::Digit4, ZeroDayType::Brick),
    ] {
        if keyboard.just_pressed(key) {
            match &state.zero_day_slot {
                ZeroDayState::Idle => {
                    state.zero_day_slot = ZeroDayState::Building {
                        exploit_type: zd_type,
                        progress: 0.0,
                        required: zero_day_build_cost(zd_type),
                    };
                }
                ZeroDayState::Ready(ready_type) if *ready_type == zd_type => {
                    input_state.mode = StraitInputMode::ZeroDayDeploy(zd_type);
                }
                _ => {}
            }
        }
    }

    // Ctrl+A: select all alive drones
    if keyboard.just_pressed(KeyCode::KeyA)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
    {
        input_state.selected_drones.clear();
        for (entity, drone, _) in drones.iter() {
            if drone.alive {
                input_state.selected_drones.push(entity);
            }
        }
    }

    // Escape: clear selection / revert to normal mode
    if keyboard.just_pressed(KeyCode::Escape) {
        input_state.selected_drones.clear();
        input_state.mode = StraitInputMode::Normal;
    }

    // --- Mouse input ---

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera_q;
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let (grid_x, grid_y) = screen_to_grid(world_pos.x, world_pos.y);

    // Left-click
    if mouse_button.just_pressed(MouseButton::Left) {
        consumed.0 = true;

        match input_state.mode {
            StraitInputMode::SatelliteScan => {
                // Spawn satellite scan at cursor position
                let scan_cost = 15.0;
                if state.compute >= scan_cost {
                    state.compute -= scan_cost;
                    commands.spawn((
                        DreamEntity,
                        StraitSatelliteScan {
                            center: GridPos::new(grid_x, grid_y),
                            remaining_ticks: SATELLITE_SCAN_DURATION,
                        },
                    ));
                }
                input_state.mode = StraitInputMode::Normal;
            }
            StraitInputMode::ZeroDayDeploy(zd_type) => {
                // Deploy zero-day at cursor position
                if matches!(state.zero_day_slot, ZeroDayState::Ready(_)) {
                    state.zero_day_slot = ZeroDayState::Idle;
                    let idx = match zd_type {
                        ZeroDayType::Spoof => 0,
                        ZeroDayType::Blind => 1,
                        ZeroDayType::Hijack => 2,
                        ZeroDayType::Brick => 3,
                    };
                    state.zero_days_deployed[idx] = true;
                    info!("Deployed zero-day {:?} at ({}, {})", zd_type, grid_x, grid_y);
                    // TODO: Apply zero-day effects (brick launcher, blind area, etc.)
                }
                input_state.mode = StraitInputMode::Normal;
            }
            StraitInputMode::Normal => {
                // Hit-test against drones for selection
                let mut hit: Option<Entity> = None;
                let hit_dist_sq = 20.0 * 20.0; // 20 pixel hit radius
                for (entity, drone, xform) in drones.iter() {
                    if !drone.alive {
                        continue;
                    }
                    let dx = world_pos.x - xform.translation.x;
                    let dy = world_pos.y - xform.translation.y;
                    let d2 = dx * dx + dy * dy;
                    if d2 < hit_dist_sq {
                        hit = Some(entity);
                        break;
                    }
                }

                if let Some(entity) = hit {
                    // Toggle selection
                    if let Some(pos) = input_state.selected_drones.iter().position(|&e| e == entity) {
                        input_state.selected_drones.remove(pos);
                    } else {
                        if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
                            input_state.selected_drones.clear();
                        }
                        input_state.selected_drones.push(entity);
                    }
                } else {
                    // Click on empty space: clear selection
                    if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
                        input_state.selected_drones.clear();
                    }
                }
            }
        }
    }

    // Right-click: move selected drones
    if mouse_button.just_pressed(MouseButton::Right) && !input_state.selected_drones.is_empty() {
        consumed.0 = true;

        let target = GridPos::new(grid_x, grid_y);

        // Retain only entities that still exist and are alive
        let selected: Vec<Entity> = input_state
            .selected_drones
            .iter()
            .copied()
            .filter(|&e| drones.get(e).map_or(false, |(_, d, _)| d.alive))
            .collect();
        input_state.selected_drones = selected.clone();

        for entity in &selected {
            if let Ok((_, mut drone, xform)) = drones.get_mut(*entity) {
                // Get drone's current approximate grid position
                let (cur_x, cur_y) = screen_to_grid(xform.translation.x, xform.translation.y);
                // Set one-shot move: current → target
                drone.patrol_waypoints = vec![
                    GridPos::new(cur_x, cur_y),
                    target,
                ];
                drone.current_wp_index = 0;
            }
        }
    }
}

/// Decay satellite scans and despawn expired ones.
fn strait_satellite_decay(
    mut commands: Commands,
    mut scans: Query<(Entity, &mut StraitSatelliteScan)>,
) {
    for (entity, mut scan) in scans.iter_mut() {
        if scan.remaining_ticks == 0 {
            commands.entity(entity).despawn();
        } else {
            scan.remaining_ticks -= 1;
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strait_vision_reveal_and_query() {
        let mut vis = StraitVision::new(10, 10);
        assert!(!vis.is_visible(5, 5));

        vis.reveal(5, 5, 2);
        assert!(vis.is_visible(5, 5));
        assert!(vis.is_visible(6, 5));
        assert!(vis.is_visible(5, 6));
        assert!(!vis.is_visible(8, 8));

        vis.clear();
        assert!(!vis.is_visible(5, 5));
    }

    #[test]
    fn strait_vision_boundary_clamp() {
        let mut vis = StraitVision::new(5, 5);
        // Reveal at corner — should not panic
        vis.reveal(0, 0, 3);
        assert!(vis.is_visible(0, 0));
        assert!(vis.is_visible(2, 2));
        assert!(!vis.is_visible(4, 4));
    }

    #[test]
    fn compute_allocation_default_sums_to_one() {
        let alloc = ComputeAllocation::default();
        let sum = alloc.drone_vision + alloc.satellite + alloc.zero_day;
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn wave_configs_have_increasing_threat() {
        let w1 = EnemyWaveConfig::wave_1();
        let w2 = EnemyWaveConfig::wave_2();
        let w3 = EnemyWaveConfig::wave_3();
        let w4 = EnemyWaveConfig::wave_4();

        assert!(w2.launcher_count > w1.launcher_count);
        assert!(w3.aa_drone_count > w1.aa_drone_count);
        assert!(w4.coordinated_diversions);
    }
}
