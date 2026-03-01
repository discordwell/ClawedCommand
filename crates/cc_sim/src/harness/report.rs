//! JSON match report generation.

use serde::Serialize;

use super::invariants::{InvariantViolation, Severity};
use super::{HarnessConfig, MatchResult};

#[derive(Serialize, Debug)]
pub struct BotReport {
    pub player_id: u8,
    pub difficulty: String,
    pub personality: String,
}

#[derive(Serialize, Debug)]
pub struct ViolationSummary {
    pub warnings: u32,
    pub errors: u32,
    pub fatals: u32,
}

#[derive(Serialize, Debug)]
pub struct MatchReport {
    pub seed: u64,
    pub outcome: String,
    pub duration_ticks: u64,
    pub wall_time_ms: u64,
    pub bots: Vec<BotReport>,
    pub violations: Vec<InvariantViolation>,
    pub violation_summary: ViolationSummary,
    pub passed: bool,
    pub voice_commands_injected: u32,
    pub voice_commands_resolved: u32,
}

impl MatchReport {
    pub fn from_result(result: &MatchResult, config: &HarnessConfig) -> Self {
        let bots: Vec<BotReport> = config
            .bots
            .iter()
            .map(|b| BotReport {
                player_id: b.player_id,
                difficulty: format!("{:?}", b.difficulty),
                personality: format!("{:?}", b.profile),
            })
            .collect();

        let mut warnings = 0u32;
        let mut errors = 0u32;
        let mut fatals = 0u32;

        for v in &result.violations {
            match v.severity {
                Severity::Warning => warnings += 1,
                Severity::Error => errors += 1,
                Severity::Fatal => fatals += 1,
            }
        }

        Self {
            seed: config.seed,
            outcome: format!("{}", result.outcome),
            duration_ticks: result.final_tick,
            wall_time_ms: result.wall_time_ms,
            bots,
            violations: result.violations.clone(),
            violation_summary: ViolationSummary {
                warnings,
                errors,
                fatals,
            },
            passed: result.passed(),
            voice_commands_injected: result.voice_commands_injected,
            voice_commands_resolved: result.voice_commands_resolved,
        }
    }
}
