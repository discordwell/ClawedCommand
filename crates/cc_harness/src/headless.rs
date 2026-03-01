//! Headless simulation wrapper for MCP server and testing.
//! Wraps a Bevy World + Schedule without any rendering.

use bevy::prelude::*;

use cc_core::building_stats::building_stats;
use cc_core::commands::{EntityId, GameCommand};
use cc_core::components::*;
use cc_core::coords::{GridPos, WorldPos};
use cc_core::map::GameMap;
use cc_core::unit_stats::base_stats;

use cc_sim::resources::*;
use cc_sim::systems::{
    ability_system, aura_system, cleanup_system, combat_system, command_system,
    grid_sync_system, movement_system, production_system, projectile_system,
    research_system, resource_system, stat_modifier_system, status_effect_system,
    target_acquisition_system, tick_system, tower_combat_system, victory_system,
};

use cc_agent::script_context::ScriptContext;
use cc_agent::snapshot::GameStateSnapshot;
use cc_agent::lua_runtime;

/// Headless simulation — no rendering, no AI decision system.
/// Script control replaces bot AI.
pub struct HeadlessSim {
    world: World,
    schedule: Schedule,
}

impl HeadlessSim {
    /// Create a new headless sim with a flat grass map.
    pub fn new(width: u32, height: u32) -> Self {
        let map = GameMap::new(width, height);
        let (world, schedule) = Self::build_world_and_schedule(map);
        Self { world, schedule }
    }

    fn build_world_and_schedule(map: GameMap) -> (World, Schedule) {
        let mut world = World::new();

        world.insert_resource(CommandQueue::default());
        world.insert_resource(SimClock::default());
        world.insert_resource(ControlGroups::default());
        world.insert_resource(GameState::Playing);
        world.insert_resource(CombatStats::default());
        world.insert_resource(SimRng::default());
        world.insert_resource(SpawnPositions::default());

        let mut player_res = PlayerResources::default();
        while player_res.players.len() < 2 {
            player_res.players.push(Default::default());
        }
        world.insert_resource(player_res);
        world.insert_resource(MapResource { map });
        world.init_resource::<bevy::prelude::Messages<cc_sim::systems::projectile_system::ProjectileHit>>();

        let mut schedule = Schedule::new(FixedUpdate);
        schedule.add_systems(
            (
                tick_system::tick_system,
                // No multi_ai_decision_system — script control replaces bot AI
                command_system::process_commands,
                ability_system::ability_cooldown_system,
                status_effect_system::status_effect_system,
                aura_system::aura_system,
                stat_modifier_system::stat_modifier_system,
                production_system::production_system,
                research_system::research_system,
                resource_system::gathering_system,
                target_acquisition_system::target_acquisition_system,
                combat_system::combat_system,
                tower_combat_system::tower_combat_system,
                projectile_system::projectile_system,
                movement_system::movement_system,
                grid_sync_system::grid_sync_system,
                cleanup_system::cleanup_system,
                headless_despawn_system,
            )
                .chain(),
        );
        schedule.add_systems(victory_system::victory_system.after(headless_despawn_system));

        (world, schedule)
    }

    /// Spawn a unit at a grid position and return its entity bits as u64.
    pub fn spawn_unit(&mut self, kind: UnitKind, pos: GridPos, player_id: u8) -> u64 {
        let stats = base_stats(kind);
        let entity = self
            .world
            .spawn((
                Position {
                    world: WorldPos::from_grid(pos),
                },
                Velocity::zero(),
                GridCell { pos },
                Owner { player_id },
                UnitType { kind },
                Health {
                    current: stats.health,
                    max: stats.health,
                },
                MovementSpeed { speed: stats.speed },
                AttackStats {
                    damage: stats.damage,
                    range: stats.range,
                    attack_speed: stats.attack_speed,
                    cooldown_remaining: 0,
                },
                AttackTypeMarker {
                    attack_type: stats.attack_type,
                },
            ))
            .id();

        // Track supply
        if let Some(pres) = self
            .world
            .resource_mut::<PlayerResources>()
            .players
            .get_mut(player_id as usize)
        {
            pres.supply += stats.supply_cost;
        }

        entity.to_bits()
    }

    /// Spawn a building at a grid position and return its entity bits as u64.
    pub fn spawn_building(&mut self, kind: BuildingKind, pos: GridPos, player_id: u8) -> u64 {
        let stats = building_stats(kind);
        let entity = self
            .world
            .spawn((
                Position {
                    world: WorldPos::from_grid(pos),
                },
                GridCell { pos },
                Owner { player_id },
                Building { kind },
                Health {
                    current: stats.health,
                    max: stats.health,
                },
                Producer,
                ProductionQueue::default(),
            ))
            .id();

        // Grant supply cap
        if let Some(pres) = self
            .world
            .resource_mut::<PlayerResources>()
            .players
            .get_mut(player_id as usize)
        {
            pres.supply_cap += stats.supply_provided;
        }

        entity.to_bits()
    }

    /// Spawn a resource deposit at a grid position and return its entity bits as u64.
    pub fn spawn_deposit(
        &mut self,
        resource_type: ResourceType,
        pos: GridPos,
        amount: u32,
    ) -> u64 {
        let entity = self
            .world
            .spawn((
                Position {
                    world: WorldPos::from_grid(pos),
                },
                GridCell { pos },
                ResourceDeposit {
                    resource_type,
                    remaining: amount,
                },
            ))
            .id();
        entity.to_bits()
    }

    /// Advance the simulation by N ticks.
    pub fn advance(&mut self, n_ticks: u32) {
        for _ in 0..n_ticks {
            self.schedule.run(&mut self.world);
        }
    }

    /// Push a command into the queue for the next tick.
    pub fn inject_command(&mut self, cmd: GameCommand) {
        self.world.resource_mut::<CommandQueue>().push(cmd);
    }

    /// Build a game state snapshot for a given player.
    pub fn snapshot(&mut self, player_id: u8) -> GameStateSnapshot {
        use cc_core::math::Fixed;
        use cc_core::abilities::unit_abilities;
        use cc_core::status_effects::StatusEffects;
        use cc_agent::snapshot::{
            UnitSnapshot, BuildingSnapshot, ResourceSnapshot,
            StatusEffectSnapshot, AbilitySnapshot,
        };

        // Clone resources upfront to release borrows before queries
        let (width, height) = {
            let map_res = self.world.resource::<MapResource>();
            (map_res.map.width, map_res.map.height)
        };
        let tick = self.world.resource::<SimClock>().tick;
        let player_resources_clone = self.world.resource::<PlayerResources>().players.clone();

        // Query units — build snapshots inline so references don't outlive the query
        let mut my_units = Vec::new();
        let mut enemy_units = Vec::new();
        {
            let mut query = self.world.query::<(
                Entity, &Position, &Owner, &UnitType, &Health, &MovementSpeed,
                Option<&AttackStats>, Option<&AttackTypeMarker>,
                Option<&MoveTarget>, Option<&AttackTarget>, Option<&Path>,
                Option<&Gathering>,
                (
                    Option<&ChasingTarget>,
                    Option<&AttackMoveTarget>, Option<&Dead>,
                    Option<&StatusEffects>, Option<&AbilitySlots>,
                ),
            )>();
            for (entity, pos, owner, unit_type, health, speed,
                 attack_stats, attack_type_marker,
                 move_target, attack_target, path,
                 gathering,
                 (chasing, attack_move, dead, status_effects, ability_slots))
                in query.iter(&self.world)
            {
                let is_moving = move_target.is_some() || path.is_some() || chasing.is_some();
                let is_attacking = attack_target.is_some() || attack_move.is_some();
                let is_dead = dead.is_some();
                let is_idle = !is_moving && !is_attacking && !is_dead && gathering.is_none();

                let (atk_damage, atk_range, atk_speed) = attack_stats
                    .map(|s| (s.damage, s.range, s.attack_speed))
                    .unwrap_or((Fixed::ZERO, Fixed::ZERO, 0));

                let atk_type = attack_type_marker
                    .map(|m| m.attack_type)
                    .unwrap_or(AttackType::Melee);

                let se_snaps: Vec<StatusEffectSnapshot> = status_effects
                    .map(|se| {
                        se.effects
                            .iter()
                            .filter(|e| e.remaining_ticks > 0)
                            .map(|e| StatusEffectSnapshot {
                                effect_type: format!("{:?}", e.effect),
                                remaining_ticks: e.remaining_ticks,
                                stacks: e.stacks,
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let ability_snaps: Vec<AbilitySnapshot> = ability_slots
                    .map(|slots| {
                        slots.slots.iter().enumerate().map(|(i, state)| {
                            AbilitySnapshot {
                                slot: i as u8,
                                id: format!("{:?}", state.id),
                                cooldown_remaining: state.cooldown_remaining,
                                ready: state.cooldown_remaining == 0,
                            }
                        }).collect()
                    })
                    .unwrap_or_else(|| {
                        let ids = unit_abilities(unit_type.kind);
                        ids.iter().enumerate().map(|(i, id)| {
                            AbilitySnapshot {
                                slot: i as u8,
                                id: format!("{:?}", id),
                                cooldown_remaining: 0,
                                ready: true,
                            }
                        }).collect()
                    });

                let snap = UnitSnapshot {
                    id: EntityId(entity.to_bits()),
                    kind: unit_type.kind,
                    pos: pos.world.to_grid(),
                    world_pos: pos.world,
                    owner: owner.player_id,
                    health_current: health.current,
                    health_max: health.max,
                    speed: speed.speed,
                    attack_damage: atk_damage,
                    attack_range: atk_range,
                    attack_speed: atk_speed,
                    attack_type: atk_type,
                    is_moving,
                    is_attacking,
                    is_idle,
                    is_dead,
                    is_gathering: gathering.is_some(),
                    status_effects: se_snaps,
                    abilities: ability_snaps,
                };

                if owner.player_id == player_id {
                    my_units.push(snap);
                } else {
                    enemy_units.push(snap);
                }
            }
        }

        // Query buildings
        let mut my_buildings = Vec::new();
        let mut enemy_buildings = Vec::new();
        {
            let mut query = self.world.query::<(
                Entity, &Position, &Owner, &Building, &Health,
                Option<&UnderConstruction>, Option<&ProductionQueue>,
                Option<&ResearchQueue>,
            )>();
            for (entity, pos, owner, building, health, under_construction, production_queue, research_queue)
                in query.iter(&self.world)
            {
                let (is_constructing, progress) = under_construction
                    .map(|uc| {
                        let total = uc.total_ticks as f32;
                        let remaining = uc.remaining_ticks as f32;
                        (true, if total > 0.0 { 1.0 - remaining / total } else { 1.0 })
                    })
                    .unwrap_or((false, 1.0));

                let queue = production_queue
                    .map(|pq| pq.queue.iter().map(|(kind, _)| *kind).collect())
                    .unwrap_or_default();

                let rq: Vec<String> = research_queue
                    .map(|rq| rq.queue.iter().map(|(upgrade, _)| format!("{}", upgrade)).collect())
                    .unwrap_or_default();

                let snap = BuildingSnapshot {
                    id: EntityId(entity.to_bits()),
                    kind: building.kind,
                    pos: pos.world.to_grid(),
                    owner: owner.player_id,
                    health_current: health.current,
                    health_max: health.max,
                    under_construction: is_constructing,
                    construction_progress: progress,
                    production_queue: queue,
                    research_queue: rq,
                };

                if owner.player_id == player_id {
                    my_buildings.push(snap);
                } else {
                    enemy_buildings.push(snap);
                }
            }
        }

        // Query deposits
        let resource_deposits: Vec<ResourceSnapshot> = {
            let mut query = self.world.query::<(Entity, &Position, &ResourceDeposit)>();
            query.iter(&self.world)
                .map(|(entity, pos, deposit)| ResourceSnapshot {
                    id: EntityId(entity.to_bits()),
                    resource_type: deposit.resource_type,
                    pos: pos.world.to_grid(),
                    remaining: deposit.remaining,
                })
                .collect()
        };

        let my_resources = player_resources_clone
            .get(player_id as usize)
            .cloned()
            .unwrap_or_default();

        GameStateSnapshot {
            tick,
            map_width: width,
            map_height: height,
            player_id,
            my_units,
            enemy_units,
            my_buildings,
            enemy_buildings,
            resource_deposits,
            my_resources,
        }
    }

    /// Execute a Lua script against the current state for a given player.
    /// Returns the commands produced by the script.
    pub fn run_script(
        &mut self,
        player_id: u8,
        lua_source: &str,
    ) -> Result<Vec<GameCommand>, String> {
        let snap = self.snapshot(player_id);
        let map_res = self.world.resource::<MapResource>();
        let map = &map_res.map;

        let mut ctx = ScriptContext::new(
            &snap,
            map,
            player_id,
            cc_core::terrain::FactionId::from_u8(player_id).unwrap_or(cc_core::terrain::FactionId::CatGPT),
        );

        lua_runtime::execute_script_with_context(lua_source, &mut ctx)
            .map_err(|e| e.to_string())
    }

    /// Get the current simulation tick.
    pub fn tick(&self) -> u64 {
        self.world.resource::<SimClock>().tick
    }

    /// Get the current game state (Playing or Victory).
    pub fn game_state(&self) -> GameState {
        *self.world.resource::<GameState>()
    }

    /// Reset the simulation with a new map.
    pub fn reset(&mut self, width: u32, height: u32) {
        let map = GameMap::new(width, height);
        let (world, schedule) = Self::build_world_and_schedule(map);
        self.world = world;
        self.schedule = schedule;
    }

    /// Get map dimensions.
    pub fn map_size(&self) -> (u32, u32) {
        let map_res = self.world.resource::<MapResource>();
        (map_res.map.width, map_res.map.height)
    }

    /// Get player resources.
    pub fn player_resources(&self, player_id: u8) -> PlayerResourceState {
        self.world
            .resource::<PlayerResources>()
            .players
            .get(player_id as usize)
            .cloned()
            .unwrap_or_default()
    }

    /// Borrow the simulation's actual GameMap.
    pub fn map(&self) -> &GameMap {
        &self.world.resource::<MapResource>().map
    }
}

/// Headless despawn: in headless mode there's no client death_fade_system,
/// so we despawn Dead entities immediately after cleanup marks them.
fn headless_despawn_system(mut commands: Commands, dead: Query<Entity, With<Dead>>) {
    for entity in dead.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_unit_appears_in_snapshot() {
        let mut sim = HeadlessSim::new(32, 32);
        let entity_bits = sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        assert_eq!(snap.my_units[0].id, EntityId(entity_bits));
        assert_eq!(snap.my_units[0].kind, UnitKind::Hisser);
    }

    #[test]
    fn advance_increments_clock() {
        let mut sim = HeadlessSim::new(32, 32);
        assert_eq!(sim.tick(), 0);

        sim.advance(5);
        assert_eq!(sim.tick(), 5);

        sim.advance(3);
        assert_eq!(sim.tick(), 8);
    }

    #[test]
    fn inject_command_moves_unit() {
        let mut sim = HeadlessSim::new(32, 32);
        let entity_bits = sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        // Inject a move command
        sim.inject_command(GameCommand::Move {
            unit_ids: vec![EntityId(entity_bits)],
            target: GridPos::new(10, 10),
        });

        // Advance enough ticks for movement to start
        sim.advance(1);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        // Unit should have started moving (or already moved)
        let unit = &snap.my_units[0];
        assert!(unit.is_moving || unit.pos != GridPos::new(5, 5));
    }

    #[test]
    fn run_script_produces_commands() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        let script = r#"
            local units = ctx:my_units()
            if #units > 0 then
                ctx:move_units({units[1].id}, 10, 10)
            end
        "#;

        let cmds = sim.run_script(0, script).unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], GameCommand::Move { .. }));
    }

    #[test]
    fn snapshot_separates_players() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);
        sim.spawn_unit(UnitKind::Chonk, GridPos::new(20, 20), 1);

        // Player 0's perspective
        let snap0 = sim.snapshot(0);
        assert_eq!(snap0.my_units.len(), 1);
        assert_eq!(snap0.enemy_units.len(), 1);
        assert_eq!(snap0.my_units[0].kind, UnitKind::Hisser);
        assert_eq!(snap0.enemy_units[0].kind, UnitKind::Chonk);

        // Player 1's perspective
        let snap1 = sim.snapshot(1);
        assert_eq!(snap1.my_units.len(), 1);
        assert_eq!(snap1.enemy_units.len(), 1);
        assert_eq!(snap1.my_units[0].kind, UnitKind::Chonk);
        assert_eq!(snap1.enemy_units[0].kind, UnitKind::Hisser);
    }

    #[test]
    fn cancel_queue_produces_correct_command() {
        let mut sim = HeadlessSim::new(32, 32);
        let building_bits = sim.spawn_building(BuildingKind::CatTree, GridPos::new(10, 10), 0);

        // Inject a CancelQueue command
        sim.inject_command(GameCommand::CancelQueue {
            building: EntityId(building_bits),
        });

        // Verify the command was placed in the queue
        let queue = sim.world.resource::<CommandQueue>();
        assert_eq!(queue.commands.len(), 1);
        assert!(matches!(
            &queue.commands[0].command,
            GameCommand::CancelQueue { building } if building.0 == building_bits
        ));
    }

    #[test]
    fn cancel_research_produces_correct_command() {
        let mut sim = HeadlessSim::new(32, 32);
        let building_bits = sim.spawn_building(BuildingKind::ScratchingPost, GridPos::new(10, 10), 0);

        sim.inject_command(GameCommand::CancelResearch {
            building: EntityId(building_bits),
        });

        let queue = sim.world.resource::<CommandQueue>();
        assert_eq!(queue.commands.len(), 1);
        assert!(matches!(
            &queue.commands[0].command,
            GameCommand::CancelResearch { building } if building.0 == building_bits
        ));
    }

    #[test]
    fn set_rally_point_produces_correct_command() {
        let mut sim = HeadlessSim::new(32, 32);
        let building_bits = sim.spawn_building(BuildingKind::CatTree, GridPos::new(10, 10), 0);

        sim.inject_command(GameCommand::SetRallyPoint {
            building: EntityId(building_bits),
            target: GridPos::new(15, 20),
        });

        let queue = sim.world.resource::<CommandQueue>();
        assert_eq!(queue.commands.len(), 1);
        match &queue.commands[0].command {
            GameCommand::SetRallyPoint { building, target } => {
                assert_eq!(building.0, building_bits);
                assert_eq!(*target, GridPos::new(15, 20));
            }
            other => panic!("Expected SetRallyPoint, got {:?}", other),
        }
    }

    #[test]
    fn get_resource_deposits_returns_deposits() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_deposit(ResourceType::Food, GridPos::new(5, 5), 500);
        sim.spawn_deposit(ResourceType::GpuCores, GridPos::new(10, 10), 300);
        sim.spawn_deposit(ResourceType::Nft, GridPos::new(20, 20), 100);

        let snap = sim.snapshot(0);
        let map = sim.map();
        let mut ctx = ScriptContext::new(
            &snap, map, 0,
            cc_core::terrain::FactionId::from_u8(0).unwrap_or(cc_core::terrain::FactionId::CatGPT),
        );
        let deposits = ctx.resource_deposits();
        assert_eq!(deposits.len(), 3);

        // Verify all deposit types are present
        let types: Vec<ResourceType> = deposits.iter().map(|d| d.resource_type).collect();
        assert!(types.contains(&ResourceType::Food));
        assert!(types.contains(&ResourceType::GpuCores));
        assert!(types.contains(&ResourceType::Nft));
    }

    #[test]
    fn get_nearest_deposit_finds_closest() {
        let mut sim = HeadlessSim::new(32, 32);
        // Spawn two food deposits at different distances from (0,0)
        sim.spawn_deposit(ResourceType::Food, GridPos::new(10, 10), 500);
        sim.spawn_deposit(ResourceType::Food, GridPos::new(3, 3), 200);
        sim.spawn_deposit(ResourceType::GpuCores, GridPos::new(1, 1), 100);

        let snap = sim.snapshot(0);
        let map = sim.map();
        let mut ctx = ScriptContext::new(
            &snap, map, 0,
            cc_core::terrain::FactionId::from_u8(0).unwrap_or(cc_core::terrain::FactionId::CatGPT),
        );

        // Nearest Food deposit to (0,0) should be the one at (3,3)
        let nearest = ctx.nearest_deposit(GridPos::new(0, 0), Some(ResourceType::Food));
        assert!(nearest.is_some());
        let nearest = nearest.unwrap();
        assert_eq!(nearest.resource_type, ResourceType::Food);
        assert_eq!(nearest.pos, GridPos::new(3, 3));
        assert_eq!(nearest.remaining, 200);
    }

    #[test]
    fn get_nearest_deposit_returns_none_for_missing_type() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_deposit(ResourceType::Food, GridPos::new(5, 5), 500);

        let snap = sim.snapshot(0);
        let map = sim.map();
        let mut ctx = ScriptContext::new(
            &snap, map, 0,
            cc_core::terrain::FactionId::from_u8(0).unwrap_or(cc_core::terrain::FactionId::CatGPT),
        );

        // No GpuCores deposits exist
        let nearest = ctx.nearest_deposit(GridPos::new(0, 0), Some(ResourceType::GpuCores));
        assert!(nearest.is_none());
    }

    #[test]
    fn snapshot_captures_status_effects_from_ecs() {
        use cc_core::status_effects::{StatusEffects, StatusInstance, StatusEffectId};

        let mut sim = HeadlessSim::new(32, 32);
        let entity_bits = sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        // Manually add StatusEffects component to the entity
        let entity = Entity::from_bits(entity_bits);
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Corroded,
            remaining_ticks: 20,
            stacks: 3,
            source: EntityId(0),
        });
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Stunned,
            remaining_ticks: 5,
            stacks: 1,
            source: EntityId(0),
        });
        // Also add an expired effect that should be filtered out
        se.effects.push(StatusInstance {
            effect: StatusEffectId::Zoomies,
            remaining_ticks: 0,
            stacks: 1,
            source: EntityId(0),
        });
        sim.world.entity_mut(entity).insert(se);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        let unit = &snap.my_units[0];
        // Only 2 active effects (Zoomies expired with 0 ticks)
        assert_eq!(unit.status_effects.len(), 2);
        assert_eq!(unit.status_effects[0].effect_type, "Corroded");
        assert_eq!(unit.status_effects[0].remaining_ticks, 20);
        assert_eq!(unit.status_effects[0].stacks, 3);
        assert_eq!(unit.status_effects[1].effect_type, "Stunned");
    }

    #[test]
    fn snapshot_empty_status_effects_without_component() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        // No StatusEffects component => empty vec
        assert!(snap.my_units[0].status_effects.is_empty());
    }

    #[test]
    fn snapshot_captures_ability_slots_from_ecs() {
        let mut sim = HeadlessSim::new(32, 32);
        let entity_bits = sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        // Add AbilitySlots component
        let entity = Entity::from_bits(entity_bits);
        let mut ability_slots = AbilitySlots::from_abilities(
            cc_core::abilities::unit_abilities(UnitKind::Hisser),
        );
        // Put slot 1 on cooldown
        ability_slots.slots[1].cooldown_remaining = 25;
        sim.world.entity_mut(entity).insert(ability_slots);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        let unit = &snap.my_units[0];
        assert_eq!(unit.abilities.len(), 3);
        // Slot 0: ready
        assert_eq!(unit.abilities[0].slot, 0);
        assert_eq!(unit.abilities[0].id, "CorrosiveSpit");
        assert!(unit.abilities[0].ready);
        assert_eq!(unit.abilities[0].cooldown_remaining, 0);
        // Slot 1: on cooldown
        assert_eq!(unit.abilities[1].slot, 1);
        assert_eq!(unit.abilities[1].id, "DisgustMortar");
        assert!(!unit.abilities[1].ready);
        assert_eq!(unit.abilities[1].cooldown_remaining, 25);
        // Slot 2: ready
        assert_eq!(unit.abilities[2].slot, 2);
        assert!(unit.abilities[2].ready);
    }

    #[test]
    fn snapshot_fallback_abilities_without_component() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_unit(UnitKind::Hisser, GridPos::new(5, 5), 0);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_units.len(), 1);
        let unit = &snap.my_units[0];
        // Without AbilitySlots component, falls back to unit_abilities lookup
        assert_eq!(unit.abilities.len(), 3);
        assert_eq!(unit.abilities[0].id, "CorrosiveSpit");
        assert_eq!(unit.abilities[1].id, "DisgustMortar");
        assert_eq!(unit.abilities[2].id, "Misinformation");
        // All should be ready (default)
        assert!(unit.abilities.iter().all(|a| a.ready));
    }

    #[test]
    fn snapshot_captures_research_queue_from_ecs() {
        let mut sim = HeadlessSim::new(32, 32);
        let building_bits = sim.spawn_building(BuildingKind::ScratchingPost, GridPos::new(10, 10), 0);

        // Add ResearchQueue to the building
        let entity = Entity::from_bits(building_bits);
        let mut rq = ResearchQueue::default();
        rq.queue.push_back((UpgradeType::SharperClaws, 100));
        rq.queue.push_back((UpgradeType::ThickerFur, 150));
        sim.world.entity_mut(entity).insert(rq);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_buildings.len(), 1);
        let bld = &snap.my_buildings[0];
        assert_eq!(bld.research_queue.len(), 2);
        assert_eq!(bld.research_queue[0], "SharperClaws");
        assert_eq!(bld.research_queue[1], "ThickerFur");
    }

    #[test]
    fn snapshot_empty_research_queue_without_component() {
        let mut sim = HeadlessSim::new(32, 32);
        sim.spawn_building(BuildingKind::ScratchingPost, GridPos::new(10, 10), 0);

        let snap = sim.snapshot(0);
        assert_eq!(snap.my_buildings.len(), 1);
        assert!(snap.my_buildings[0].research_queue.is_empty());
    }

    #[test]
    fn get_unit_details_via_snapshot() {
        use cc_core::status_effects::{StatusEffects, StatusInstance, StatusEffectId};

        let mut sim = HeadlessSim::new(32, 32);
        let entity_bits = sim.spawn_unit(UnitKind::Chonk, GridPos::new(8, 8), 0);

        // Add status effects and ability slots
        let entity = Entity::from_bits(entity_bits);
        let mut se = StatusEffects::default();
        se.effects.push(StatusInstance {
            effect: StatusEffectId::LoafModeActive,
            remaining_ticks: 50,
            stacks: 1,
            source: EntityId(0),
        });
        let ability_slots = AbilitySlots::from_abilities(
            cc_core::abilities::unit_abilities(UnitKind::Chonk),
        );
        sim.world.entity_mut(entity).insert((se, ability_slots));

        let snap = sim.snapshot(0);
        let unit = snap.unit_by_id(EntityId(entity_bits)).unwrap();
        assert_eq!(unit.kind, UnitKind::Chonk);
        assert_eq!(unit.status_effects.len(), 1);
        assert_eq!(unit.status_effects[0].effect_type, "LoafModeActive");
        assert_eq!(unit.abilities.len(), 3);
        assert_eq!(unit.abilities[0].id, "GravitationalChonk");
    }

    #[test]
    fn completed_upgrades_accessible_via_player_resources() {
        let mut sim = HeadlessSim::new(32, 32);

        // Add a completed upgrade
        sim.world
            .resource_mut::<PlayerResources>()
            .players[0]
            .completed_upgrades
            .insert(UpgradeType::SharperClaws);

        let res = sim.player_resources(0);
        assert!(res.completed_upgrades.contains(&UpgradeType::SharperClaws));
        assert!(!res.completed_upgrades.contains(&UpgradeType::ThickerFur));
    }
}
