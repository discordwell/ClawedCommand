use bevy::prelude::*;

use cc_core::abilities::unit_abilities;
use cc_core::building_stats::building_stats;
use cc_core::commands::EntityId;
use cc_core::components::{
    AbilitySlots, AttackStats, AttackType, AttackTypeMarker, Aura, AuraType, BogPatchCounter,
    Building, BuildingKind, ContagionCloudOnDeath, CorrodedApplicator, DreamSiegeTimer,
    FeignDeathTracker, BloodgreedTracker, FrankensteinTracker, FrenzyStacks, GatherState,
    Gathering, GridCell, Health, HeavyUnit, JunkLauncherState, LimbTracker, MoveTarget,
    MovementSpeed, NineLivesTracker, Owner, PocketStashInventory, StationaryTimer,
    StaticChargeStacks, StructuralWeaknessTimer,
    Position, Producer, ProductionQueue, RallyPoint, ResearchQueue, Researcher, ResourceDeposit,
    SpawnlingCounter, StatModifiers, UnderConstruction, Aerial, PanopticGazeCone, Stealth, TrinketWardTracker, UniqueBuildingLimit, UnitKind, UnitType, Velocity,
};
use cc_core::coords::WorldPos;
use cc_core::math::Fixed;
use cc_core::status_effects::StatusEffects;
use cc_core::unit_stats::base_stats;

use crate::resources::PlayerResources;
use crate::systems::research_system::apply_upgrades_to_new_unit;

/// Ticks UnderConstruction, ticks ProductionQueue, spawns units on completion.
pub fn production_system(
    mut commands: Commands,
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
    mut player_resources: ResMut<PlayerResources>,
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
                let progress = Fixed::from_num(1.0f32)
                    - Fixed::from_num(uc.remaining_ticks as f32)
                        / Fixed::from_num(uc.total_ticks as f32);
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

                // Grant supply cap on construction completion (not on build start)
                if bstats.supply_provided > 0 {
                    if let Some(pres) = player_resources
                        .players
                        .get_mut(owner.player_id as usize)
                    {
                        pres.supply_cap += bstats.supply_provided;
                    }
                }
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
                            damage: cc_core::tuning::LASER_POINTER_DAMAGE,
                            range: cc_core::tuning::LASER_POINTER_RANGE,
                            attack_speed: cc_core::tuning::LASER_POINTER_ATTACK_SPEED,
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
                            damage: Fixed::from_bits(5 << 16), // 5 damage
                            range: Fixed::from_bits(5 << 16),  // 5 range
                            attack_speed: 20,                   // 2s between attacks
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
                            damage: Fixed::from_bits(8 << 16),  // 8 damage
                            range: Fixed::from_bits(5 << 16),   // 5 range
                            attack_speed: 12,                    // 1.2s between attacks
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
                    commands
                        .entity(entity)
                        .insert((Researcher, ResearchQueue::default(), UniqueBuildingLimit));
                }

                // Watchtower (Murder defense tower) gets AttackStats
                if building.kind == BuildingKind::Watchtower {
                    commands.entity(entity).insert((
                        AttackStats {
                            damage: Fixed::from_bits(12 << 16), // 12 damage
                            range: Fixed::from_bits(7 << 16),   // 7 range (longer than LaserPointer)
                            attack_speed: 18,                    // 1.8s between attacks
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
                            damage: Fixed::from_bits(5 << 16), // 5 damage
                            range: Fixed::from_bits(4 << 16),  // 4 range
                            attack_speed: 20,                   // 2s between attacks
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
                            damage: Fixed::from_bits(15 << 16), // 15 damage
                            range: Fixed::from_bits(7 << 16),   // 7 range
                            attack_speed: 30,                    // 3s between attacks (slower, AoE)
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
        if let Some(mut queue) = prod_queue {
            if let Some((unit_kind, ticks_remaining)) = queue.queue.front_mut() {
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
                    if let Some(pres) = player_resources.players.get(player_id) {
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
                    if matches!(kind,
                        UnitKind::MurderScrounger | UnitKind::Sentinel | UnitKind::Rookclaw |
                        UnitKind::Magpike | UnitKind::Magpyre | UnitKind::Jaycaller |
                        UnitKind::Jayflicker | UnitKind::Hootseer
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
                    // --- Seekers of the Deep spawn-time components ---
                    if matches!(kind, UnitKind::Delver | UnitKind::Ironhide | UnitKind::Cragback | UnitKind::Warden | UnitKind::Sapjaw | UnitKind::Wardenmother | UnitKind::SeekerTunneler | UnitKind::Embermaw | UnitKind::Dustclaw | UnitKind::Gutripper) {
                        entity_cmds.insert(StationaryTimer::default());
                    }
                    if matches!(kind, UnitKind::Ironhide | UnitKind::Cragback | UnitKind::Wardenmother | UnitKind::Gutripper) {
                        entity_cmds.insert(HeavyUnit);
                    }
                    if kind == UnitKind::Warden {
                        entity_cmds.insert(Aura { aura_type: AuraType::VigilanceAura, radius: Fixed::from_bits(5 << 16), active: true });
                    }
                    if kind == UnitKind::Wardenmother {
                        entity_cmds.insert(Aura { aura_type: AuraType::DeepseekUplinkAura, radius: Fixed::from_bits(8 << 16), active: true });
                    }
                    if kind == UnitKind::Gutripper {
                        entity_cmds.insert((FrenzyStacks::default(), BloodgreedTracker { lifesteal_fraction: Fixed::from_num(0.20f32) }));
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

                    // Auto-move to rally point if set
                    if let Some(rally) = rally {
                        let rally_world = WorldPos::from_grid(rally.target);
                        commands
                            .entity(new_entity)
                            .insert(MoveTarget { target: rally_world });
                    } else if kind == UnitKind::Pawdler || kind == UnitKind::Ponderer || kind == UnitKind::MurderScrounger || kind == UnitKind::Scrounger || kind == UnitKind::Delver || kind == UnitKind::Nibblet {
                        // Auto-gather: send newly produced workers to nearest deposit
                        let spawn_pos = spawn_world;
                        let mut best_dist_sq = i64::MAX;
                        let mut best_deposit: Option<(Entity, WorldPos)> = None;

                        for (dep_entity, dep_pos, dep) in deposits.iter() {
                            if dep.remaining == 0 {
                                continue;
                            }
                            let dx =
                                spawn_pos.x.to_bits() as i64 - dep_pos.world.x.to_bits() as i64;
                            let dy =
                                spawn_pos.y.to_bits() as i64 - dep_pos.world.y.to_bits() as i64;
                            let dist_sq = dx * dx + dy * dy;
                            if dist_sq < best_dist_sq {
                                best_dist_sq = dist_sq;
                                best_deposit = Some((dep_entity, dep_pos.world));
                            }
                        }

                        if let Some((dep_entity, dep_world)) = best_deposit {
                            let dep_resource = deposits.get(dep_entity).unwrap().2.resource_type;
                            commands.entity(new_entity).insert((
                                Gathering {
                                    deposit_entity: EntityId(dep_entity.to_bits()),
                                    carried_type: dep_resource,
                                    carried_amount: 0,
                                    state: GatherState::MovingToDeposit,
                                    last_pos: (spawn_pos.x, spawn_pos.y),
                                    stale_ticks: 0,
                                },
                                MoveTarget { target: dep_world },
                            ));
                        }
                    }
                }
            }
        }
    }
}
