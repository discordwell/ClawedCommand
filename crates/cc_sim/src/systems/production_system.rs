use bevy::prelude::*;
use std::collections::VecDeque;

use crate::pathfinding;
use crate::resources::{MapResource, PlayerResources};
use cc_core::abilities::unit_abilities;
use cc_core::building_stats::building_stats;
use cc_core::commands::EntityId;
use cc_core::components::{
    AbilitySlots, Aerial, AttackStats, AttackType, AttackTypeMarker, Aura, AuraType,
    BloodgreedTracker, BogPatchCounter, Building, BuildingKind, ContagionCloudOnDeath,
    CorrodedApplicator, DreamSiegeTimer, FeignDeathTracker, FrankensteinTracker, FrenzyStacks,
    GatherState, Gathering, GridCell, Health, HeavyUnit, JunkLauncherState, LimbTracker,
    MoveTarget, MovementSpeed, NineLivesTracker, Owner, PanopticGazeCone, Path,
    PocketStashInventory, Position, Producer, ProductionQueue, RallyPoint, ResearchQueue,
    Researcher, ResourceDeposit, ResourceType, SpawnlingCounter, StatModifiers, StaticChargeStacks,
    StationaryTimer, Stealth, StructuralWeaknessTimer, TrinketWardTracker, UnderConstruction,
    UniqueBuildingLimit, UnitKind, UnitType, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::math::Fixed;
use cc_core::status_effects::StatusEffects;
use cc_core::terrain::FactionId;
use cc_core::tuning;
use cc_core::unit_stats::base_stats;

use crate::systems::research_system::apply_upgrades_to_new_unit;

/// Ticks UnderConstruction, ticks ProductionQueue, spawns units on completion.
pub fn production_system(
    mut commands: Commands,
    map_res: Res<MapResource>,
    mut buildings: Query<(
        Entity,
        &Building,
        &Owner,
        &Position,
        Option<&mut UnderConstruction>,
        Option<&mut ProductionQueue>,
        Option<&RallyPoint>,
        &mut Health,
    )>,
    _player_resources: ResMut<PlayerResources>,
    deposits: Query<(Entity, &Position, &ResourceDeposit), Without<Building>>,
) {
    for (entity, building, owner, pos, under_construction, prod_queue, rally, mut health) in
        buildings.iter_mut()
    {
        // Phase 1: Tick construction countdown
        if let Some(mut uc) = under_construction {
            if uc.remaining_ticks > 0 {
                uc.remaining_ticks -= 1;
            }

            // Scale HP proportionally to construction progress (10% to 100%)
            // Use min() so combat damage is preserved — never heal above the formula value
            if uc.remaining_ticks > 0 && uc.total_ticks > 0 {
                let progress = Fixed::from_num(uc.progress_f32());
                let formula_hp =
                    health.max * (Fixed::from_num(0.1f32) + Fixed::from_num(0.9f32) * progress);
                if health.current < formula_hp {
                    health.current = formula_hp;
                }
            }

            if uc.remaining_ticks == 0 {
                // Full HP on completion
                health.current = health.max;
                // Construction complete - promote to producer if applicable
                commands.entity(entity).remove::<UnderConstruction>();
                let bstats = building_stats(building.kind);
                if !bstats.can_produce.is_empty() {
                    commands
                        .entity(entity)
                        .insert((Producer, ProductionQueue::default()));
                }

                // ScratchingPost gets Researcher + ResearchQueue
                if building.kind == BuildingKind::ScratchingPost {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // FossilStones (Croak research) gets Researcher + ResearchQueue
                if building.kind == BuildingKind::FossilStones {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // LaserPointer gets AttackStats for tower combat
                if building.kind == BuildingKind::LaserPointer {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_LASER_POINTER,
                            range: tuning::TOWER_RANGE_LASER_POINTER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_LASER_POINTER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }

                // SporeTower (Croak defense) gets AttackStats
                if building.kind == BuildingKind::SporeTower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_SPORE_TOWER,
                            range: tuning::TOWER_RANGE_SPORE_TOWER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_SPORE_TOWER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }

                // TinkerBench (LLAMA research) gets Researcher + ResearchQueue
                if building.kind == BuildingKind::TinkerBench {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // TetanusTower (LLAMA defense) gets AttackStats + CorrodedApplicator
                if building.kind == BuildingKind::TetanusTower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_TETANUS_TOWER,
                            range: tuning::TOWER_RANGE_TETANUS_TOWER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_TETANUS_TOWER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                        CorrodedApplicator {
                            stacks_per_hit: 1,
                            max_stacks: 5,
                        },
                    ));
                }

                // DumpsterRelay (LLAMA comms) gets DumpsterRelayAura
                if building.kind == BuildingKind::DumpsterRelay {
                    commands.entity(entity).insert(Aura {
                        aura_type: AuraType::DumpsterRelayAura,
                        radius: Fixed::from_bits(10 << 16), // 10 tile radius
                        active: true,
                    });
                }

                // Panopticon (Murder research, limit 1) gets Researcher + ResearchQueue + UniqueBuildingLimit
                if building.kind == BuildingKind::Panopticon {
                    commands.entity(entity).insert((
                        Researcher,
                        ResearchQueue::default(),
                        UniqueBuildingLimit,
                    ));
                }

                // Watchtower (Murder defense tower) gets AttackStats
                if building.kind == BuildingKind::Watchtower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_WATCHTOWER,
                            range: tuning::TOWER_RANGE_WATCHTOWER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_WATCHTOWER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }

                // ClawMarks (Seekers research) gets Researcher + ResearchQueue
                if building.kind == BuildingKind::ClawMarks {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // GnawLab (Clawed research) gets Researcher + ResearchQueue
                if building.kind == BuildingKind::GnawLab {
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default()));
                }

                // SqueakTower (Clawed defense tower) gets AttackStats
                if building.kind == BuildingKind::SqueakTower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_SQUEAK_TOWER,
                            range: tuning::TOWER_RANGE_SQUEAK_TOWER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_SQUEAK_TOWER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }

                // SlagThrower (Seekers defense tower) gets AttackStats
                if building.kind == BuildingKind::SlagThrower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: tuning::TOWER_DAMAGE_SLAG_THROWER,
                            range: tuning::TOWER_RANGE_SLAG_THROWER,
                            attack_speed: tuning::TOWER_ATTACK_SPEED_SLAG_THROWER,
                            cooldown_remaining: 0,
                        },
                        AttackTypeMarker {
                            attack_type: AttackType::Ranged,
                        },
                    ));
                }
            }
            continue; // Don't process production while under construction
        }

        // Phase 2: Tick production queue
        if let Some(mut queue) = prod_queue
            && let Some((unit_kind, ticks_remaining)) = queue.queue.front_mut()
        {
            if *ticks_remaining > 0 {
                *ticks_remaining -= 1;
            }
            if *ticks_remaining == 0 {
                let kind = *unit_kind;
                queue.queue.pop_front();

                // Spawn the trained unit at the building's position
                let stats = base_stats(kind);
                let spawn_grid = pos.world.to_grid();
                let spawn_world = WorldPos::from_grid(spawn_grid);

                let mut unit_health = Health {
                    current: stats.health,
                    max: stats.health,
                };
                let mut attack_stats = AttackStats {
                    damage: stats.damage,
                    range: stats.range,
                    attack_speed: stats.attack_speed,
                    cooldown_remaining: 0,
                };
                let mut move_speed = MovementSpeed { speed: stats.speed };

                // Apply completed upgrades to newly spawned unit
                let player_id = owner.player_id as usize;
                if let Some(pres) = _player_resources.players.get(player_id) {
                    apply_upgrades_to_new_unit(
                        kind,
                        &pres.completed_upgrades,
                        &mut unit_health,
                        &mut attack_stats,
                        &mut move_speed,
                    );
                }

                let mut entity_cmds = commands.spawn((
                    Position { world: spawn_world },
                    Velocity::zero(),
                    GridCell { pos: spawn_grid },
                    Owner {
                        player_id: owner.player_id,
                    },
                    UnitType { kind },
                    unit_health,
                    move_speed,
                    attack_stats,
                    AttackTypeMarker {
                        attack_type: stats.attack_type,
                    },
                    AbilitySlots::from_abilities(unit_abilities(kind)),
                    StatusEffects::default(),
                    StatModifiers::default(),
                ));

                // DreamSiegeTimer for Catnappers
                if kind == UnitKind::Catnapper {
                    entity_cmds.insert(DreamSiegeTimer::default());
                }

                // --- Croak spawn-time components ---

                // Regeneron: LimbTracker (starts with full limbs)
                if kind == UnitKind::Regeneron {
                    entity_cmds.insert(LimbTracker {
                        current_limbs: 4,
                        max_limbs: 4,
                        regen_ticks: 200,
                    });
                }

                // Broodmother: SpawnlingCounter
                if kind == UnitKind::Broodmother {
                    entity_cmds.insert(SpawnlingCounter {
                        count: 0,
                        spawn_cooldown: 300, // 30s
                    });
                }

                // Shellwarden: AncientMoss aura
                if kind == UnitKind::Shellwarden {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::AncientMoss,
                        radius: Fixed::from_bits(3 << 16), // 3 tiles
                        active: true,
                    });
                }

                // Bogwhisper: BogSong aura
                if kind == UnitKind::Bogwhisper {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::BogSong,
                        radius: Fixed::from_bits(5 << 16), // 5 tiles
                        active: true,
                    });
                }

                // MurkCommander: UndyingPresence aura
                if kind == UnitKind::MurkCommander {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::UndyingPresence,
                        radius: Fixed::from_bits(8 << 16), // 8 tiles
                        active: true,
                    });
                }

                // Croaker: BogPatchCounter
                if kind == UnitKind::Croaker {
                    entity_cmds.insert(BogPatchCounter {
                        active_patches: Vec::new(),
                    });
                }

                // --- LLAMA spawn-time components ---

                // Scrounger: PocketStashInventory (PocketStash ability)
                if kind == UnitKind::Scrounger {
                    entity_cmds.insert(PocketStashInventory::default());
                }

                // HeapTitan: ScrapArmor aura
                if kind == UnitKind::HeapTitan {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::ScrapArmorAura,
                        radius: Fixed::from_bits(4 << 16), // 4 tiles
                        active: true,
                    });
                }

                // PatchPossum: FeignDeathTracker (passive auto-trigger)
                if kind == UnitKind::PatchPossum {
                    entity_cmds.insert(FeignDeathTracker::default());
                }

                // GreaseMonkey: JunkLauncherState (crit tracking)
                if kind == UnitKind::GreaseMonkey {
                    entity_cmds.insert(JunkLauncherState::default());
                }

                // JunkyardKing: OpenSourceUplink aura + FrankensteinTracker
                if kind == UnitKind::JunkyardKing {
                    entity_cmds.insert((
                        Aura {
                            aura_type: AuraType::OpenSourceUplinkAura,
                            radius: Fixed::from_bits(8 << 16), // 8 tiles
                            active: true,
                        },
                        FrankensteinTracker::default(),
                    ));
                }

                // Chonk passive components: GravitationalChonk aura + NineLives tracker
                if kind == UnitKind::Chonk {
                    entity_cmds.insert((
                        Aura {
                            aura_type: AuraType::GravitationalChonk,
                            radius: Fixed::from_bits(3 << 16),
                            active: true,
                        },
                        NineLivesTracker::default(),
                    ));
                }

                // --- Murder spawn-time components ---

                // Aerial marker for flying Murder units
                if matches!(
                    kind,
                    UnitKind::MurderScrounger
                        | UnitKind::Sentinel
                        | UnitKind::Rookclaw
                        | UnitKind::Magpike
                        | UnitKind::Magpyre
                        | UnitKind::Jaycaller
                        | UnitKind::Jayflicker
                        | UnitKind::Hootseer
                ) {
                    entity_cmds.insert(Aerial);
                }

                // Dusktalon: ground-based stealth assassin
                if kind == UnitKind::Dusktalon {
                    entity_cmds.insert(Stealth {
                        stealthed: true,
                        detection_radius: Fixed::from_bits(3 << 16),
                    });
                }

                // Hootseer: DreadAura + PanopticGazeCone
                if kind == UnitKind::Hootseer {
                    entity_cmds.insert((
                        Aura {
                            aura_type: AuraType::DreadAura,
                            radius: Fixed::from_bits(5 << 16),
                            active: true,
                        },
                        PanopticGazeCone {
                            direction: Fixed::ZERO,
                            half_angle: Fixed::from_bits(1 << 16), // ~1 radian (~57 degrees, half of 120-degree cone)
                        },
                    ));
                }

                // CorvusRex: CorvidNetwork aura
                if kind == UnitKind::CorvusRex {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::CorvidNetwork,
                        radius: Fixed::from_bits(10 << 16),
                        active: true,
                    });
                }

                // Magpike: TrinketWard passive tracker
                if kind == UnitKind::Magpike {
                    entity_cmds.insert(TrinketWardTracker {
                        trinkets_collected: 0,
                    });
                }
                // --- StationaryTimer for all combat units ---
                // Needed for anti-static damage bonus (targets must be trackable).
                if !kind.is_worker() {
                    entity_cmds.insert(StationaryTimer::default());
                }
                if matches!(
                    kind,
                    UnitKind::Ironhide
                        | UnitKind::Cragback
                        | UnitKind::Wardenmother
                        | UnitKind::Gutripper
                ) {
                    entity_cmds.insert(HeavyUnit);
                }
                if kind == UnitKind::Warden {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::VigilanceAura,
                        radius: Fixed::from_bits(5 << 16),
                        active: true,
                    });
                }
                if kind == UnitKind::Wardenmother {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::DeepseekUplinkAura,
                        radius: Fixed::from_bits(8 << 16),
                        active: true,
                    });
                }
                if kind == UnitKind::Gutripper {
                    entity_cmds.insert((
                        FrenzyStacks::default(),
                        BloodgreedTracker {
                            lifesteal_fraction: Fixed::from_num(0.20f32),
                        },
                    ));
                }

                // --- The Clawed (Mice) spawn-time components ---
                if kind == UnitKind::Gnawer {
                    entity_cmds.insert(StructuralWeaknessTimer::default());
                }
                if kind == UnitKind::Sparks {
                    entity_cmds.insert(StaticChargeStacks::default());
                }
                if kind == UnitKind::Plaguetail {
                    entity_cmds.insert(ContagionCloudOnDeath);
                }
                if kind == UnitKind::WarrenMarshal {
                    entity_cmds.insert(Aura {
                        aura_type: AuraType::RallyTheSwarm,
                        radius: Fixed::from_bits(6 << 16),
                        active: true,
                    });
                }

                let new_entity = entity_cmds.id();

                let faction = FactionId::from_u8(owner.player_id).unwrap_or(FactionId::CatGPT);

                // Auto-move to rally point if set
                if let Some(rally) = rally {
                    if let Some((path, move_target)) =
                        pathing_components(&map_res, spawn_grid, rally.target, faction)
                    {
                        commands.entity(new_entity).insert((path, move_target));
                    }
                } else if kind.is_worker() {
                    // Auto-gather: send newly produced workers to nearest deposit
                    let spawn_pos = spawn_world;
                    let mut best_dist_sq = i64::MAX;
                    let mut best_deposit: Option<(Entity, WorldPos, ResourceType)> = None;

                    for (dep_entity, dep_pos, dep) in deposits.iter() {
                        if dep.remaining == 0 {
                            continue;
                        }
                        let dx = spawn_pos.x.to_bits() as i64 - dep_pos.world.x.to_bits() as i64;
                        let dy = spawn_pos.y.to_bits() as i64 - dep_pos.world.y.to_bits() as i64;
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq < best_dist_sq {
                            best_dist_sq = dist_sq;
                            best_deposit = Some((dep_entity, dep_pos.world, dep.resource_type));
                        }
                    }

                    if let Some((dep_entity, dep_world, dep_resource)) = best_deposit {
                        commands.entity(new_entity).insert(Gathering {
                            deposit_entity: EntityId::from_entity(dep_entity),
                            carried_type: dep_resource,
                            carried_amount: 0,
                            state: GatherState::MovingToDeposit,
                            last_pos: (spawn_pos.x, spawn_pos.y),
                            stale_ticks: 0,
                        });

                        if let Some((path, move_target)) =
                            pathing_components(&map_res, spawn_grid, dep_world.to_grid(), faction)
                        {
                            commands.entity(new_entity).insert((path, move_target));
                        }
                    }
                }
            }
        }
    }
}

fn pathing_components(
    map_res: &MapResource,
    start: cc_core::coords::GridPos,
    target: cc_core::coords::GridPos,
    faction: FactionId,
) -> Option<(Path, MoveTarget)> {
    let waypoints = pathfinding::find_path(&map_res.map, start, target, faction)?;
    let first_waypoint = waypoints[0];

    Some((
        Path {
            waypoints: VecDeque::from(waypoints),
        },
        MoveTarget {
            target: WorldPos::from_grid(first_waypoint),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use cc_core::components::ResourceType;
    use cc_core::coords::GridPos;
    use cc_core::map::GameMap;
    use cc_core::terrain::TerrainType;

    fn run_production_once(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(production_system);
        schedule.run(world);
    }

    fn world_with_map(map: GameMap) -> World {
        let mut world = World::new();
        world.insert_resource(MapResource { map });
        world.insert_resource(PlayerResources::default());
        world
    }

    fn spawn_producer(
        world: &mut World,
        building_kind: BuildingKind,
        grid: GridPos,
        player_id: u8,
        unit_kind: UnitKind,
        rally: Option<GridPos>,
    ) -> Entity {
        let bstats = building_stats(building_kind);
        let mut queue = ProductionQueue::default();
        queue.queue.push_back((unit_kind, 1));

        let entity = world
            .spawn((
                Building {
                    kind: building_kind,
                },
                Owner { player_id },
                Position {
                    world: WorldPos::from_grid(grid),
                },
                Health {
                    current: bstats.health,
                    max: bstats.health,
                },
                Producer,
                queue,
            ))
            .id();

        if let Some(target) = rally {
            world.entity_mut(entity).insert(RallyPoint { target });
        }

        entity
    }

    fn spawn_deposit(world: &mut World, grid: GridPos, resource_type: ResourceType) -> Entity {
        world
            .spawn((
                Position {
                    world: WorldPos::from_grid(grid),
                },
                ResourceDeposit {
                    resource_type,
                    remaining: 100,
                },
            ))
            .id()
    }

    fn produced_unit(world: &mut World, kind: UnitKind) -> Entity {
        world
            .query::<(Entity, &UnitType)>()
            .iter(world)
            .find_map(|(entity, unit_type)| (unit_type.kind == kind).then_some(entity))
            .expect("produced unit should exist")
    }

    #[test]
    fn rally_spawn_uses_path_first_waypoint() {
        let mut map = GameMap::new(7, 5);
        for y in 0..4 {
            map.get_mut(GridPos::new(2, y)).unwrap().terrain = TerrainType::Rock;
        }

        let mut world = world_with_map(map);
        let rally = GridPos::new(4, 1);
        spawn_producer(
            &mut world,
            BuildingKind::CatTree,
            GridPos::new(1, 1),
            0,
            UnitKind::Nuisance,
            Some(rally),
        );

        run_production_once(&mut world);

        let unit = produced_unit(&mut world, UnitKind::Nuisance);
        let path = world
            .get::<Path>(unit)
            .expect("rally spawn should pathfind");
        let move_target = world
            .get::<MoveTarget>(unit)
            .expect("rally spawn should move to the first waypoint");

        assert_eq!(path.waypoints.back().copied(), Some(rally));
        assert_eq!(
            move_target.target.to_grid(),
            path.waypoints.front().copied().unwrap()
        );
        assert_ne!(move_target.target.to_grid(), rally);

        let map = &world.resource::<MapResource>().map;
        for waypoint in &path.waypoints {
            assert!(
                map.is_passable_for(*waypoint, FactionId::CatGPT),
                "rally path should avoid impassable terrain, but visited {waypoint:?}"
            );
        }
    }

    #[test]
    fn auto_gather_spawn_preserves_gathering_and_uses_owner_faction_pathing() {
        let mut map = GameMap::new(7, 5);
        for y in 0..5 {
            map.get_mut(GridPos::new(2, y)).unwrap().terrain = TerrainType::Water;
        }

        let mut world = world_with_map(map);
        let deposit = spawn_deposit(&mut world, GridPos::new(4, 1), ResourceType::GpuCores);
        spawn_producer(
            &mut world,
            BuildingKind::TheGrotto,
            GridPos::new(1, 1),
            5,
            UnitKind::Ponderer,
            None,
        );

        run_production_once(&mut world);

        let worker = produced_unit(&mut world, UnitKind::Ponderer);
        let gathering = world
            .get::<Gathering>(worker)
            .expect("auto-gather spawn should keep gathering state");
        assert_eq!(gathering.deposit_entity, EntityId::from_entity(deposit));
        assert_eq!(gathering.carried_type, ResourceType::GpuCores);
        assert_eq!(gathering.state, GatherState::MovingToDeposit);

        let path = world
            .get::<Path>(worker)
            .expect("Croak worker should path through water to deposit");
        let move_target = world
            .get::<MoveTarget>(worker)
            .expect("auto-gather should move to the first waypoint");

        assert_eq!(path.waypoints.back().copied(), Some(GridPos::new(4, 1)));
        assert_eq!(
            move_target.target.to_grid(),
            path.waypoints.front().copied().unwrap()
        );
        assert!(
            path.waypoints.iter().any(|waypoint| waypoint.x == 2),
            "Croak-owned worker should use faction-aware water traversal"
        );
    }
}
