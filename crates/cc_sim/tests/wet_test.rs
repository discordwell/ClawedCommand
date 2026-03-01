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
            result
                .violations
                .iter()
                .filter(|v| matches!(
                    v.severity,
                    invariants::Severity::Error | invariants::Severity::Fatal
                ))
                .collect::<Vec<_>>()
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
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    profile: AiPersonalityProfile::defensive(),
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
                result.violations.iter()
                    .filter(|v| matches!(v.severity, invariants::Severity::Error | invariants::Severity::Fatal))
                    .collect::<Vec<_>>()
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
        for seed in [999, 7, 314] {
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

        // Assert critical milestones were hit in at least one seed
        let critical = [
            "Construction observed",
            "Combat occurred",
            "Reached BuildUp",
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
}
