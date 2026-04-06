//! Strait dream sequence simulation — pure game logic, no rendering.
//!
//! All systems operate on `StraitPos` (grid coordinates), tick-based timers,
//! and `Res<StraitConfigRes>`. No Bevy Transform, Mesh, or Gizmos dependencies.
//! The client layer (`cc_client::dream_strait`) syncs `StraitPos` → `Transform`
//! for visual rendering.
//!
//! Used by both the full game (via cc_client) and the headless test harness
//! (via cc_harness).

use bevy::prelude::*;

use cc_core::coords::GridPos;
use cc_core::strait::*;

// ---------------------------------------------------------------------------
// Position component (sim-only, replaces Transform)
// ---------------------------------------------------------------------------

/// Resource wrapper for `StraitConfig` (cc_core is Bevy-agnostic).
#[derive(Resource, Deref)]
pub struct StraitConfigRes(pub StraitConfig);

/// Grid-space position for strait entities. Client syncs this to Transform.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct StraitPos {
    pub x: f32,
    pub y: f32,
}

impl StraitPos {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn dist_sq(&self, other: &StraitPos) -> f32 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Friendly patrol drone.
#[derive(Component)]
pub struct StraitDrone {
    pub patrol_waypoints: Vec<GridPos>,
    pub current_wp_index: usize,
    pub alive: bool,
    pub drone_id: u32,
    pub mode: DroneMode,
    pub flare_cooldown: u32,
    pub bomb_ready: bool,
    pub bomb_reload_timer: u32,
}

/// Oil tanker in the convoy.
#[derive(Component)]
pub struct StraitTanker {
    pub hp: u32,
    pub lane_y: i32,
    pub target_x: f32,
    pub arrived: bool,
    pub destroyed: bool,
    pub tanker_index: u32,
}

/// Enemy mobile missile launcher.
#[derive(Component)]
pub struct StraitLauncher {
    pub phase: LauncherPhase,
    pub phase_timer: u32,
    pub hidden_pos: GridPos,
    pub firing_pos: GridPos,
    pub is_decoy: bool,
    pub salvo_count: u32,
    pub has_fired_this_phase: bool,
}

/// Enemy AA drone that hunts player patrol drones.
#[derive(Component)]
pub struct StraitAaDrone {
    pub target_drone: Option<Entity>,
    pub alive: bool,
    /// Ticks spent suppressing the current target.
    pub suppress_timer: u32,
}

/// In-flight anti-ship missile.
#[derive(Component)]
pub struct StraitMissile {
    pub state: MissileState,
    pub target_tanker: Option<Entity>,
    pub age_ticks: u32,
}

/// Enemy ground soldier.
#[derive(Component)]
pub struct StraitSoldier {
    pub phase: SoldierPhase,
    pub hp: u32,
    pub entity_id: u32,
    /// Ticks until next attack. 0 = ready.
    pub attack_cooldown: u32,
}

/// Enemy suicide drone (shaheed).
#[derive(Component)]
pub struct StraitShaheed {
    pub target: ShaheedTarget,
    pub entity_id: u32,
    pub alive: bool,
    /// Ticks until this shaheed activates and starts flying. 0 = active.
    pub launch_delay: u32,
}

/// Player base marker.
#[derive(Component)]
pub struct StraitBase;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Master state for the strait mission.
#[derive(Resource)]
pub struct StraitState {
    pub initialized: bool,

    // Compute flow
    pub allocation: ComputeAllocation,
    pub satellite_focal: Option<(i32, i32)>,

    // Logistics charges
    pub airstrike_charges: u32,
    pub airstrike_charge_timer: u32,
    pub drone_rebuild_charges: u32,
    pub drone_rebuild_charge_timer: u32,

    // Patriots (finite, no regen)
    pub patriot_count: u32,
    pub patriot_mode: PatriotMode,

    // Base
    pub base_hp: u32,

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
    pub zero_days_deployed: [bool; 4],

    // Enemy tracking
    pub initialized_enemies: bool,
    pub reinforcements_spawned: u32,
    pub next_entity_id: u32,

    // Mission phase
    pub mission_tick: u64,
    pub mission_complete: bool,
    pub convoy_hold: bool,
    pub convoy_ticks: u64,

    // Drone rebuilding
    pub drone_rebuilding: bool,
    pub drone_rebuild_timer: u32,

    // Airstrikes
    pub pending_airstrikes: Vec<(i32, i32, u32)>,

    // Scripts
    pub active_scripts: Vec<StraitScript>,
}

/// A Lua script registered for execution during the strait dream.
pub struct StraitScript {
    pub name: String,
    pub source: String,
    pub interval: u64,
    pub ticks_since_run: u64,
}

impl StraitState {
    pub fn new(config: &StraitConfig) -> Self {
        Self {
            initialized: false,
            allocation: ComputeAllocation::default(),
            satellite_focal: None,
            airstrike_charges: config.airstrike_max_charges,
            airstrike_charge_timer: 0,
            drone_rebuild_charges: config.drone_rebuild_max_charges,
            drone_rebuild_charge_timer: 0,
            patriot_count: config.initial_patriots,
            patriot_mode: PatriotMode::default(),
            base_hp: config.base_hp,
            tankers_spawned: 0,
            tankers_arrived: 0,
            tankers_destroyed: 0,
            tanker_spawn_timer: 0,
            next_drone_id: 0,
            drones_alive: 0,
            zero_day_slot: ZeroDayState::default(),
            zero_days_deployed: [false; 4],
            initialized_enemies: false,
            reinforcements_spawned: 0,
            next_entity_id: 0,
            mission_tick: 0,
            mission_complete: false,
            convoy_hold: true, // hold by default — script launches when ready
            convoy_ticks: 0,
            drone_rebuilding: false,
            drone_rebuild_timer: 0,
            pending_airstrikes: Vec::new(),
            active_scripts: Vec::new(),
        }
    }
}

/// Per-tile vision state.
#[derive(Resource)]
pub struct StraitVision {
    pub width: usize,
    pub height: usize,
    pub visible: Vec<bool>,
}

impl StraitVision {
    pub fn new(w: usize, h: usize) -> Self {
        Self { width: w, height: h, visible: vec![false; w * h] }
    }
    pub fn clear(&mut self) { self.visible.fill(false); }
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
// Spawn helpers (sim-only, no meshes)
// ---------------------------------------------------------------------------

pub fn spawn_drone(commands: &mut Commands, state: &mut StraitState, x: f32, y: f32) -> Entity {
    let id = state.next_drone_id;
    state.next_drone_id += 1;
    state.drones_alive += 1;
    commands.spawn((
        StraitDrone {
            patrol_waypoints: vec![GridPos::new(x as i32, y as i32)],
            current_wp_index: 0,
            alive: true,
            drone_id: id,
            mode: DroneMode::Patrol,
            flare_cooldown: 0,
            bomb_ready: true,
            bomb_reload_timer: 0,
        },
        StraitPos::new(x, y),
    )).id()
}

pub fn spawn_tanker(commands: &mut Commands, index: u32, lane_y: i32, target_x: f32) -> Entity {
    commands.spawn((
        StraitTanker {
            hp: TANKER_HP,
            lane_y,
            target_x,
            arrived: false,
            destroyed: false,
            tanker_index: index,
        },
        StraitPos::new(0.0, lane_y as f32),
    )).id()
}

pub fn spawn_launcher(
    commands: &mut Commands,
    hidden_pos: GridPos,
    firing_pos: GridPos,
    stagger_ticks: u32,
) -> Entity {
    commands.spawn((
        StraitLauncher {
            phase: LauncherPhase::Hidden,
            phase_timer: stagger_ticks,
            hidden_pos,
            firing_pos,
            is_decoy: false,
            salvo_count: 0,
            has_fired_this_phase: false,
        },
        StraitPos::new(hidden_pos.x as f32, hidden_pos.y as f32),
    )).id()
}

pub fn spawn_aa(commands: &mut Commands, x: f32, y: f32) -> Entity {
    commands.spawn((
        StraitAaDrone {
            target_drone: None,
            alive: true,
            suppress_timer: 0,
        },
        StraitPos::new(x, y),
    )).id()
}

pub fn spawn_soldier(commands: &mut Commands, state: &mut StraitState, x: f32, y: f32, config: &StraitConfig) -> Entity {
    let id = state.next_entity_id;
    state.next_entity_id += 1;
    commands.spawn((
        StraitSoldier {
            phase: SoldierPhase::Advancing,
            hp: config.soldier_hp,
            entity_id: id,
            attack_cooldown: 0,
        },
        StraitPos::new(x, y),
    )).id()
}

pub fn spawn_shaheed(commands: &mut Commands, state: &mut StraitState, x: f32, y: f32, target: ShaheedTarget, launch_delay: u32) -> Entity {
    let id = state.next_entity_id;
    state.next_entity_id += 1;
    commands.spawn((
        StraitShaheed {
            target,
            entity_id: id,
            alive: true,
            launch_delay,
        },
        StraitPos::new(x, y),
    )).id()
}

pub fn spawn_base(commands: &mut Commands, config: &StraitConfig) -> Entity {
    commands.spawn((
        StraitBase,
        StraitPos::new(config.base_x as f32, config.base_y as f32),
    )).id()
}

// ---------------------------------------------------------------------------
// Simulation systems
// ---------------------------------------------------------------------------

fn strait_tick(mut state: ResMut<StraitState>) {
    if state.mission_complete { return; }
    state.mission_tick += 1;
}

/// Flow economy: zero-day progress + logistics charge regen.
fn strait_flow_tick(mut state: ResMut<StraitState>, config: Res<StraitConfigRes>) {
    if state.mission_complete { return; }

    // Zero-day progress
    let zd_slice = state.allocation.zero_day;
    if let ZeroDayState::Building { exploit_type, progress, required } = &mut state.zero_day_slot {
        *progress += zd_slice;
        if *progress >= *required {
            let zt = *exploit_type;
            state.zero_day_slot = ZeroDayState::Ready(zt);
        }
    }

    // Airstrike charge regen
    if state.airstrike_charges < config.airstrike_max_charges {
        state.airstrike_charge_timer += 1;
        if state.airstrike_charge_timer >= config.airstrike_charge_regen_ticks {
            state.airstrike_charge_timer = 0;
            state.airstrike_charges += 1;
        }
    } else {
        state.airstrike_charge_timer = 0;
    }

    // Drone rebuild charge regen
    if state.drone_rebuild_charges < config.drone_rebuild_max_charges {
        state.drone_rebuild_charge_timer += 1;
        if state.drone_rebuild_charge_timer >= config.drone_rebuild_charge_regen_ticks {
            state.drone_rebuild_charge_timer = 0;
            state.drone_rebuild_charges += 1;
        }
    } else {
        state.drone_rebuild_charge_timer = 0;
    }
}

/// Drone flare cooldown tick.
fn strait_drone_flare_tick(mut drones: Query<&mut StraitDrone>) {
    for mut drone in drones.iter_mut() {
        if !drone.alive { continue; }
        if drone.flare_cooldown > 0 { drone.flare_cooldown -= 1; }
        if !drone.bomb_ready && drone.bomb_reload_timer > 0 {
            drone.bomb_reload_timer -= 1;
            if drone.bomb_reload_timer == 0 {
                drone.bomb_ready = true;
            }
        }
    }
}

/// Move drones based on their mode.
fn strait_move_drones(
    mut drones: Query<(&mut StraitDrone, &mut StraitPos)>,
    config: Res<StraitConfigRes>,
    state: Res<StraitState>,
) {
    if state.mission_complete { return; }
    let speed = config.drone_speed * 0.02; // grid units per tick

    for (mut drone, mut pos) in drones.iter_mut() {
        if !drone.alive { continue; }

        let target = match drone.mode {
            DroneMode::Patrol => {
                if drone.patrol_waypoints.is_empty() { continue; }
                let wp = drone.patrol_waypoints[drone.current_wp_index];
                Some((wp.x as f32, wp.y as f32))
            }
            DroneMode::MoveTo { x, y } => Some((x, y)),
            DroneMode::BombTarget { x, y } => Some((x, y)),
            DroneMode::GuardBase => {
                Some((config.base_x as f32, config.base_y as f32))
            }
        };

        if let Some((tx, ty)) = target {
            let dx = tx - pos.x;
            let dy = ty - pos.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 0.5 {
                // Arrived
                match drone.mode {
                    DroneMode::Patrol => {
                        drone.current_wp_index =
                            (drone.current_wp_index + 1) % drone.patrol_waypoints.len();
                    }
                    DroneMode::MoveTo { .. } => {
                        drone.mode = DroneMode::Patrol;
                    }
                    _ => {} // BombTarget and GuardBase stay
                }
            } else {
                let step = speed / dist;
                pos.x += dx * step;
                pos.y += dy * step;
            }
        }
    }
}

/// Drone bombing: when a BombTarget drone is in range, drop the bomb.
fn strait_drone_bomb(
    mut commands: Commands,
    mut drones: Query<(&mut StraitDrone, &StraitPos)>,
    launchers: Query<(Entity, &StraitLauncher, &StraitPos), Without<StraitDrone>>,
    aa_drones: Query<(Entity, &StraitAaDrone, &StraitPos), (Without<StraitDrone>, Without<StraitLauncher>)>,
    soldiers: Query<(Entity, &StraitSoldier, &StraitPos), (Without<StraitDrone>, Without<StraitLauncher>, Without<StraitAaDrone>)>,
    config: Res<StraitConfigRes>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete { return; }
    let bomb_range_sq = config.drone_bomb_range * config.drone_bomb_range;

    for (mut drone, dpos) in drones.iter_mut() {
        if !drone.alive || !drone.bomb_ready { continue; }
        let DroneMode::BombTarget { x: tx, y: ty } = drone.mode else { continue; };

        let dx = tx - dpos.x;
        let dy = ty - dpos.y;
        if dx * dx + dy * dy > bomb_range_sq { continue; }

        // In range — drop bomb
        drone.bomb_ready = false;
        drone.bomb_reload_timer = config.drone_bomb_reload;
        drone.mode = DroneMode::Patrol;

        let bomb_pos = StraitPos::new(tx, ty);

        // Kill AA drones in bomb range
        for (entity, aa, apos) in aa_drones.iter() {
            if !aa.alive { continue; }
            if bomb_pos.dist_sq(apos) <= bomb_range_sq {
                commands.entity(entity).despawn();
            }
        }
        // Kill soldiers in bomb range
        for (entity, _soldier, spos) in soldiers.iter() {
            if bomb_pos.dist_sq(spos) <= bomb_range_sq {
                commands.entity(entity).despawn();
            }
        }
        // Destroy launchers in any visible (non-Hidden) phase
        for (entity, launcher, lpos) in launchers.iter() {
            if launcher.phase == LauncherPhase::Hidden { continue; }
            if bomb_pos.dist_sq(lpos) <= bomb_range_sq {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// ALL alive drones passively intercept nearby active shaheeds.
/// This is a passive ability — any drone near a shaheed shoots it down.
/// GuardBase drones station at base specifically for this purpose, but
/// patrol drones along the shipping lane also intercept ship-targeting shaheeds.
fn strait_drone_intercept_shaheeds(
    drones: Query<(&StraitDrone, &StraitPos)>,
    mut shaheeds: Query<(&mut StraitShaheed, &StraitPos), Without<StraitDrone>>,
    config: Res<StraitConfigRes>,
    state: Res<StraitState>,
) {
    if state.mission_complete { return; }
    let range_sq = config.drone_intercept_range * config.drone_intercept_range;

    // Collect all alive drone positions
    let drone_positions: Vec<StraitPos> = drones.iter()
        .filter(|(d, _)| d.alive)
        .map(|(_, pos)| *pos)
        .collect();

    if drone_positions.is_empty() { return; }

    // For each active shaheed, check if any drone is close enough to intercept
    for (mut shaheed, spos) in shaheeds.iter_mut() {
        if !shaheed.alive || shaheed.launch_delay > 0 { continue; }
        for dpos in &drone_positions {
            if dpos.dist_sq(spos) <= range_sq {
                shaheed.alive = false;
                break;
            }
        }
    }
}

/// Update vision based on drone positions and satellite focal.
fn strait_update_vision(
    mut vision: ResMut<StraitVision>,
    drones: Query<(&StraitDrone, &StraitPos)>,
    state: Res<StraitState>,
    config: Res<StraitConfigRes>,
) {
    vision.clear();

    let drone_slice = state.allocation.drone_vision.clamp(0.0, 1.0);
    let drone_radius = (config.drone_vision_radius as f32 * drone_slice).ceil() as i32;

    if drone_radius > 0 {
        for (drone, pos) in drones.iter() {
            if !drone.alive { continue; }
            vision.reveal(pos.x as i32, pos.y as i32, drone_radius);
        }
    }

    if let Some((fx, fy)) = state.satellite_focal {
        let sat_slice = state.allocation.satellite.clamp(0.0, 1.0);
        let sat_radius = (config.satellite_vision_radius as f32 * sat_slice).ceil() as i32;
        if sat_radius > 0 {
            vision.reveal(fx, fy, sat_radius);
        }
    }
}

/// Spawn tankers when convoy is launched.
fn strait_spawn_tankers(
    mut commands: Commands,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete || state.tankers_spawned >= config.total_tankers { return; }
    if state.convoy_hold { return; }

    state.tanker_spawn_timer += 1;
    if state.tanker_spawn_timer < config.tanker_spawn_interval { return; }
    state.tanker_spawn_timer = 0;

    let idx = state.tankers_spawned;
    state.tankers_spawned += 1;
    let lane_y = 29 + (idx % 3) as i32;
    spawn_tanker(&mut commands, idx, lane_y, (config.map_width - 1) as f32);
}

/// Move tankers east along the shipping lane.
fn strait_move_tankers(
    mut state: ResMut<StraitState>,
    mut tankers: Query<(&mut StraitTanker, &mut StraitPos)>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }

    for (mut tanker, mut pos) in tankers.iter_mut() {
        if tanker.arrived || tanker.destroyed { continue; }

        pos.x += config.tanker_speed;
        if pos.x >= tanker.target_x {
            tanker.arrived = true;
            state.tankers_arrived += 1;
        }
    }
}

/// Enemy director: pre-deployed force at tick 0 + slow reinforcement trickle.
///
/// The bulk of the Iranian force is on the map from the start. The player
/// scouts and clears it before launching the convoy. Reinforcements arrive
/// slowly — enough to punish pure turtling but not enough to overwhelm
/// an active clearing strategy.
fn strait_enemy_director(
    mut commands: Commands,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }

    let map_w = config.map_width as f32;

    // === TICK 0: deploy the full Iranian force ===
    if state.mission_tick == 1 && !state.initialized_enemies {
        state.initialized_enemies = true;

        // Launchers — spread along hostile coast, heavily staggered so they
        // don't all fire in the first 200 ticks
        for i in 0..config.deployed_launchers {
            let spacing = map_w / (config.deployed_launchers + 1) as f32;
            let x = ((i + 1) as f32 * spacing) as i32;
            spawn_launcher(&mut commands, GridPos::new(x, 4), GridPos::new(x, 9), 100 + i * 200);
        }

        // AA drones — patrol the upper strait
        for i in 0..config.deployed_aa {
            let x = map_w * (i as f32 + 1.0) / (config.deployed_aa as f32 + 1.0);
            spawn_aa(&mut commands, x, 12.0);
        }

        // Soldiers — positioned between coast and shipping lane
        for i in 0..config.deployed_soldiers {
            let x = map_w * (i as f32 + 1.0) / (config.deployed_soldiers as f32 + 1.0);
            let y = 8.0 + (i % 3) as f32 * 3.0; // stagger depth
            spawn_soldier(&mut commands, &mut state, x, y, &config);
        }

        // Shaheeds — on launch pads, staggered launch delays so they
        // don't all fly at once. One launches every ~200 ticks.
        for i in 0..config.deployed_shaheeds {
            let x = map_w * (i as f32 + 1.0) / (config.deployed_shaheeds as f32 + 1.0);
            let target = ShaheedTarget::Base;
            let delay = 100 + i * 200; // first at tick 100, last at tick 1500
            spawn_shaheed(&mut commands, &mut state, x, 3.0, target, delay);
        }

        return;
    }

    // === REINFORCEMENT TRICKLE ===
    if config.reinforcement_interval == 0 { return; }
    if state.reinforcements_spawned >= config.reinforcement_cap { return; }
    if state.mission_tick % config.reinforcement_interval as u64 != 0 { return; }
    if state.mission_tick < 2 { return; } // skip tick 0/1

    let (r_launchers, r_aa, r_soldiers, r_shaheeds) = config.reinforcement_batch;

    for i in 0..r_launchers {
        if state.reinforcements_spawned >= config.reinforcement_cap { break; }
        let x = (map_w * 0.3 + (i as f32 * 60.0)) as i32 % config.map_width as i32;
        spawn_launcher(&mut commands, GridPos::new(x, 4), GridPos::new(x, 9), 40);
        state.reinforcements_spawned += 1;
    }
    for i in 0..r_aa {
        if state.reinforcements_spawned >= config.reinforcement_cap { break; }
        let x = map_w * 0.5 + i as f32 * 40.0;
        spawn_aa(&mut commands, x, 14.0);
        state.reinforcements_spawned += 1;
    }
    for i in 0..r_soldiers {
        if state.reinforcements_spawned >= config.reinforcement_cap { break; }
        let x = map_w * (0.2 + (state.reinforcements_spawned as f32 * 0.1) % 0.6);
        spawn_soldier(&mut commands, &mut state, x, 6.0, &config);
        state.reinforcements_spawned += 1;
    }
    for i in 0..r_shaheeds {
        if state.reinforcements_spawned >= config.reinforcement_cap { break; }
        let x = map_w * (0.3 + (state.reinforcements_spawned as f32 * 0.1) % 0.5);
        let target = if state.convoy_hold { ShaheedTarget::Base } else { ShaheedTarget::Ship(0) };
        spawn_shaheed(&mut commands, &mut state, x, 3.0, target, 0); // reinforcements launch immediately
        state.reinforcements_spawned += 1;
    }
}

/// Launcher state machine (tick-based).
fn strait_launcher_fsm(
    mut launchers: Query<(&mut StraitLauncher, &mut StraitPos)>,
    state: Res<StraitState>,
) {
    if state.mission_complete { return; }

    for (mut launcher, mut pos) in launchers.iter_mut() {
        if launcher.phase_timer > 0 {
            launcher.phase_timer -= 1;
            continue;
        }

        match launcher.phase {
            LauncherPhase::Hidden => {
                launcher.phase = LauncherPhase::Setting;
                launcher.phase_timer = 30; // ~3 seconds at 10hz
                launcher.has_fired_this_phase = false;
                pos.x = launcher.firing_pos.x as f32;
                pos.y = launcher.firing_pos.y as f32;
            }
            LauncherPhase::Setting => {
                launcher.phase = LauncherPhase::Firing;
                launcher.phase_timer = 20;
            }
            LauncherPhase::Firing => {
                launcher.phase = LauncherPhase::Retreating;
                launcher.phase_timer = 20;
                pos.x = launcher.hidden_pos.x as f32;
                pos.y = launcher.hidden_pos.y as f32;
            }
            LauncherPhase::Retreating => {
                launcher.phase = LauncherPhase::Hidden;
                let jitter = (launcher.salvo_count as i32 * 3) % 7;
                launcher.hidden_pos.x = (launcher.hidden_pos.x + jitter).clamp(1, 298);
                launcher.phase_timer = 80 + launcher.salvo_count * 20;
                launcher.salvo_count += 1;
                pos.x = launcher.hidden_pos.x as f32;
                pos.y = launcher.hidden_pos.y as f32;
            }
        }
    }
}

/// Spawn missiles from firing launchers. Targets ships if present, base otherwise.
fn strait_spawn_missiles(
    mut commands: Commands,
    mut launchers: Query<&mut StraitLauncher>,
    tankers: Query<(Entity, &StraitTanker, &StraitPos)>,
    config: Res<StraitConfigRes>,
    state: Res<StraitState>,
) {
    if state.mission_complete { return; }

    for mut launcher in launchers.iter_mut() {
        if launcher.phase != LauncherPhase::Firing || launcher.is_decoy { continue; }
        if launcher.has_fired_this_phase { continue; }
        launcher.has_fired_this_phase = true;

        let fx = launcher.firing_pos.x as f32;
        let fy = launcher.firing_pos.y as f32;

        // Try to target a tanker; if no tankers, target base
        let target = tankers.iter()
            .filter(|(_, t, _)| !t.arrived && !t.destroyed)
            .min_by(|(_, _, pa), (_, _, pb)| {
                let da = (pa.x - fx).powi(2) + (pa.y - fy).powi(2);
                let db = (pb.x - fx).powi(2) + (pb.y - fy).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

        let (target_entity, target_x, target_y) = if let Some((entity, tanker, tpos)) = target {
            (Some(entity), tpos.x, tanker.lane_y as f32)
        } else {
            // No ships — target base
            (None, config.base_x as f32, config.base_y as f32)
        };

        commands.spawn((
            StraitMissile {
                state: MissileState::InFlight {
                    origin_x: fx,
                    origin_y: fy,
                    target_x,
                    target_y,
                    progress: 0.0,
                },
                target_tanker: target_entity,
                age_ticks: 0,
            },
            StraitPos::new(fx, fy),
        ));
    }
}

/// Advance missile positions.
fn strait_missile_flight(
    mut missiles: Query<(&mut StraitMissile, &mut StraitPos)>,
    config: Res<StraitConfigRes>,
    state: Res<StraitState>,
) {
    if state.mission_complete { return; }
    let progress_per_tick = 1.0 / config.missile_flight_ticks as f32;

    for (mut missile, mut pos) in missiles.iter_mut() {
        if let MissileState::InFlight { origin_x, origin_y, target_x, target_y, progress } = &mut missile.state {
            *progress += progress_per_tick;
            pos.x = *origin_x + (*target_x - *origin_x) * *progress;
            pos.y = *origin_y + (*target_y - *origin_y) * *progress;
        }
        missile.age_ticks += 1;
    }
}

/// Patriot system: base defense against incoming missiles and shaheeds.
fn strait_patriot_system(
    mut commands: Commands,
    mut state: ResMut<StraitState>,
    mut missiles: Query<(Entity, &mut StraitMissile, &StraitPos)>,
    shaheeds: Query<(Entity, &StraitShaheed, &StraitPos), Without<StraitMissile>>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete || state.patriot_count == 0 { return; }

    let base_pos = StraitPos::new(config.base_x as f32, config.base_y as f32);
    let range_sq = config.patriot_range * config.patriot_range;

    // Engage ALL in-flight missiles within Patriot range (regardless of target)
    for (entity, mut missile, mpos) in missiles.iter_mut() {
        if !matches!(missile.state, MissileState::InFlight { .. }) { continue; }
        if state.patriot_count == 0 { break; }

        if base_pos.dist_sq(mpos) <= range_sq {
            if let MissileState::InFlight { progress, .. } = missile.state {
                if progress > 0.3 {
                    state.patriot_count -= 1;
                    missile.state = MissileState::Intercepted;
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // Engage shaheeds targeting base (only in Auto mode)
    if state.patriot_mode == PatriotMode::Auto {
        for (entity, shaheed, spos) in shaheeds.iter() {
            if state.patriot_count == 0 { break; }
            if !shaheed.alive { continue; }
            if !matches!(shaheed.target, ShaheedTarget::Base) { continue; }

            if base_pos.dist_sq(spos) <= range_sq {
                state.patriot_count -= 1;
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Missile impact: damage tankers or base.
fn strait_missile_impact(
    mut commands: Commands,
    mut missiles: Query<(Entity, &mut StraitMissile)>,
    mut tankers: Query<(Entity, &mut StraitTanker)>,
    mut state: ResMut<StraitState>,
) {
    if state.mission_complete { return; }

    for (entity, mut missile) in missiles.iter_mut() {
        if matches!(missile.state, MissileState::Intercepted | MissileState::Impact) { continue; }

        if let MissileState::InFlight { progress, .. } = missile.state {
            if progress < 1.0 { continue; }

            missile.state = MissileState::Impact;

            if let Some(target_entity) = missile.target_tanker {
                if let Ok((_, mut tanker)) = tankers.get_mut(target_entity) {
                    if !tanker.destroyed && !tanker.arrived {
                        tanker.hp = tanker.hp.saturating_sub(1);
                        if tanker.hp == 0 {
                            tanker.destroyed = true;
                            state.tankers_destroyed += 1;
                        }
                    }
                }
            } else {
                // Missile targeted base
                state.base_hp = state.base_hp.saturating_sub(1);
            }

            commands.entity(entity).despawn();
        }
    }
}

/// AA drone behavior: hunt drones, suppression with flare interaction.
fn strait_enemy_aa(
    mut aa_drones: Query<(&mut StraitAaDrone, &mut StraitPos), Without<StraitDrone>>,
    mut drones: Query<(Entity, &mut StraitDrone, &StraitPos), Without<StraitAaDrone>>,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }

    let suppress_range_sq = config.aa_suppress_range * config.aa_suppress_range;

    for (mut aa, mut aa_pos) in aa_drones.iter_mut() {
        if !aa.alive { continue; }

        // Find nearest alive drone
        let mut nearest: Option<(Entity, f32, StraitPos)> = None;
        for (entity, drone, dpos) in drones.iter() {
            if !drone.alive { continue; }
            let d2 = aa_pos.dist_sq(dpos);
            if nearest.map_or(true, |(_, nd, _)| d2 < nd) {
                nearest = Some((entity, d2, *dpos));
            }
        }

        let Some((target_entity, dist_sq, target_pos)) = nearest else { continue; };
        aa.target_drone = Some(target_entity);

        // Move toward target
        let dx = target_pos.x - aa_pos.x;
        let dy = target_pos.y - aa_pos.y;
        let dist = dist_sq.sqrt();
        if dist > 0.5 {
            let speed = 0.04; // slightly slower than patrol drones
            aa_pos.x += dx / dist * speed;
            aa_pos.y += dy / dist * speed;
        }

        // Suppression check
        if dist_sq > suppress_range_sq { aa.suppress_timer = 0; continue; }

        // Count drones with ready flares in suppress range
        let mut drones_with_flares = 0u32;
        for (_, drone, dpos) in drones.iter() {
            if !drone.alive { continue; }
            if aa_pos.dist_sq(dpos) <= suppress_range_sq && drone.flare_cooldown == 0 {
                drones_with_flares += 1;
            }
        }

        if drones_with_flares >= config.aa_swarm_threshold {
            // Overwhelmed! AA can't suppress, drones can bomb.
            // Pop flares on the drones that were counted
            for (_, mut drone, dpos) in drones.iter_mut() {
                if !drone.alive { continue; }
                if aa_pos.dist_sq(dpos) <= suppress_range_sq && drone.flare_cooldown == 0 {
                    drone.flare_cooldown = config.drone_flare_cooldown;
                }
            }
            aa.suppress_timer = 0;
            continue;
        }

        // Suppressing: tick toward kill
        aa.suppress_timer += 1;
        if aa.suppress_timer >= config.aa_engagement_ticks {
            // Kill the target drone
            if let Ok((_, mut drone, _)) = drones.get_mut(target_entity) {
                if drone.alive {
                    // If drone has flare ready, pop it and survive
                    if drone.flare_cooldown == 0 {
                        drone.flare_cooldown = config.drone_flare_cooldown;
                        aa.suppress_timer = 0;
                    } else {
                        drone.alive = false;
                        state.drones_alive = state.drones_alive.saturating_sub(1);
                        aa.suppress_timer = 0;
                    }
                }
            }
        }
    }
}

/// Soldier AI: advance toward shipping lane, damage tankers at close range.
fn strait_soldier_ai(
    mut commands: Commands,
    mut soldiers: Query<(Entity, &mut StraitSoldier, &mut StraitPos)>,
    mut tankers: Query<(&mut StraitTanker, &StraitPos), Without<StraitSoldier>>,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }
    let danger_sq = config.soldier_danger_range * config.soldier_danger_range;

    for (_entity, mut soldier, mut pos) in soldiers.iter_mut() {
        match soldier.phase {
            SoldierPhase::Advancing => {
                // Move south toward shipping lane (y=30)
                pos.y += config.soldier_speed;
                if pos.y >= 28.0 {
                    soldier.phase = SoldierPhase::Engaged;
                }
            }
            SoldierPhase::Engaged => {
                // Attack cooldown
                if soldier.attack_cooldown > 0 {
                    soldier.attack_cooldown -= 1;
                    continue;
                }
                // Look for tankers in danger range
                for (mut tanker, tpos) in tankers.iter_mut() {
                    if tanker.arrived || tanker.destroyed { continue; }
                    let d2 = pos.dist_sq(tpos);
                    if d2 <= danger_sq {
                        tanker.hp = tanker.hp.saturating_sub(1);
                        if tanker.hp == 0 {
                            tanker.destroyed = true;
                            state.tankers_destroyed += 1;
                        }
                        soldier.attack_cooldown = 100; // ~10 seconds between attacks
                        break;
                    }
                }
            }
        }
    }
}

/// Shaheed AI: fly toward target (ship or base).
fn strait_shaheed_ai(
    mut commands: Commands,
    mut shaheeds: Query<(Entity, &mut StraitShaheed, &mut StraitPos)>,
    mut tankers: Query<(Entity, &mut StraitTanker, &StraitPos), Without<StraitShaheed>>,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }
    let impact_range_sq: f32 = 1.0;

    for (entity, mut shaheed, mut pos) in shaheeds.iter_mut() {
        if !shaheed.alive {
            commands.entity(entity).despawn();
            continue;
        }
        // Still on the launch pad — count down
        if shaheed.launch_delay > 0 {
            shaheed.launch_delay -= 1;
            continue;
        }
        let (target_x, target_y) = match shaheed.target {
            ShaheedTarget::Base => (config.base_x as f32, config.base_y as f32),
            ShaheedTarget::Ship(idx) => {
                // Find the tanker by index
                if let Some((_, _, tpos)) = tankers.iter()
                    .find(|(_, t, _)| t.tanker_index == idx && !t.arrived && !t.destroyed)
                {
                    (tpos.x, tpos.y)
                } else {
                    // Tanker gone — retarget to base
                    (config.base_x as f32, config.base_y as f32)
                }
            }
        };

        let dx = target_x - pos.x;
        let dy = target_y - pos.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= impact_range_sq.sqrt() {
            // Impact!
            match shaheed.target {
                ShaheedTarget::Base => {
                    state.base_hp = state.base_hp.saturating_sub(config.shaheed_damage);
                }
                ShaheedTarget::Ship(idx) => {
                    for (_, mut tanker, _) in tankers.iter_mut() {
                        if tanker.tanker_index == idx && !tanker.destroyed && !tanker.arrived {
                            tanker.hp = tanker.hp.saturating_sub(config.shaheed_damage);
                            if tanker.hp == 0 {
                                tanker.destroyed = true;
                                state.tankers_destroyed += 1;
                            }
                            break;
                        }
                    }
                }
            }
            commands.entity(entity).despawn();
        } else {
            let step = config.shaheed_speed / dist;
            pos.x += dx * step;
            pos.y += dy * step;
        }
    }
}

/// Drone rebuild: tick the timer and spawn a new drone when done.
fn strait_drone_rebuild(
    mut commands: Commands,
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if !state.drone_rebuilding || state.mission_complete { return; }

    if state.drone_rebuild_timer > 0 {
        state.drone_rebuild_timer -= 1;
        return;
    }

    state.drone_rebuilding = false;
    spawn_drone(&mut commands, &mut state, config.base_x as f32, config.base_y as f32);
}

/// Airstrike countdown and damage.
fn strait_airstrike(
    mut commands: Commands,
    mut state: ResMut<StraitState>,
    launchers: Query<(Entity, &StraitLauncher, &StraitPos)>,
    aa_drones: Query<(Entity, &StraitAaDrone, &StraitPos), Without<StraitLauncher>>,
    soldiers: Query<(Entity, &StraitSoldier, &StraitPos), (Without<StraitLauncher>, Without<StraitAaDrone>)>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }
    let radius_sq = (config.airstrike_radius * config.airstrike_radius) as f32;

    let mut completed = Vec::new();
    for (i, strike) in state.pending_airstrikes.iter_mut().enumerate() {
        if strike.2 > 0 {
            strike.2 -= 1;
        } else {
            completed.push(i);
            let center = StraitPos::new(strike.0 as f32, strike.1 as f32);

            for (entity, _, lpos) in launchers.iter() {
                if center.dist_sq(lpos) <= radius_sq {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, aa, apos) in aa_drones.iter() {
                if !aa.alive { continue; }
                if center.dist_sq(apos) <= radius_sq {
                    commands.entity(entity).despawn();
                }
            }
            for (entity, _, spos) in soldiers.iter() {
                if center.dist_sq(spos) <= radius_sq {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    for i in completed.into_iter().rev() {
        state.pending_airstrikes.remove(i);
    }
}

/// Check win/lose conditions.
fn strait_check_win_lose(
    mut state: ResMut<StraitState>,
    config: Res<StraitConfigRes>,
) {
    if state.mission_complete { return; }

    // Base destroyed
    if state.base_hp == 0 {
        state.mission_complete = true;
        info!("Strait LOST: base destroyed");
        return;
    }

    // Track convoy ticks (no hard limit — soft pressure from Patriot drain)
    if !state.convoy_hold {
        state.convoy_ticks += 1;
    }

    // Too many tankers lost
    if state.tankers_destroyed >= config.max_tankers_lost {
        state.mission_complete = true;
        info!("Strait LOST: {} tankers destroyed", state.tankers_destroyed);
        return;
    }

    // All tankers resolved
    let all_resolved = state.tankers_arrived + state.tankers_destroyed >= state.tankers_spawned
        && state.tankers_spawned >= config.total_tankers;

    if all_resolved {
        state.mission_complete = true;
        if state.tankers_arrived >= config.min_tankers_win {
            info!("Strait WON: {}/{} tankers arrived", state.tankers_arrived, config.total_tankers);
        } else {
            info!("Strait LOST: only {}/{} arrived", state.tankers_arrived, config.min_tankers_win);
        }
    }
}

// ---------------------------------------------------------------------------
// System registration
// ---------------------------------------------------------------------------

/// Register all sim-only strait systems (no rendering deps).
pub fn register_strait_sim_systems(app: &mut App) {
    // Group 1: core tick + drone movement + vision
    app.add_systems(
        Update,
        (
            strait_tick,
            strait_flow_tick.after(strait_tick),
            strait_drone_flare_tick.after(strait_tick),
            strait_move_drones.after(strait_tick),
            strait_drone_bomb.after(strait_move_drones),
            strait_drone_intercept_shaheeds.after(strait_move_drones),
            strait_update_vision.after(strait_move_drones),
            strait_spawn_tankers.after(strait_tick),
            strait_move_tankers.after(strait_spawn_tankers),
        ),
    );
    // Group 2: enemy AI + combat + win/lose
    app.add_systems(
        Update,
        (
            strait_enemy_director,
            strait_launcher_fsm.after(strait_enemy_director),
            strait_spawn_missiles.after(strait_launcher_fsm),
            strait_missile_flight.after(strait_spawn_missiles),
            strait_patriot_system.after(strait_missile_flight),
            strait_missile_impact.after(strait_patriot_system),
            strait_enemy_aa,
            strait_soldier_ai,
            strait_shaheed_ai,
            strait_drone_rebuild,
            strait_airstrike,
            strait_check_win_lose.after(strait_missile_impact),
        ),
    );
}

/// Build a headless world + schedule for the strait sim.
pub fn build_headless_world(config: StraitConfig) -> (World, Schedule) {
    let mut world = World::new();

    let mut state = StraitState::new(&config);
    let vision = StraitVision::new(config.map_width as usize, config.map_height as usize);
    let drone_count = config.initial_patrol_drones;
    let base_x = config.base_x as f32;
    let base_y = config.base_y as f32;

    // Spawn drones (pre-allocate IDs before inserting state as resource)
    for i in 0..drone_count {
        let offset_x = (i % 4) as f32 * 2.0 - 3.0;
        let offset_y = (i / 4) as f32 * 2.0 - 3.0;
        let id = state.next_drone_id;
        state.next_drone_id += 1;
        state.drones_alive += 1;

        world.spawn((
            StraitDrone {
                patrol_waypoints: vec![GridPos::new((base_x + offset_x) as i32, (base_y + offset_y) as i32)],
                current_wp_index: 0,
                alive: true,
                drone_id: id,
                mode: DroneMode::Patrol,
                flare_cooldown: 0,
                bomb_ready: true,
                bomb_reload_timer: 0,
            },
            StraitPos::new(base_x + offset_x, base_y + offset_y),
        ));
    }

    // Spawn base
    world.spawn((
        StraitBase,
        StraitPos::new(base_x, base_y),
    ));

    // Insert resources after entity setup
    world.insert_resource(state);
    world.insert_resource(vision);
    world.insert_resource(StraitConfigRes(config.clone()));

    // Build schedule with all sim systems. apply_deferred between groups
    // ensures Commands (entity spawns, despawns) are flushed.
    let mut schedule = Schedule::default();
    schedule.add_systems((
        strait_tick,
        strait_flow_tick.after(strait_tick),
        strait_drone_flare_tick.after(strait_tick),
        strait_move_drones.after(strait_tick),
        strait_drone_bomb.after(strait_move_drones),
        strait_drone_intercept_shaheeds.after(strait_move_drones),
        strait_update_vision.after(strait_move_drones),
        strait_spawn_tankers.after(strait_tick),
        strait_move_tankers.after(strait_spawn_tankers),
    ));
    schedule.add_systems((
        strait_enemy_director,
        strait_launcher_fsm.after(strait_enemy_director),
        strait_spawn_missiles.after(strait_launcher_fsm),
        strait_missile_flight.after(strait_spawn_missiles),
        strait_patriot_system.after(strait_missile_flight),
        strait_missile_impact.after(strait_patriot_system),
        strait_enemy_aa,
        strait_soldier_ai,
        strait_shaheed_ai,
        strait_drone_rebuild,
        strait_airstrike,
        strait_check_win_lose.after(strait_missile_impact),
    ));
    // Ensure deferred Commands (spawns/despawns) are applied at end of each run.
    schedule.set_apply_final_deferred(true);

    (world, schedule)
}

// ---------------------------------------------------------------------------
// Outcome
// ---------------------------------------------------------------------------

/// Mission outcome after completion.
#[derive(Debug, Clone)]
pub enum StraitOutcome {
    Win { tankers_arrived: u32, total: u32 },
    Lose { reason: String },
}

impl StraitState {
    pub fn outcome(&self, config: &StraitConfig) -> Option<StraitOutcome> {
        if !self.mission_complete { return None; }

        if self.tankers_arrived >= config.min_tankers_win {
            Some(StraitOutcome::Win {
                tankers_arrived: self.tankers_arrived,
                total: config.total_tankers,
            })
        } else if self.base_hp == 0 {
            Some(StraitOutcome::Lose { reason: "base destroyed".into() })
        } else if self.tankers_destroyed >= config.max_tankers_lost {
            Some(StraitOutcome::Lose {
                reason: format!("{} tankers destroyed", self.tankers_destroyed),
            })
        } else {
            Some(StraitOutcome::Lose { reason: "time limit".into() })
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
    fn vision_reveal_and_query() {
        let mut vis = StraitVision::new(10, 10);
        assert!(!vis.is_visible(5, 5));
        vis.reveal(5, 5, 2);
        assert!(vis.is_visible(5, 5));
        assert!(vis.is_visible(6, 5));
        assert!(!vis.is_visible(8, 8));
        vis.clear();
        assert!(!vis.is_visible(5, 5));
    }

    #[test]
    fn headless_world_initializes() {
        let config = StraitConfig::default();
        let (world, _schedule) = build_headless_world(config.clone());
        let state = world.resource::<StraitState>();
        assert_eq!(state.drones_alive, config.initial_patrol_drones);
        assert_eq!(state.patriot_count, config.initial_patriots);
        assert!(state.convoy_hold);
        assert!(!state.mission_complete);
    }

    #[test]
    fn headless_sim_ticks() {
        let config = StraitConfig::default();
        let (mut world, mut schedule) = build_headless_world(config);

        for _ in 0..100 {
            schedule.run(&mut world);
        }

        let state = world.resource::<StraitState>();
        assert_eq!(state.mission_tick, 100);
        assert!(!state.mission_complete);
        // Convoy held — no tankers spawned
        assert_eq!(state.tankers_spawned, 0);
    }

    #[test]
    fn predeployed_shaheeds_target_base_when_convoy_held() {
        let mut config = StraitConfig::default();
        // Only shaheeds, no other enemies, no patriots (so they aren't intercepted)
        config.deployed_launchers = 0;
        config.deployed_aa = 0;
        config.deployed_soldiers = 0;
        config.deployed_shaheeds = 3;
        config.initial_patriots = 0;
        config.reinforcement_interval = 0; // no reinforcements
        let (mut world, mut schedule) = build_headless_world(config);

        // Advance so pre-deployed force spawns (tick 1) and entities flush
        for _ in 0..5 {
            schedule.run(&mut world);
        }

        // Shaheeds should exist
        let mut q = world.query::<&StraitShaheed>();
        let shaheed_count = q.iter(&world).count();
        assert_eq!(shaheed_count, 3);

        // All target base since convoy is held
        for shaheed in q.iter(&world) {
            assert_eq!(shaheed.target, ShaheedTarget::Base);
        }
    }

    #[test]
    fn strait_pos_distance() {
        let a = StraitPos::new(0.0, 0.0);
        let b = StraitPos::new(3.0, 4.0);
        assert!((a.dist_sq(&b) - 25.0).abs() < 0.01);
    }

    #[test]
    fn allocation_default_sums_to_one() {
        let alloc = ComputeAllocation::default();
        let sum = alloc.drone_vision + alloc.satellite + alloc.zero_day;
        assert!((sum - 1.0).abs() < 0.01);
    }
}
