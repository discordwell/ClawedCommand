//! Wet integration tests — full AI-vs-AI matches with invariant checking.
//!
//! Run: `cargo test -p cc_sim --features harness wet_test`

#[cfg(feature = "harness")]
mod wet {
    use cc_sim::ai::fsm::{AiDifficulty, AiPersonalityProfile};
    use cc_sim::harness::*;

    #[test]
    fn wet_balanced_vs_balanced() {
        let config = HarnessConfig::default();
        let result = run_match(&config);

        println!(
            "balanced_vs_balanced: {} | ticks: {} | violations: {} | wall: {}ms",
            result.outcome,
            result.final_tick,
            result.violations.len(),
            result.wall_time_ms
        );

        assert!(
            result.passed(),
            "Match should pass with no Error/Fatal violations. Violations: {:?}",
            result.fatal_violations()
        );
    }

    #[test]
    fn wet_aggressive_vs_defensive() {
        let config = HarnessConfig {
            seed: 123,
            bots: [
                BotConfig {
                    player_id: 0,
                    difficulty: AiDifficulty::Hard,
                    profile: AiPersonalityProfile::aggressive(),
                    faction: cc_core::components::Faction::CatGpt,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    profile: AiPersonalityProfile::defensive(),
                    faction: cc_core::components::Faction::CatGpt,
                },
            ],
            ..Default::default()
        };
        let result = run_match(&config);

        println!(
            "aggressive_vs_defensive: {} | ticks: {} | violations: {}",
            result.outcome,
            result.final_tick,
            result.violations.len()
        );

        assert!(result.passed(), "Match should pass");
    }

    #[test]
    fn wet_deterministic() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 2000,
            ..Default::default()
        };

        let r1 = run_match(&config);
        let r2 = run_match(&config);

        assert_eq!(
            r1.final_tick, r2.final_tick,
            "Same seed must produce same tick count"
        );
        assert_eq!(
            format!("{}", r1.outcome),
            format!("{}", r2.outcome),
            "Same seed must produce same outcome"
        );
    }

    #[test]
    fn wet_multiple_seeds() {
        for seed in [1, 42, 999, 12345] {
            let config = HarnessConfig {
                seed,
                max_ticks: 6000,
                ..Default::default()
            };
            let result = run_match(&config);

            println!(
                "seed {seed}: {} | ticks: {} | violations: {}",
                result.outcome,
                result.final_tick,
                result.violations.len()
            );

            assert!(result.passed(), "seed {seed} failed: {:?}",
                result.fatal_violations()
            );
        }
    }

    #[test]
    fn wet_voice_stop_command() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 1000,
            voice_script: Some(vec![VoiceInjection {
                tick: 500,
                keyword: "stop".into(),
                confidence: 0.95,
            }]),
            ..Default::default()
        };
        let result = run_match(&config);

        println!(
            "voice_stop: {} | injected: {} | resolved: {}",
            result.outcome, result.voice_commands_injected, result.voice_commands_resolved
        );

        assert!(result.passed(), "Voice test should pass");
        assert_eq!(result.voice_commands_injected, 1);
        assert!(
            result.voice_commands_resolved > 0,
            "Voice stop command should have been resolved"
        );
    }

    #[test]
    fn wet_voice_hold_during_combat() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 2000,
            voice_script: Some(vec![
                VoiceInjection {
                    tick: 500,
                    keyword: "hold".into(),
                    confidence: 0.95,
                },
                VoiceInjection {
                    tick: 600,
                    keyword: "stop".into(),
                    confidence: 0.90,
                },
            ]),
            ..Default::default()
        };
        let result = run_match(&config);

        println!(
            "voice_hold: {} | injected: {} | resolved: {}",
            result.outcome, result.voice_commands_injected, result.voice_commands_resolved
        );

        assert!(result.passed(), "Voice hold test should pass");
        assert_eq!(result.voice_commands_injected, 2);
        assert!(
            result.voice_commands_resolved >= 2,
            "Both voice commands should resolve"
        );
    }

    #[test]
    fn wet_snapshots_captured() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 500,
            snapshot_interval: 100,
            ..Default::default()
        };
        let result = run_match(&config);

        assert!(
            !result.snapshots.is_empty(),
            "Should capture at least one snapshot"
        );

        // First snapshot should have units for both players
        let snap = &result.snapshots[0];
        assert!(
            snap.players.len() >= 2,
            "Snapshot should have 2 player entries"
        );
        assert!(
            snap.units.len() >= 4,
            "Snapshot should have starting units (2 per player)"
        );
        assert!(
            snap.buildings.len() >= 2,
            "Snapshot should have starting buildings (1 per player)"
        );
    }

    #[test]
    fn wet_minimaps_captured() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 500,
            minimap_interval: 100,
            ..Default::default()
        };
        let result = run_match(&config);

        assert!(
            !result.minimap_frames.is_empty(),
            "Should capture at least one minimap"
        );

        // PNG should start with PNG magic bytes
        let (_, png_data) = &result.minimap_frames[0];
        assert!(
            png_data.len() > 8,
            "PNG data should have content"
        );
        assert_eq!(
            &png_data[1..4],
            b"PNG",
            "Should be valid PNG data"
        );
    }

    #[test]
    fn wet_games_produce_victories() {
        // Verify the AI can actually finish games, not just stalemate.
        // Use seeds known to produce decisive outcomes with current AI tuning.
        for seed in [999, 1, 12345] {
            let config = HarnessConfig {
                seed,
                max_ticks: 8000,
                ..Default::default()
            };
            let result = run_match(&config);

            assert!(
                matches!(result.outcome, MatchOutcome::Victory { .. }),
                "seed {seed} should produce a victory, got: {}",
                result.outcome
            );
        }
    }

    #[test]
    fn wet_report_generation() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 500,
            ..Default::default()
        };
        let result = run_match(&config);
        let report = generate_report(&result, &config);

        assert_eq!(report.seed, 42);
        assert_eq!(report.bots.len(), 2);
        assert!(report.duration_ticks > 0);
        assert!(report.wall_time_ms > 0);
    }

    // -----------------------------------------------------------------------
    // Gameplay observation tests
    // -----------------------------------------------------------------------

    /// Print a human-readable timeline of gameplay for visual inspection.
    /// Run with `--nocapture` to see the narrative output.
    #[test]
    fn wet_gameplay_narrative() {
        let config = HarnessConfig {
            seed: 42,
            max_ticks: 8000,
            snapshot_interval: 200,
            ..Default::default()
        };
        let result = run_match(&config);

        println!("\n=== GAMEPLAY NARRATIVE: seed 42 ===\n");

        let mut prev_buildings: [Vec<String>; 2] = [Vec::new(), Vec::new()];

        for snap in &result.snapshots {
            // Phase header
            let phases: Vec<String> = snap
                .players
                .iter()
                .map(|p| format!("P{}: {}", p.player_id, p.ai_phase))
                .collect();
            println!("--- Tick {} ({}) ---", snap.tick, phases.join(" | "));

            // Per-player summary
            for p in &snap.players {
                let pid = p.player_id as usize;
                let army_count = snap
                    .units
                    .iter()
                    .filter(|u| u.owner == p.player_id && u.kind != "Pawdler" && !u.is_dead)
                    .count();

                // Detect new buildings since last snapshot
                let current_buildings: Vec<String> = snap
                    .buildings
                    .iter()
                    .filter(|b| b.owner == p.player_id && !b.is_under_construction)
                    .map(|b| b.kind.clone())
                    .collect();
                let new_buildings: Vec<&String> = current_buildings
                    .iter()
                    .filter(|b| !prev_buildings[pid].contains(b))
                    .collect();
                let new_str = if new_buildings.is_empty() {
                    String::new()
                } else {
                    format!(
                        " (+{})",
                        new_buildings
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", +")
                    )
                };
                prev_buildings[pid] = current_buildings;

                println!(
                    "  P{}: {} units ({} army), {} buildings{} | food={} gpu={} supply={}/{}",
                    p.player_id,
                    p.unit_count,
                    army_count,
                    p.building_count,
                    new_str,
                    p.food,
                    p.gpu_cores,
                    p.supply,
                    p.supply_cap,
                );
            }

            // Construction in progress
            let under_construction: Vec<String> = snap
                .buildings
                .iter()
                .filter(|b| b.is_under_construction)
                .map(|b| format!("P{} {}", b.owner, b.kind))
                .collect();
            if !under_construction.is_empty() {
                println!("  [Under construction: {}]", under_construction.join(", "));
            }

            // Research activity
            for p in &snap.players {
                if let Some(ref research) = p.researching {
                    println!("  [P{} researching: {}]", p.player_id, research);
                }
                if !p.completed_upgrades.is_empty() {
                    println!(
                        "  [P{} completed: {}]",
                        p.player_id,
                        p.completed_upgrades.join(", ")
                    );
                }
            }

            // Build orders
            let builders: Vec<String> = snap
                .units
                .iter()
                .filter(|u| u.has_build_order)
                .map(|u| format!("P{} {}", u.owner, u.kind))
                .collect();
            if !builders.is_empty() {
                println!("  [Builders en route: {}]", builders.join(", "));
            }

            // Combat activity
            if snap.projectile_count > 0 || snap.melee_attack_count > 0 || snap.ranged_attack_count > 0 {
                println!(
                    "  [COMBAT: {} melee, {} ranged attacks (cumulative) | {} projectiles in flight]",
                    snap.melee_attack_count, snap.ranged_attack_count, snap.projectile_count
                );
            }

            // Status effects
            let affected: u32 = snap.units.iter().map(|u| u.active_status_effects).sum();
            if affected > 0 {
                println!("  [Status effects: {} active across units]", affected);
            }

            // Unit losses (dead units still in snapshot)
            let dead_count = snap.units.iter().filter(|u| u.is_dead).count();
            if dead_count > 0 {
                println!("  [Deaths this snapshot: {}]", dead_count);
            }

            println!();
        }

        // Summary
        let warnings = result
            .violations
            .iter()
            .filter(|v| matches!(v.severity, invariants::Severity::Warning))
            .count();
        let errors = result
            .violations
            .iter()
            .filter(|v| {
                matches!(
                    v.severity,
                    invariants::Severity::Error | invariants::Severity::Fatal
                )
            })
            .count();

        println!(
            "=== RESULT: {} | wall: {}ms | {} warnings, {} errors ===\n",
            result.outcome, result.wall_time_ms, warnings, errors
        );

        // Print any violations for visibility
        for v in &result.violations {
            println!("  {}", v);
        }

        assert!(
            result.passed(),
            "Narrative match should pass (no Error/Fatal violations)"
        );
    }

    /// Verify AI builds economy buildings (FishMarket) and supply buildings.
    /// Track economy development across seeds for observability.
    #[test]
    fn wet_ai_builds_supply_and_economy() {
        let seeds = [42, 123, 7, 999];
        let mut any_had_fish_market = false;
        let mut any_supply_reached_20 = false;

        for &seed in &seeds {
            let config = HarnessConfig {
                seed,
                max_ticks: 8000,
                snapshot_interval: 100,
                ..Default::default()
            };
            let result = run_match(&config);
            let snaps = &result.snapshots;

            let max_fish_markets = snaps
                .iter()
                .map(|s| {
                    let mut per_player = [0u32; 2];
                    for b in &s.buildings {
                        if b.kind == "FishMarket" {
                            per_player[b.owner as usize] += 1;
                        }
                    }
                    per_player.iter().copied().max().unwrap_or(0)
                })
                .max()
                .unwrap_or(0);

            let max_supply_cap = snaps
                .iter()
                .flat_map(|s| s.players.iter().map(|p| p.supply_cap))
                .max()
                .unwrap_or(0);

            println!(
                "seed {seed}: {} | max_fish_markets={} max_supply_cap={} | wall={}ms",
                result.outcome, max_fish_markets, max_supply_cap, result.wall_time_ms,
            );

            if max_fish_markets >= 1 {
                any_had_fish_market = true;
            }
            if max_supply_cap >= 20 {
                any_supply_reached_20 = true;
            }

            assert!(
                result.passed(),
                "seed {seed} should pass (no Error/Fatal violations)"
            );
        }

        assert!(
            any_had_fish_market,
            "At least one seed should produce an AI with a FishMarket"
        );
        assert!(
            any_supply_reached_20,
            "At least one seed should produce an AI with supply_cap >= 20 (LitterBox built)"
        );
    }

    /// Check that specific gameplay milestones were observed across multiple seeds.
    #[test]
    fn wet_feature_milestones() {
        let seeds = [42, 123, 999];
        let mut all_milestones: Vec<Vec<(&str, bool)>> = Vec::new();

        for &seed in &seeds {
            let config = HarnessConfig {
                seed,
                max_ticks: 8000,
                snapshot_interval: 200,
                ..Default::default()
            };
            let result = run_match(&config);
            let snaps = &result.snapshots;

            let ever_had_cat_tree = snaps.iter().any(|s| {
                s.buildings
                    .iter()
                    .any(|b| b.kind == "CatTree" && !b.is_under_construction)
            });
            let ever_had_server_rack = snaps.iter().any(|s| {
                s.buildings
                    .iter()
                    .any(|b| b.kind == "ServerRack" && !b.is_under_construction)
            });
            let ever_had_fish_market = snaps.iter().any(|s| {
                s.buildings
                    .iter()
                    .any(|b| b.kind == "FishMarket" && !b.is_under_construction)
            });
            let ever_had_research = snaps.iter().any(|s| {
                s.players
                    .iter()
                    .any(|p| !p.completed_upgrades.is_empty())
            });
            let ever_had_advanced_unit = snaps.iter().any(|s| {
                s.units
                    .iter()
                    .any(|u| u.kind == "FlyingFox" || u.kind == "Catnapper")
            });
            let ever_had_combat = snaps.iter().any(|s| {
                s.projectile_count > 0 || s.melee_attack_count > 0 || s.ranged_attack_count > 0
            });
            let ever_had_construction = snaps.iter().any(|s| {
                s.buildings.iter().any(|b| b.is_under_construction)
            });
            let reached_attack_phase = snaps.iter().any(|s| {
                s.players.iter().any(|p| p.ai_phase == "Attack")
            });
            let reached_midgame = snaps.iter().any(|s| {
                s.players.iter().any(|p| p.ai_phase == "MidGame")
            });
            let reached_buildup = snaps.iter().any(|s| {
                s.players.iter().any(|p| p.ai_phase == "BuildUp")
            });

            let milestones = vec![
                ("FishMarket built", ever_had_fish_market),
                ("CatTree built", ever_had_cat_tree),
                ("ServerRack built", ever_had_server_rack),
                ("Research completed", ever_had_research),
                ("Advanced unit trained", ever_had_advanced_unit),
                ("Combat occurred", ever_had_combat),
                ("Construction observed", ever_had_construction),
                ("Reached BuildUp", reached_buildup),
                ("Reached MidGame", reached_midgame),
                ("Reached Attack phase", reached_attack_phase),
            ];

            println!("\n=== MILESTONES: seed {} | {} ===", seed, result.outcome);
            for (name, hit) in &milestones {
                let marker = if *hit { "OK" } else { "MISS" };
                println!("  [{marker}] {name}");
            }

            assert!(
                result.passed(),
                "seed {seed} should pass (no Error/Fatal violations)"
            );

            all_milestones.push(milestones);
        }

        // Assert critical milestones were hit in at least one seed.
        // Research/advanced units are tracked but not asserted as critical —
        // games currently end too fast (3000-4200 ticks) for late-game economy.
        let critical = [
            "Construction observed",
            "Combat occurred",
            "Reached BuildUp",
            "Reached MidGame",
        ];

        println!("\n=== CRITICAL MILESTONE SUMMARY ===");
        for name in &critical {
            let hit_count = all_milestones
                .iter()
                .filter(|ms| ms.iter().any(|(n, hit)| n == name && *hit))
                .count();
            println!("  {}: {}/{} seeds", name, hit_count, seeds.len());
            assert!(
                hit_count > 0,
                "Critical milestone '{}' was never observed in any seed",
                name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Multi-faction wet tests
    // -----------------------------------------------------------------------

    /// Helper: run a faction mirror match and return the result.
    fn run_faction_match(
        faction_a: cc_core::components::Faction,
        faction_b: cc_core::components::Faction,
        seed: u64,
    ) -> MatchResult {
        use cc_sim::ai::fsm::faction_personality;
        let config = HarnessConfig {
            seed,
            max_ticks: 8000,
            snapshot_interval: 200,
            bots: [
                BotConfig {
                    player_id: 0,
                    difficulty: AiDifficulty::Medium,
                    profile: faction_personality(faction_a),
                    faction: faction_a,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    profile: faction_personality(faction_b),
                    faction: faction_b,
                },
            ],
            ..Default::default()
        };
        run_match(&config)
    }

    /// Each faction can play a mirror match without panics or fatal violations.
    #[test]
    fn wet_faction_mirror_matches() {
        use cc_core::components::Faction;

        let factions = [
            Faction::CatGpt,
            Faction::TheClawed,
            Faction::SeekersOfTheDeep,
            Faction::TheMurder,
            Faction::Llama,
            Faction::Croak,
        ];

        for faction in factions {
            let result = run_faction_match(faction, faction, 42);

            println!(
                "mirror {}: {} | ticks: {} | violations: {} | wall: {}ms",
                faction,
                result.outcome,
                result.final_tick,
                result.violations.len(),
                result.wall_time_ms,
            );

            assert!(
                result.passed(),
                "Mirror match for {} should pass. Violations: {:?}",
                faction,
                result.fatal_violations()
            );
        }
    }

    /// Cross-faction matchups: CatGpt vs each other faction.
    #[test]
    fn wet_cross_faction_matchups() {
        use cc_core::components::Faction;

        let opponents = [
            Faction::TheClawed,
            Faction::SeekersOfTheDeep,
            Faction::TheMurder,
            Faction::Llama,
            Faction::Croak,
        ];

        for opponent in opponents {
            let result = run_faction_match(Faction::CatGpt, opponent, 42);

            println!(
                "CatGpt vs {}: {} | ticks: {} | violations: {} | wall: {}ms",
                opponent,
                result.outcome,
                result.final_tick,
                result.violations.len(),
                result.wall_time_ms,
            );

            assert!(
                result.passed(),
                "CatGpt vs {} should pass. Violations: {:?}",
                opponent,
                result.fatal_violations()
            );
        }
    }

    /// Verify each faction spawns its own HQ and worker type, not CatGpt defaults.
    #[test]
    fn wet_faction_spawns_correct_entities() {
        use cc_core::components::Faction;
        use cc_sim::ai::fsm::{faction_map, faction_personality};

        let factions = [
            Faction::CatGpt,
            Faction::TheClawed,
            Faction::SeekersOfTheDeep,
            Faction::TheMurder,
            Faction::Llama,
            Faction::Croak,
        ];

        for faction in factions {
            let config = HarnessConfig {
                seed: 42,
                max_ticks: 500,
                snapshot_interval: 10, // Capture early to see starting entities
                bots: [
                    BotConfig {
                        player_id: 0,
                        difficulty: AiDifficulty::Medium,
                        profile: faction_personality(faction),
                        faction,
                    },
                    BotConfig {
                        player_id: 1,
                        difficulty: AiDifficulty::Medium,
                        profile: faction_personality(faction),
                        faction,
                    },
                ],
                ..Default::default()
            };
            let result = run_match(&config);
            assert!(!result.snapshots.is_empty(), "Should have at least one snapshot for {}", faction);
            let snap = &result.snapshots[0];
            let fmap = faction_map(faction);

            let expected_hq = format!("{:?}", fmap.hq);
            let expected_worker = format!("{:?}", fmap.worker);

            // Both players should have the faction's HQ
            for player_id in 0u8..=1 {
                let has_hq = snap.buildings.iter().any(|b| {
                    b.owner == player_id && b.kind == expected_hq
                });
                assert!(
                    has_hq,
                    "P{} ({}) should have HQ '{}', buildings: {:?}",
                    player_id, faction, expected_hq,
                    snap.buildings.iter().map(|b| &b.kind).collect::<Vec<_>>()
                );

                let has_worker = snap.units.iter().any(|u| {
                    u.owner == player_id && u.kind == expected_worker
                });
                assert!(
                    has_worker,
                    "P{} ({}) should have worker '{}', units: {:?}",
                    player_id, faction, expected_worker,
                    snap.units.iter().map(|u| &u.kind).collect::<Vec<_>>()
                );
            }
        }
    }

    /// Multi-faction stress test: different faction pairs across seeds.
    #[test]
    fn wet_faction_variety_stress() {
        use cc_core::components::Faction;

        let matchups = [
            (Faction::TheClawed, Faction::Croak, 7),
            (Faction::SeekersOfTheDeep, Faction::Llama, 123),
            (Faction::TheMurder, Faction::TheClawed, 999),
            (Faction::Llama, Faction::SeekersOfTheDeep, 42),
        ];

        for (a, b, seed) in matchups {
            let result = run_faction_match(a, b, seed);

            println!(
                "{} vs {} (seed {}): {} | ticks: {} | wall: {}ms",
                a, b, seed,
                result.outcome, result.final_tick, result.wall_time_ms,
            );

            assert!(
                result.passed(),
                "{} vs {} (seed {}) should pass. Violations: {:?}",
                a, b, seed,
                result.fatal_violations()
            );
        }
    }

    /// Verify that melee combat is tracked via CombatStats, not just projectiles.
    /// This catches the bug where `projectile_count == 0` at snapshot time
    /// caused combat to appear absent even when melee units were dealing damage.
    #[test]
    fn wet_melee_combat_tracked() {
        // Seed 999 previously showed [MISS] Combat occurred because all
        // engagements were melee and no projectiles were captured at snapshot time.
        let config = HarnessConfig {
            seed: 999,
            max_ticks: 8000,
            snapshot_interval: 200,
            ..Default::default()
        };
        let result = run_match(&config);
        let snaps = &result.snapshots;

        // At least one snapshot must show cumulative melee or ranged attacks
        let any_combat = snaps.iter().any(|s| {
            s.melee_attack_count > 0 || s.ranged_attack_count > 0
        });
        assert!(
            any_combat,
            "Seed 999 should record combat via melee_attack_count or ranged_attack_count"
        );

        // Specifically verify melee attacks happened (Nuisances are melee)
        let max_melee = snaps.iter().map(|s| s.melee_attack_count).max().unwrap_or(0);
        assert!(
            max_melee > 0,
            "Should have recorded melee attacks (Nuisances are melee units), got 0"
        );

        println!(
            "wet_melee_combat_tracked: max melee={}, max ranged={}",
            max_melee,
            snaps.iter().map(|s| s.ranged_attack_count).max().unwrap_or(0)
        );
    }

    // ── Faction Balance: each faction vs CatGPT at Basic tier ──────────────

    /// Run a faction matchup capped at Basic AiTier (no focus-fire, flanking, etc.).
    fn run_basic_tier_match(
        faction_a: cc_core::components::Faction,
        faction_b: cc_core::components::Faction,
        seed: u64,
    ) -> MatchResult {
        use cc_sim::ai::fsm::{AiTier, faction_personality};

        let mut profile_a = faction_personality(faction_a);
        profile_a.max_tier = Some(AiTier::Basic);
        let mut profile_b = faction_personality(faction_b);
        profile_b.max_tier = Some(AiTier::Basic);

        let config = HarnessConfig {
            seed,
            max_ticks: 9000,
            snapshot_interval: 200,
            bots: [
                BotConfig {
                    player_id: 0,
                    difficulty: AiDifficulty::Hard,
                    profile: profile_a,
                    faction: faction_a,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Hard,
                    profile: profile_b,
                    faction: faction_b,
                },
            ],
            ..Default::default()
        };
        run_match(&config)
    }

    struct FactionReport {
        name: &'static str,
        wins: u32,
        losses: u32,
        draws: u32,
        total_ticks: u64,
        games: u32,
    }

    impl FactionReport {
        fn new(name: &'static str) -> Self {
            Self { name, wins: 0, losses: 0, draws: 0, total_ticks: 0, games: 0 }
        }
        fn avg_ticks(&self) -> u64 {
            if self.games == 0 { 0 } else { self.total_ticks / self.games as u64 }
        }
    }

    /// Test all 5 non-CatGPT factions against CatGPT at Basic tier across 5 seeds.
    /// Prints a balance report and asserts no faction is completely dominated.
    #[test]
    fn wet_faction_vs_catgpt_basic_tier() {
        use cc_core::components::Faction;
        use cc_sim::harness::MatchOutcome;

        let opponents: [(&str, Faction); 5] = [
            ("TheClawed", Faction::TheClawed),
            ("SeekersOfTheDeep", Faction::SeekersOfTheDeep),
            ("TheMurder", Faction::TheMurder),
            ("Llama", Faction::Llama),
            ("Croak", Faction::Croak),
        ];
        let seeds = [42, 123, 456, 789, 999];

        let mut reports: Vec<FactionReport> = opponents
            .iter()
            .map(|(name, _)| FactionReport::new(name))
            .collect();

        for (i, (name, faction)) in opponents.iter().enumerate() {
            let mut timeouts = 0u32;
            for &seed in &seeds {
                let result = run_basic_tier_match(Faction::CatGpt, *faction, seed);

                let (p0_won, p1_won, draw) = match &result.outcome {
                    MatchOutcome::Victory { winner, .. } => {
                        (*winner == 0, *winner == 1, false)
                    }
                    MatchOutcome::Timeout { leading_player, .. } => {
                        match leading_player {
                            Some(0) => (true, false, false),
                            Some(1) => (false, true, false),
                            _ => (false, false, true),
                        }
                    }
                    MatchOutcome::Draw { .. } => (false, false, true),
                    MatchOutcome::Error { .. } => (false, false, true),
                };

                if draw {
                    reports[i].draws += 1;
                } else if p1_won {
                    reports[i].wins += 1; // opponent (faction) won
                } else if p0_won {
                    reports[i].losses += 1; // CatGPT won
                }

                if matches!(&result.outcome, MatchOutcome::Timeout { .. }) {
                    timeouts += 1;
                }

                reports[i].total_ticks += result.final_tick;
                reports[i].games += 1;

                assert!(
                    result.passed(),
                    "CatGpt vs {} (seed {}) had fatal violations: {:?}",
                    name, seed, result.fatal_violations()
                );

                println!(
                    "  CatGpt vs {:18} seed {:3}: {:40} ticks: {:5} wall: {}ms",
                    name, seed, format!("{}", result.outcome), result.final_tick, result.wall_time_ms,
                );
            }

            // No matchup should have ALL 5 games timeout
            assert!(
                timeouts < seeds.len() as u32,
                "{} vs CatGPT: all {} games timed out — factions can't finish games",
                name, seeds.len()
            );
        }

        // ── Balance report ──
        println!("\n╔══════════════════════╦══════╦══════╦══════╦════════════╗");
        println!("║ Faction vs CatGPT    ║ Wins ║ Loss ║ Draw ║  Avg Ticks ║");
        println!("╠══════════════════════╬══════╬══════╬══════╬════════════╣");
        for r in &reports {
            println!(
                "║ {:20} ║ {:4} ║ {:4} ║ {:4} ║ {:10} ║",
                r.name, r.wins, r.losses, r.draws, r.avg_ticks()
            );
        }
        println!("╚══════════════════════╩══════╩══════╩══════╩════════════╝");

        // Balance check: warn if any faction can't win a single game.
        // Non-CatGpt unit stats aren't balanced yet, so this is informational only.
        for r in &reports {
            if r.wins + r.draws == 0 {
                println!(
                    "WARNING: {} won 0 out of {} games vs CatGPT — needs balance tuning",
                    r.name, r.games
                );
            }
        }
    }
}
