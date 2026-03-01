//! Claude Code CLI backend — invokes `claude -p` as a subprocess.
//!
//! This runs on the LLM runner's background thread (blocking is fine).

use std::process::Command;
use std::time::Duration;

/// Errors from invoking the Claude Code CLI.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeCliError {
    #[error("Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code")]
    NotInstalled,
    #[error("Claude CLI failed to start: {0}")]
    SpawnFailed(std::io::Error),
    #[error("Claude CLI timed out after {0}s")]
    Timeout(u64),
    #[error("Claude CLI exited with status {status}: {stderr}")]
    NonZeroExit { status: i32, stderr: String },
    #[error("Claude CLI produced no output")]
    EmptyOutput,
}

const TIMEOUT_SECS: u64 = 120;

/// Check whether the `claude` binary is available on PATH.
pub fn is_claude_installed() -> bool {
    Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Invoke `claude -p` with the given prompt and system prompt.
/// Blocks until the subprocess completes (run on background thread).
pub fn invoke_claude_cli(prompt: &str, system_prompt: &str) -> Result<String, ClaudeCliError> {
    if !is_claude_installed() {
        return Err(ClaudeCliError::NotInstalled);
    }

    let mut child = Command::new("claude")
        .arg("-p")
        .arg(prompt)
        .arg("--system-prompt")
        .arg(system_prompt)
        .arg("--output-format")
        .arg("text")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(ClaudeCliError::SpawnFailed)?;

    // Poll child with timeout using try_wait loop
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(TIMEOUT_SECS);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Child exited — collect output
                let mut stdout_buf = Vec::new();
                let mut stderr_buf = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout_buf);
                }
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_end(&mut stderr_buf);
                }

                if !status.success() {
                    let stderr = String::from_utf8_lossy(&stderr_buf).to_string();
                    return Err(ClaudeCliError::NonZeroExit {
                        status: status.code().unwrap_or(-1),
                        stderr,
                    });
                }

                let stdout = String::from_utf8_lossy(&stdout_buf).trim().to_string();
                if stdout.is_empty() {
                    return Err(ClaudeCliError::EmptyOutput);
                }

                return Ok(stdout);
            }
            Ok(None) => {
                // Still running — check timeout
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // Reap zombie
                    return Err(ClaudeCliError::Timeout(TIMEOUT_SECS));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(ClaudeCliError::SpawnFailed(e));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_claude_installed_returns_bool() {
        // Just verify it doesn't panic — result depends on system
        let _installed = is_claude_installed();
    }

    #[test]
    fn invoke_claude_cli_not_installed() {
        // If claude is installed this test won't trigger the NotInstalled path,
        // but we verify the function doesn't panic either way.
        let result = invoke_claude_cli("test", "test");
        match result {
            Err(ClaudeCliError::NotInstalled) => {} // Expected on systems without claude
            Ok(_) => {}                             // Claude is installed, got a response
            Err(_) => {}                            // Some other error is acceptable
        }
    }
}
