//! Headless strait simulation for Lua script testing.
//!
//! Wraps `cc_sim::strait_sim::build_headless_world()` and provides
//! a high-level API for advancing the sim, running Lua scripts, and
//! querying the outcome.

use bevy::prelude::*;

use cc_core::strait::*;
use cc_sim::strait_sim::*;
use cc_agent::strait_bindings::*;

/// Headless strait simulation.
pub struct StraitHeadlessSim {
    world: World,
    schedule: Schedule,
    config: StraitConfig,
}

impl StraitHeadlessSim {
    pub fn new(config: StraitConfig) -> Self {
        let (world, schedule) = build_headless_world(config.clone());
        Self { world, schedule, config }
    }

    /// Advance the simulation by N ticks.
    pub fn advance(&mut self, n_ticks: u32) {
        for _ in 0..n_ticks {
            self.schedule.run(&mut self.world);
        }
    }

    /// Get the current StraitState.
    pub fn state(&self) -> &StraitState {
        self.world.resource::<StraitState>()
    }

    /// Whether the mission is complete.
    pub fn is_complete(&self) -> bool {
        self.state().mission_complete
    }

    /// Mission outcome (None if still running).
    pub fn outcome(&self) -> Option<StraitOutcome> {
        self.state().outcome(&self.config)
    }

    /// Build a snapshot for Lua consumption.
    pub fn snapshot(&mut self) -> StraitSnapshot {
        // Clone state data upfront (all Copy types, no borrow held).
        let (allocation, satellite_focal, airstrike_charges, drone_rebuild_charges,
             patriot_count, patriot_mode, base_hp, convoy_hold,
             tankers_arrived, tankers_destroyed, tankers_spawned, drones_alive,
             zero_day_slot, mission_tick, mission_complete) = {
            let s = self.world.resource::<StraitState>();
            (s.allocation, s.satellite_focal, s.airstrike_charges, s.drone_rebuild_charges,
             s.patriot_count, s.patriot_mode, s.base_hp, s.convoy_hold,
             s.tankers_arrived, s.tankers_destroyed, s.tankers_spawned, s.drones_alive,
             s.zero_day_slot, s.mission_tick, s.mission_complete)
        };

        // Each query in its own scope so the QueryState borrow is released.
        let drone_positions = {
            let mut out = Vec::new();
            let mut q = self.world.query::<(&StraitDrone, &StraitPos)>();
            for (drone, pos) in q.iter(&self.world) {
                out.push(DroneInfo {
                    id: drone.drone_id, x: pos.x, y: pos.y, alive: drone.alive,
                    mode: drone_mode_string(&drone.mode).to_string(),
                    flare_cooldown: drone.flare_cooldown, bomb_ready: drone.bomb_ready,
                });
            }
            out
        };

        let tanker_positions = {
            let mut out = Vec::new();
            let mut q = self.world.query::<(&StraitTanker, &StraitPos)>();
            for (t, pos) in q.iter(&self.world) {
                out.push(TankerInfo {
                    index: t.tanker_index, x: pos.x, y: t.lane_y,
                    hp: t.hp, arrived: t.arrived, destroyed: t.destroyed,
                });
            }
            out
        };

        let visible_enemies = {
            let mut out: Vec<EnemyInfo> = Vec::new();
            {
                let mut q = self.world.query::<(&StraitLauncher, &StraitPos)>();
                for (launcher, pos) in q.iter(&self.world) {
                    if launcher.phase != LauncherPhase::Hidden {
                        out.push(EnemyInfo {
                            kind: if launcher.is_decoy { "decoy" } else { "launcher" }.to_string(),
                            x: pos.x, y: pos.y,
                        });
                    }
                }
            }
            {
                let mut q = self.world.query::<(&StraitAaDrone, &StraitPos)>();
                for (aa, pos) in q.iter(&self.world) {
                    if aa.alive {
                        out.push(EnemyInfo { kind: "aa".to_string(), x: pos.x, y: pos.y });
                    }
                }
            }
            {
                let mut q = self.world.query::<(&StraitSoldier, &StraitPos)>();
                for (_, pos) in q.iter(&self.world) {
                    out.push(EnemyInfo { kind: "soldier".to_string(), x: pos.x, y: pos.y });
                }
            }
            out
        };

        let incoming_shaheeds = {
            let mut out = Vec::new();
            let mut q = self.world.query::<(&StraitShaheed, &StraitPos)>();
            for (s, pos) in q.iter(&self.world) {
                out.push(ShaheedInfo {
                    entity_id: s.entity_id, x: pos.x, y: pos.y,
                    target: match s.target {
                        ShaheedTarget::Base => "base".to_string(),
                        ShaheedTarget::Ship(idx) => format!("ship_{}", idx),
                    },
                });
            }
            out
        };

        let incoming_missiles = {
            let mut out = Vec::new();
            let mut q = self.world.query::<(&StraitMissile, &StraitPos)>();
            for (m, pos) in q.iter(&self.world) {
                if let MissileState::InFlight { target_x, target_y, progress, .. } = m.state {
                    out.push(MissileInfo { x: pos.x, y: pos.y, target_x, target_y, progress });
                }
            }
            out
        };

        StraitSnapshot {
            allocation,
            satellite_focal,
            airstrike_charges,
            airstrike_max_charges: self.config.airstrike_max_charges,
            drone_rebuild_charges,
            drone_rebuild_max_charges: self.config.drone_rebuild_max_charges,
            patriot_count,
            patriot_mode,
            base_hp,
            convoy_hold,
            tankers_arrived,
            tankers_destroyed,
            tankers_spawned,
            total_tankers: self.config.total_tankers,
            min_tankers_win: self.config.min_tankers_win,
            max_tankers_lost: self.config.max_tankers_lost,
            drones_alive,
            drone_positions,
            zero_day_slot,
            mission_tick,
            mission_complete,
            tanker_positions,
            visible_enemies,
            incoming_shaheeds,
            incoming_missiles,
        }
    }

    /// Run a Lua script against the current state and apply commands.
    pub fn run_script(&mut self, lua_source: &str) -> Result<(), String> {
        let snapshot = self.snapshot();

        // Build a minimal GameStateSnapshot (strait doesn't use standard units)
        let empty_snapshot = cc_agent::snapshot::GameStateSnapshot {
            tick: self.state().mission_tick,
            map_width: self.config.map_width,
            map_height: self.config.map_height,
            player_id: 0,
            my_units: Vec::new(),
            enemy_units: Vec::new(),
            my_buildings: Vec::new(),
            enemy_buildings: Vec::new(),
            resource_deposits: Vec::new(),
            my_resources: cc_sim::resources::PlayerResourceState::default(),
        };

        // Need a GameMap for ScriptContext (strait uses a simple flat map)
        let map = cc_core::map::GameMap::new(
            self.config.map_width,
            self.config.map_height,
        );

        let mut ctx = cc_agent::script_context::ScriptContext::new(
            &empty_snapshot,
            &map,
            0,
            cc_core::terrain::FactionId::CatGPT,
        )
        .with_strait_snapshot(snapshot);

        match cc_agent::lua_runtime::execute_script_with_context_tiered(
            lua_source,
            &mut ctx,
            cc_agent::tool_tier::ToolTier::Advanced,
        ) {
            Ok(_) => {
                // Apply strait commands
                for cmd in std::mem::take(&mut ctx.strait_commands) {
                    self.apply_command(cmd);
                }
                Ok(())
            }
            Err(e) => Err(format!("{}", e)),
        }
    }

    /// Apply a single strait command to the world.
    fn apply_command(&mut self, cmd: StraitCommand) {
        // Commands that modify drone entities need exclusive world access,
        // so handle them separately to avoid double-borrow with StraitState.
        match &cmd {
            StraitCommand::SetPatrol { drone_id, waypoints } => {
                let wps: Vec<cc_core::coords::GridPos> = waypoints
                    .iter()
                    .map(|&(x, y)| cc_core::coords::GridPos::new(x, y))
                    .collect();
                let did = *drone_id;
                let mut q = self.world.query::<&mut StraitDrone>();
                for mut drone in q.iter_mut(&mut self.world) {
                    if drone.drone_id == did && drone.alive {
                        drone.patrol_waypoints = wps;
                        drone.current_wp_index = 0;
                        drone.mode = DroneMode::Patrol;
                        break;
                    }
                }
                return;
            }
            StraitCommand::DroneBomb { drone_id, target_x, target_y } => {
                let (did, tx, ty) = (*drone_id, *target_x as f32, *target_y as f32);
                let mut q = self.world.query::<&mut StraitDrone>();
                for mut drone in q.iter_mut(&mut self.world) {
                    if drone.drone_id == did && drone.alive {
                        drone.mode = DroneMode::BombTarget { x: tx, y: ty };
                        break;
                    }
                }
                return;
            }
            StraitCommand::DroneGuardBase { drone_id } => {
                let did = *drone_id;
                let mut q = self.world.query::<&mut StraitDrone>();
                for mut drone in q.iter_mut(&mut self.world) {
                    if drone.drone_id == did && drone.alive {
                        drone.mode = DroneMode::GuardBase;
                        break;
                    }
                }
                return;
            }
            StraitCommand::DroneMoveTo { drone_id, x, y } => {
                let (did, tx, ty) = (*drone_id, *x as f32, *y as f32);
                let mut q = self.world.query::<&mut StraitDrone>();
                for mut drone in q.iter_mut(&mut self.world) {
                    if drone.drone_id == did && drone.alive {
                        drone.mode = DroneMode::MoveTo { x: tx, y: ty };
                        break;
                    }
                }
                return;
            }
            _ => {}
        }

        // State-only commands
        let mut state = self.world.resource_mut::<StraitState>();
        match cmd {
            StraitCommand::SetSatelliteFocal { x, y } => {
                state.satellite_focal = Some((x, y));
            }
            StraitCommand::AllocateCompute(alloc) => {
                state.allocation = alloc;
            }
            StraitCommand::BuildZeroDay(zd_type) => {
                if matches!(state.zero_day_slot, ZeroDayState::Idle) {
                    state.zero_day_slot = ZeroDayState::Building {
                        exploit_type: zd_type,
                        progress: 0.0,
                        required: self.config.zero_day_build_ticks(zd_type),
                    };
                }
            }
            StraitCommand::DeployZeroDay { exploit_type, .. } => {
                if matches!(state.zero_day_slot, ZeroDayState::Ready(_)) {
                    state.zero_day_slot = ZeroDayState::Idle;
                    let idx = match exploit_type {
                        ZeroDayType::Spoof => 0, ZeroDayType::Blind => 1,
                        ZeroDayType::Hijack => 2, ZeroDayType::Brick => 3,
                    };
                    state.zero_days_deployed[idx] = true;
                }
            }
            StraitCommand::CallAirstrike { x, y } => {
                if state.airstrike_charges > 0 {
                    state.airstrike_charges -= 1;
                    state.pending_airstrikes.push((x, y, self.config.airstrike_delay_ticks));
                }
            }
            StraitCommand::RebuildDrone => {
                if !state.drone_rebuilding && state.drone_rebuild_charges > 0 {
                    state.drone_rebuild_charges -= 1;
                    state.drone_rebuilding = true;
                    state.drone_rebuild_timer = self.config.drone_rebuild_ticks;
                }
            }
            StraitCommand::LaunchAllBoats => {
                state.convoy_hold = false;
            }
            StraitCommand::SetPatriotMode { missiles_only } => {
                state.patriot_mode = if missiles_only { PatriotMode::MissilesOnly } else { PatriotMode::Auto };
            }
            // Already handled above
            _ => {}
        }
    }

    /// Print a status summary line.
    pub fn status_line(&self) -> String {
        let s = self.state();
        format!(
            "T{:>5} | D:{:>2} | P:{:>2} | B:{:>2}/{} | S:{}/{} A:{}/{} | boats: {}/{}",
            s.mission_tick,
            s.drones_alive,
            s.patriot_count,
            s.base_hp, self.config.base_hp,
            s.tankers_arrived, self.config.total_tankers,
            s.tankers_destroyed, self.config.max_tankers_lost,
            if s.convoy_hold { "HELD" } else { "RUNNING" },
            s.tankers_spawned,
        )
    }
}
