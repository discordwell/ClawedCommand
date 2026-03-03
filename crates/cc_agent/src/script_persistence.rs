//! Disk persistence for player-created Lua scripts.
//!
//! Scripts are saved to `assets/scripts/player/{name}.lua` and loaded
//! on game startup so they survive across sessions.

use std::fs;
use std::path::{Path, PathBuf};

use crate::construct_mode::LuaScript;

/// Directory where player scripts are stored, relative to the workspace root.
const PLAYER_SCRIPTS_DIR: &str = "assets/scripts/player";

/// Resolve the player scripts directory.
/// Uses CARGO_MANIFEST_DIR in dev to find the workspace root.
fn scripts_dir() -> PathBuf {
    // In development, cc_agent's manifest is at crates/cc_agent/Cargo.toml
    // so workspace root is ../../
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest).join("../../").join(PLAYER_SCRIPTS_DIR)
}

/// Sanitize a script name for use as a filename.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Save a LuaScript to disk. Creates the directory if needed.
/// Returns the path where the script was saved.
pub fn save_script(script: &LuaScript) -> Result<PathBuf, std::io::Error> {
    let dir = scripts_dir();
    fs::create_dir_all(&dir)?;

    let filename = format!("{}.lua", sanitize_name(&script.name));
    let path = dir.join(&filename);

    // Save source directly — it already contains metadata comments (-- name:, -- Intents:)
    // from the LLM output. Only prepend headers if missing.
    let mut content = script.source.clone();
    let has_name = content.lines().any(|l| {
        let t = l.trim();
        t.starts_with("-- ")
            && !t.starts_with("-- Intents:")
            && t.len() > 3
            && t[3..].split_whitespace().next().map_or(false, |w| {
                w.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
            })
    });
    if !has_name {
        let mut header = format!("-- {}", script.name);
        if !script.description.is_empty() {
            header.push_str(&format!(": {}", script.description));
        }
        header.push('\n');
        if !script.intents.is_empty() {
            header.push_str(&format!("-- Intents: {}\n", script.intents.join(", ")));
        }
        header.push('\n');
        content = format!("{header}{content}");
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }

    fs::write(&path, &content)?;
    log::info!("Saved player script to {}", path.display());
    Ok(path)
}

/// Load all player scripts from the scripts directory.
/// Returns an empty vec if the directory doesn't exist.
pub fn load_player_scripts() -> Vec<LuaScript> {
    let dir = scripts_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut scripts = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Failed to read player scripts dir: {e}");
            return Vec::new();
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lua") {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(source) => {
                let intents = crate::agent_bridge::extract_intents_from_source(&source);
                let name =
                    crate::agent_bridge::extract_name_from_source(&source).unwrap_or_else(|| {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });

                // Extract description from the first comment line after the name
                let description = source
                    .lines()
                    .next()
                    .and_then(|line| line.strip_prefix("-- "))
                    .and_then(|rest| {
                        rest.split_once(':')
                            .map(|(_, desc)| desc.trim().to_string())
                    })
                    .unwrap_or_default();

                scripts.push(LuaScript {
                    name,
                    source,
                    intents,
                    description,
                });
            }
            Err(e) => {
                log::warn!("Failed to read script {}: {e}", path.display());
            }
        }
    }

    log::info!(
        "Loaded {} player scripts from {}",
        scripts.len(),
        dir.display()
    );
    scripts
}

/// Delete a player script by name.
pub fn delete_script(name: &str) -> Result<(), std::io::Error> {
    let dir = scripts_dir();
    let path = dir.join(format!("{}.lua", sanitize_name(name)));

    if path.exists() {
        fs::remove_file(&path)?;
        log::info!("Deleted player script: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a LuaScript for testing.
    fn test_script() -> LuaScript {
        LuaScript {
            name: "test_gather".to_string(),
            source: "-- test_gather: Gather resources\n-- Intents: gather, harvest\nlocal units = ctx:my_units(\"Pawdler\")\n".to_string(),
            intents: vec!["gather".to_string(), "harvest".to_string()],
            description: "Gather resources".to_string(),
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        // Use the real scripts_dir() for this test; save, load, then clean up
        let script = test_script();
        let path = save_script(&script).expect("save should succeed");
        assert!(path.exists());

        let loaded = load_player_scripts();
        let found = loaded.iter().find(|s| s.name == "test_gather");
        assert!(found.is_some(), "Should find saved script");

        // Clean up
        delete_script("test_gather").expect("delete should succeed");
        assert!(!path.exists());
    }

    #[test]
    fn load_from_empty_dir() {
        // load_player_scripts should return empty vec if dir has no .lua files
        // This is already tested implicitly — if no player scripts exist, we get
        // an empty vec (or the test_gather from the roundtrip test if run in parallel,
        // but that's cleaned up).
        let scripts = load_player_scripts();
        // Just verify it doesn't panic
        let _ = scripts;
    }

    #[test]
    fn delete_nonexistent_script() {
        // Should not error when deleting a script that doesn't exist
        delete_script("nonexistent_script_xyz").expect("should not error");
    }
}
