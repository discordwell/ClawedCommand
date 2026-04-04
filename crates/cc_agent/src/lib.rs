#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod agent_bridge;
#[cfg(feature = "harness")]
pub mod arena;
pub mod behaviors;
pub mod construct_mode;
pub mod decision;
pub mod events;
pub mod llm_client;
pub mod mcp_tools;
pub mod script_registry;
pub mod snapshot;
pub mod spatial;
pub mod tool_tier;

pub mod script_context;
pub mod strait_bindings;

// Native-only modules (depend on mlua/tokio/crossbeam)
#[cfg(not(target_arch = "wasm32"))]
pub mod claude_cli;
#[cfg(not(target_arch = "wasm32"))]
pub mod llm_runner;
#[cfg(not(target_arch = "wasm32"))]
pub mod lua_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod runner;
#[cfg(not(target_arch = "wasm32"))]
pub mod script_persistence;

// WASM-only modules
#[cfg(target_arch = "wasm32")]
pub mod fallback_client;
#[cfg(target_arch = "wasm32")]
pub mod wasm_persistence;
#[cfg(target_arch = "wasm32")]
pub mod wasm_runner;
#[cfg(target_arch = "wasm32")]
pub mod webllm_client;

#[cfg(test)]
pub(crate) mod test_fixtures;

use bevy::prelude::*;

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<construct_mode::ConstructModeState>()
            .insert_resource(construct_mode::ScriptLibrary::with_starters())
            .init_resource::<agent_bridge::AgentChatLog>()
            .init_resource::<agent_bridge::UndoScriptActivation>()
            .init_resource::<tool_tier::ToolRegistry>()
            .init_resource::<tool_tier::FactionToolStates>()
            .init_resource::<decision::AgentDecisionState>()
            .init_resource::<llm_client::AgentStatus>()
            .init_resource::<script_registry::ScriptRegistry>();

        // Native: create connected bridge + channels, spawn LLM thread on Startup
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (bridge, channels) = agent_bridge::AgentBridge::new();
            app.insert_resource(bridge);
            app.insert_resource(llm_runner::LlmRunnerChannels(Some(channels)));
            app.insert_resource(llm_client::LlmConfig::from_env());
            app.add_plugins(runner::ScriptRunnerPlugin);
            app.add_systems(
                Startup,
                (llm_runner::startup_llm_runner, auto_register_saved_scripts),
            );
        }

        // WASM: dead-channel bridge (wasm_runner handles its own channels)
        #[cfg(target_arch = "wasm32")]
        {
            app.init_resource::<agent_bridge::AgentBridge>();
            app.add_systems(Startup, wasm_runner::init_wasm_agent);
        }

        app.add_systems(
            Update,
            (
                agent_bridge::poll_streaming_tokens,
                agent_bridge::poll_agent_responses.after(agent_bridge::poll_streaming_tokens),
                decision::agent_decision_system,
            ),
        )
        .add_systems(FixedUpdate, tool_tier::update_tool_tiers);
    }
}

/// Startup system: auto-register all scripts from ScriptLibrary into the ScriptRegistry.
/// Runs once at game start, after ScriptLibrary::with_starters() is inserted.
#[cfg(not(target_arch = "wasm32"))]
fn auto_register_saved_scripts(
    library: Res<construct_mode::ScriptLibrary>,
    mut registry: ResMut<script_registry::ScriptRegistry>,
) {
    for script in &library.scripts {
        registry.register_from_source(&script.source, &script.name, 0);
    }
    if !library.scripts.is_empty() {
        info!(
            "Auto-registered {} scripts from library",
            library.scripts.len()
        );
    }
}
