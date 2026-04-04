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

/// Marker for the dark DEFCON background overlay.
#[derive(Component)]
pub struct StraitOverlayBg;

/// Marker for coastline visual mesh.
#[derive(Component)]
pub struct StraitCoastline;

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
        .insert_resource(StraitVision::new(60, 20))
        .add_systems(
            Update,
            (
                strait_init_system.run_if(is_dream_strait_active),
                // Core simulation (Phase 3)
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
                // Drone patrol (Phase 4)
                strait_move_drones
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_update_vision
                    .after(strait_move_drones)
                    .run_if(is_dream_strait_active),
                // Enemy AI (Phase 5)
                strait_enemy_director
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                strait_spawn_wave_entities
                    .after(strait_enemy_director)
                    .run_if(is_dream_strait_active),
                strait_launcher_fsm
                    .after(strait_spawn_wave_entities)
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
                    .after(strait_tick_system)
                    .run_if(is_dream_strait_active),
                // Visuals (Phase 2)
                strait_radar_sweep
                    .after(strait_move_drones)
                    .run_if(is_dream_strait_active),
                strait_update_hud
                    .after(strait_move_tankers)
                    .run_if(is_dream_strait_active),
                // Win/lose
                strait_check_win_lose
                    .after(strait_missile_impact)
                    .after(strait_move_tankers)
                    .run_if(is_dream_strait_active),
            ),
        );
}

// ---------------------------------------------------------------------------
// Helper: world position from grid for the 60x20 strait map
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
) {
    if state.initialized {
        return;
    }
    state.initialized = true;

    // -- Dark DEFCON background overlay --
    commands.spawn((
        DreamEntity,
        StraitOverlayBg,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(DEFCON_BG),
        ZIndex(3),
    ));

    // -- Spawn HUD --
    spawn_strait_hud(&mut commands);

    // -- Spawn patrol drones --
    let map_width = 60;
    let drone_count = INITIAL_PATROL_DRONES;
    let spacing = map_width as f32 / drone_count as f32;

    for i in 0..drone_count {
        let base_x = (i as f32 * spacing + spacing * 0.5) as i32;
        let patrol_y = 8; // middle of the strait
        let waypoints = vec![
            GridPos::new(base_x, patrol_y - 2),
            GridPos::new(base_x + 3, patrol_y),
            GridPos::new(base_x, patrol_y + 2),
            GridPos::new(base_x.saturating_sub(3).max(1), patrol_y),
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
        let hidden_y = 1; // deep in hostile shore
        let firing_y = 3; // at the shallows edge

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
        let y = 5.0; // patrol in the upper strait
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
    if state.mission_tick < 60 {
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
    let lane_y = 9 + (tanker_index % 3) as i32; // slight lane variation
    let pos = strait_screen_from_world(0.0, lane_y as f32);
    let tanker_mesh = meshes.add(Rectangle::new(10.0, 5.0));
    let tanker_mat = materials.add(ColorMaterial::from_color(TANKER_BLUE));

    commands.spawn((
        DreamEntity,
        StraitTanker {
            hp: TANKER_HP,
            lane_y,
            world_x: 0.0,
            target_x: 59.0,
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
                launcher.hidden_pos.x = (launcher.hidden_pos.x + jitter).clamp(1, 58);
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
            if cur_y >= 6.0 && cur_y <= 14.0 && state.interceptor_count > 0 {
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
    mut aa_drones: Query<(&mut StraitAaDrone, &mut Transform)>,
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
