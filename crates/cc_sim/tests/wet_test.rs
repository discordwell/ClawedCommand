//! Wet integration tests — full AI-vs-AI matches with invariant checking.
//!
//! Run: `cargo test -p cc_sim --features harness wet_test`

#[cfg(feature = "harness")]
mod wet {
    use cc_sim::ai::fsm::{AiDifficulty, BotPersonality};
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
                    personality: BotPersonality::Aggressive,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    personality: BotPersonality::Defensive,
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
        for seed in [42, 123, 999] {
            let config = HarnessConfig {
                seed,
                max_ticks: 6000,
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
}
