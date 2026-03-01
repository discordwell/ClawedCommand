//! Wet test binary runner — runs AI-vs-AI matches and writes results to disk.
//!
//! Run: `cargo run -p cc_sim --bin wet-test --features harness`

#[cfg(feature = "harness")]
fn main() {
    use cc_sim::ai::fsm::{AiDifficulty, AiPersonalityProfile};
    use cc_sim::harness::*;
    use std::path::PathBuf;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let base_dir = PathBuf::from("wet_tests");

    let configs: Vec<(&str, HarnessConfig)> = vec![
        (
            "balanced",
            HarnessConfig {
                seed: 42,
                output_dir: Some(base_dir.join(format!("run_{timestamp}_seed42"))),
                ..Default::default()
            },
        ),
        (
            "aggressive_vs_defensive",
            HarnessConfig {
                seed: 123,
                output_dir: Some(base_dir.join(format!("run_{timestamp}_seed123"))),
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
            },
        ),
        (
            "seed_7777",
            HarnessConfig {
                seed: 7777,
                output_dir: Some(base_dir.join(format!("run_{timestamp}_seed7777"))),
                ..Default::default()
            },
        ),
        (
            "with_voice",
            HarnessConfig {
                seed: 42,
                output_dir: Some(base_dir.join(format!("run_{timestamp}_voice"))),
                voice_script: Some(vec![
                    VoiceInjection {
                        tick: 500,
                        keyword: "stop".into(),
                        confidence: 0.95,
                    },
                    VoiceInjection {
                        tick: 600,
                        keyword: "hold".into(),
                        confidence: 0.90,
                    },
                    VoiceInjection {
                        tick: 700,
                        keyword: "stop".into(),
                        confidence: 0.85,
                    },
                ]),
                ..Default::default()
            },
        ),
    ];

    println!("=== Wet Test Harness ===\n");

    for (name, config) in &configs {
        println!("Running: {name}...");
        let result = run_match(config);
        let report = generate_report(&result, config);

        // Write summary JSON
        if let Some(ref dir) = config.output_dir {
            let _ = std::fs::create_dir_all(dir);
            if let Ok(json) = serde_json::to_string_pretty(&report) {
                let path = dir.join("summary.json");
                let _ = std::fs::write(&path, &json);
                println!("  Report: {}", path.display());
            }
        }

        println!(
            "  {name}: {} | ticks: {} | violations: {} | wall: {}ms | passed: {}",
            result.outcome,
            result.final_tick,
            result.violations.len(),
            result.wall_time_ms,
            result.passed()
        );

        if !result.violations.is_empty() {
            let errors: Vec<_> = result
                .violations
                .iter()
                .filter(|v| !matches!(v.severity, cc_sim::harness::invariants::Severity::Warning))
                .collect();
            if !errors.is_empty() {
                println!("  Errors:");
                for v in &errors {
                    println!("    {v}");
                }
            }
        }

        if result.voice_commands_injected > 0 {
            println!(
                "  Voice: {}/{} commands resolved",
                result.voice_commands_resolved, result.voice_commands_injected
            );
        }

        println!();
    }
}

#[cfg(not(feature = "harness"))]
fn main() {
    eprintln!("Error: wet test binary requires --features harness");
    eprintln!("Run: cargo run -p cc_sim --bin wet-test --features harness");
    std::process::exit(1);
}
