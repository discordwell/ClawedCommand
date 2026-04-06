//! Headless strait mission runner.
//!
//! Usage: cargo run --bin strait_harness -- <lua_script_path>
//!
//! Runs the strait sim with the given Lua script until mission completes
//! or times out. Reports outcome and stats.

use cc_core::strait::StraitConfig;
use cc_harness::strait_headless::StraitHeadlessSim;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let script_path = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: strait_harness <lua_script_path>");
        std::process::exit(1);
    });

    let lua_source = std::fs::read_to_string(script_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", script_path, e);
        std::process::exit(1);
    });

    let config = StraitConfig::default();
    let mut sim = StraitHeadlessSim::new(config);

    println!("=== Strait Harness ===");
    println!("Script: {}", script_path);
    println!();

    let script_interval = 10; // run script every 10 ticks
    let max_ticks = 10000u64;
    let status_interval = 500;

    for tick in 0..max_ticks {
        // Run Lua script at intervals
        if tick % script_interval == 0 {
            if let Err(e) = sim.run_script(&lua_source) {
                eprintln!("Script error at tick {}: {}", tick, e);
            }
        }

        sim.advance(1);

        // Status updates
        if tick % status_interval == 0 {
            println!("{}", sim.status_line());
        }

        if sim.is_complete() {
            break;
        }
    }

    println!();
    println!("=== RESULT ===");
    println!("{}", sim.status_line());
    match sim.outcome() {
        Some(cc_sim::strait_sim::StraitOutcome::Win { tankers_arrived, total }) => {
            println!("WIN — {}/{} tankers arrived safely", tankers_arrived, total);
        }
        Some(cc_sim::strait_sim::StraitOutcome::Lose { reason }) => {
            println!("LOSE — {}", reason);
        }
        None => {
            println!("TIMEOUT — mission did not complete in {} ticks", max_ticks);
        }
    }
}
