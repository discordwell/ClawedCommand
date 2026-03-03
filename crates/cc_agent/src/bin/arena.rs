//! Arena match runner — runs FSM+Script AI matches for the training pipeline.
//!
//! Run: `cargo run -p cc_agent --bin arena --features harness -- --seed 42`

#[cfg(feature = "harness")]
fn main() {
    use clap::Parser;
    use cc_agent::arena::*;
    use cc_sim::ai::fsm::{AiDifficulty, AiPersonalityProfile, BotConfig};
    use cc_sim::harness::MatchOutcome;
    use cc_sim::harness::snapshot::capture_snapshot;
    use std::path::PathBuf;

    #[derive(Parser, Debug)]
    #[command(name = "arena", about = "Run AI arena matches with FSM + Lua scripts")]
    struct Args {
        /// Random seed(s) for map generation (comma-separated for multiple)
        #[arg(long, default_value = "42")]
        seeds: String,

        /// Directory of Lua scripts for BOTH players (overridden by per-player flags)
        #[arg(long)]
        shared_scripts: Option<PathBuf>,

        /// Directory of Lua scripts for player 0
        #[arg(long)]
        p0_scripts: Option<PathBuf>,

        /// Directory of Lua scripts for player 1
        #[arg(long)]
        p1_scripts: Option<PathBuf>,

        /// Inline Lua source for player 0 (alternative to --p0-scripts)
        #[arg(long)]
        p0_inline: Option<String>,

        /// Inline Lua source for player 1 (alternative to --p1-scripts)
        #[arg(long)]
        p1_inline: Option<String>,

        /// AI personality profile for player 0
        #[arg(long, default_value = "balanced")]
        p0_profile: String,

        /// AI personality profile for player 1
        #[arg(long, default_value = "balanced")]
        p1_profile: String,

        /// Maximum ticks per match (default: 6000 = 10 min at 10hz)
        #[arg(long, default_value_t = 6000)]
        max_ticks: u64,

        /// Output directory for match results
        #[arg(long)]
        output: Option<PathBuf>,

        /// Map dimensions (WxH)
        #[arg(long, default_value = "64x64")]
        map_size: String,

        /// Dump GameStateSnapshot as JSON every N ticks (0 = disabled)
        #[arg(long, default_value_t = 0)]
        snapshot_interval: u64,
    }

    fn parse_profile(name: &str) -> AiPersonalityProfile {
        match name {
            "aggressive" => AiPersonalityProfile::aggressive(),
            "defensive" => AiPersonalityProfile::defensive(),
            _ => AiPersonalityProfile::balanced(),
        }
    }

    fn parse_map_size(s: &str) -> (u32, u32) {
        if let Some((w, h)) = s.split_once('x') {
            let w = w.parse().unwrap_or(64);
            let h = h.parse().unwrap_or(64);
            (w, h)
        } else {
            (64, 64)
        }
    }

    let args = Args::parse();

    let seeds: Vec<u64> = args
        .seeds
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if seeds.is_empty() {
        eprintln!("Error: no valid seeds provided");
        std::process::exit(1);
    }

    let map_size = parse_map_size(&args.map_size);

    let shared = args.shared_scripts.map(|p| vec![ScriptSource::File(p)]);

    let p0_scripts = if let Some(src) = args.p0_inline {
        Some(vec![ScriptSource::Inline {
            name: "p0_inline".into(),
            source: src,
        }])
    } else {
        args.p0_scripts
            .map(|p| vec![ScriptSource::File(p)])
            .or(shared.clone())
    };

    let p1_scripts = if let Some(src) = args.p1_inline {
        Some(vec![ScriptSource::Inline {
            name: "p1_inline".into(),
            source: src,
        }])
    } else {
        args.p1_scripts
            .map(|p| vec![ScriptSource::File(p)])
            .or(shared)
    };

    println!("=== Arena Match Runner ===");
    println!("Seeds: {:?}", seeds);
    println!(
        "P0 scripts: {}",
        p0_scripts
            .as_ref()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "none (FSM only)".into())
    );
    println!(
        "P1 scripts: {}",
        p1_scripts
            .as_ref()
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "none (FSM only)".into())
    );
    println!();

    let mut all_results = Vec::new();

    for &seed in &seeds {
        let config = ArenaConfig {
            seed,
            map_size,
            max_ticks: args.max_ticks,
            snapshot_interval: args.snapshot_interval,
            output_path: args
                .output
                .as_ref()
                .map(|d| d.join(format!("match_seed{seed}.json"))),
            bots: [
                BotConfig {
                    player_id: 0,
                    difficulty: AiDifficulty::Medium,
                    profile: parse_profile(&args.p0_profile),
                    faction: cc_core::components::Faction::CatGpt,
                },
                BotConfig {
                    player_id: 1,
                    difficulty: AiDifficulty::Medium,
                    profile: parse_profile(&args.p1_profile),
                    faction: cc_core::components::Faction::CatGpt,
                },
            ],
            scripts: [p0_scripts.clone(), p1_scripts.clone()],
            script_budget: 500,
            extra_spawns: Vec::new(),
        };

        print!("Seed {seed}: ");
        let result = run_arena_match(&config);

        println!(
            "{} | ticks: {} | wall: {}ms | p0 kills: {} lost: {} | p1 kills: {} lost: {}",
            result.outcome,
            result.final_tick,
            result.wall_time_ms,
            result.stats.players[0].units_killed,
            result.stats.players[0].units_lost,
            result.stats.players[1].units_killed,
            result.stats.players[1].units_lost,
        );

        if !result.scripts_loaded[0].is_empty() || !result.scripts_loaded[1].is_empty() {
            println!(
                "  Scripts: P0={:?}, P1={:?}",
                result.scripts_loaded[0], result.scripts_loaded[1]
            );
        }

        if let Some(ref path) = config.output_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let report = ArenaReport::from_result(&result, &config);
            if let Ok(json) = serde_json::to_string_pretty(&report) {
                let _ = std::fs::write(path, &json);
                println!("  Report: {}", path.display());
            }
        }

        // Save snapshots if captured
        if !result.snapshots.is_empty() {
            if let Some(ref output_dir) = args.output {
                let snap_dir = output_dir.join("snapshots");
                let _ = std::fs::create_dir_all(&snap_dir);
                for snap in &result.snapshots {
                    let snap_path = snap_dir.join(format!(
                        "seed{seed}_tick{}.json",
                        snap.tick
                    ));
                    if let Ok(json) = serde_json::to_string_pretty(&snap) {
                        let _ = std::fs::write(&snap_path, &json);
                    }
                }
                println!(
                    "  Snapshots: {} captured → {}",
                    result.snapshots.len(),
                    snap_dir.display()
                );
            }
        }

        all_results.push((seed, result));
    }

    if seeds.len() > 1 {
        println!("\n=== Summary ===");
        let mut p0_wins = 0u32;
        let mut p1_wins = 0u32;
        let mut draws = 0u32;
        let mut timeouts = 0u32;

        for (_, result) in &all_results {
            match &result.outcome {
                MatchOutcome::Victory { winner, .. } => {
                    if *winner == 0 {
                        p0_wins += 1;
                    } else {
                        p1_wins += 1;
                    }
                }
                MatchOutcome::Draw { .. } => draws += 1,
                MatchOutcome::Timeout { .. } => timeouts += 1,
                MatchOutcome::Error { .. } => {}
            }
        }

        let total = seeds.len() as f64;
        println!(
            "P0 wins: {p0_wins} ({:.0}%)",
            p0_wins as f64 / total * 100.0
        );
        println!(
            "P1 wins: {p1_wins} ({:.0}%)",
            p1_wins as f64 / total * 100.0
        );
        println!("Draws: {draws}, Timeouts: {timeouts}");

        if let Some(ref output_dir) = args.output {
            let _ = std::fs::create_dir_all(output_dir);
            let summary = serde_json::json!({
                "seeds": seeds,
                "matches": seeds.len(),
                "p0_wins": p0_wins,
                "p1_wins": p1_wins,
                "draws": draws,
                "timeouts": timeouts,
                "p0_win_rate": p0_wins as f64 / total,
                "p1_win_rate": p1_wins as f64 / total,
            });
            if let Ok(json) = serde_json::to_string_pretty(&summary) {
                let path = output_dir.join("summary.json");
                let _ = std::fs::write(&path, &json);
                println!("Summary: {}", path.display());
            }
        }
    }
}

#[cfg(not(feature = "harness"))]
fn main() {
    eprintln!("Error: arena binary requires --features harness");
    eprintln!("Run: cargo run -p cc_agent --bin arena --features harness -- --seed 42");
    std::process::exit(1);
}
