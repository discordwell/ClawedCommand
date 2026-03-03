//! WASM script persistence via localStorage.
//!
//! Mirrors the native `script_persistence.rs` API but uses the browser's
//! localStorage instead of the filesystem. Scripts are stored as JSON
//! under keys prefixed with `cc_script_`.

use wasm_bindgen::prelude::*;

use crate::construct_mode::LuaScript;

const KEY_PREFIX: &str = "cc_script_";

/// Stored script format for JSON serialization.
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredScript {
    name: String,
    source: String,
    intents: Vec<String>,
    description: String,
}

impl From<&LuaScript> for StoredScript {
    fn from(s: &LuaScript) -> Self {
        Self {
            name: s.name.clone(),
            source: s.source.clone(),
            intents: s.intents.clone(),
            description: s.description.clone(),
        }
    }
}

impl From<StoredScript> for LuaScript {
    fn from(s: StoredScript) -> Self {
        Self {
            name: s.name,
            source: s.source,
            intents: s.intents,
            description: s.description,
        }
    }
}

fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Save a LuaScript to localStorage.
pub fn save_script(script: &LuaScript) -> Result<(), String> {
    let storage = get_storage().ok_or("localStorage not available")?;
    let stored = StoredScript::from(script);
    let json = serde_json::to_string(&stored).map_err(|e| format!("serialize error: {e}"))?;
    let key = format!("{}{}", KEY_PREFIX, sanitize_name(&script.name));
    storage
        .set_item(&key, &json)
        .map_err(|_| "localStorage.setItem failed".to_string())?;
    log::info!("Saved script '{}' to localStorage", script.name);
    Ok(())
}

/// Load all player scripts from localStorage.
pub fn load_player_scripts() -> Vec<LuaScript> {
    let storage = match get_storage() {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut scripts = Vec::new();
    let len = storage.length().unwrap_or(0);

    for i in 0..len {
        if let Ok(Some(key)) = storage.key(i) {
            if key.starts_with(KEY_PREFIX) {
                if let Ok(Some(json)) = storage.get_item(&key) {
                    match serde_json::from_str::<StoredScript>(&json) {
                        Ok(stored) => scripts.push(LuaScript::from(stored)),
                        Err(e) => log::warn!("Failed to parse script {key}: {e}"),
                    }
                }
            }
        }
    }

    log::info!("Loaded {} player scripts from localStorage", scripts.len());
    scripts
}

/// Delete a script from localStorage by name.
pub fn delete_script(name: &str) {
    if let Some(storage) = get_storage() {
        let key = format!("{}{}", KEY_PREFIX, sanitize_name(name));
        let _ = storage.remove_item(&key);
    }
}

/// Sanitize a script name for use as a storage key.
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

// --- wasm_bindgen exports for JS access ---

/// JS-callable: list all saved scripts as a JSON array of {name, description}.
#[wasm_bindgen]
pub fn cc_list_scripts() -> String {
    let scripts = load_player_scripts();
    let entries: Vec<serde_json::Value> = scripts
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "description": s.description,
            })
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
}

/// JS-callable: get the Lua source of a saved script by name.
#[wasm_bindgen]
pub fn cc_get_script_source(name: &str) -> String {
    let storage = match get_storage() {
        Some(s) => s,
        None => return String::new(),
    };
    let key = format!("{}{}", KEY_PREFIX, sanitize_name(name));
    match storage.get_item(&key) {
        Ok(Some(json)) => serde_json::from_str::<StoredScript>(&json)
            .map(|s| s.source)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// JS-callable: delete a saved script by name.
#[wasm_bindgen]
pub fn cc_delete_script(name: &str) {
    delete_script(name);
}
